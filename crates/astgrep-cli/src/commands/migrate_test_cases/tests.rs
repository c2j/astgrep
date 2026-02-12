use super::*;
use std::fs;
use tempfile::TempDir;

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

    let checksum = TestCaseMigrator::calculate_file_checksum(&test_file)
        .await
        .unwrap();
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
    let target_path = migrator
        .generate_target_path(&source_path, "java", &TestType::Security)
        .await
        .unwrap();

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
