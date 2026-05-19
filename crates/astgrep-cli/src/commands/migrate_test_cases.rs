//! Test case migration engine for ASTGreP
//!
//! This module provides functionality to migrate test cases from the current directory
//! structure to the new hierarchical organization based on language and test type.

use anyhow::{anyhow, Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use tracing::{info, debug, error, instrument};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use walkdir::WalkDir;

use astgrep_core::models::{TestCase, TestType, TestComplexity, TestCaseStatus, TestCaseMetadata, LanguageMapping};
use crate::validation::{ValidationReport};
use crate::validation::migration_validator::{MigrationValidator, ValidationConfig};
use crate::backup::backup_manager::BackupManager;
use crate::services::migration_orchestrator::MigrationOrchestrator;
use crate::utils::path_utils::PathHandler;

/// Configuration for test case migration
#[derive(Debug, Clone)]
pub struct TestCaseMigrationConfig {
    /// Root directory containing test cases to migrate
    pub source_root: PathBuf,
    /// Root directory for migrated test cases
    pub target_root: PathBuf,
    /// Language mapping configuration
    pub language_mapping: LanguageMapping,
    /// Whether to create backups before migration
    pub create_backups: bool,
    /// Backup directory
    pub backup_directory: Option<PathBuf>,
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: usize,
    /// Whether to perform dry run
    pub dry_run: bool,
    /// Whether to validate migration results
    pub validate_migration: bool,
    /// Whether to update dependencies
    pub update_dependencies: bool,
    /// File size limits (min, max bytes)
    pub file_size_limits: Option<(u64, u64)>,
    /// Patterns to exclude from migration
    pub exclude_patterns: Vec<String>,
    /// Whether to preserve timestamps
    pub preserve_timestamps: bool,
    /// Whether to calculate checksums for integrity verification
    pub calculate_checksums: bool,
    /// Whether to detect and update cross-references
    pub update_cross_references: bool,
}

impl Default for TestCaseMigrationConfig {
    fn default() -> Self {
        Self {
            source_root: PathBuf::from("."),
            target_root: PathBuf::from("newtest/testcases"),
            language_mapping: LanguageMapping::new(),
            create_backups: true,
            backup_directory: Some(PathBuf::from("backup/test_cases")),
            max_concurrent_operations: 10,
            dry_run: false,
            validate_migration: true,
            update_dependencies: true,
            file_size_limits: Some((10, 10_000_000)), // 10 bytes to 10MB
            exclude_patterns: vec![
                ".git/*".to_string(),
                "node_modules/*".to_string(),
                "target/*".to_string(),
                "build/*".to_string(),
                "dist/*".to_string(),
                "*.tmp".to_string(),
                "*.bak".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
            ],
            preserve_timestamps: true,
            calculate_checksums: true,
            update_cross_references: true,
        }
    }
}

/// Result of test case migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseMigrationResult {
    /// Successfully migrated test cases
    pub successful_migrations: Vec<MigratedTestCase>,
    /// Failed migrations
    pub failed_migrations: Vec<FailedMigration>,
    /// Migration summary statistics
    pub summary: MigrationSummary,
    /// Language distribution
    pub language_distribution: HashMap<String, usize>,
    /// Test type distribution
    pub type_distribution: HashMap<String, usize>,
    /// Cross-references updated
    pub cross_references_updated: Vec<String>,
    /// Validation results
    pub validation_results: Option<ValidationReport>,
    /// Migration timestamp
    pub migrated_at: chrono::DateTime<chrono::Utc>,
    /// Warnings and issues
    pub warnings: Vec<String>,
}

/// Summary of migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSummary {
    /// Total test cases processed
    pub total_test_cases: usize,
    /// Successfully migrated test cases
    pub successful_migrations: usize,
    /// Failed migrations
    pub failed_migrations: usize,
    /// Skipped test cases
    pub skipped_test_cases: usize,
    /// Total bytes migrated
    pub total_bytes_migrated: u64,
    /// Migration duration in milliseconds
    pub migration_duration_ms: u64,
    /// Number of files backed up
    pub files_backed_up: usize,
    /// Number of cross-references updated
    pub cross_references_updated: usize,
    /// Number of validation failures
    pub validation_failures: usize,
}

