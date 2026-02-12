use super::*;

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
    )
    .with_languages(vec!["java".to_string(), "python".to_string()]);

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
    )
    .with_languages(vec!["java".to_string()]);

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
    )
    .with_complexity(TestComplexity::Simple);

    assert_eq!(easy_test.migration_difficulty(), MigrationDifficulty::Easy);

    let hard_test = TestCase::new(
        "tc-006".to_string(),
        "Hard Test".to_string(),
        TestType::Security,
        PathBuf::from("/tests/HardTest.java"),
        PathBuf::from("/newtest/testcases/java/security/HardTest.java"),
    )
    .with_complexity(TestComplexity::Complex)
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
    )
    .with_languages(vec!["java".to_string()]);

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
