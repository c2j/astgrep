//! Script migration engine for reorganizing test scripts

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::{
    backup::BackupManager,
    output::{OutputFormat, OutputFormatter},
    progress::{ProgressConfig, ProgressTracker},
    services::{
        migration_orchestrator::{
            MigrationOperation, MigrationOrchestrator, OperationStatus, OperationType,
        },
        migration_state::MigrationState,
    },
    utils::path_utils::PathHandler,
    validation::{MigrationValidator, ValidationConfig},
};

use astgrep_core::models::test_asset::ScriptType;
use astgrep_core::models::test_asset::{AssetType, TestAsset};
use astgrep_matcher::script_classifier::ScriptClassifier;
use astgrep_parser::script_discovery::{DiscoveryConfig, ScriptDiscovery};

/// Configuration for script migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMigrationConfig {
    /// Source directories to scan for scripts
    pub source_directories: Vec<PathBuf>,
    /// Target base directory for organized scripts
    pub target_directory: PathBuf,
    /// Enable dry run mode (no actual file operations)
    pub dry_run: bool,
    /// Create backups before migration
    pub create_backups: bool,
    /// Validate migration after completion
    pub validate_after_migration: bool,
    /// Preserve original file permissions
    pub preserve_permissions: bool,
    /// Preserve original file timestamps
    pub preserve_timestamps: bool,
    /// Enable progress reporting
    pub enable_progress: bool,
    /// Output format for reports
    pub output_format: OutputFormat,
    /// Number of parallel migration jobs
    pub parallel_jobs: usize,
    /// Include hidden files
    pub include_hidden: bool,
    /// Override existing files in target
    pub force_overwrite: bool,
}

impl Default for ScriptMigrationConfig {
    fn default() -> Self {
        Self {
            source_directories: vec![
                PathBuf::from("tests"),
                PathBuf::from("test"),
                PathBuf::from("scripts"),
            ],
            target_directory: PathBuf::from("newtest/scripts"),
            dry_run: false,
            create_backups: true,
            validate_after_migration: true,
            preserve_permissions: true,
            preserve_timestamps: true,
            enable_progress: true,
            output_format: OutputFormat::Human,
            parallel_jobs: 4,
            include_hidden: false,
            force_overwrite: false,
        }
    }
}

/// Migration result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub migration_id: String,
    pub total_scripts_found: usize,
    pub scripts_migrated: usize,
    pub scripts_failed: usize,
    pub scripts_skipped: usize,
    pub directories_created: usize,
    pub total_bytes_migrated: u64,
    pub migration_time: std::time::Duration,
    pub validation_passed: Option<bool>,
    pub backup_id: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Script classification mapping
#[derive(Debug, Clone)]
struct ScriptMapping {
    /// Original path of the script
    original_path: PathBuf,
    /// Target path in the organized structure
    target_path: PathBuf,
    /// Script type classification
    script_type: ScriptType,
    /// Classification confidence
    confidence: f64,
}

/// Script migration engine
pub struct ScriptMigrator {
    config: ScriptMigrationConfig,
    path_handler: PathHandler,
    script_discovery: ScriptDiscovery,
    script_classifier: ScriptClassifier,
    migration_orchestrator: MigrationOrchestrator,
    backup_manager: BackupManager,
    migration_validator: MigrationValidator,
    output_formatter: OutputFormatter,
    progress_tracker: Option<ProgressTracker>,
    migration_state: MigrationState,
}

