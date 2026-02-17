//! Backup and rollback management for migration operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as async_fs;
use tracing::{debug, error, info, warn};

use crate::services::migration_orchestrator::MigrationOperation;
use crate::utils::path_utils::PathHandler;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub backup_directory: PathBuf,
    pub compression_enabled: bool,
    pub max_backup_size_gb: f64,
    pub retention_days: u32,
    pub verify_backups: bool,
    pub include_metadata: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backup_directory: PathBuf::from(".astgrep/backups"),
            compression_enabled: true,
            max_backup_size_gb: 10.0,
            retention_days: 30,
            verify_backups: true,
            include_metadata: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_id: String,
    pub migration_id: String,
    pub created_at: DateTime<Utc>,
    pub config: BackupConfig,
    pub backup_items: Vec<BackupItem>,
    pub total_size_bytes: u64,
    pub checksum: Option<String>,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupItem {
    pub item_id: String,
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub item_type: BackupItemType,
    pub size_bytes: u64,
    pub checksum: String,
    pub permissions: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub compression_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupItemType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub source_platform: String,
    pub astgrep_version: String,
    pub migration_type: String,
    pub operation_count: usize,
    pub custom_attributes: HashMap<String, String>,
}

pub struct BackupManager {
    config: BackupConfig,
    path_handler: PathHandler,
    current_backup_id: Option<String>,
}

