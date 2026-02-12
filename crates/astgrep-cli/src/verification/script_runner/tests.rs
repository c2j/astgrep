use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_script_runner_config_default() {
    let config = ScriptRunnerConfig::default();
    assert_eq!(config.execution_timeout, Duration::from_secs(120));
    assert!(config.capture_output);
    assert!(config.verify_dependencies);
    assert!(config.parallel_execution);
    assert_eq!(config.max_concurrent_executions, 4);
    assert_eq!(config.max_retries, 2);
    assert!(config.validate_syntax);
}

#[test]
fn test_script_type_detection() {
    let runner = ScriptRunner::new(ScriptRunnerConfig::default());

    // Test bash script detection
    let bash_content = "#!/bin/bash\necho 'test'";
    assert_eq!(runner.detect_script_type(&PathBuf::from("test.sh"), bash_content), "bash");

    // Test python script detection
    let python_content = "#!/usr/bin/env python3\nprint('test')";
    assert_eq!(runner.detect_script_type(&PathBuf::from("test.py"), python_content), "python");

    // Test extension-based detection
    assert_eq!(runner.detect_script_type(&PathBuf::from("test.js"), ""), "javascript");
    assert_eq!(runner.detect_script_type(&PathBuf::from("test.py"), ""), "python");
}

#[test]
fn test_dependency_extraction() {
    let runner = ScriptRunner::new(ScriptRunnerConfig::default());

    // Test bash dependency extraction
    let bash_content = r#"
#!/bin/bash
source helper.sh
. /utils/functions.sh
echo "test"
"#;
    let bash_deps = runner.extract_dependencies(bash_content);
    assert!(bash_deps.contains(&"helper.sh".to_string()));
    assert!(bash_deps.contains(&"utils/functions.sh".to_string()));

    // Test python dependency extraction
    let python_content = r#"
#!/usr/bin/env python3
import os
import sys
from utils import helper
print("test")
"#;
    let python_deps = runner.extract_dependencies(python_content);
    assert!(python_deps.contains(&"os".to_string()));
    assert!(python_deps.contains(&"utils".to_string()));
}

#[tokio::test]
async fn test_file_existence_check() {
    let temp_dir = TempDir::new().unwrap();
    let existing_file = temp_dir.path().join("existing.sh");
    let non_existent_file = temp_dir.path().join("nonexistent.sh");

    fs::write(&existing_file, "#!/bin/bash\necho 'test'").unwrap();

    let runner = ScriptRunner::new(ScriptRunnerConfig::default());

    // Test existing file
    let check = runner.check_file_existence(&existing_file).await.unwrap();
    assert!(check.passed);

    // Test non-existent file
    let check = runner.check_file_existence(&non_existent_file).await.unwrap();
    assert!(!check.passed);
}

#[test]
fn test_verification_statistics_calculation() {
    let runner = ScriptRunner::new(ScriptRunnerConfig::default());

    let results = vec![
        ScriptExecutionResult {
            script_path: PathBuf::from("test1.sh"),
            success: true,
            exit_code: Some(0),
            stdout: None,
            stderr: None,
            execution_time_ms: 100,
            error_message: None,
            verification_checks: Vec::new(),
            metadata: ScriptMetadata {
                file_size: 100,
                permissions: None,
                modified_at: None,
                script_type: "bash".to_string(),
                shebang: Some("#!/bin/bash".to_string()),
                dependencies: Vec::new(),
                checksum: None,
            },
            executed_at: chrono::Utc::now(),
        },
        ScriptExecutionResult {
            script_path: PathBuf::from("test2.py"),
            success: false,
            exit_code: Some(1),
            stdout: None,
            stderr: None,
            execution_time_ms: 200,
            error_message: Some("Script failed".to_string()),
            verification_checks: Vec::new(),
            metadata: ScriptMetadata {
                file_size: 200,
                permissions: None,
                modified_at: None,
                script_type: "python".to_string(),
                shebang: Some("#!/usr/bin/env python3".to_string()),
                dependencies: vec!["os".to_string()],
                checksum: None,
            },
            executed_at: chrono::Utc::now(),
        },
    ];

    let stats = runner.calculate_statistics(&results);

    assert_eq!(stats.average_execution_time_ms, 150.0);
    assert_eq!(stats.fastest_execution_time_ms, 100);
    assert_eq!(stats.slowest_execution_time_ms, 200);
    assert_eq!(stats.success_rate, 50.0);
    assert_eq!(stats.scripts_with_dependencies, 1);
    assert_eq!(stats.most_common_script_type, Some("bash".to_string()));
}