/// A successfully migrated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigratedTestCase {
    /// Original test case
    pub original_test_case: TestCase,
    /// New location
    pub new_path: PathBuf,
    /// Migration metadata
    pub migration_metadata: MigrationMetadata,
}

/// Metadata for a migrated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetadata {
    /// Migration timestamp
    pub migrated_at: chrono::DateTime<chrono::Utc>,
    /// Original file size
    pub original_file_size: u64,
    /// New file size
    pub new_file_size: u64,
    /// Checksum before migration
    pub original_checksum: Option<String>,
    /// Checksum after migration
    pub new_checksum: Option<String>,
    /// Whether the file was modified during migration
    pub was_modified: bool,
    /// Migration operations performed
    pub operations_performed: Vec<MigrationOperation>,
}

/// Operations performed during migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOperation {
    /// File was moved
    FileMoved { from: PathBuf, to: PathBuf },
    /// Dependencies were updated
    DependenciesUpdated { count: usize },
    /// Cross-references were updated
    CrossReferencesUpdated { count: usize },
    /// File was transformed
    FileTransformed { description: String },
    /// Metadata was updated
    MetadataUpdated { fields: Vec<String> },
}

/// A failed migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedMigration {
    /// Original test case path
    pub original_path: PathBuf,
    /// Error message
    pub error_message: String,
    /// Error type
    pub error_type: MigrationErrorType,
    /// Partial results (if any)
    pub partial_results: Option<MigratedTestCase>,
}

/// Types of migration errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationErrorType {
    /// File access error
    FileAccess,
    /// Language detection error
    LanguageDetection,
    /// Test type classification error
    TestTypeClassification,
    /// Path generation error
    PathGeneration,
    /// File operation error
    FileOperation,
    /// Dependency update error
    DependencyUpdate,
    /// Cross-reference update error
    CrossReferenceUpdate,
    /// Validation error
    ValidationError,
    /// Backup error
    BackupError,
    /// Other error
    Other(String),
}

/// Test case migration engine
pub struct TestCaseMigrator {
    config: TestCaseMigrationConfig,
    path_handler: PathHandler,
    migration_orchestrator: MigrationOrchestrator,
    backup_manager: BackupManager,
    migration_validator: MigrationValidator,
    semaphore: Semaphore,
}

impl TestCaseMigrator {
    /// Create a new test case migrator
    pub fn new(config: TestCaseMigrationConfig) -> Result<Self> {
        let path_handler = PathHandler::new();
        let migration_orchestrator = MigrationOrchestrator::new(
            config.dry_run,
            config.preserve_timestamps,
            config.create_backups,
            config.max_concurrent_operations,
        );
        let backup_manager = BackupManager::new(crate::backup::BackupConfig {
            enabled: config.backup_directory.is_some(),
            backup_directory: config.backup_directory.clone().unwrap_or_else(|| std::path::PathBuf::from("./backups")),
            compression_enabled: false,
            max_backup_size_gb: 10.0,
            retention_days: 30,
            verify_backups: true,
            include_metadata: true,
        });
        let validation_config = ValidationConfig::default();
        let migration_validator = MigrationValidator::new(validation_config);

        Ok(Self {
            config,
            path_handler,
            migration_orchestrator,
            backup_manager,
            migration_validator,
            semaphore: Semaphore::new(10), // Default semaphore limit
        })
    }

