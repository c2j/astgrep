//! Test case models for migration operations

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use chrono::{DateTime, Utc};

/// Represents a collection of test files for specific functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Unique identifier for this test case
    pub asset_id: String,
    /// Test case name or identifier
    pub name: String,
    /// Type of test (pattern matching, validation, etc.)
    pub test_type: TestType,
    /// Programming languages this test case applies to
    pub languages: Vec<String>,
    /// ASTGreP rule files associated with this test case
    pub rule_files: Vec<PathBuf>,
    /// Source code files to be tested
    pub source_files: Vec<PathBuf>,
    /// Expected result files or output
    pub expected_results: Vec<PathBuf>,
    /// Complexity level of the test
    pub complexity: TestComplexity,
    /// Current status in migration process
    pub status: TestCaseStatus,
    /// Target location in new directory structure
    pub target_path: PathBuf,
    /// Current/original location
    pub current_path: PathBuf,
    /// Classification category
    pub category: Option<TestCategory>,
    /// Test metadata and annotations
    pub metadata: TestCaseMetadata,
    /// Dependencies on other test cases or assets
    pub dependencies: Vec<String>,
    /// Tags for better organization and discovery
    pub tags: Vec<String>,
    /// Test case description
    pub description: Option<String>,
}

/// Types of test cases
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestType {
    /// Pattern matching test cases
    PatternMatching,
    /// Rule validation test cases
    RuleValidation,
    /// Parsing and AST generation tests
    Parsing,
    /// End-to-end integration tests
    Integration,
    /// Performance benchmark tests
    Performance,
    /// Security vulnerability tests
    Security,
    /// Compatibility tests
    Compatibility,
    /// Data flow analysis tests
    DataFlow,
    /// Custom or miscellaneous tests
    Custom,
}

/// Complexity levels for test cases
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TestComplexity {
    /// Simple test cases with basic functionality
    Simple,
    /// Medium complexity with multiple components
    Medium,
    /// Complex test cases with advanced features
    Complex,
    /// Expert-level test cases requiring deep understanding
    Expert,
}

/// Status of test case in migration process
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestCaseStatus {
    /// Test case discovered but not yet processed
    Pending,
    /// Currently being processed or analyzed
    InProgress,
    /// Successfully migrated to new structure
    Migrated,
    /// Verified to work correctly in new location
    Verified,
    /// Migration failed
    Failed,
    /// Test case skipped during migration
    Skipped,
    /// Test case requires manual intervention
    RequiresManualAction,
}

/// Categories for organizing test cases
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestCategory {
    /// Basic functionality tests
    Basic,
    /// Language-specific tests
    LanguageSpecific,
    /// Framework or library tests
    Framework,
    /// Integration tests
    Integration,
    /// Performance benchmarks
    Performance,
    /// Security vulnerability tests
    Security,
    /// Compatibility tests
    Compatibility,
    /// Regression tests
    Regression,
    /// Edge case tests
    EdgeCase,
    /// Custom or uncategorized tests
    Other(String),
}

/// Metadata associated with a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseMetadata {
    /// File size in bytes
    pub file_size: u64,
    /// Number of lines of code
    pub line_count: usize,
    /// Creation timestamp
    pub created_at: Option<DateTime<Utc>>,
    /// Last modification timestamp
    pub modified_at: Option<DateTime<Utc>>,
    /// Author or creator
    pub author: Option<String>,
    /// Version or revision
    pub version: Option<String>,
    /// Test framework or tool used
    pub framework: Option<String>,
    /// Execution environment requirements
    pub environment_requirements: Vec<String>,
    /// Estimated execution time in seconds
    pub estimated_execution_time: Option<u64>,
    /// Test priority
    pub priority: TestPriority,
    /// Additional custom properties
    pub custom_properties: HashMap<String, String>,
}

/// Priority levels for test execution
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TestPriority {
    /// Critical tests (must pass)
    Critical,
    /// High priority tests
    High,
    /// Normal priority tests
    Normal,
    /// Low priority tests
    Low,
}

/// Result of migrating a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseMigrationResult {
    /// Test case that was migrated
    pub test_case: TestCase,
    /// Whether the migration was successful
    pub success: bool,
    /// Original path before migration
    pub original_path: PathBuf,
    /// New path after migration
    pub new_path: PathBuf,
    /// Migration timestamp
    pub migrated_at: DateTime<Utc>,
    /// Execution time in milliseconds
    pub migration_time_ms: u64,
    /// Any warnings or messages during migration
    pub warnings: Vec<String>,
    /// Error message if migration failed
    pub error_message: Option<String>,
    /// Files that were created or modified
    pub affected_files: Vec<PathBuf>,
    /// Verification results
    pub verification_results: Vec<VerificationResult>,
}