impl ScriptMigrator {
    /// Create a new script migrator with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(ScriptMigrationConfig::default())
    }

    /// Create a script migrator with custom configuration
    pub fn with_config(config: ScriptMigrationConfig) -> Result<Self> {
        let discovery_config = DiscoveryConfig {
            search_paths: config.source_directories.clone(),
            include_hidden: config.include_hidden,
            ..Default::default()
        };

        let path_handler = PathHandler::new();
        let script_discovery = ScriptDiscovery::with_config(discovery_config);
        let script_classifier = ScriptClassifier::new();

        let migration_orchestrator = MigrationOrchestrator::new(
            config.dry_run,
            config.preserve_timestamps,
            config.create_backups,
            config.parallel_jobs,
        );

        let backup_config = crate::backup::BackupConfig::default();
        let backup_manager = BackupManager::new(backup_config);

        let validation_config = ValidationConfig::default();
        let migration_validator = MigrationValidator::new(validation_config);

        let output_formatter = OutputFormatter::new(
            config.output_format.clone(),
            true, // Use colors
            true, // Verbose output
        );

        let progress_tracker = if config.enable_progress {
            let progress_config = ProgressConfig::default();
            Some(ProgressTracker::new(progress_config))
        } else {
            None
        };

        let migration_id = uuid::Uuid::new_v4().to_string();
        let migration_state = MigrationState::new(migration_id.clone());

        Ok(Self {
            config,
            path_handler,
            script_discovery,
            script_classifier,
            migration_orchestrator,
            backup_manager,
            migration_validator,
            output_formatter,
            progress_tracker,
            migration_state,
        })
    }

    /// Execute the complete script migration process
    pub async fn migrate_scripts(&mut self) -> Result<MigrationResult> {
        let start_time = std::time::Instant::now();
        info!(
            "Starting script migration from {:?} to {:?}",
            self.config.source_directories, self.config.target_directory
        );

        // Initialize migration state
        self.migration_state.start_migration();

        // Initialize components
        if let Some(ref mut tracker) = self.progress_tracker {
            tracker.initialize_migration(self.migration_state.migration_id.clone(), 0)?;
        }

        self.backup_manager.initialize().await?;

        // Step 1: Discover scripts
        let discovered_scripts = self.discover_scripts().await?;
        info!(
            "Discovered {} scripts",
            discovered_scripts.total_scripts_found
        );

        if discovered_scripts.total_scripts_found == 0 {
            return Ok(MigrationResult {
                migration_id: self.migration_state.migration_id.clone(),
                total_scripts_found: 0,
                scripts_migrated: 0,
                scripts_failed: 0,
                scripts_skipped: 0,
                directories_created: 0,
                total_bytes_migrated: 0,
                migration_time: start_time.elapsed(),
                validation_passed: None,
                backup_id: None,
                errors: vec!["No scripts found to migrate".to_string()],
                warnings: Vec::new(),
            });
        }

        // Step 2: Classify and map scripts
        let script_mappings = self.classify_and_map_scripts(&discovered_scripts).await?;
        info!("Classified and mapped {} scripts", script_mappings.len());

        // Step 3: Create backup if enabled
        let backup_id = if self.config.create_backups {
            let _assets: Vec<TestAsset> = script_mappings
                .iter()
                .map(|mapping| {
                    TestAsset::new(
                        format!("script-{}", uuid::Uuid::new_v4()),
                        mapping
                            .original_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        AssetType::Script,
                        mapping.original_path.clone(),
                        mapping.target_path.clone(),
                    )
                })
                .collect();

            Some(
                self.backup_manager
                    .create_backup(
                        &self.migration_state.migration_id,
                        &self.create_migration_operations(&script_mappings)?,
                    )
                    .await?,
            )
        } else {
            None
        };

        // Step 4: Prepare target directory structure
        let directories_created = self.prepare_target_structure(&script_mappings).await?;

        // Step 5: Execute migration
        let migration_operations = self.create_migration_operations(&script_mappings)?;
        let completed_operations = self
            .migration_orchestrator
            .execute_migration(migration_operations)
            .await?;

        // Step 6: Calculate results
        let migration_result = self
            .calculate_migration_result(
                &completed_operations,
                directories_created,
                backup_id,
                start_time.elapsed(),
            )
            .await?;

        // Step 7: Validate migration if enabled
        let validation_passed = if self.config.validate_after_migration && !self.config.dry_run {
            info!("Validating migration results...");
            let validation_report = self
                .migration_validator
                .validate_migration(&self.migration_state.migration_id, &completed_operations)
                .await?;

            Some(matches!(
                validation_report.overall_status,
                crate::validation::ValidationStatus::Passed
            ))
        } else {
            None
        };

        // Step 8: Complete migration state
        self.migration_state.complete_migration();

        let final_result = MigrationResult {
            validation_passed,
            ..migration_result
        };

        // Step 9: Generate output report
        self.generate_migration_report(&final_result).await?;

        info!(
            "Script migration completed: {} scripts migrated, {} failed in {:?}",
            final_result.scripts_migrated, final_result.scripts_failed, final_result.migration_time
        );

        Ok(final_result)
    }

    /// Discover all scripts in the source directories
    async fn discover_scripts(&self) -> Result<astgrep_parser::script_discovery::DiscoveryResults> {
        info!("Discovering scripts in source directories...");
        self.script_discovery.discover_scripts().await
    }

    /// Classify scripts and determine target paths
    async fn classify_and_map_scripts(
        &self,
        discovery_results: &astgrep_parser::script_discovery::DiscoveryResults,
    ) -> Result<Vec<ScriptMapping>> {
        info!("Classifying scripts and determining target paths...");

        let mut mappings = Vec::new();
        let mut script_assets = Vec::new();

        // Convert discovered scripts to test assets
        for script in discovery_results.scripts_by_type.values().flatten() {
            let asset = TestAsset::new(
                format!("script-{}", uuid::Uuid::new_v4()),
                script.name.clone(),
                AssetType::Script,
                script.path.clone(),
                PathBuf::new(), // Will be set by mapping
            )
            .with_language(
                script
                    .language
                    .as_ref()
                    .map(|lang| format!("{:?}", lang))
                    .unwrap_or_else(|| "Unknown".to_string()),
            );

            script_assets.push(asset);
        }

        // Classify each script
        let classification_results = self.script_classifier.classify_scripts(&script_assets)?;

        for (i, asset) in script_assets.iter().enumerate() {
            if let Some(classification) = classification_results.get(i) {
                let target_path = self.determine_target_path(
                    &asset.current_path,
                    &classification.script_type,
                    &classification.confidence,
                )?;

                mappings.push(ScriptMapping {
                    original_path: asset.current_path.clone(),
                    target_path,
                    script_type: classification.script_type.clone(),
                    confidence: classification.confidence,
                });
            } else {
                warn!(
                    "No classification result for script: {:?}",
                    asset.current_path
                );
                // Fallback to utility category
                let target_path =
                    self.determine_target_path(&asset.current_path, &ScriptType::Utility, &0.1)?;

                mappings.push(ScriptMapping {
                    original_path: asset.current_path.clone(),
                    target_path,
                    script_type: ScriptType::Utility,
                    confidence: 0.1,
                });
            }
        }

        Ok(mappings)
    }

    /// Determine the target path for a script based on its type and confidence
    fn determine_target_path(
        &self,
        original_path: &Path,
        script_type: &ScriptType,
        _confidence: &f64,
    ) -> Result<PathBuf> {
        let file_name = original_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("script");

        // Create target directory based on script type
        let target_dir = match script_type {
            ScriptType::Validator => self.config.target_directory.join("validation"),
            ScriptType::Runner => self.config.target_directory.join("runners"),
            ScriptType::Utility => self.config.target_directory.join("utilities"),
            ScriptType::CiIntegration => self.config.target_directory.join("ci"),
        };

        // Ensure target directory exists
        self.path_handler.create_directory(&target_dir)?;

        // Preserve original filename
        Ok(target_dir.join(file_name))
    }

    /// Prepare target directory structure
    async fn prepare_target_structure(&self, mappings: &[ScriptMapping]) -> Result<usize> {
        info!("Preparing target directory structure...");

        let mut directories_created = 0;
        let mut created_dirs = std::collections::HashSet::new();

        for mapping in mappings {
            let target_dir = mapping.target_path.parent().unwrap_or(&mapping.target_path);

            if !created_dirs.contains(target_dir) {
                self.path_handler.create_directory(target_dir)?;
                created_dirs.insert(target_dir.to_path_buf());
                directories_created += 1;
            }
        }

        info!("Created {} directories", directories_created);
        Ok(directories_created)
    }

    /// Create migration operations from script mappings
    fn create_migration_operations(
        &self,
        mappings: &[ScriptMapping],
    ) -> Result<Vec<MigrationOperation>> {
        let mut operations = Vec::new();

        for (index, mapping) in mappings.iter().enumerate() {
            let operation_id = format!("script-migration-{:04}", index + 1);

            operations.push(MigrationOperation {
                id: operation_id,
                source_path: mapping.original_path.clone(),
                target_path: mapping.target_path.clone(),
                operation_type: OperationType::Copy, // Use copy to preserve originals
                status: OperationStatus::Pending,
                error_message: None,
                bytes_transferred: 0,
                checksum_before: None,
                checksum_after: None,
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(operations)
    }

    /// Calculate migration result from completed operations
    async fn calculate_migration_result(
        &self,
        operations: &[MigrationOperation],
        directories_created: usize,
        backup_id: Option<String>,
        elapsed: std::time::Duration,
    ) -> Result<MigrationResult> {
        let mut scripts_migrated = 0;
        let mut scripts_failed = 0;
        let mut scripts_skipped = 0;
        let mut total_bytes_migrated = 0;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for operation in operations {
            match operation.status {
                OperationStatus::Completed => {
                    scripts_migrated += 1;
                    total_bytes_migrated += operation.bytes_transferred;
                }
                OperationStatus::Failed => {
                    scripts_failed += 1;
                    if let Some(ref error) = operation.error_message {
                        errors.push(format!(
                            "Failed to migrate {:?}: {}",
                            operation.source_path, error
                        ));
                    }
                }
                OperationStatus::Skipped => {
                    scripts_skipped += 1;
                    warnings.push(format!("Skipped migration of {:?}", operation.source_path));
                }
                _ => {}
            }
        }

        Ok(MigrationResult {
            migration_id: self.migration_state.migration_id.clone(),
            total_scripts_found: operations.len(),
            scripts_migrated,
            scripts_failed,
            scripts_skipped,
            directories_created,
            total_bytes_migrated,
            migration_time: elapsed,
            validation_passed: None,
            backup_id,
            errors,
            warnings,
        })
    }

    /// Generate migration report
    async fn generate_migration_report(&self, result: &MigrationResult) -> Result<()> {
        let output = self.output_formatter.format_migration_summary(
            &crate::output::formatter::MigrationSummary {
                migration_id: result.migration_id.clone(),
                status: if result.validation_passed.unwrap_or(true) {
                    crate::output::formatter::MigrationStatus::Completed
                } else {
                    crate::output::formatter::MigrationStatus::Failed
                },
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                duration: Some(format!("{:.2}s", result.migration_time.as_secs_f64())),
                total_operations: result.total_scripts_found,
                completed_operations: result.scripts_migrated,
                failed_operations: result.scripts_failed,
                skipped_operations: result.scripts_skipped,
                bytes_transferred: result.total_bytes_migrated,
                files_processed: result.scripts_migrated,
                directories_created: result.directories_created,
                symlinks_created: 0,
                errors: result.errors.clone(),
                warnings: result.warnings.clone(),
            },
        )?;

        // Output to stdout for now (could be extended to write to file)
        println!("{}", output);
        Ok(())
    }

    /// Get the current migration state
    pub fn get_migration_state(&self) -> &MigrationState {
        &self.migration_state
    }

    /// Check if a dry run is configured
    pub fn is_dry_run(&self) -> bool {
        self.config.dry_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_script_migrator_creation() {
        let config = ScriptMigrationConfig::default();
        let migrator = ScriptMigrator::with_config(config);
        assert!(migrator.is_ok());
    }

    #[tokio::test]
    async fn test_determine_target_path() {
        let config = ScriptMigrationConfig::default();
        let migrator = ScriptMigrator::with_config(config).unwrap();

        let original_path = PathBuf::from("/tests/validate.sh");
        let target_path = migrator
            .determine_target_path(&original_path, &ScriptType::Validator, &0.8)
            .unwrap();

        assert_eq!(
            target_path,
            PathBuf::from("newtest/scripts/validation/validate.sh")
        );
    }

    #[test]
    fn test_target_directory_creation() -> Result<()> {
        let temp_dir = tempdir()?;
        let config = ScriptMigrationConfig {
            target_directory: temp_dir.path().join("newtest/scripts"),
            ..Default::default()
        };

        let migrator = ScriptMigrator::with_config(config)?;

        let target_dir = migrator.determine_target_path(
            &PathBuf::from("/test/script.sh"),
            &ScriptType::Utility,
            &0.5,
        )?;

        // Directory should be created
        assert!(target_dir.exists());
        assert!(target_dir.is_dir());

        Ok(())
    }

    #[test]
    fn test_migration_config_default() {
        let config = ScriptMigrationConfig::default();
        assert!(!config.dry_run);
        assert!(config.create_backups);
        assert!(config.validate_after_migration);
        assert!(config.preserve_permissions);
        assert!(config.enable_progress);
        assert_eq!(config.parallel_jobs, 4);
    }
}
