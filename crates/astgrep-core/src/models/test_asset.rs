//! Test asset models for migration operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Represents any test-related file or directory in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAsset {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub current_path: PathBuf,
    pub target_path: PathBuf,
    pub language: Option<String>,
    pub category: Option<String>,
    pub status: AssetStatus,
    pub dependencies: Vec<String>,
    pub metadata: AssetMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    Script,
    TestCase,
    Fixture,
    RuleDefinition,
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    Pending,
    InProgress,
    Migrated,
    Verified,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub file_size: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
    pub executable: bool,
    pub platforms: Vec<String>,
}

/// Represents an executable test script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScript {
    pub asset_id: String,
    pub script_type: ScriptType,
    pub platforms: Vec<String>,
    pub execution_order: i32,
    pub arguments: Vec<ScriptArgument>,
    pub exit_codes: ScriptExitCodes,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScriptType {
    Validator,
    Runner,
    Utility,
    CiIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptArgument {
    pub name: String,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub required: bool,
    pub takes_value: bool,
    pub default_value: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExitCodes {
    pub success: i32,
    pub failure: i32,
    pub warning: Option<i32>,
}

/// Represents a collection of test files for specific functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub asset_id: String,
    pub test_type: TestType,
    pub languages: Vec<String>,
    pub rule_files: Vec<PathBuf>,
    pub source_files: Vec<PathBuf>,
    pub expected_results: Vec<PathBuf>,
    pub complexity: TestComplexity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    PatternMatching,
    RuleValidation,
    Parsing,
    Integration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestComplexity {
    Simple,
    Medium,
    Complex,
}

/// Defines the target organization hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryStructure {
    pub root_path: PathBuf,
    pub categories: Vec<CategoryDefinition>,
    pub naming_convention: NamingConvention,
    pub depth_limit: u32,
    pub migration_rules: Vec<MigrationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDefinition {
    pub name: String,
    pub path_pattern: String,
    pub description: String,
    pub target_subdirectory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConvention {
    pub file_case: String,
    pub directory_case: String,
    pub separator: String,
    pub max_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRule {
    pub pattern: String,
    pub action: RuleAction,
    pub conditions: Vec<RuleCondition>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    Include,
    Exclude,
    Transform,
    Validate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
    InList,
}

impl TestAsset {
    pub fn new(
        id: String,
        name: String,
        asset_type: AssetType,
        current_path: PathBuf,
        target_path: PathBuf,
    ) -> Self {
        Self {
            id,
            name,
            asset_type,
            current_path,
            target_path,
            language: None,
            category: None,
            status: AssetStatus::Pending,
            dependencies: Vec::new(),
            metadata: AssetMetadata {
                file_size: None,
                created_at: None,
                modified_at: None,
                executable: false,
                platforms: Vec::new(),
            },
        }
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn mark_in_progress(&mut self) {
        self.status = AssetStatus::InProgress;
    }

    pub fn mark_migrated(&mut self) {
        self.status = AssetStatus::Migrated;
    }

    pub fn mark_verified(&mut self) {
        self.status = AssetStatus::Verified;
    }

    pub fn mark_failed(&mut self, _error: &str) {
        self.status = AssetStatus::Failed;
        // Add error information to metadata if needed
    }

    pub fn mark_skipped(&mut self) {
        self.status = AssetStatus::Skipped;
    }

    pub fn is_migratable(&self) -> bool {
        matches!(self.status, AssetStatus::Pending | AssetStatus::Failed)
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    pub fn get_file_size(&self) -> Option<u64> {
        self.metadata.file_size.or_else(|| {
            std::fs::metadata(&self.current_path)
                .ok()
                .map(|m| m.len() as u64)
        })
    }

    pub fn is_executable(&self) -> bool {
        self.metadata.executable || self.asset_type == AssetType::Script
    }

    pub fn is_platform_supported(&self, platform: &str) -> bool {
        self.metadata.platforms.contains(&platform.to_string()) || self.metadata.platforms.is_empty()
    }
}

impl TestScript {
    pub fn new(
        asset_id: String,
        script_type: ScriptType,
        platforms: Vec<String>,
    ) -> Self {
        Self {
            asset_id,
            script_type,
            platforms,
            execution_order: 0,
            arguments: Vec::new(),
            exit_codes: ScriptExitCodes {
                success: 0,
                failure: 1,
                warning: None,
            },
        }
    }

    pub fn with_execution_order(mut self, order: i32) -> Self {
        self.execution_order = order;
        self
    }

    pub fn with_arguments(mut self, arguments: Vec<ScriptArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_exit_codes(mut self, exit_codes: ScriptExitCodes) -> Self {
        self.exit_codes = exit_codes;
        self
    }
}

impl ScriptArgument {
    pub fn new(
        name: &str,
        description: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            short_name: None,
            long_name: Some(name.to_string()),
            required: false,
            takes_value: false,
            default_value: None,
            description: description.to_string(),
        }
    }

    pub fn short(mut self, short: &str) -> Self {
        self.short_name = Some(short.to_string());
        self
    }

    pub fn long(mut self, long: &str) -> Self {
        self.long_name = Some(long.to_string());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn takes_value(mut self) -> Self {
        self.takes_value = true;
        self
    }

    pub fn default_value(mut self, value: &str) -> Self {
        self.default_value = Some(value.to_string());
        self
    }
}

impl TestCase {
    pub fn new(
        asset_id: String,
        test_type: TestType,
        languages: Vec<String>,
    ) -> Self {
        Self {
            asset_id,
            test_type,
            languages,
            rule_files: Vec::new(),
            source_files: Vec::new(),
            expected_results: Vec::new(),
            complexity: TestComplexity::Simple,
        }
    }

    pub fn with_complexity(mut self, complexity: TestComplexity) -> Self {
        self.complexity = complexity;
        self
    }

    pub fn with_rule_files(mut self, files: Vec<PathBuf>) -> Self {
        self.rule_files = files;
        self
    }

    pub fn with_source_files(mut self, files: Vec<PathBuf>) -> Self {
        self.source_files = files;
        self
    }

    pub fn with_expected_results(mut self, files: Vec<PathBuf>) -> Self {
        self.expected_results = files;
        self
    }

    pub fn supports_language(&self, language: &str) -> bool {
        self.languages.iter().any(|lang| lang == language)
    }

    pub fn get_primary_language(&self) -> Option<&String> {
        self.languages.first()
    }
}

impl DirectoryStructure {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            categories: Vec::new(),
            naming_convention: NamingConvention {
                file_case: "kebab-case".to_string(),
                directory_case: "kebab-case".to_string(),
                separator: "-".to_string(),
                max_length: 255,
            },
            depth_limit: 5,
            migration_rules: Vec::new(),
        }
    }

    pub fn with_categories(mut self, categories: Vec<CategoryDefinition>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_naming_convention(mut self, convention: NamingConvention) -> Self {
        self.naming_convention = convention;
        self
    }

    pub fn with_depth_limit(mut self, limit: u32) -> Self {
        self.depth_limit = limit;
        self
    }

    pub fn add_category(mut self, category: CategoryDefinition) -> Self {
        self.categories.push(category);
        self
    }
}

impl CategoryDefinition {
    pub fn new(
        name: &str,
        path_pattern: &str,
        description: &str,
        target_subdirectory: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            path_pattern: path_pattern.to_string(),
            description: description.to_string(),
            target_subdirectory: target_subdirectory.to_string(),
        }
    }
}

impl MigrationRule {
    pub fn new(
        pattern: &str,
        action: RuleAction,
        conditions: Vec<RuleCondition>,
        priority: i32,
    ) -> Self {
        Self {
            pattern: pattern.to_string(),
            action,
            conditions,
            priority,
        }
    }

    pub fn condition(mut self, field: &str, operator: ConditionOperator, value: &str) -> Self {
        self.conditions.push(RuleCondition {
            field: field.to_string(),
            operator,
            value: value.to_string(),
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_test_asset_creation() {
        let asset = TestAsset::new(
            "asset-001".to_string(),
            "Test Script".to_string(),
            AssetType::Script,
            "/tests/validate.sh".into(),
            "/newtest/scripts/runners/validate.sh".into(),
        );

        assert_eq!(asset.id, "asset-001");
        assert_eq!(asset.name, "Test Script");
        assert!(matches!(asset.asset_type, AssetType::Script));
        assert_eq!(asset.status, AssetStatus::Pending);
    }

    #[test]
    fn test_test_asset_lifecycle() {
        let mut asset = TestAsset::new(
            "asset-001".to_string(),
            "Test Script".to_string(),
            AssetType::Script,
            "/tests/validate.sh".into(),
            "/newtest/scripts/runners/validate.sh".into(),
        );

        asset.mark_in_progress();
        assert!(matches!(asset.status, AssetStatus::InProgress));

        asset.mark_migrated();
        assert!(matches!(asset.status, AssetStatus::Migrated));

        asset.mark_verified();
        assert!(matches!(asset.status, AssetStatus::Verified));

        asset.mark_failed("Test error");
        assert!(matches!(asset.status, AssetStatus::Failed));
    }

    #[test]
    fn test_test_script_creation() {
        let script = TestScript::new(
            "script-001".to_string(),
            ScriptType::Validator,
            vec!["Linux".to_string(), "macOS".to_string()],
        );

        assert_eq!(script.asset_id, "script-001");
        assert!(matches!(script.script_type, ScriptType::Validator));
        assert_eq!(script.platforms.len(), 2);
        assert_eq!(script.exit_codes.success, 0);
        assert_eq!(script.exit_codes.failure, 1);
    }

    #[test]
    fn test_test_case_creation() {
        let test_case = TestCase::new(
            "test-001".to_string(),
            TestType::PatternMatching,
            vec!["python".to_string(), "javascript".to_string()],
        );

        assert_eq!(test_case.asset_id, "test-001");
        assert!(matches!(test_case.test_type, TestType::PatternMatching));
        assert_eq!(test_case.languages.len(), 2);
        assert!(test_case.supports_language("python"));
        assert!(test_case.supports_language("javascript"));
        assert!(!test_case.supports_language("java"));
        assert_eq!(test_case.get_primary_language().unwrap(), "python");
    }

    #[test]
    fn test_directory_structure_creation() {
        let root = PathBuf::from("/test/newtest");
        let structure = DirectoryStructure::new(root.clone());

        assert_eq!(structure.root_path, root);
        assert_eq!(structure.categories.len(), 0);
        assert_eq!(structure.depth_limit, 5);
        assert_eq!(structure.naming_convention.separator, "-");
    }

    #[test]
    fn test_script_argument_creation() {
        let arg = ScriptArgument {
            name: "verbose".to_string(),
            short_name: Some("v".to_string()),
            long_name: Some("verbose".to_string()),
            required: false,
            takes_value: false,
            default_value: None,
            description: "Enable verbose logging".to_string(),
        };

        assert_eq!(arg.name, "verbose");
        assert_eq!(arg.short_name.as_ref().unwrap(), "v");
        assert_eq!(arg.long_name.as_ref().unwrap(), "verbose");
        assert!(!arg.required);
        assert!(!arg.takes_value);
    }
}