/// Result of validating a migrated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Type of verification performed
    pub verification_type: VerificationType,
    /// Whether verification passed
    pub passed: bool,
    /// Description of what was verified
    pub description: String,
    /// Additional details or context
    pub details: Option<String>,
    /// Time taken for verification in milliseconds
    pub verification_time_ms: u64,
}

/// Types of verification that can be performed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationType {
    /// Verify file exists and is readable
    FileExistence,
    /// Verify file structure is correct
    FileStructure,
    /// Verify content integrity (checksum)
    ContentIntegrity,
    /// Verify rule file syntax
    RuleSyntax,
    /// Verify test execution produces expected results
    TestExecution,
    /// Verify dependencies are available
    Dependencies,
    /// Verify performance benchmarks
    Performance,
}

/// Collection of test cases with organization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseCollection {
    /// Collection identifier
    pub collection_id: String,
    /// Collection name
    pub name: String,
    /// Description of the collection
    pub description: Option<String>,
    /// Test cases in this collection
    pub test_cases: Vec<TestCase>,
    /// Language distribution
    pub language_distribution: HashMap<String, usize>,
    /// Type distribution
    pub type_distribution: HashMap<TestType, usize>,
    /// Complexity distribution
    pub complexity_distribution: HashMap<TestComplexity, usize>,
    /// Category distribution
    pub category_distribution: HashMap<String, usize>,
    /// Collection metadata
    pub metadata: CollectionMetadata,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Metadata for a test case collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Total number of test cases
    pub total_test_cases: usize,
    /// Total size of all files in bytes
    pub total_size_bytes: u64,
    /// Languages covered by this collection
    pub covered_languages: Vec<String>,
    /// Test frameworks used
    pub frameworks: Vec<String>,
    /// Estimated total execution time
    pub estimated_execution_time_sec: u64,
    /// Collection owner or maintainer
    pub maintainer: Option<String>,
    /// Collection tags
    pub tags: Vec<String>,
    /// Version of the collection
    pub version: Option<String>,
    /// License information
    pub license: Option<String>,
}

/// Language mapping configuration for test case organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMapping {
    /// Mapping from file extensions to language names
    pub extension_to_language: HashMap<String, String>,
    /// Mapping from file patterns to language names
    pub pattern_to_language: HashMap<String, String>,
    /// Default language for unknown files
    pub default_language: String,
    /// Language-specific directory configurations
    pub language_configs: HashMap<String, LanguageConfig>,
}

/// Configuration for a specific language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Language name
    pub language: String,
    /// Directory name for this language
    pub directory_name: String,
    /// File extensions for this language
    pub extensions: Vec<String>,
    /// Test types commonly used for this language
    pub common_test_types: Vec<TestType>,
    /// Framework associations
    pub frameworks: Vec<String>,
    /// Default category for tests of this language
    pub default_category: TestCategory,
    /// Specific patterns for test files
    pub test_file_patterns: Vec<String>,
}

impl TestCase {
    /// Create a new test case
    pub fn new(
        asset_id: String,
        name: String,
        test_type: TestType,
        current_path: PathBuf,
        target_path: PathBuf,
    ) -> Self {
        Self {
            asset_id,
            name,
            test_type,
            languages: Vec::new(),
            rule_files: Vec::new(),
            source_files: Vec::new(),
            expected_results: Vec::new(),
            complexity: TestComplexity::Medium,
            status: TestCaseStatus::Pending,
            current_path,
            target_path,
            category: None,
            metadata: TestCaseMetadata::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            description: None,
        }
    }

    /// Add a language to this test case
    pub fn with_language(mut self, language: String) -> Self {
        if !self.languages.contains(&language) {
            self.languages.push(language);
        }
        self
    }

