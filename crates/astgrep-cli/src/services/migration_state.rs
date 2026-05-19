//! Migration state management and persistence

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::info;

use super::migration_orchestrator::{MigrationOperation, OperationStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationState {
    pub migration_id: String,
    pub status: MigrationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub operations: Vec<MigrationOperation>,
    pub metadata: MigrationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStatus {
    Initializing,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    RollingBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetadata {
    pub total_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub bytes_transferred: u64,
    pub estimated_duration_seconds: u64,
    pub actual_duration_seconds: Option<u64>,
    pub categories: Vec<String>,
    pub languages: Vec<String>,
}

impl MigrationState {
    pub fn new(migration_id: String) -> Self {
        let now = Utc::now();
        Self {
            migration_id,
            status: MigrationStatus::Initializing,
            created_at: now,
            updated_at: now,
            completed_at: None,
            operations: Vec::new(),
            metadata: MigrationMetadata {
                total_operations: 0,
                completed_operations: 0,
                failed_operations: 0,
                bytes_transferred: 0,
                estimated_duration_seconds: 0,
                actual_duration_seconds: None,
                categories: Vec::new(),
                languages: Vec::new(),
            },
        }
    }

    /// Add operations to the migration state
    pub fn add_operations(&mut self, operations: Vec<MigrationOperation>) {
        self.operations.extend(operations);
        self.metadata.total_operations = self.operations.len();
        self.updated_at = Utc::now();
    }

    /// Update operation status
    pub fn update_operation(&mut self, operation_id: &str, status: OperationStatus) -> Result<()> {
        if let Some(operation) = self.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.status = status.clone();
            operation.timestamp = Utc::now();

            // Update metadata counters
            self.update_metadata_counters();
            self.updated_at = Utc::now();

            Ok(())
        } else {
            Err(anyhow::anyhow!("Operation {} not found", operation_id))
        }
    }

    /// Update operation with bytes transferred
    pub fn update_operation_bytes_transferred(
        &mut self,
        operation_id: &str,
        bytes: u64,
    ) -> Result<()> {
        if let Some(operation) = self.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.bytes_transferred = bytes;
            operation.timestamp = Utc::now();

            // Update metadata
            self.metadata.bytes_transferred = self.operations
                .iter()
                .map(|op| op.bytes_transferred)
                .sum();
            self.updated_at = Utc::now();

            Ok(())
        } else {
            Err(anyhow::anyhow!("Operation {} not found", operation_id))
        }
    }

    /// Mark operation as failed with error message
    pub fn mark_operation_failed(&mut self, operation_id: &str, error: &str) -> Result<()> {
        if let Some(operation) = self.operations.iter_mut().find(|op| op.id == operation_id) {
            operation.status = OperationStatus::Failed;
            operation.error_message = Some(error.to_string());
            operation.timestamp = Utc::now();

            self.update_metadata_counters();
            self.updated_at = Utc::now();

            Ok(())
        } else {
            Err(anyhow::anyhow!("Operation {} not found", operation_id))
        }
    }

    /// Set migration status to in progress
    pub fn start_migration(&mut self) {
        self.status = MigrationStatus::InProgress;
        self.updated_at = Utc::now();
    }

    /// Set migration status to completed
    pub fn complete_migration(&mut self) {
        self.status = MigrationStatus::Completed;
        self.completed_at = Some(Utc::now());

        if let Some(completed_at) = self.completed_at {
            let duration = completed_at.signed_duration_since(self.created_at);
            self.metadata.actual_duration_seconds = Some(duration.num_seconds() as u64);
        }

        self.updated_at = Utc::now();
    }

    /// Set migration status to failed
    pub fn fail_migration(&mut self) {
        self.status = MigrationStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Set migration status to cancelled
    pub fn cancel_migration(&mut self) {
        self.status = MigrationStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Set migration status to rolling back
    pub fn start_rollback(&mut self) {
        self.status = MigrationStatus::RollingBack;
        self.updated_at = Utc::now();
    }

    /// Get operations pending execution
    pub fn get_pending_operations(&self) -> Vec<&MigrationOperation> {
        self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Pending))
            .collect()
    }

    /// Get operations in progress
    pub fn get_in_progress_operations(&self) -> Vec<&MigrationOperation> {
        self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::InProgress))
            .collect()
    }

    /// Get completed operations
    pub fn get_completed_operations(&self) -> Vec<&MigrationOperation> {
        self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Completed))
            .collect()
    }

    /// Get failed operations
    pub fn get_failed_operations(&self) -> Vec<&MigrationOperation> {
        self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Failed))
            .collect()
    }

    /// Calculate progress percentage
    pub fn progress_percentage(&self) -> f64 {
        if self.metadata.total_operations == 0 {
            return 0.0;
        }

        (self.metadata.completed_operations as f64 / self.metadata.total_operations as f64) * 100.0
    }

    /// Check if migration is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, MigrationStatus::Completed)
    }

    /// Check if migration has failures
    pub fn has_failures(&self) -> bool {
        self.metadata.failed_operations > 0
    }

    /// Update metadata counters
    fn update_metadata_counters(&mut self) {
        self.metadata.completed_operations = self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Completed))
            .count();

        self.metadata.failed_operations = self.operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Failed))
            .count();
    }
}

