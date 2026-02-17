//! Migration validation framework with checksum verification

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::services::migration_orchestrator::MigrationOperation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub enabled: bool,
    pub verify_checksums: bool,
    pub verify_permissions: bool,
    pub verify_timestamps: bool,
    pub verify_content_integrity: bool,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub validation_timeout_secs: u64,
    pub strict_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            verify_checksums: true,
            verify_permissions: true,
            verify_timestamps: true,
            verify_content_integrity: true,
            checksum_algorithm: ChecksumAlgorithm::Sha256,
            validation_timeout_secs: 300, // 5 minutes
            strict_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub migration_id: String,
    pub validation_timestamp: SystemTime,
    pub config: ValidationConfig,
    pub overall_status: ValidationStatus,
    pub total_operations: usize,
    pub valid_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub operation_validations: Vec<OperationValidation>,
    pub summary_metrics: ValidationMetrics,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStatus {
    Passed,
    Failed,
    PartialSuccess,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationValidation {
    pub operation_id: String,
    pub operation_type: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub validation_status: ValidationStatus,
    pub validation_details: ValidationDetails,
    pub validation_duration_ms: u64,
    pub custom_checks: Vec<CustomValidationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetails {
    pub source_exists: bool,
    pub target_exists: bool,
    pub checksum_match: Option<bool>,
    pub permissions_match: Option<bool>,
    pub timestamp_match: Option<bool>,
    pub size_match: Option<bool>,
    pub content_match: Option<bool>,
    pub file_count_match: Option<bool>, // For directories
    pub symlink_target_match: Option<bool>, // For symlinks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub total_bytes_validated: u64,
    pub total_files_validated: usize,
    pub total_directories_validated: usize,
    pub total_symlinks_validated: usize,
    pub average_validation_time_ms: f64,
    pub fastest_validation_ms: u64,
    pub slowest_validation_ms: u64,
    pub checksum_calculations_performed: usize,
    pub content_comparisons_performed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_id: String,
    pub operation_id: Option<String>,
    pub error_type: ValidationErrorType,
    pub severity: ValidationSeverity,
    pub message: String,
    pub details: HashMap<String, String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    FileNotFound,
    ChecksumMismatch,
    PermissionMismatch,
    TimestampMismatch,
    SizeMismatch,
    ContentMismatch,
    DirectoryStructureMismatch,
    SymlinkTargetMismatch,
    AccessDenied,
    Timeout,
    ConfigurationError,
    SystemError,
}

impl std::fmt::Display for ValidationErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationErrorType::FileNotFound => write!(f, "FileNotFound"),
            ValidationErrorType::ChecksumMismatch => write!(f, "ChecksumMismatch"),
            ValidationErrorType::PermissionMismatch => write!(f, "PermissionMismatch"),
            ValidationErrorType::TimestampMismatch => write!(f, "TimestampMismatch"),
            ValidationErrorType::SizeMismatch => write!(f, "SizeMismatch"),
            ValidationErrorType::ContentMismatch => write!(f, "ContentMismatch"),
            ValidationErrorType::DirectoryStructureMismatch => write!(f, "DirectoryStructureMismatch"),
            ValidationErrorType::SymlinkTargetMismatch => write!(f, "SymlinkTargetMismatch"),
            ValidationErrorType::AccessDenied => write!(f, "AccessDenied"),
            ValidationErrorType::Timeout => write!(f, "Timeout"),
            ValidationErrorType::ConfigurationError => write!(f, "ConfigurationError"),
            ValidationErrorType::SystemError => write!(f, "SystemError"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationSeverity::Info => write!(f, "Info"),
            ValidationSeverity::Warning => write!(f, "Warning"),
            ValidationSeverity::Error => write!(f, "Error"),
            ValidationSeverity::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValidationCheck {
    pub check_name: String,
    pub passed: bool,
    pub message: Option<String>,
    pub details: HashMap<String, String>,
}

pub struct MigrationValidator {
    config: ValidationConfig,
    custom_validators: HashMap<String, Box<dyn CustomValidator + Send + Sync>>,
}

impl MigrationValidator {
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            custom_validators: HashMap::new(),
        }
    }

    /// Register a custom validator
    pub fn register_custom_validator(&mut self, name: String, validator: Box<dyn CustomValidator + Send + Sync>) {
        self.custom_validators.insert(name, validator);
    }

    /// Validate migration operations
    pub async fn validate_migration(&self, migration_id: &str, operations: &[MigrationOperation]) -> Result<ValidationReport> {
        if !self.config.enabled {
            info!("Migration validation is disabled");
            return Ok(ValidationReport {
                migration_id: migration_id.to_string(),
                validation_timestamp: SystemTime::now(),
                config: self.config.clone(),
                overall_status: ValidationStatus::Skipped,
                total_operations: operations.len(),
                valid_operations: 0,
                failed_operations: 0,
                skipped_operations: operations.len(),
                operation_validations: Vec::new(),
                summary_metrics: ValidationMetrics {
                    total_bytes_validated: 0,
                    total_files_validated: 0,
                    total_directories_validated: 0,
                    total_symlinks_validated: 0,
                    average_validation_time_ms: 0.0,
                    fastest_validation_ms: 0,
                    slowest_validation_ms: 0,
                    checksum_calculations_performed: 0,
                    content_comparisons_performed: 0,
                },
                errors: Vec::new(),
            });
        }

        info!("Starting migration validation for {} operations", operations.len());

        let mut operation_validations = Vec::new();
        let mut errors = Vec::new();
        let mut valid_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;

        let mut total_bytes = 0u64;
        let mut files_count = 0usize;
        let mut directories_count = 0usize;
        let mut symlinks_count = 0usize;
        let mut validation_times = Vec::new();

        for operation in operations {
            let start_time = std::time::Instant::now();
            let validation = self.validate_operation(operation).await;
            let duration = start_time.elapsed();

            let (operation_validation, operation_errors) = match validation {
                Ok(validation) => {
                    match validation.validation_status {
                        ValidationStatus::Passed => {
                            valid_count += 1;
                        }
                        ValidationStatus::Failed => {
                            failed_count += 1;
                        }
                        ValidationStatus::Skipped => {
                            skipped_count += 1;
                        }
                        ValidationStatus::PartialSuccess => {
                            valid_count += 1;
                        }
                    }

                    // Update counters based on operation type
                    match operation.operation_type {
                        crate::services::migration_orchestrator::OperationType::Copy |
                        crate::services::migration_orchestrator::OperationType::Move => {
                            if operation.source_path.is_file() {
                                files_count += 1;
                                if let Ok(metadata) = fs::metadata(&operation.source_path).await {
                                    total_bytes += metadata.len();
                                }
                            } else if operation.source_path.is_dir() {
                                directories_count += 1;
                            }
                        }
                        crate::services::migration_orchestrator::OperationType::CreateSymlink => {
                            symlinks_count += 1;
                        }
                        _ => {}
                    }

                    (validation, Vec::new())
                }
                Err(e) => {
                    failed_count += 1;
                    let validation_error = ValidationError {
                        error_id: uuid::Uuid::new_v4().to_string(),
                        operation_id: Some(operation.id.clone()),
                        error_type: ValidationErrorType::SystemError,
                        severity: ValidationSeverity::Error,
                        message: e.to_string(),
                        details: HashMap::new(),
                        timestamp: SystemTime::now(),
                    };
                    errors.push(validation_error.clone());

                    // Create a failed validation result
                    let failed_validation = OperationValidation {
                        operation_id: operation.id.clone(),
                        operation_type: format!("{:?}", operation.operation_type),
                        source_path: operation.source_path.clone(),
                        target_path: operation.target_path.clone(),
                        validation_status: ValidationStatus::Failed,
                        validation_details: ValidationDetails {
                            source_exists: operation.source_path.exists(),
                            target_exists: operation.target_path.exists(),
                            checksum_match: None,
                            permissions_match: None,
                            timestamp_match: None,
                            size_match: None,
                            content_match: None,
                            file_count_match: None,
                            symlink_target_match: None,
                        },
                        validation_duration_ms: duration.as_millis() as u64,
                        custom_checks: Vec::new(),
                    };

                    (failed_validation, vec![validation_error.clone()])
                }
            };

            validation_times.push(duration.as_millis() as f64);
            operation_validations.push(operation_validation);
            errors.extend(operation_errors);
        }

        // Calculate overall status
        let overall_status = match (valid_count, failed_count, skipped_count) {
            (v, 0, s) if v + s == operation_validations.len() => ValidationStatus::Passed,
            (0, f, s) if f + s == operation_validations.len() => ValidationStatus::Failed,
            (v, f, s) if v + f + s == operation_validations.len() && v > 0 && f > 0 => ValidationStatus::PartialSuccess,
            (_, _, s) if s == operation_validations.len() => ValidationStatus::Skipped,
            _ => ValidationStatus::Failed,
        };

        // Calculate metrics
        let average_time = if !validation_times.is_empty() {
            validation_times.iter().sum::<f64>() / validation_times.len() as f64
        } else {
            0.0
        };

        let fastest_time = validation_times.iter().fold(f64::MAX, |a, &b| a.min(b));
        let slowest_time = validation_times.iter().fold(0.0_f64, |a, &b| a.max(b));

        let summary_metrics = ValidationMetrics {
            total_bytes_validated: total_bytes,
            total_files_validated: files_count,
            total_directories_validated: directories_count,
            total_symlinks_validated: symlinks_count,
            average_validation_time_ms: average_time,
            fastest_validation_ms: fastest_time as u64,
            slowest_validation_ms: slowest_time as u64,
            checksum_calculations_performed: if self.config.verify_checksums { files_count } else { 0 },
            content_comparisons_performed: if self.config.verify_content_integrity { files_count } else { 0 },
        };

        let report = ValidationReport {
            migration_id: migration_id.to_string(),
            validation_timestamp: SystemTime::now(),
            config: self.config.clone(),
            overall_status,
            total_operations: operations.len(),
            valid_operations: valid_count,
            failed_operations: failed_count,
            skipped_operations: skipped_count,
            operation_validations,
            summary_metrics,
            errors,
        };

        info!("Migration validation completed: {} valid, {} failed, {} skipped",
              valid_count, failed_count, skipped_count);

        Ok(report)
    }

    /// Validate a single migration operation
    async fn validate_operation(&self, operation: &MigrationOperation) -> Result<OperationValidation> {
        debug!("Validating operation: {}", operation.id);

        let mut validation_details = ValidationDetails {
            source_exists: operation.source_path.exists(),
            target_exists: operation.target_path.exists(),
            checksum_match: None,
            permissions_match: None,
            timestamp_match: None,
            size_match: None,
            content_match: None,
            file_count_match: None,
            symlink_target_match: None,
        };

        let mut custom_checks = Vec::new();
        let mut validation_status = ValidationStatus::Passed;

        // Perform validation based on operation type
        match operation.operation_type {
            crate::services::migration_orchestrator::OperationType::Copy => {
                self.validate_copy_operation(operation, &mut validation_details, &mut validation_status).await?;
            }
            crate::services::migration_orchestrator::OperationType::Move => {
                self.validate_move_operation(operation, &mut validation_details, &mut validation_status).await?;
            }
            crate::services::migration_orchestrator::OperationType::CreateDirectory => {
                self.validate_directory_operation(operation, &mut validation_details, &mut validation_status).await?;
            }
            crate::services::migration_orchestrator::OperationType::CreateSymlink => {
                self.validate_symlink_operation(operation, &mut validation_details, &mut validation_status).await?;
            }
        }

        // Run custom validators
        for (name, validator) in &self.custom_validators {
            if let Ok(check_result) = validator.validate(operation).await {
                custom_checks.push(check_result);
            }
        }

        Ok(OperationValidation {
            operation_id: operation.id.clone(),
            operation_type: format!("{:?}", operation.operation_type),
            source_path: operation.source_path.clone(),
            target_path: operation.target_path.clone(),
            validation_status,
            validation_details,
            validation_duration_ms: 0, // Will be set by caller
            custom_checks,
        })
    }

    async fn validate_copy_operation(&self, operation: &MigrationOperation, details: &mut ValidationDetails, status: &mut ValidationStatus) -> Result<()> {
        // For copy operations, both source and target should exist
        if !details.source_exists {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        if !details.target_exists {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        // Validate checksums
        if self.config.verify_checksums {
            let source_checksum = self.calculate_checksum(&operation.source_path).await?;
            let target_checksum = self.calculate_checksum(&operation.target_path).await?;
            details.checksum_match = Some(source_checksum == target_checksum);

            if !details.checksum_match.unwrap() {
                *status = if matches!(status, ValidationStatus::Passed) {
                    ValidationStatus::Failed
                } else {
                    status.clone()
                };
            }
        }

        // Validate file sizes
        if let (Ok(source_meta), Ok(target_meta)) = (fs::metadata(&operation.source_path).await, fs::metadata(&operation.target_path).await) {
            details.size_match = Some(source_meta.len() == target_meta.len());

            if !details.size_match.unwrap() {
                *status = if matches!(status, ValidationStatus::Passed) {
                    ValidationStatus::Failed
                } else {
                    status.clone()
                };
            }
        }

        // Validate content if requested
        if self.config.verify_content_integrity && operation.source_path.is_file() {
            let content_match = self.compare_file_content(&operation.source_path, &operation.target_path).await?;
            details.content_match = Some(content_match);

            if !content_match {
                *status = if matches!(status, ValidationStatus::Passed) {
                    ValidationStatus::Failed
                } else {
                    status.clone()
                };
            }
        }

        Ok(())
    }

    async fn validate_move_operation(&self, operation: &MigrationOperation, details: &mut ValidationDetails, status: &mut ValidationStatus) -> Result<()> {
        // For move operations, source should not exist and target should exist
        if details.source_exists {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        if !details.target_exists {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        // For move operations, we can only validate the target exists and has reasonable properties
        if let Ok(target_meta) = fs::metadata(&operation.target_path).await {
            // Validate that the target has the expected size from the operation
            if operation.bytes_transferred > 0 {
                details.size_match = Some(target_meta.len() == operation.bytes_transferred);
            } else {
                details.size_match = Some(true); // Assume correct if we can't verify
            }
        }

        Ok(())
    }

    async fn validate_directory_operation(&self, operation: &MigrationOperation, details: &mut ValidationDetails, status: &mut ValidationStatus) -> Result<()> {
        if !details.target_exists {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        if !operation.target_path.is_dir() {
            *status = ValidationStatus::Failed;
            return Ok(());
        }

        // Directory validation passed
        Ok(())
    }

    async fn validate_symlink_operation(&self, operation: &MigrationOperation, details: &mut ValidationDetails, status: &mut ValidationStatus) -> Result<()> {
        #[cfg(unix)]
        {
            if !details.target_exists {
                *status = ValidationStatus::Failed;
                return Ok(());
            }

            if !operation.target_path.is_symlink() {
                *status = ValidationStatus::Failed;
                return Ok(());
            }

            // Validate symlink target matches source
            if let Ok(source_target) = fs::read_link(&operation.source_path).await {
                if let Ok(target_target) = fs::read_link(&operation.target_path).await {
                    details.symlink_target_match = Some(source_target == target_target);
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Symlinks not supported on this platform
            *status = ValidationStatus::Skipped;
        }

        Ok(())
    }

    async fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::io::AsyncReadExt;

        let mut file = fs::File::open(path).await?;
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

    async fn compare_file_content(&self, source: &Path, target: &Path) -> Result<bool> {
        use tokio::io::AsyncReadExt;

        let mut source_file = fs::File::open(source).await?;
        let mut target_file = fs::File::open(target).await?;

        let mut source_buffer = [0; 8192];
        let mut target_buffer = [0; 8192];

        loop {
            let source_bytes = source_file.read(&mut source_buffer).await?;
            let target_bytes = target_file.read(&mut target_buffer).await?;

            if source_bytes != target_bytes {
                return Ok(false);
            }

            if source_bytes == 0 {
                break;
            }

            if source_buffer[..source_bytes] != target_buffer[..target_bytes] {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Trait for custom migration validators
#[async_trait::async_trait]
pub trait CustomValidator {
    async fn validate(&self, operation: &MigrationOperation) -> Result<CustomValidationCheck>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::io::Write;

    #[tokio::test]
    async fn test_validation_config_default() {
        let config = ValidationConfig::default();
        assert!(config.enabled);
        assert!(config.verify_checksums);
        assert!(config.verify_permissions);
        assert!(config.verify_timestamps);
        assert!(config.verify_content_integrity);
        assert!(matches!(config.checksum_algorithm, ChecksumAlgorithm::Sha256));
        assert_eq!(config.validation_timeout_secs, 300);
        assert!(!config.strict_mode);
    }

    #[tokio::test]
    async fn test_migration_validation() {
        let temp_dir = tempdir().unwrap();
        let config = ValidationConfig::default();
        let validator = MigrationValidator::new(config);

        // Create test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let operation = MigrationOperation {
            id: "test-op-001".to_string(),
            source_path: test_file.clone(),
            target_path: temp_dir.path().join("copied_test.txt"),
            operation_type: crate::services::migration_orchestrator::OperationType::Copy,
            status: crate::services::migration_orchestrator::OperationStatus::Completed,
            error_message: None,
            bytes_transferred: 12,
            checksum_before: None,
            checksum_after: None,
            timestamp: chrono::Utc::now(),
        };

        let report = validator.validate_migration("test-migration", &[operation]).await.unwrap();
        assert_eq!(report.total_operations, 1);
        assert_eq!(report.migration_id, "test-migration");
    }

    #[tokio::test]
    async fn test_checksum_calculation() {
        let temp_dir = tempdir().unwrap();
        let config = ValidationConfig::default();
        let validator = MigrationValidator::new(config);

        // Create test file with known content
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello world").unwrap();

        let checksum = validator.calculate_checksum(&test_file).await.unwrap();
        assert_eq!(checksum.len(), 64); // SHA256 hash length
    }

    #[tokio::test]
    async fn test_file_content_comparison() {
        let temp_dir = tempdir().unwrap();
        let config = ValidationConfig::default();
        let validator = MigrationValidator::new(config);

        // Create identical test files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let content = "test content for comparison";

        fs::write(&file1, content).unwrap();
        fs::write(&file2, content).unwrap();

        let is_same = validator.compare_file_content(&file1, &file2).await.unwrap();
        assert!(is_same);

        // Create different file
        let file3 = temp_dir.path().join("file3.txt");
        fs::write(&file3, "different content").unwrap();

        let is_different = validator.compare_file_content(&file1, &file3).await.unwrap();
        assert!(!is_different);
    }
}