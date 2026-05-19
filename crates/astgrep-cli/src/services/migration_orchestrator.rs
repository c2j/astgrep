//! Migration orchestrator for file system operations with rsync support

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, info, warn};
use chrono::{DateTime, Utc};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOperation {
    pub id: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    pub error_message: Option<String>,
    pub bytes_transferred: u64,
    pub checksum_before: Option<String>,
    pub checksum_after: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Copy,
    Move,
    CreateDirectory,
    CreateSymlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

pub struct MigrationOrchestrator {
    dry_run: bool,
    preserve_timestamps: bool,
    create_backups: bool,
    parallel_jobs: usize,
}

impl MigrationOrchestrator {
    pub fn new(
        dry_run: bool,
        preserve_timestamps: bool,
        create_backups: bool,
        parallel_jobs: usize,
    ) -> Self {
        Self {
            dry_run,
            preserve_timestamps,
            create_backups,
            parallel_jobs,
        }
    }

    /// Execute migration operations using rsync with optimized parameters
    pub async fn execute_migration(
        &self,
        operations: Vec<MigrationOperation>,
    ) -> Result<Vec<MigrationOperation>> {
        info!("Starting migration execution with {} operations", operations.len());

        if self.dry_run {
            info!("DRY RUN MODE: No actual file operations will be performed");
            return self.execute_dry_run(operations).await;
        }

        let mut completed_operations = Vec::new();
        let progress_bar = self.create_progress_bar(operations.len() as u64);

        for (index, mut operation) in operations.into_iter().enumerate() {
            progress_bar.set_message(format!("Processing {}", operation.id));
            progress_bar.set_position(index as u64);

            operation.status = OperationStatus::InProgress;
            operation.timestamp = Utc::now();

            let result = match operation.operation_type {
                OperationType::Copy | OperationType::Move => {
                    self.execute_rsync_operation(&operation).await
                }
                OperationType::CreateDirectory => {
                    self.create_directory(&operation).await
                }
                OperationType::CreateSymlink => {
                    self.create_symlink(&operation).await
                }
            };

            match result {
                Ok(bytes_transferred) => {
                    operation.status = OperationStatus::Completed;
                    operation.bytes_transferred = bytes_transferred;
                    debug!("Successfully completed operation: {}", operation.id);
                }
                Err(e) => {
                    operation.status = OperationStatus::Failed;
                    operation.error_message = Some(e.to_string());
                    error!("Failed operation {}: {}", operation.id, e);
                }
            }

            progress_bar.inc(1); // progress bar increment

            completed_operations.push(operation);
        }

        progress_bar.finish_with_message("Migration completed");
        info!("Migration execution completed: {} operations processed", completed_operations.len());

        Ok(completed_operations)
    }

    async fn execute_dry_run(
        &self,
        operations: Vec<MigrationOperation>,
    ) -> Result<Vec<MigrationOperation>> {
        let mut completed_operations = Vec::new();

        for mut operation in operations {
            operation.status = if self.validate_operation(&operation) {
                OperationStatus::Completed
            } else {
                OperationStatus::Failed
            };
            operation.timestamp = Utc::now();
            operation.bytes_transferred = 0; // Dry run - no actual transfer

            info!("DRY RUN: Would {} from {} to {}",
                match operation.operation_type {
                    OperationType::Copy => "copy",
                    OperationType::Move => "move",
                    OperationType::CreateDirectory => "create directory",
                    OperationType::CreateSymlink => "create symlink",
                },
                operation.source_path.display(),
                operation.target_path.display()
            );

            completed_operations.push(operation);
        }

        Ok(completed_operations)
    }

    /// Execute rsync operation with optimized parameters for large-scale file operations
    async fn execute_rsync_operation(
        &self,
        operation: &MigrationOperation,
    ) -> Result<u64> {
        let source = &operation.source_path;
        let target = &operation.target_path;

        // Ensure target directory exists
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create target directory: {}", parent.display()))?;
        }

        // Build rsync command with optimized parameters
        let mut cmd = TokioCommand::new("rsync");

        // rsync options for optimized large-scale migration
        cmd.arg("-a")                   // Archive mode (preserves permissions, timestamps, etc.)
            .arg("-h")                   // Human-readable numbers
            .arg("--progress")           // Show progress per file
            .arg("--stats")              // Show transfer statistics
            .arg("--no-i-r")             // Don't skip directories based on inode
            .arg("--delete")             // Delete extraneous files from dest dirs (for moves)
            .arg("--exclude='.*'")       // Exclude hidden files
            .arg("--exclude='.DS_Store'") // Exclude macOS metadata
            .arg("--exclude='Thumbs.db'"); // Exclude Windows thumbnails

        if self.preserve_timestamps {
            cmd.arg("-t"); // Preserve modification times
        }

        match operation.operation_type {
            OperationType::Move => {
                cmd.arg("--remove-source-files"); // Delete source files after successful transfer
            }
            OperationType::Copy => {
                // No additional flags needed for copy
            }
            _ => return Err(anyhow::anyhow!("Invalid operation type for rsync")),
        }

        cmd.arg(source.to_str().unwrap_or_default())
            .arg(target.to_str().unwrap_or_default());

        // Execute rsync and capture output
        let output = cmd.output().await
            .with_context(|| format!("Failed to execute rsync command: {}", format!("{:?}", cmd)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("rsync failed: {}", error_msg));
        }

        // Parse rsync output for statistics
        let stats_output = String::from_utf8_lossy(&output.stderr);
        let bytes_transferred = self.parse_rsync_stats(&stats_output)?;

        info!("Successfully rsynced {} bytes from {} to {}",
              bytes_transferred, source.display(), target.display());

        Ok(bytes_transferred)
    }

    /// Create directory with proper permissions
    async fn create_directory(&self, operation: &MigrationOperation) -> Result<u64> {
        tokio::fs::create_dir_all(&operation.target_path).await
            .with_context(|| format!("Failed to create directory: {}", operation.target_path.display()))?;

        info!("Created directory: {}", operation.target_path.display());
        Ok(0)
    }

    /// Create symbolic link with error handling
    async fn create_symlink(&self, operation: &MigrationOperation) -> Result<u64> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                &operation.source_path,
                &operation.target_path,
            ).with_context(|| {
                format!("Failed to create symlink from {} to {}",
                       operation.source_path.display(),
                       operation.target_path.display())
            })?;

            info!("Created symlink: {} -> {}",
                  operation.target_path.display(),
                  operation.source_path.display());
        }

        #[cfg(not(unix))]
        {
            return Err(anyhow::anyhow!("Symlinks are not supported on this platform"));
        }

        Ok(0)
    }

    /// Validate that an operation can be performed safely
    fn validate_operation(&self, operation: &MigrationOperation) -> bool {
        match operation.operation_type {
            OperationType::Copy | OperationType::Move => {
                operation.source_path.exists()
            }
            OperationType::CreateDirectory => {
                true // Can always validate directory creation
            }
            OperationType::CreateSymlink => {
                operation.source_path.exists()
            }
        }
    }

    /// Parse rsync statistics output to extract bytes transferred
    fn parse_rsync_stats(&self, output: &str) -> Result<u64> {
        // Look for pattern like: "sent 1,234,567 bytes  received 35 bytes 1,234,602 bytes/sec"
        let lines: Vec<&str> = output.lines().collect();

        for line in lines.iter().rev() { // Check from bottom for final stats
            if line.contains("sent") && line.contains("bytes") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "sent" && i + 1 < parts.len() {
                        if let Ok(bytes) = parts[i + 1].replace(',', "").parse::<u64>() {
                            return Ok(bytes);
                        }
                    }
                }
            }
        }

        // If we can't parse the stats, default to 0 (this shouldn't happen in normal operation)
        warn!("Could not parse rsync statistics from output: {}", output);
        Ok(0)
    }

    /// Create progress bar for migration operations
    fn create_progress_bar(&self, total: u64) -> ProgressBar {
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-");

        ProgressBar::new(total)
            .with_style(style)
            .with_message("Initializing migration...")
    }

    /// Generate checksum for file integrity verification
    pub async fn calculate_checksum(&self, file_path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(file_path).await
            .with_context(|| format!("Failed to open file for checksum: {}", file_path.display()))?;

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

    /// Verify file integrity after migration
    pub async fn verify_integrity(
        &self,
        source_path: &Path,
        target_path: &Path,
    ) -> Result<bool> {
        let source_checksum = self.calculate_checksum(source_path).await?;
        let target_checksum = self.calculate_checksum(target_path).await?;

        Ok(source_checksum == target_checksum)
    }

    /// Rollback failed migration operations
    pub async fn rollback_operations(
        &self,
        operations: &[MigrationOperation],
    ) -> Result<()> {
        info!("Starting rollback of {} operations", operations.len());

        for operation in operations.iter().rev() {
            match operation.operation_type {
                OperationType::Move => {
                    // Move target back to source
                    if operation.target_path.exists() {
                        self.move_file(&operation.target_path, &operation.source_path).await
                            .with_context(|| format!("Failed to rollback move operation: {}", operation.id))?;
                    }
                }
                OperationType::CreateSymlink => {
                    // Remove symlink
                    if operation.target_path.exists() {
                        tokio::fs::remove_file(&operation.target_path).await
                            .with_context(|| format!("Failed to remove symlink during rollback: {}", operation.id))?;
                    }
                }
                OperationType::CreateDirectory => {
                    // Remove directory if empty
                    if operation.target_path.exists() && operation.target_path.read_dir()?.next().is_none() {
                        tokio::fs::remove_dir(&operation.target_path).await
                            .with_context(|| format!("Failed to remove directory during rollback: {}", operation.id))?;
                    }
                }
                _ => {} // No rollback needed for copy operations
            }

            debug!("Rolled back operation: {}", operation.id);
        }

        info!("Rollback completed successfully");
        Ok(())
    }

    /// Move file with error handling
    async fn move_file(&self, from: &Path, to: &Path) -> Result<()> {
        tokio::fs::rename(from, to).await
            .with_context(|| format!("Failed to move file from {} to {}", from.display(), to.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::io::Write;

    #[tokio::test]
    async fn test_checksum_calculation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);

        let orchestrator = MigrationOrchestrator::new(false, true, false, 1);
        let checksum = orchestrator.calculate_checksum(&file_path).await.unwrap();

        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // SHA256 hash length
    }

    #[tokio::test]
    async fn test_directory_creation() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");

        let operation = MigrationOperation {
            id: "test-001".to_string(),
            source_path: temp_dir.path().to_path_buf(),
            target_path: dir_path.clone(),
            operation_type: OperationType::CreateDirectory,
            status: OperationStatus::Pending,
            error_message: None,
            bytes_transferred: 0,
            checksum_before: None,
            checksum_after: None,
            timestamp: Utc::now(),
        };

        let orchestrator = MigrationOrchestrator::new(false, true, false, 1);
        orchestrator.create_directory(&operation).await.unwrap();

        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }
}