    /// Migrate all test cases from source to target directory
    #[instrument(skip(self))]
    pub async fn migrate_test_cases(&mut self) -> Result<TestCaseMigrationResult> {
        info!("Starting test case migration from {} to {}",
              self.config.source_root.display(),
              self.config.target_root.display());

        let start_time = std::time::Instant::now();
        let mut successful_migrations = Vec::new();
        let mut failed_migrations = Vec::new();
        let mut language_distribution = HashMap::new();
        let mut type_distribution = HashMap::new();
        let mut cross_references_updated = Vec::new();
        let warnings = Vec::new();
        let mut total_bytes_migrated = 0u64;
        let files_backed_up = 0usize;

        // Discover test cases to migrate
        let test_cases = self.discover_test_cases().await
            .context("Failed to discover test cases for migration")?;

        info!("Discovered {} test cases for migration", test_cases.len());

        // Create backup if requested
        if self.config.create_backups && !self.config.dry_run {
            let migration_id = format!("migration-{}", chrono::Utc::now().timestamp());
            let backup_id = self.backup_manager.create_backup(&migration_id, &[]).await
                .context("Failed to create backup")?;
            info!("Created backup with ID: {}", backup_id);
        }

        // Process test cases with concurrency control
        let mut join_set = JoinSet::new();
        let permit = self.semaphore.acquire().await?;

        for test_case in test_cases {
            let config = self.config.clone();
            let path_handler = self.path_handler.clone();

            join_set.spawn(async move {
                Self::migrate_single_test_case(test_case, config, path_handler).await
            });
        }

        drop(permit);

        // Collect migration results
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(migration_result) => {
                    match migration_result {
                        Ok(migrated_test_case) => {
                            total_bytes_migrated += migrated_test_case.migration_metadata.original_file_size;

                            // Update distributions
                            for language in &migrated_test_case.original_test_case.languages {
                                *language_distribution.entry(language.clone()).or_insert(0) += 1;
                            }
                            *type_distribution.entry(format!("{:?}", migrated_test_case.original_test_case.test_type))
                                .or_insert(0) += 1;

                            successful_migrations.push(migrated_test_case);
                        }
                        Err(failed_migration) => {
                            error!("Migration failed for {}: {}",
                                   failed_migration.original_path.display(),
                                   failed_migration.error_message);
                            failed_migrations.push(failed_migration);
                        }
                    }
                }
                Err(e) => {
                    error!("Task failed: {:?}", e);
                    failed_migrations.push(FailedMigration {
                        original_path: PathBuf::from("unknown"),
                        error_message: format!("Task failed: {:?}", e),
                        error_type: MigrationErrorType::Other("Task failure".to_string()),
                        partial_results: None,
                    });
                }
            }
        }

        // Update cross-references if requested
        if self.config.update_cross_references {
            let cross_refs = self.update_cross_references(&successful_migrations).await
                .context("Failed to update cross-references")?;
            cross_references_updated = cross_refs;
        }

        let migration_duration = start_time.elapsed();

        // Perform validation if requested
        let validation_results = if self.config.validate_migration && !self.config.dry_run {
            Some(self.migration_validator.validate_migration("test-migration", &[]).await
                .context("Migration validation failed")?)
        } else {
            None
        };

        // Calculate migration statistics
        let successful_count = successful_migrations.len();
        let failed_count = failed_migrations.len();
        let skipped_count = 0; // TODO: Implement skipped logic
        let validation_failures = validation_results.as_ref()
            .map(|v| v.failed_operations)
            .unwrap_or(0);

        let summary = MigrationSummary {
            total_test_cases: successful_count + failed_count + skipped_count,
            successful_migrations: successful_count,
            failed_migrations: failed_count,
            skipped_test_cases: skipped_count,
            total_bytes_migrated,
            migration_duration_ms: migration_duration.as_millis() as u64,
            files_backed_up,
            cross_references_updated: cross_references_updated.len(),
            validation_failures,
        };

        info!("Migration completed: {} successful, {} failed",
              successful_count, failed_count);

        Ok(TestCaseMigrationResult {
            successful_migrations,
            failed_migrations,
            summary,
            language_distribution,
            type_distribution,
            cross_references_updated,
            validation_results,
            migrated_at: chrono::Utc::now(),
            warnings,
        })
    }

    /// Discover test cases for migration
    async fn discover_test_cases(&self) -> Result<Vec<TestCase>> {
        let mut test_cases = Vec::new();
        let walker = WalkDir::new(&self.config.source_root)
            .follow_links(false)
            .into_iter();

        for entry in walker {
            let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;

            if entry.file_type().is_file() {
                let file_path = entry.path();

                // Check if file should be excluded
                if self.should_exclude_file(file_path) {
                    continue;
                }

                // Check file size limits
                if let Some((min_size, max_size)) = self.config.file_size_limits {
                    if let Ok(metadata) = entry.metadata() {
                        let file_size = metadata.len();
                        if file_size < min_size || file_size > max_size {
                            continue;
                        }
                    }
                }

                // Try to classify as test case
                if let Some(test_case) = self.classify_test_case(file_path).await? {
                    test_cases.push(test_case);
                }
            }
        }

        Ok(test_cases)
    }

    /// Check if a file should be excluded from migration
    fn should_exclude_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        self.config.exclude_patterns
            .iter()
            .any(|pattern| {
                let regex_pattern = pattern.replace('*', ".*").replace('?', ".");
                if let Ok(regex) = regex::Regex::new(&regex_pattern) {
                    regex.is_match(&path_str)
                } else {
                    false
                }
            })
    }

    /// Classify a file as a test case
    async fn classify_test_case(&self, file_path: &Path) -> Result<Option<TestCase>> {
        // Detect language
        let content = fs::read_to_string(file_path).ok();
        let language = self.config.language_mapping.detect_language(&file_path.to_path_buf(), content.as_deref());

        // Check if it's a test case based on filename patterns
        if let Some(filename) = file_path.file_stem().and_then(|s| s.to_str()) {
            let filename_lower = filename.to_lowercase();

            if filename_lower.contains("test") ||
               filename_lower.contains("spec") ||
               filename_lower.contains("validate") {

                // Determine test type based on filename
                let test_type = if filename_lower.contains("security") {
                    TestType::Security
                } else if filename_lower.contains("performance") || filename_lower.contains("perf") {
                    TestType::Performance
                } else if filename_lower.contains("integration") {
                    TestType::Integration
                } else if filename_lower.contains("parsing") || filename_lower.contains("parse") {
                    TestType::Parsing
                } else if filename_lower.contains("basic") {
                    TestType::RuleValidation
                } else {
                    TestType::RuleValidation
                };

                // Generate target path
                let target_path = self.generate_target_path(file_path, &language, &test_type).await?;

                // Create test case
                let test_case_id = format!("tc-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default());
                let test_case_name = filename.to_string();

                let test_case = TestCase {
                    asset_id: test_case_id,
                    name: test_case_name,
                    test_type,
                    languages: vec![language],
                    rule_files: vec![],
                    source_files: vec![file_path.to_path_buf()],
                    expected_results: vec![],
                    complexity: TestComplexity::Medium,
                    status: TestCaseStatus::Pending,
                    target_path,
                    current_path: file_path.to_path_buf(),
                    category: None,
                    metadata: TestCaseMetadata::default(),
                    dependencies: vec![],
                    tags: vec![],
                    description: Some(format!("Test case for {}", filename)),
                };

                return Ok(Some(test_case));
            }
        }

        Ok(None)
    }

    /// Generate target path for a test case
    async fn generate_target_path(&self, source_path: &Path, language: &str, test_type: &TestType) -> Result<PathBuf> {
        // Get language configuration
        let lang_config = self.config.language_mapping.get_language_config(language)
            .ok_or_else(|| anyhow!("No language configuration found for {}", language))?;

        // Build path: newtest/testcases/{language}/{test-type}/
        let test_type_dir = match test_type {
            TestType::RuleValidation => "rule-validation",
            TestType::PatternMatching => "pattern-matching",
            TestType::Parsing => "parsing",
            TestType::Integration => "integration",
            TestType::Performance => "performance",
            TestType::Security => "security",
            TestType::Compatibility => "compatibility",
            TestType::DataFlow => "data-flow",
            TestType::Custom => "custom",
        };

        let mut target_path = self.config.target_root
            .join(&lang_config.directory_name)
            .join(test_type_dir);

        // Add filename based on original file
        if let Some(filename) = source_path.file_stem() {
            target_path = target_path.join(filename);
        } else {
            // Generate a default filename
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            target_path = target_path.join(format!("test_{}", timestamp));
        }

        // Ensure file extension matches the language
        let default_ext = "txt".to_string();
        let target_ext = lang_config.extensions.first()
            .unwrap_or(&default_ext);
        target_path = target_path.with_extension(target_ext);

        Ok(target_path)
    }

    /// Migrate a single test case
    async fn migrate_single_test_case(
        test_case: TestCase,
        config: TestCaseMigrationConfig,
        _path_handler: PathHandler,
    ) -> Result<MigratedTestCase, FailedMigration> {
        let original_path = test_case.current_path.clone();
        let target_path = test_case.target_path.clone();

        debug!("Migrating test case from {} to {}",
               original_path.display(),
               target_path.display());

        let _migration_start = SystemTime::now();
        let mut operations_performed = Vec::new();

        // Get original file metadata
        let original_metadata = fs::metadata(&original_path)
            .map_err(|e| FailedMigration {
                original_path: original_path.clone(),
                error_message: format!("Failed to read file metadata: {}", e),
                error_type: MigrationErrorType::FileAccess,
                partial_results: None,
            })?;

        let original_file_size = original_metadata.len();
        let original_checksum = if config.calculate_checksums {
            Self::calculate_file_checksum(&original_path).await.ok()
        } else {
            None
        };

        // Perform actual migration
        if config.dry_run {
            info!("[DRY RUN] Would migrate {} to {}",
                  original_path.display(),
                  target_path.display());
        } else {
            // Create target directory
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| FailedMigration {
                        original_path: original_path.clone(),
                        error_message: format!("Failed to create target directory: {}", e),
                        error_type: MigrationErrorType::FileOperation,
                        partial_results: None,
                    })?;
            }

            // Copy file
            fs::copy(&original_path, &target_path)
                .map_err(|e| FailedMigration {
                    original_path: original_path.clone(),
                    error_message: format!("Failed to copy file: {}", e),
                    error_type: MigrationErrorType::FileOperation,
                    partial_results: None,
                })?;

            operations_performed.push(MigrationOperation::FileMoved {
                from: original_path.clone(),
                to: target_path.clone(),
            });

            // Preserve timestamps if requested
            if config.preserve_timestamps {
                let _modified = original_metadata.modified()
                    .map_err(|e| FailedMigration {
                        original_path: original_path.clone(),
                        error_message: format!("Failed to get modification time: {}", e),
                        error_type: MigrationErrorType::FileOperation,
                        partial_results: None,
                    })?;

                let _accessed = original_metadata.accessed()
                    .map_err(|e| FailedMigration {
                        original_path: original_path.clone(),
                        error_message: format!("Failed to get access time: {}", e),
                        error_type: MigrationErrorType::FileOperation,
                        partial_results: None,
                    })?;

                // Skip setting file times as filetime crate is not available
                // TODO: Add filetime dependency for proper file time preservation
            }
        }

        // Calculate new checksum
        let new_checksum = if !config.dry_run && config.calculate_checksums {
            Self::calculate_file_checksum(&target_path).await.ok()
        } else {
            None
        };

        let migration_metadata = MigrationMetadata {
            migrated_at: chrono::Utc::now(),
            original_file_size,
            new_file_size: original_file_size, // File size shouldn't change during migration
            original_checksum,
            new_checksum,
            was_modified: false, // We're just copying files, not modifying content
            operations_performed,
        };

        Ok(MigratedTestCase {
            original_test_case: test_case,
            new_path: target_path.clone(),
            migration_metadata,
        })
    }

    /// Calculate file checksum
    async fn calculate_file_checksum(file_path: &Path) -> Result<String> {
        let content = fs::read(file_path)
            .context("Failed to read file for checksum calculation")?;
        {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&content);
            Ok(format!("{:x}", hasher.finalize()))
        }
    }

    /// Update cross-references between migrated test cases
    async fn update_cross_references(&mut self, _migrated_cases: &[MigratedTestCase]) -> Result<Vec<String>> {
        let updated_refs = Vec::new();

        // TODO: Implement cross-reference update logic
        // This would involve:
        // 1. Scanning all migrated files for references to other test files
        // 2. Updating those references to point to the new locations
        // 3. Keeping track of what was updated

        info!("Updated {} cross-references during migration", updated_refs.len());
        Ok(updated_refs)
    }

    /// Generate migration report
    pub fn generate_migration_report(&self, result: &TestCaseMigrationResult) -> String {
        let mut report = String::new();

        report.push_str("# Test Case Migration Report\n\n");
        report.push_str(&format!("Generated at: {}\n\n", result.migrated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary section
        report.push_str("## Migration Summary\n\n");
        report.push_str(&format!("- **Total test cases processed**: {}\n", result.summary.total_test_cases));
        report.push_str(&format!("- **Successfully migrated**: {}\n", result.summary.successful_migrations));
        report.push_str(&format!("- **Failed migrations**: {}\n", result.summary.failed_migrations));
        report.push_str(&format!("- **Skipped test cases**: {}\n", result.summary.skipped_test_cases));
        report.push_str(&format!("- **Total bytes migrated**: {} MB\n", result.summary.total_bytes_migrated / 1_048_576));
        report.push_str(&format!("- **Migration duration**: {:.2} seconds\n", result.summary.migration_duration_ms as f64 / 1000.0));
        report.push_str(&format!("- **Files backed up**: {}\n", result.summary.files_backed_up));
        report.push_str(&format!("- **Cross-references updated**: {}\n", result.summary.cross_references_updated));
        report.push_str(&format!("- **Validation failures**: {}\n\n", result.summary.validation_failures));

        // Language distribution
        report.push_str("## Language Distribution\n\n");
        for (language, count) in &result.language_distribution {
            report.push_str(&format!("- **{}**: {} test cases\n", language, count));
        }
        report.push_str("\n");

        // Test type distribution
        report.push_str("## Test Type Distribution\n\n");
        for (test_type, count) in &result.type_distribution {
            report.push_str(&format!("- **{}**: {} test cases\n", test_type, count));
        }
        report.push_str("\n");

        // Warnings
        if !result.warnings.is_empty() {
            report.push_str("## Warnings\n\n");
            for warning in &result.warnings {
                report.push_str(&format!("- {}\n", warning));
            }
            report.push_str("\n");
        }

        // Failed migrations
        if !result.failed_migrations.is_empty() {
            report.push_str("## Failed Migrations\n\n");
            for failed in &result.failed_migrations {
                report.push_str(&format!("### {}\n", failed.original_path.display()));
                report.push_str(&format!("- **Error**: {}\n", failed.error_message));
                report.push_str(&format!("- **Type**: {:?}\n\n", failed.error_type));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_test_case_migration_config_default() {
        let config = TestCaseMigrationConfig::default();
        assert_eq!(config.source_root, PathBuf::from("."));
        assert_eq!(config.target_root, PathBuf::from("newtest/testcases"));
        assert!(config.create_backups);
        assert!(config.validate_migration);
        assert!(config.preserve_timestamps);
        assert_eq!(config.max_concurrent_operations, 10);
    }

    #[test]
    fn test_exclude_pattern_matching() {
        let config = TestCaseMigrationConfig::default();
        let migrator = TestCaseMigrator::new(config).unwrap();

        assert!(migrator.should_exclude_file(&PathBuf::from("target/test.java")));
        assert!(migrator.should_exclude_file(&PathBuf::from("node_modules/test.py")));
        assert!(migrator.should_exclude_file(&PathBuf::from("test.tmp")));
    }

    #[tokio::test]
    async fn test_file_checksum_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!").unwrap();

        let checksum = TestCaseMigrator::calculate_file_checksum(&test_file).await.unwrap();
        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // SHA256 hex string length
    }

    #[tokio::test]
    async fn test_target_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let config = TestCaseMigrationConfig {
            source_root: temp_dir.path().to_path_buf(),
            target_root: temp_dir.path().join("newtest/testcases"),
            language_mapping: LanguageMapping::new(),
            ..Default::default()
        };
        let migrator = TestCaseMigrator::new(config).unwrap();

        let source_path = temp_dir.path().join("SecurityTest.java");
        let target_path = migrator.generate_target_path(
            &source_path,
            "java",
            &TestType::Security
        ).await.unwrap();

        assert!(target_path.starts_with(temp_dir.path().join("newtest/testcases/java/security/")));
        assert!(target_path.ends_with(".java"));
    }

    #[test]
    fn test_migration_report_generation() {
        let result = TestCaseMigrationResult {
            successful_migrations: Vec::new(),
            failed_migrations: Vec::new(),
            summary: MigrationSummary {
                total_test_cases: 10,
                successful_migrations: 8,
                failed_migrations: 2,
                skipped_test_cases: 0,
                total_bytes_migrated: 1024,
                migration_duration_ms: 5000,
                files_backed_up: 8,
                cross_references_updated: 5,
                validation_failures: 0,
            },
            language_distribution: HashMap::new(),
            type_distribution: HashMap::new(),
            cross_references_updated: Vec::new(),
            validation_results: None,
            migrated_at: chrono::Utc::now(),
            warnings: Vec::new(),
        };

        let config = TestCaseMigrationConfig::default();
        let migrator = TestCaseMigrator::new(config).unwrap();
        let report = migrator.generate_migration_report(&result);

        assert!(report.contains("Test Case Migration Report"));
        assert!(report.contains("Migration Summary"));
        assert!(report.contains("Total test cases processed: 10"));
        assert!(report.contains("Successfully migrated: 8"));
        assert!(report.contains("Failed migrations: 2"));
    }
}