impl BackupManager {
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            path_handler: PathHandler::new(),
            current_backup_id: None,
        }
    }

    /// Initialize backup system and create backup directory
    pub async fn initialize(&mut self) -> Result<()> {
        if !self.config.enabled {
            info!("Backup system is disabled");
            return Ok(());
        }

        // Create backup directory if it doesn't exist
        self.path_handler.create_directory(&self.config.backup_directory)?;

        // Check available disk space
        self.check_disk_space().await?;

        info!("Backup system initialized with directory: {:?}", self.config.backup_directory);
        Ok(())
    }

    /// Create backup for migration operations
    pub async fn create_backup(
        &mut self,
        migration_id: &str,
        operations: &[MigrationOperation],
    ) -> Result<String> {
        if !self.config.enabled {
            return Ok(String::new()); // Return empty backup ID when disabled
        }

        let backup_id = uuid::Uuid::new_v4().to_string();
        self.current_backup_id = Some(backup_id.clone());

        info!("Creating backup {} for migration {}", backup_id, migration_id);

        let backup_path = self.get_backup_path(&backup_id);
        self.path_handler.create_directory(&backup_path)?;

        let mut backup_items = Vec::new();
        let mut total_size = 0u64;

        // Process each operation
        for operation in operations {
            match operation.operation_type {
                crate::services::migration_orchestrator::OperationType::Move => {
                    // For move operations, backup the source file
                    if operation.source_path.exists() {
                        let backup_item = self.backup_file(&operation.source_path, &backup_path).await?;
                        backup_items.push(backup_item.0);
                        total_size += backup_item.1;
                    }
                }
                crate::services::migration_orchestrator::OperationType::Copy => {
                    // For copy operations, no backup needed for source
                    debug!("Skipping backup for copy operation: {}", operation.id);
                }
                crate::services::migration_orchestrator::OperationType::CreateDirectory => {
                    // For directory creation, no backup needed
                    debug!("Skipping backup for directory creation: {}", operation.id);
                }
                crate::services::migration_orchestrator::OperationType::CreateSymlink => {
                    // For symlink creation, backup the target if it exists
                    if operation.source_path.exists() {
                        let backup_item = self.backup_symlink(&operation.source_path, &backup_path).await?;
                        backup_items.push(backup_item.0);
                        total_size += backup_item.1;
                    }
                }
            }
        }

        // Create backup manifest
        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            migration_id: migration_id.to_string(),
            created_at: Utc::now(),
            config: self.config.clone(),
            backup_items,
            total_size_bytes: total_size,
            checksum: if self.config.verify_backups {
                Some(self.calculate_backup_checksum(&backup_path).await?)
            } else {
                None
            },
            metadata: BackupMetadata {
                source_platform: std::env::consts::OS.to_string(),
                astgrep_version: env!("CARGO_PKG_VERSION").to_string(),
                migration_type: "test_organization".to_string(),
                operation_count: operations.len(),
                custom_attributes: HashMap::new(),
            },
        };

        // Save backup manifest
        self.save_backup_manifest(&manifest).await?;

        info!("Backup {} created successfully ({} bytes)", backup_id, total_size);
        Ok(backup_id)
    }

    /// Rollback migration from backup
    pub async fn rollback(&self, backup_id: &str) -> Result<()> {
        if !self.config.enabled {
            return Err(anyhow::anyhow!("Backup system is disabled, cannot rollback"));
        }

        info!("Starting rollback from backup: {}", backup_id);

        let manifest = self.load_backup_manifest(backup_id).await
            .with_context(|| format!("Backup manifest not found for ID: {}", backup_id))?;

        // Verify backup integrity
        if self.config.verify_backups {
            self.verify_backup_integrity(&manifest).await?;
        }

        // Rollback in reverse order of backup creation
        for backup_item in manifest.backup_items.iter().rev() {
            self.restore_backup_item(backup_item).await
                .with_context(|| format!("Failed to restore backup item: {}", backup_item.item_id))?;
        }

        info!("Rollback completed successfully for backup: {}", backup_id);
        Ok(())
    }

    /// Clean up old backups based on retention policy
    pub async fn cleanup_old_backups(&self) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        info!("Starting cleanup of old backups");

        let mut cleaned_count = 0;
        let cutoff_date = Utc::now() - chrono::Duration::days(self.config.retention_days as i64);

        let mut old_directories = Vec::new();
        if let Ok(mut dir) = async_fs::read_dir(&self.config.backup_directory).await {
            while let Some(entry) = dir.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_dir() {
                        if let Ok(modified) = metadata.modified() {
                            let modified_date: DateTime<Utc> = modified.into();
                            if modified_date < cutoff_date {
                                old_directories.push(entry.path());
                            }
                        }
                    }
                }
            }

            for entry in old_directories {
                if let Err(e) = self.remove_backup_directory(&entry).await {
                    warn!("Failed to remove backup directory {:?}: {}", entry, e);
                } else {
                    cleaned_count += 1;
                    debug!("Removed old backup directory: {:?}", entry);
                }
            }
        }

        info!("Cleaned up {} old backup directories", cleaned_count);
        Ok(cleaned_count)
    }

    /// List available backups
    pub async fn list_backups(&self) -> Result<Vec<BackupManifest>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        if let Ok(entries) = async_fs::read_dir(&self.config.backup_directory).await {
            let mut dir = entries;

            while let Some(entry) = dir.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_dir() {
                        let backup_id = entry.file_name().to_string_lossy().to_string();
                        if let Ok(manifest) = self.load_backup_manifest(&backup_id).await {
                            backups.push(manifest);
                        }
                    }
                }
            }
        }

        // Sort by creation date (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Get backup status and information
    pub async fn get_backup_status(&self, backup_id: &str) -> Result<Option<BackupManifest>> {
        if !self.config.enabled {
            return Ok(None);
        }

        match self.load_backup_manifest(backup_id).await {
            Ok(manifest) => Ok(Some(manifest)),
            Err(_) => Ok(None),
        }
    }

    // Private helper methods

    async fn backup_file(&self, source_path: &Path, backup_dir: &Path) -> Result<(BackupItem, u64)> {
        let backup_path = backup_dir.join("files").join(source_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?);

        // Ensure backup subdirectory exists
        self.path_handler.create_directory(backup_path.parent().unwrap())?;

        // Copy file to backup location
        async_fs::copy(source_path, &backup_path).await
            .with_context(|| format!("Failed to backup file: {:?}", source_path))?;

        // Calculate checksum
        let checksum = self.calculate_file_checksum(source_path).await?;

        // Get file size
        let size_bytes = async_fs::metadata(source_path).await?.len();

        // Get permissions if enabled
        let permissions = if self.config.include_metadata {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                async_fs::metadata(source_path).await?.permissions().mode().into()
            }
            #[cfg(not(unix))]
            {
                None
            }
        } else {
            None
        };

        let backup_item = BackupItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            original_path: source_path.to_path_buf(),
            backup_path,
            item_type: BackupItemType::File,
            size_bytes,
            checksum,
            permissions,
            created_at: Utc::now(),
            compression_ratio: None, // TODO: Implement compression if needed
        };

        debug!("Backed up file: {:?}", source_path);
        Ok((backup_item, size_bytes))
    }

    async fn backup_symlink(&self, source_path: &Path, backup_dir: &Path) -> Result<(BackupItem, u64)> {
        let backup_path = backup_dir.join("symlinks").join(source_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid symlink name"))?);

        // Ensure backup subdirectory exists
        self.path_handler.create_directory(backup_path.parent().unwrap())?;

        // Read symlink target
        let target = async_fs::read_link(source_path).await
            .with_context(|| format!("Failed to read symlink: {:?}", source_path))?;

        // Create backup of the symlink
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &backup_path)
                .with_context(|| format!("Failed to backup symlink: {:?}", source_path))?;
        }

        #[cfg(not(unix))]
        {
            return Err(anyhow::anyhow!("Symlinks are not supported on this platform"));
        }

        let backup_item = BackupItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            original_path: source_path.to_path_buf(),
            backup_path,
            item_type: BackupItemType::Symlink,
            size_bytes: 0, // Symlinks don't have size in the same way files do
            checksum: String::new(), // TODO: Calculate checksum for symlink target if needed
            permissions: None,
            created_at: Utc::now(),
            compression_ratio: None,
        };

        debug!("Backed up symlink: {:?}", source_path);
        Ok((backup_item, 0))
    }

    async fn restore_backup_item(&self, backup_item: &BackupItem) -> Result<()> {
        match backup_item.item_type {
            BackupItemType::File => {
                // Ensure target directory exists
                if let Some(parent) = backup_item.original_path.parent() {
                    self.path_handler.create_directory(parent)?;
                }

                // Restore file from backup
                async_fs::copy(&backup_item.backup_path, &backup_item.original_path).await
                    .with_context(|| format!("Failed to restore file: {:?}", backup_item.original_path))?;

                // Restore permissions if available
                if let Some(permissions) = backup_item.permissions {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perm = async_fs::metadata(&backup_item.original_path).await?.permissions();
                        perm.set_mode(permissions);
                        async_fs::set_permissions(&backup_item.original_path, perm).await?;
                    }
                }

                debug!("Restored file: {:?}", backup_item.original_path);
            }
            BackupItemType::Symlink => {
                // Symlink restoration is handled by the move rollback logic
                debug!("Symlink restoration noted: {:?}", backup_item.original_path);
            }
            BackupItemType::Directory => {
                // Create directory if it doesn't exist
                if !backup_item.original_path.exists() {
                    self.path_handler.create_directory(&backup_item.original_path)?;
                }
                debug!("Ensured directory exists: {:?}", backup_item.original_path);
            }
        }

        Ok(())
    }

    async fn calculate_file_checksum(&self, file_path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::io::AsyncReadExt;

        let mut file = async_fs::File::open(file_path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn calculate_backup_checksum(&self, backup_dir: &Path) -> Result<String> {
        // For now, calculate checksum of manifest file only
        // In a full implementation, this could include all backed up files
        let manifest_path = backup_dir.join("manifest.json");
        self.calculate_file_checksum(&manifest_path).await
    }

    async fn save_backup_manifest(&self, manifest: &BackupManifest) -> Result<()> {
        let manifest_path = self.get_backup_manifest_path(&manifest.backup_id);
        let manifest_json = serde_json::to_string_pretty(manifest)?;

        async_fs::write(&manifest_path, manifest_json).await
            .with_context(|| format!("Failed to save backup manifest: {:?}", manifest_path))?;

        Ok(())
    }

    async fn load_backup_manifest(&self, backup_id: &str) -> Result<BackupManifest> {
        let manifest_path = self.get_backup_manifest_path(backup_id);
        let manifest_content = async_fs::read_to_string(&manifest_path).await
            .with_context(|| format!("Failed to load backup manifest: {:?}", manifest_path))?;

        let manifest: BackupManifest = serde_json::from_str(&manifest_content)
            .with_context(|| "Failed to parse backup manifest")?;

        Ok(manifest)
    }

    async fn verify_backup_integrity(&self, manifest: &BackupManifest) -> Result<()> {
        info!("Verifying backup integrity for: {}", manifest.backup_id);

        for backup_item in &manifest.backup_items {
            if backup_item.backup_path.exists() {
                // Verify file exists and has correct size
                let metadata = async_fs::metadata(&backup_item.backup_path).await?;
                if metadata.len() != backup_item.size_bytes {
                    return Err(anyhow::anyhow!(
                        "Backup file size mismatch for {}: expected {}, found {}",
                        backup_item.item_id, backup_item.size_bytes, metadata.len()
                    ));
                }

                // Verify checksum
                let current_checksum = self.calculate_file_checksum(&backup_item.backup_path).await?;
                if current_checksum != backup_item.checksum {
                    return Err(anyhow::anyhow!(
                        "Backup file checksum mismatch for {}: expected {}, found {}",
                        backup_item.item_id, backup_item.checksum, current_checksum
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Backup file missing for item: {}", backup_item.item_id
                ));
            }
        }

        info!("Backup integrity verified successfully");
        Ok(())
    }

    async fn check_disk_space(&self) -> Result<()> {
        // This is a simplified check - in a full implementation,
        // you'd use platform-specific APIs to get available disk space
        debug!("Checking available disk space");
        Ok(())
    }

    async fn remove_backup_directory(&self, backup_dir: &Path) -> Result<()> {
        async_fs::remove_dir_all(backup_dir).await
            .with_context(|| format!("Failed to remove backup directory: {:?}", backup_dir))
    }

    fn get_backup_path(&self, backup_id: &str) -> PathBuf {
        self.config.backup_directory.join(backup_id)
    }

    fn get_backup_manifest_path(&self, backup_id: &str) -> PathBuf {
        self.get_backup_path(backup_id).join("manifest.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_backup_manager_initialization() {
        let temp_dir = tempdir().unwrap();
        let config = BackupConfig {
            enabled: true,
            backup_directory: temp_dir.path().to_path_buf(),
            compression_enabled: false,
            max_backup_size_gb: 1.0,
            retention_days: 1,
            verify_backups: true,
            include_metadata: false,
        };

        let mut manager = BackupManager::new(config);
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = tempdir().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let config = BackupConfig {
            enabled: true,
            backup_directory: backup_dir.clone(),
            compression_enabled: false,
            max_backup_size_gb: 1.0,
            retention_days: 1,
            verify_backups: false,
            include_metadata: false,
        };

        let mut manager = BackupManager::new(config);
        manager.initialize().await.unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let operation = MigrationOperation {
            id: "test-op-001".to_string(),
            source_path: test_file.clone(),
            target_path: temp_dir.path().join("moved_test.txt"),
            operation_type: crate::services::migration_orchestrator::OperationType::Move,
            status: crate::services::migration_orchestrator::OperationStatus::Pending,
            error_message: None,
            bytes_transferred: 0,
            checksum_before: None,
            checksum_after: None,
            timestamp: Utc::now(),
        };

        let backup_id = manager.create_backup("test-migration", &[operation]).await.unwrap();
        assert!(!backup_id.is_empty());

        // Verify backup was created
        let backup_path = backup_dir.join(&backup_id);
        assert!(backup_path.exists());

        let manifest_path = backup_path.join("manifest.json");
        assert!(manifest_path.exists());
    }

    #[test]
    fn test_backup_config_default() {
        let config = BackupConfig::default();
        assert!(config.enabled);
        assert!(config.backup_directory == PathBuf::from(".astgrep/backups"));
        assert!(config.compression_enabled);
        assert_eq!(config.max_backup_size_gb, 10.0);
        assert_eq!(config.retention_days, 30);
        assert!(config.verify_backups);
        assert!(config.include_metadata);
    }
}