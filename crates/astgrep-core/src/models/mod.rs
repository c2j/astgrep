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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_status_equality() {
        assert_eq!(ValidationStatus::Valid, ValidationStatus::Valid);
        assert_eq!(ValidationStatus::Invalid, ValidationStatus::Invalid);
        assert_eq!(ValidationStatus::Skipped, ValidationStatus::Skipped);
        assert_eq!(ValidationStatus::Warning, ValidationStatus::Warning);
    }

    #[test]
    fn test_validation_status_inequality() {
        assert_ne!(ValidationStatus::Valid, ValidationStatus::Invalid);
        assert_ne!(ValidationStatus::Skipped, ValidationStatus::Warning);
    }

    #[test]
    fn test_validation_result_new() {
        let path = PathBuf::from("/assets/test.java");
        let result = ValidationResult::new(
            path.clone(),
            ValidationStatus::Valid,
            "All checks passed".to_string(),
        );
        assert_eq!(result.asset_path, path);
        assert_eq!(result.status, ValidationStatus::Valid);
        assert_eq!(result.message, "All checks passed");
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_validation_result_new_invalid() {
        let result = ValidationResult::new(
            PathBuf::from("/bad/asset.txt"),
            ValidationStatus::Invalid,
            "Syntax error on line 5".to_string(),
        );
        assert_eq!(result.status, ValidationStatus::Invalid);
        assert_eq!(result.message, "Syntax error on line 5");
    }

    #[test]
    fn test_validation_result_new_skipped() {
        let result = ValidationResult::new(
            PathBuf::from("/skip.me"),
            ValidationStatus::Skipped,
            "File excluded by config".to_string(),
        );
        assert_eq!(result.status, ValidationStatus::Skipped);
    }

    #[test]
    fn test_validation_result_with_metadata_single() {
        let result = ValidationResult::new(
            PathBuf::from("/assets/test.java"),
            ValidationStatus::Valid,
            "ok".to_string(),
        )
        .with_metadata("rule_count".to_string(), "3".to_string());

        assert_eq!(result.metadata.get("rule_count"), Some(&"3".to_string()));
        assert_eq!(result.metadata.len(), 1);
    }

    #[test]
    fn test_validation_result_with_metadata_chained() {
        let result = ValidationResult::new(
            PathBuf::from("/assets/test.java"),
            ValidationStatus::Warning,
            "Minor issues found".to_string(),
        )
        .with_metadata("key1".to_string(), "value1".to_string())
        .with_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(result.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(result.metadata.get("key2"), Some(&"value2".to_string()));
        assert_eq!(result.metadata.len(), 2);
    }

    #[test]
    fn test_validation_result_with_metadata_overwrite() {
        let result = ValidationResult::new(
            PathBuf::from("/assets/test.java"),
            ValidationStatus::Valid,
            "ok".to_string(),
        )
        .with_metadata("key".to_string(), "old".to_string())
        .with_metadata("key".to_string(), "new".to_string());

        assert_eq!(result.metadata.get("key"), Some(&"new".to_string()));
        assert_eq!(result.metadata.len(), 1);
    }

    #[test]
    fn test_test_asset_construction() {
        let asset = TestAsset {
            path: PathBuf::from("/src/main.java"),
            relative_path: PathBuf::from("src/main.java"),
            asset_type: AssetType::Script,
            content: "public class Main {}".to_string(),
            shebang: None,
            size_bytes: 1024,
            checksum: "abc123".to_string(),
            detected_language: Some("java".to_string()),
            metadata: std::collections::HashMap::new(),
        };

        assert_eq!(asset.path, PathBuf::from("/src/main.java"));
        assert_eq!(asset.relative_path, PathBuf::from("src/main.java"));
        assert_eq!(asset.asset_type, AssetType::Script);
        assert_eq!(asset.content, "public class Main {}");
        assert_eq!(asset.shebang, None);
        assert_eq!(asset.size_bytes, 1024);
        assert_eq!(asset.checksum, "abc123");
        assert_eq!(asset.detected_language, Some("java".to_string()));
        assert!(asset.metadata.is_empty());
    }

    #[test]
    fn test_test_asset_with_shebang() {
        let asset = TestAsset {
            path: PathBuf::from("/scripts/run.sh"),
            relative_path: PathBuf::from("scripts/run.sh"),
            asset_type: AssetType::Script,
            content: "#!/bin/bash\necho hello".to_string(),
            shebang: Some("#!/bin/bash".to_string()),
            size_bytes: 25,
            checksum: "def456".to_string(),
            detected_language: Some("bash".to_string()),
            metadata: std::collections::HashMap::new(),
        };

        assert_eq!(asset.shebang, Some("#!/bin/bash".to_string()));
        assert_eq!(asset.detected_language, Some("bash".to_string()));
    }

    #[test]
    fn test_test_asset_minimal_fields() {
        let asset = TestAsset {
            path: PathBuf::from("/f.txt"),
            relative_path: PathBuf::from("f.txt"),
            asset_type: AssetType::Fixture,
            content: String::new(),
            shebang: None,
            size_bytes: 0,
            checksum: String::new(),
            detected_language: None,
            metadata: std::collections::HashMap::new(),
        };

        assert_eq!(asset.size_bytes, 0);
        assert!(asset.content.is_empty());
        assert!(asset.detected_language.is_none());
    }

    #[test]
    fn test_asset_type_variants() {
        assert_eq!(AssetType::Script, AssetType::Script);
        assert_eq!(AssetType::TestCase, AssetType::TestCase);
        assert_eq!(AssetType::Fixture, AssetType::Fixture);
        assert_eq!(AssetType::RuleDefinition, AssetType::RuleDefinition);
        assert_eq!(AssetType::Report, AssetType::Report);
        assert_ne!(AssetType::Script, AssetType::TestCase);
    }
}