/// Migration state manager for persistence and loading
pub struct MigrationStateManager {
    storage_path: PathBuf,
}

impl MigrationStateManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self { storage_path }
    }

    /// Save migration state to disk
    pub async fn save_state(&self, state: &MigrationState) -> Result<()> {
        let filename = format!("migration_{}.json", state.migration_id);
        let file_path = self.storage_path.join(filename);

        // Ensure storage directory exists
        fs::create_dir_all(&self.storage_path).await
            .with_context(|| format!("Failed to create storage directory: {}", self.storage_path.display()))?;

        // Serialize and write state
        let json_content = serde_json::to_string_pretty(state)
            .with_context(|| "Failed to serialize migration state")?;

        let mut file = fs::File::create(&file_path).await
            .with_context(|| format!("Failed to create state file: {}", file_path.display()))?;

        file.write_all(json_content.as_bytes()).await
            .with_context(|| format!("Failed to write state file: {}", file_path.display()))?;

        info!("Saved migration state: {}", state.migration_id);
        Ok(())
    }

    /// Load migration state from disk
    pub async fn load_state(&self, migration_id: &str) -> Result<Option<MigrationState>> {
        let filename = format!("migration_{}.json", migration_id);
        let file_path = self.storage_path.join(filename);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path).await
            .with_context(|| format!("Failed to read state file: {}", file_path.display()))?;

        let state: MigrationState = serde_json::from_str(&content)
            .with_context(|| "Failed to deserialize migration state")?;

        Ok(Some(state))
    }

    /// List all available migration states
    pub async fn list_migrations(&self) -> Result<Vec<String>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.storage_path).await
            .with_context(|| format!("Failed to read storage directory: {}", self.storage_path.display()))?;

        let mut migration_ids = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {

            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with("migration_") && file_name.ends_with(".json") {
                // Extract migration ID from filename
                let migration_id = file_name
                    .strip_prefix("migration_")
                    .unwrap_or("")
                    .strip_suffix(".json")
                    .unwrap_or("");

                if !migration_id.is_empty() {
                    migration_ids.push(migration_id.to_string());
                }
            }
        }

        migration_ids.sort(); // Sort for consistent ordering
        Ok(migration_ids)
    }

    /// Delete migration state
    pub async fn delete_state(&self, migration_id: &str) -> Result<()> {
        let filename = format!("migration_{}.json", migration_id);
        let file_path = self.storage_path.join(filename);

        if file_path.exists() {
            fs::remove_file(&file_path).await
                .with_context(|| format!("Failed to delete state file: {}", file_path.display()))?;

            info!("Deleted migration state: {}", migration_id);
        }

        Ok(())
    }

    /// Clean up old migration states
    pub async fn cleanup_old_states(&self, max_age_days: i64) -> Result<usize> {
        if !self.storage_path.exists() {
            return Ok(0);
        }

        let mut deleted_count = 0;
        let cutoff_time = Utc::now() - chrono::Duration::days(max_age_days);
        let mut entries = fs::read_dir(&self.storage_path).await
            .with_context(|| format!("Failed to read storage directory: {}", self.storage_path.display()))?;

        while let Ok(Some(entry)) = entries.next_entry().await {

            let metadata = entry.metadata().await
                .with_context(|| "Failed to read file metadata")?;

            if let Ok(modified_time) = metadata.modified() {
                let file_age: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(modified_time);

                if file_age < cutoff_time {
                    let file_path = entry.path();
                    if file_path.exists() {
                        fs::remove_file(&file_path).await
                            .with_context(|| format!("Failed to delete old state file: {}", file_path.display()))?;
                        deleted_count += 1;
                        info!("Deleted old migration state: {:?}", file_path.file_name());
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// Generate migration summary report
    pub async fn generate_summary(&self, migration_id: &str) -> Result<String> {
        if let Some(state) = self.load_state(migration_id).await? {
            let mut summary = format!(
                "# Migration Summary: {}\n\n",
                state.migration_id
            );

            summary.push_str(&format!(
                "**Status**: {}\n",
                match state.status {
                    MigrationStatus::Initializing => "Initializing",
                    MigrationStatus::InProgress => "In Progress",
                    MigrationStatus::Completed => "Completed",
                    MigrationStatus::Failed => "Failed",
                    MigrationStatus::Cancelled => "Cancelled",
                    MigrationStatus::RollingBack => "Rolling Back",
                }
            ));

            summary.push_str(&format!(
                "**Created**: {}\n",
                state.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            ));

            if let Some(completed_at) = state.completed_at {
                summary.push_str(&format!(
                    "**Completed**: {}\n",
                    completed_at.format("%Y-%m-%d %H:%M:%S UTC")
                ));
            }

            summary.push_str("\n## Statistics\n");
            summary.push_str(&format!("- **Total Operations**: {}\n", state.metadata.total_operations));
            summary.push_str(&format!("- **Completed**: {}\n", state.metadata.completed_operations));
            summary.push_str(&format!("- **Failed**: {}\n", state.metadata.failed_operations));
            summary.push_str(&format!("- **Progress**: {:.1}%\n", state.progress_percentage()));
            summary.push_str(&format!("- **Bytes Transferred**: {}\n", state.metadata.bytes_transferred));

            if let Some(actual_duration) = state.metadata.actual_duration_seconds {
                summary.push_str(&format!("- **Duration**: {} seconds\n", actual_duration));
            }

            if !state.metadata.categories.is_empty() {
                summary.push_str(&format!("- **Categories**: {}\n", state.metadata.categories.join(", ")));
            }

            if !state.metadata.languages.is_empty() {
                summary.push_str(&format!("- **Languages**: {}\n", state.metadata.languages.join(", ")));
            }

            summary.push_str("\n## Failed Operations\n");
            let failed_ops = state.get_failed_operations();
            if failed_ops.is_empty() {
                summary.push_str("No failed operations.\n");
            } else {
                for operation in failed_ops {
                    if let Some(error) = &operation.error_message {
                        summary.push_str(&format!(
                            "- **{}**: {}\n",
                            operation.id,
                            error
                        ));
                    }
                }
            }

            Ok(summary)
        } else {
            Err(anyhow::anyhow!("Migration {} not found", migration_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::migration_orchestrator::OperationType;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_migration_state_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let state_manager = MigrationStateManager::new(temp_dir.path().to_path_buf());

        let migration_id = "test-001".to_string();
        let mut state = MigrationState::new(migration_id.clone());

        // Add test operations
        state.add_operations(vec![
            MigrationOperation {
                id: "op-001".to_string(),
                source_path: "/source/file1".into(),
                target_path: "/target/file1".into(),
                operation_type: OperationType::Copy,
                status: OperationStatus::Pending,
                error_message: None,
                bytes_transferred: 0,
                checksum_before: None,
                checksum_after: None,
                timestamp: Utc::now(),
            },
            MigrationOperation {
                id: "op-002".to_string(),
                source_path: "/source/file2".into(),
                target_path: "/target/file2".into(),
                operation_type: OperationType::Move,
                status: OperationStatus::Pending,
                error_message: None,
                bytes_transferred: 0,
                checksum_before: None,
                checksum_after: None,
                timestamp: Utc::now(),
            },
        ]);

        // Save state
        state_manager.save_state(&state).await.unwrap();

        // Load state
        let mut loaded_state = state_manager.load_state(&migration_id).await.unwrap().unwrap();

        assert_eq!(loaded_state.migration_id, migration_id);
        assert_eq!(loaded_state.operations.len(), 2);
        assert_eq!(loaded_state.metadata.total_operations, 2);

        // Test updates
        loaded_state.start_migration();
        assert!(matches!(loaded_state.status, MigrationStatus::InProgress));

        loaded_state.update_operation("op-001", OperationStatus::Completed).unwrap();
        assert_eq!(loaded_state.metadata.completed_operations, 1);

        // Delete state
        state_manager.delete_state(&migration_id).await.unwrap();

        // Verify deletion
        let deleted_state = state_manager.load_state(&migration_id).await.unwrap();
        assert!(deleted_state.is_none());
    }

    #[tokio::test]
    async fn test_migration_state_list() {
        let temp_dir = tempdir().unwrap();
        let state_manager = MigrationStateManager::new(temp_dir.path().to_path_buf());

        // Create some test states
        for i in 1..=3 {
            let migration_id = format!("test-{:03}", i);
            let state = MigrationState::new(migration_id);
            state_manager.save_state(&state).await.unwrap();
        }

        // List migrations
        let migrations = state_manager.list_migrations().await.unwrap();
        assert_eq!(migrations.len(), 3);
        assert!(migrations.contains(&"test-001".to_string()));
        assert!(migrations.contains(&"test-002".to_string()));
        assert!(migrations.contains(&"test-003".to_string()));
    }
}