    /// Add multiple languages to this test case
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        for language in languages {
            if !self.languages.contains(&language) {
                self.languages.push(language);
            }
        }
        self
    }

    /// Set the complexity level
    pub fn with_complexity(mut self, complexity: TestComplexity) -> Self {
        self.complexity = complexity;
        self
    }

    /// Add a rule file
    pub fn with_rule_file(mut self, rule_file: PathBuf) -> Self {
        self.rule_files.push(rule_file);
        self
    }

    /// Add multiple rule files
    pub fn with_rule_files(mut self, rule_files: Vec<PathBuf>) -> Self {
        self.rule_files.extend(rule_files);
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: TestCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Add tags for organization
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Mark test case as in progress
    pub fn mark_in_progress(&mut self) {
        self.status = TestCaseStatus::InProgress;
    }

    /// Mark test case as migrated
    pub fn mark_migrated(&mut self) {
        self.status = TestCaseStatus::Migrated;
    }

    /// Mark test case as verified
    pub fn mark_verified(&mut self) {
        self.status = TestCaseStatus::Verified;
    }

    /// Mark test case as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.status = TestCaseStatus::Failed;
        // Add error information to metadata if needed
    }

    /// Mark test case as skipped
    pub fn mark_skipped(&mut self) {
        self.status = TestCaseStatus::Skipped;
    }

    /// Mark test case as requiring manual action
    pub fn mark_requires_manual_action(&mut self, reason: &str) {
        self.status = TestCaseStatus::RequiresManualAction;
        // Add reason to metadata if needed
    }

    /// Get primary language (first in list)
    pub fn primary_language(&self) -> Option<&String> {
        self.languages.first()
    }

    /// Check if test case supports a specific language
    pub fn supports_language(&self, language: &str) -> bool {
        self.languages.iter().any(|lang| lang == language)
    }

    /// Check if test case has any dependencies
    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    /// Get relative path from project root
    pub fn relative_path(&self) -> Option<PathBuf> {
        self.current_path.strip_prefix(".").ok().map(PathBuf::from)
    }

    /// Validate test case structure
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if self.asset_id.is_empty() {
            issues.push("Asset ID cannot be empty".to_string());
        }

        if self.name.is_empty() {
            issues.push("Test case name cannot be empty".to_string());
        }

        if self.current_path.as_os_str().is_empty() {
            issues.push("Current path cannot be empty".to_string());
        }

        if self.target_path.as_os_str().is_empty() {
            issues.push("Target path cannot be empty".to_string());
        }

        if self.languages.is_empty() {
            issues.push("Test case must specify at least one language".to_string());
        }

        issues
    }

    /// Get estimated migration difficulty
    pub fn migration_difficulty(&self) -> MigrationDifficulty {
        match (self.complexity.clone(), self.dependencies.len(), self.rule_files.len()) {
            (TestComplexity::Simple, 0, 0) => MigrationDifficulty::Easy,
            (TestComplexity::Simple, 0, _) => MigrationDifficulty::Medium,
            (TestComplexity::Simple, _, _) | (TestComplexity::Medium, _, _) => MigrationDifficulty::Medium,
            (TestComplexity::Complex, _, _) | (TestComplexity::Expert, _, _) => MigrationDifficulty::Hard,
        }
    }
}

/// Difficulty level for migrating test cases
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MigrationDifficulty {
    /// Easy migration with minimal dependencies
    Easy,
    /// Medium complexity with some considerations
    Medium,
    /// Complex migration requiring careful planning
    Hard,
}

impl Default for TestCaseMetadata {
    fn default() -> Self {
        Self {
            file_size: 0,
            line_count: 0,
            created_at: None,
            modified_at: None,
            author: None,
            version: None,
            framework: None,
            environment_requirements: Vec::new(),
            estimated_execution_time: None,
            priority: TestPriority::Normal,
            custom_properties: HashMap::new(),
        }
    }
}

impl Default for TestPriority {
    fn default() -> Self {
        TestPriority::Normal
    }
}

