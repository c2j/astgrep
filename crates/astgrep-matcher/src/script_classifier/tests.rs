use super::*;
use std::fs;
use tempfile::tempdir;
use astgrep_core::models::test_asset::AssetType;

#[test]
fn test_classification_config_default() {
    let config = ClassificationConfig::default();
    assert!(config.keyword_matching);
    assert!(config.content_analysis);
    assert!(config.shebang_analysis);
    assert!(config.filename_analysis);
    assert!(!config.dependency_analysis);
    assert_eq!(config.confidence_threshold, 0.5);
    assert!(config.enable_fallback);
}

#[test]
fn test_script_classifier_creation() {
    let _classifier = ScriptClassifier::new();
    // Should not panic
    assert!(true);
}

#[test]
fn test_filename_classification() -> Result<()> {
    let temp_dir = tempdir()?;
    let script_path = temp_dir.path().join("validate_test.sh");

    // Create a test script
    fs::write(&script_path, "#!/bin/bash\necho 'validation'\n")?;

    let asset = TestAsset::new(
        "test-001".to_string(),
        "Validation Script".to_string(),
        AssetType::Script,
        script_path.clone(),
        script_path.clone(),
    );

    let classifier = ScriptClassifier::new();
    let result = classifier.classify_by_filename(&asset)?;

    assert!(result.confidence > 0.5); // Should detect "validate" in filename
    assert_eq!(result.classification_method, "filename_analysis");

    Ok(())
}

#[test]
fn test_shebang_classification() -> Result<()> {
    let temp_dir = tempdir()?;
    let script_path = temp_dir.path().join("test.sh");

    // Create a test script with Python shebang
    fs::write(&script_path, "#!/usr/bin/python3\nprint('test')\n")?;

    let asset = TestAsset::new(
        "test-002".to_string(),
        "Test Script".to_string(),
        AssetType::Script,
        script_path.clone(),
        script_path.clone(),
    );

    let classifier = ScriptClassifier::new();
    let result = classifier.classify_by_shebang(&asset)?;

    assert_eq!(result.shebang_detected, Some("/usr/bin/python3".to_string()));

    Ok(())
}

#[test]
fn test_keyword_classification() -> Result<()> {
    let temp_dir = tempdir()?;
    let script_path = temp_dir.path().join("test_script.sh");

    // Create a script with validation keywords
    fs::write(
        &script_path,
        "#!/bin/bash\nvalidate_function() {\n  check_output\n  verify_result\n}\n",
    )?;

    let asset = TestAsset::new(
        "test-003".to_string(),
        "Test Script".to_string(),
        AssetType::Script,
        script_path.clone(),
        script_path.clone(),
    );

    let classifier = ScriptClassifier::new();
    let result = classifier.classify_by_keywords(&asset)?;

    assert!(result.confidence > 0.0); // Should detect validation keywords
    assert!(result.metadata.content_analyzed);

    Ok(())
}
