//! Test case creation and path generation
//!
//! This module handles the creation of test case objects from file analysis
//! and generation of target paths in the new structure.

use super::{LanguageDiscoveryConfig};
use astgrep_core::{
    models::{TestCase, TestType, TestComplexity, TestCategory, TestCaseMetadata, TestPriority, LanguageConfig},
};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use anyhow::Result;

use super::detection::{ContentAnalysis, classify_test_file};
use super::detection::LanguagePattern;

/// Analysis of a file for test case classification
#[derive(Debug, Clone)]
pub struct FileAnalysis {
    /// File path
    pub file_path: PathBuf,
    /// Detected language
    pub detected_language: String,
    /// File size in bytes
    pub file_size: u64,
    /// Last modification time
    pub last_modified: SystemTime,
    /// File checksum (if calculated)
    pub checksum: Option<String>,
    /// Detected test type
    pub test_type: TestType,
    /// Detected complexity
    pub complexity: TestComplexity,
    /// Relationships to other files
    pub relationships: Vec<String>,
    /// Content analysis results
    pub content_analysis: ContentAnalysis,
}

/// Generate target path for a test case in new structure
pub fn generate_target_path(
    config: &LanguageDiscoveryConfig,
    analysis: &FileAnalysis,
) -> Result<PathBuf> {
    let language = &analysis.detected_language;
    let test_type_str = format!("{:?}", analysis.test_type).to_lowercase();
    let complexity_str = format!("{:?}", analysis.complexity).to_lowercase();

    // Get language configuration
    let lang_config = config.language_mapping.get_language_config(language)
        .ok_or_else(|| {
            // Create default config
            LanguageConfig {
                language: language.clone(),
                directory_name: language.clone(),
                extensions: vec![],
                common_test_types: vec![TestType::RuleValidation],
                frameworks: vec![],
                default_category: TestCategory::LanguageSpecific,
                test_file_patterns: vec![],
            }
        });

    // Build path: newtest/testcases/{language}/{test-type}/
    let test_type_dir = match analysis.test_type {
        TestType::PatternMatching => "pattern-matching",
        TestType::RuleValidation => "rule-validation",
        TestType::Parsing => "parsing",
        TestType::Integration => "integration",
        TestType::Performance => "performance",
        TestType::Security => "security",
        TestType::Compatibility => "compatibility",
        TestType::Custom => "custom",
    };

    let mut target_path = config.root_directory
        .join("newtest")
        .join("testcases")
        .join(&lang_config.directory_name)
        .join(test_type_dir);

    // Add filename based on original file
    if let Some(filename) = analysis.file_path.file_stem() {
        target_path = target_path.join(filename);
    } else {
        // Generate a default filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        target_path = target_path.join(format!("test_{}.java", timestamp));
    }

    // Ensure file extension matches language
    let target_ext = lang_config.extensions.first()
        .unwrap_or("txt");
    target_path = target_path.with_extension(target_ext);

    Ok(target_path)
}

/// Determine category for a test case
pub fn determine_category(analysis: &FileAnalysis) -> Option<TestCategory> {
    match analysis.test_type {
        TestType::Basic => Some(TestCategory::Basic),
        TestType::PatternMatching => Some(TestCategory::Framework),
        TestType::RuleValidation => Some(TestCategory::Basic),
        TestType::Parsing => Some(TestCategory::Framework),
        TestType::Integration => Some(TestCategory::Integration),
        TestType::Performance => Some(TestCategory::Performance),
        TestType::Security => Some(TestCategory::Security),
        TestType::Compatibility => Some(TestCategory::Compatibility),
        TestType::Custom => Some(TestCategory::Other("Custom".to_string())),
    }
}

/// Create a TestCase from file analysis
pub async fn create_test_case_from_analysis(
    analysis: &FileAnalysis,
    config: &LanguageDiscoveryConfig,
) -> Result<TestCase> {
    let test_case_id = format!("tc-{}",
        chrono::Utc::now().timestamp_nanos()
    );

    let test_case_name = analysis.file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let current_path = analysis.file_path.clone();
    let target_path = generate_target_path(config, analysis)?;

    // Determine category based on test type and language
    let category = determine_category(&analysis);

    let metadata = TestCaseMetadata {
        file_size: analysis.file_size,
        line_count: analysis.content_analysis.line_count,
        created_at: None,
        modified_at: analysis.last_modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)),
        author: None,
        version: None,
        framework: analysis.content_analysis.frameworks.first().cloned(),
        environment_requirements: Vec::new(),
        estimated_execution_time: None,
        priority: TestPriority::Normal,
        custom_properties: std::collections::HashMap::new(),
    };

    let test_case = TestCase::new(
        test_case_id,
        test_case_name,
        analysis.test_type.clone(),
        current_path,
        target_path,
    )
    .with_languages(vec![analysis.detected_language.clone()])
    .with_complexity(analysis.complexity.clone())
    .with_category(category)
    .with_dependencies(analysis.relationships.clone())
    .with_tags(analysis.content_analysis.test_keywords.clone())
    .with_description(format!("Test case for {}", test_case_name));

    Ok(test_case)
}