impl LanguageMapping {
    /// Create a new language mapping with default configurations
    pub fn new() -> Self {
        let extension_to_language = HashMap::from([
            ("java".to_string(), "java".to_string()),
            ("class".to_string(), "java".to_string()),
            ("py".to_string(), "python".to_string()),
            ("pyw".to_string(), "python".to_string()),
            ("js".to_string(), "javascript".to_string()),
            ("jsx".to_string(), "javascript".to_string()),
            ("ts".to_string(), "typescript".to_string()),
            ("tsx".to_string(), "typescript".to_string()),
            ("sql".to_string(), "sql".to_string()),
            ("sh".to_string(), "bash".to_string()),
            ("bash".to_string(), "bash".to_string()),
            ("zsh".to_string(), "bash".to_string()),
            ("php".to_string(), "php".to_string()),
            ("cs".to_string(), "csharp".to_string()),
            ("c".to_string(), "c".to_string()),
            ("h".to_string(), "c".to_string()),
            ("cpp".to_string(), "cpp".to_string()),
            ("hpp".to_string(), "cpp".to_string()),
            ("cc".to_string(), "cpp".to_string()),
            ("rb".to_string(), "ruby".to_string()),
            ("rbw".to_string(), "ruby".to_string()),
            ("kt".to_string(), "kotlin".to_string()),
            ("kts".to_string(), "kotlin".to_string()),
            ("swift".to_string(), "swift".to_string()),
            ("xml".to_string(), "xml".to_string()),
            ("xsd".to_string(), "xml".to_string()),
        ]);

        let language_configs = HashMap::from([
            ("java".to_string(), LanguageConfig {
                language: "java".to_string(),
                directory_name: "java".to_string(),
                extensions: vec!["java".to_string(), "class".to_string()],
                common_test_types: vec![
                    TestType::PatternMatching,
                    TestType::RuleValidation,
                    TestType::Integration,
                ],
                frameworks: vec!["JUnit".to_string()],
                default_category: TestCategory::LanguageSpecific,
                test_file_patterns: vec![
                    "*Test*.java".to_string(),
                    "*TestCase*.java".to_string(),
                    "Test*".to_string(),
                ],
            }),
            ("python".to_string(), LanguageConfig {
                language: "python".to_string(),
                directory_name: "python".to_string(),
                extensions: vec!["py".to_string(), "pyw".to_string()],
                common_test_types: vec![
                    TestType::PatternMatching,
                    TestType::RuleValidation,
                    TestType::Integration,
                ],
                frameworks: vec!["pytest".to_string(), "unittest".to_string()],
                default_category: TestCategory::LanguageSpecific,
                test_file_patterns: vec![
                    "test_*.py".to_string(),
                    "*_test.py".to_string(),
                    "tests.py".to_string(),
                    "test_*.py".to_string(),
                ],
            }),
            ("javascript".to_string(), LanguageConfig {
                language: "javascript".to_string(),
                directory_name: "javascript".to_string(),
                extensions: vec!["js".to_string(), "jsx".to_string()],
                common_test_types: vec![
                    TestType::PatternMatching,
                    TestType::RuleValidation,
                    TestType::Integration,
                ],
                frameworks: vec!["Jest".to_string(), "Mocha".to_string()],
                default_category: TestCategory::LanguageSpecific,
                test_file_patterns: vec![
                    "*.test.js".to_string(),
                    "*.spec.js".to_string(),
                    "test_*.js".to_string(),
                    "*_test.js".to_string(),
                ],
            }),
            ("sql".to_string(), LanguageConfig {
                language: "sql".to_string(),
                directory_name: "sql".to_string(),
                extensions: vec!["sql".to_string(), "ddl".to_string(), "dml".to_string()],
                common_test_types: vec![
                    TestType::PatternMatching,
                    TestType::RuleValidation,
                ],
                frameworks: vec!["sqlcheck".to_string()],
                default_category: TestCategory::LanguageSpecific,
                test_file_patterns: vec![
                    "*_test.sql".to_string(),
                    "test_*.sql".to_string(),
                    "*_validate.sql".to_string(),
                ],
            }),
        ]);

        Self {
            extension_to_language,
            pattern_to_language: HashMap::new(),
            default_language: "unknown".to_string(),
            language_configs,
        }
    }

    /// Detect language from file path and content
    pub fn detect_language(&self, file_path: &PathBuf, content: Option<&str>) -> String {
        // First try to detect from file extension
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            if let Some(language) = self.extension_to_language.get(extension) {
                return language.clone();
            }
        }

        // If content is provided, try pattern matching
        if let Some(content) = content {
            for (pattern, language) in &self.pattern_to_language {
                // Simple pattern matching - could be enhanced with regex
                if content.contains(pattern) {
                    return language.clone();
                }
            }
        }

        // Return default language
        self.default_language.clone()
    }

    /// Get configuration for a specific language
    pub fn get_language_config(&self, language: &str) -> Option<&LanguageConfig> {
        self.language_configs.get(language)
    }
}

