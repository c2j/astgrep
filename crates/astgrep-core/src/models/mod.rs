//! Models module for ASTGreP core types

pub mod test_asset;
pub mod test_case;

// Re-export commonly used types (avoid conflicts with glob imports)
pub use test_asset::{ScriptType, TestScript};
// Re-export specific test_case types to avoid conflicts
pub use test_case::{
    LanguageConfig, LanguageMapping, MigrationDifficulty, TestCase, TestCaseMetadata,
    TestCaseStatus, TestCategory, TestComplexity, TestPriority, TestType,
};

/// Additional validation result types for script execution
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Validation status for asset verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// Asset passed validation
    Valid,
    /// Asset failed validation
    Invalid,
    /// Asset was skipped during validation
    Skipped,
    /// Asset has warnings but is considered valid
    Warning,
}

/// Result of validating a test asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Asset path
    pub asset_path: PathBuf,
    /// Validation status
    pub status: ValidationStatus,
    /// Validation message
    pub message: String,
    /// Validation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl ValidationResult {
    pub fn new(asset_path: PathBuf, status: ValidationStatus, message: String) -> Self {
        Self {
            asset_path,
            status,
            message,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Simplified test asset for compatibility with existing modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAsset {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub asset_type: AssetType,
    pub content: String,
    pub shebang: Option<String>,
    pub size_bytes: u64,
    pub checksum: String,
    pub detected_language: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Asset type alias for compatibility
pub type AssetType = test_asset::AssetType;