impl TestCaseCollection {
    /// Create a new test case collection
    pub fn new(collection_id: String, name: String) -> Self {
        Self {
            collection_id,
            name,
            description: None,
            test_cases: Vec::new(),
            language_distribution: HashMap::new(),
            type_distribution: HashMap::new(),
            complexity_distribution: HashMap::new(),
            category_distribution: HashMap::new(),
            metadata: CollectionMetadata::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Add a test case to the collection
    pub fn add_test_case(&mut self, test_case: TestCase) {
        self.update_distributions(&test_case);
        self.test_cases.push(test_case);
        self.updated_at = Utc::now();
    }

    /// Add multiple test cases
    pub fn add_test_cases(&mut self, test_cases: Vec<TestCase>) {
        for test_case in test_cases {
            self.add_test_case(test_case);
        }
    }

    /// Update distribution statistics
    fn update_distributions(&mut self, test_case: &TestCase) {
        // Update language distribution
        for language in &test_case.languages {
            *self.language_distribution.entry(language.clone()).or_insert(0) += 1;
        }

        // Update type distribution
        *self.type_distribution.entry(test_case.test_type.clone()).or_insert(0) += 1;

        // Update complexity distribution
        *self.complexity_distribution.entry(test_case.complexity.clone()).or_insert(0) += 1;

        // Update category distribution
        if let Some(category) = &test_case.category {
            let category_key = match category {
                TestCategory::Other(ref name) => name.clone(),
                _ => format!("{:?}", category),
            };
            *self.category_distribution.entry(category_key).or_insert(0) += 1;
        }
    }

    /// Get statistics for the collection
    pub fn get_statistics(&self) -> &CollectionMetadata {
        &self.metadata
    }

    /// Update collection metadata
    pub fn update_metadata(&mut self) {
        self.metadata.total_test_cases = self.test_cases.len();

        let mut total_size = 0;
        let mut all_languages = std::collections::HashSet::new();
        let mut all_frameworks = std::collections::HashSet::new();
        let mut total_execution_time = 0u64;

        for test_case in &self.test_cases {
            total_size += test_case.metadata.file_size;

            for language in &test_case.languages {
                all_languages.insert(language.clone());
            }

            if let Some(framework) = &test_case.metadata.framework {
                all_frameworks.insert(framework.clone());
            }

            if let Some(execution_time) = test_case.metadata.estimated_execution_time {
                total_execution_time += execution_time;
            }
        }

        self.metadata.total_size_bytes = total_size;
        self.metadata.covered_languages = all_languages.into_iter().collect();
        self.metadata.frameworks = all_frameworks.into_iter().collect();
        self.metadata.estimated_execution_time_sec = total_execution_time;

        self.updated_at = Utc::now();
    }
}

impl Default for CollectionMetadata {
    fn default() -> Self {
        Self {
            total_test_cases: 0,
            total_size_bytes: 0,
            covered_languages: Vec::new(),
            frameworks: Vec::new(),
            estimated_execution_time_sec: 0,
            maintainer: None,
            tags: Vec::new(),
            version: None,
            license: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_test_case_creation() {
        let test_case = TestCase::new(
            "tc-001".to_string(),
            "Java Security Test".to_string(),
            TestType::Security,
            PathBuf::from("/tests/SecurityTest.java"),
            PathBuf::from("/newtest/testcases/java/security/SecurityTest.java"),
        );

        assert_eq!(test_case.asset_id, "tc-001");
        assert_eq!(test_case.name, "Java Security Test");
        assert_eq!(test_case.test_type, TestType::Security);
        assert_eq!(test_case.status, TestCaseStatus::Pending);
        assert_eq!(test_case.complexity, TestComplexity::Medium);
    }

    #[test]
    fn test_test_case_with_languages() {
        let test_case = TestCase::new(
            "tc-002".to_string(),
            "Multi-language Test".to_string(),
            TestType::Integration,
            PathBuf::from("/tests/MultiTest.java"),
            PathBuf::from("/newtest/testcases/java/integration/MultiTest.java"),
        ).with_languages(vec!["java".to_string(), "python".to_string()]);

        assert_eq!(test_case.languages.len(), 2);
        assert!(test_case.supports_language("java"));
        assert!(test_case.supports_language("python"));
        assert!(!test_case.supports_language("javascript"));
    }

    #[test]
    fn test_test_case_lifecycle() {
        let mut test_case = TestCase::new(
            "tc-003".to_string(),
            "Lifecycle Test".to_string(),
            TestType::Basic,
            PathBuf::from("/tests/BasicTest.java"),
            PathBuf::from("/newtest/testcases/java/basic/BasicTest.java"),
        );

        assert_eq!(test_case.status, TestCaseStatus::Pending);

        test_case.mark_in_progress();
        assert_eq!(test_case.status, TestCaseStatus::InProgress);

        test_case.mark_migrated();
        assert_eq!(test_case.status, TestCaseStatus::Migrated);

        test_case.mark_verified();
        assert_eq!(test_case.status, TestCaseStatus::Verified);
    }

    #[test]
    fn test_test_case_validation() {
        let valid_test_case = TestCase::new(
            "tc-004".to_string(),
            "Valid Test".to_string(),
            TestType::RuleValidation,
            PathBuf::from("/tests/ValidTest.java"),
            PathBuf::from("/newtest/testcases/java/validation/ValidTest.java"),
        ).with_languages(vec!["java".to_string()]);

        let issues = valid_test_case.validate();
        assert!(issues.is_empty());

        let invalid_test_case = TestCase::new(
            "".to_string(),
            "".to_string(),
            TestType::PatternMatching,
            PathBuf::new(),
            PathBuf::new(),
        );

        let issues = invalid_test_case.validate();
        assert!(!issues.is_empty());
        assert!(issues.contains(&"Asset ID cannot be empty".to_string()));
    }

    #[test]
    fn test_migration_difficulty() {
        let easy_test = TestCase::new(
            "tc-005".to_string(),
            "Easy Test".to_string(),
            TestType::Basic,
            PathBuf::from("/tests/EasyTest.java"),
            PathBuf::from("/newtest/testcases/java/basic/EasyTest.java"),
        ).with_complexity(TestComplexity::Simple);

        assert_eq!(easy_test.migration_difficulty(), MigrationDifficulty::Easy);

        let hard_test = TestCase::new(
            "tc-006".to_string(),
            "Hard Test".to_string(),
            TestType::Security,
            PathBuf::from("/tests/HardTest.java"),
            PathBuf::from("/newtest/testcases/java/security/HardTest.java"),
        ).with_complexity(TestComplexity::Complex)
            .with_dependencies(vec!["complex_dep".to_string()]);

        assert_eq!(hard_test.migration_difficulty(), MigrationDifficulty::Hard);
    }

    #[test]
    fn test_language_mapping_creation() {
        let mapping = LanguageMapping::new();

        assert_eq!(mapping.detect_language(&PathBuf::from("test.java"), None), "java");
        assert_eq!(mapping.detect_language(&PathBuf::from("test.py"), None), "python");
        assert_eq!(mapping.detect_language(&PathBuf::from("test.js"), None), "javascript");
        assert_eq!(mapping.detect_language(&PathBuf::from("test.sql"), None), "sql");
        assert_eq!(mapping.detect_language(&PathBuf::from("test.unknown"), None), "unknown");
    }

    #[test]
    fn test_test_case_collection_creation() {
        let mut collection = TestCaseCollection::new(
            "collection-001".to_string(),
            "Security Tests".to_string(),
        );

        assert_eq!(collection.collection_id, "collection-001");
        assert_eq!(collection.name, "Security Tests");
        assert!(collection.test_cases.is_empty());
    }

    #[test]
    fn test_test_case_collection_add() {
        let mut collection = TestCaseCollection::new(
            "collection-002".to_string(),
            "Integration Tests".to_string(),
        );

        let test_case = TestCase::new(
            "tc-007".to_string(),
            "Integration Test".to_string(),
            TestType::Integration,
            PathBuf::from("/tests/IntegrationTest.java"),
            PathBuf::from("/newtest/testcases/java/integration/IntegrationTest.java"),
        ).with_languages(vec!["java".to_string()]);

        collection.add_test_case(test_case);

        assert_eq!(collection.test_cases.len(), 1);
        assert_eq!(collection.language_distribution.get("java"), Some(&1));
        assert_eq!(collection.type_distribution.get(&TestType::Integration), Some(&1));
    }

    #[test]
    fn test_test_case_complexity_ordering() {
        assert!(TestComplexity::Simple < TestComplexity::Medium);
        assert!(TestComplexity::Medium < TestComplexity::Complex);
        assert!(TestComplexity::Complex < TestComplexity::Expert);
    }

    #[test]
    fn test_test_priority_ordering() {
        assert!(TestPriority::Low < TestPriority::Normal);
        assert!(TestPriority::Normal < TestPriority::High);
        assert!(TestPriority::High < TestPriority::Critical);
    }
}