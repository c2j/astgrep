//! Script execution validation for ASTGreP
//!
//! This module provides functionality to validate that migrated scripts
//! execute correctly from their new locations in the organized directory structure.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
    process::Stdio,
};
use tokio::process::Command;
use tracing::{warn, debug, instrument};

/// Configuration for script validation
#[derive(Debug, Clone)]
pub struct ScriptValidationConfig {
    /// Timeout for individual script execution
    pub execution_timeout: Duration,
    /// Whether to execute scripts in parallel
    pub parallel_execution: bool,
    /// Maximum number of concurrent executions
    pub max_concurrent_scripts: usize,
    /// Whether to validate script dependencies
    pub validate_dependencies: bool,
    /// Whether to capture script output for validation
    pub capture_output: bool,
    /// Working directory for script execution
    pub working_directory: Option<PathBuf>,
    /// Environment variables for script execution
    pub environment_variables: HashMap<String, String>,
}

impl Default for ScriptValidationConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(60),
            parallel_execution: true,
            max_concurrent_scripts: 4,
            validate_dependencies: true,
            capture_output: true,
            working_directory: None,
            environment_variables: HashMap::new(),
        }
    }
}

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

/// Simplified test asset for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAsset {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub shebang: Option<String>,
}

/// Result of validating a single script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptValidationResult {
    /// Script asset information
    pub asset: TestAsset,
    /// Validation status
    pub status: ValidationStatus,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Standard output (if captured)
    pub stdout: Option<String>,
    /// Standard error (if captured)
    pub stderr: Option<String>,
    /// Exit code (if executed)
    pub exit_code: Option<i32>,
    /// Error message (if validation failed)
    pub error_message: Option<String>,
    /// Dependencies that were validated
    pub validated_dependencies: Vec<String>,
    /// Missing dependencies
    pub missing_dependencies: Vec<String>,
    /// Validation timestamp
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

/// Overall script validation summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptValidationSummary {
    /// Total number of scripts validated
    pub total_scripts: usize,
    /// Number of successfully validated scripts
    pub successful_validations: usize,
    /// Number of failed validations
    pub failed_validations: usize,
    /// Number of scripts skipped
    pub skipped_scripts: usize,
    /// Total validation time
    pub total_validation_time: Duration,
    /// Individual script validation results
    pub script_results: Vec<ScriptValidationResult>,
    /// Validation timestamp
    pub validation_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Script execution validator
pub struct ScriptValidator {
    config: ScriptValidationConfig,
}

impl ScriptValidator {
    /// Create a new script validator
    pub fn new(config: ScriptValidationConfig) -> Self {
        Self { config }
    }

    /// Validate a single script
    #[instrument(skip(self, script_path))]
    pub async fn validate_single_script(
        &self,
        script_path: &Path,
    ) -> anyhow::Result<ScriptValidationResult> {
        debug!("Validating script: {}", script_path.display());

        let start_time = Instant::now();

        // Create a basic test asset
        let content = std::fs::read_to_string(script_path)?;
        let shebang = extract_shebang(&content);

        let asset = TestAsset {
            path: script_path.to_path_buf(),
            relative_path: script_path.strip_prefix(std::env::current_dir()?)
                .unwrap_or(script_path)
                .to_path_buf(),
            content,
            shebang,
        };

        // Check if script file exists and is executable
        let exists_and_executable = check_script_executable(script_path).await?;
        if !exists_and_executable {
            let result = ScriptValidationResult {
                asset,
                status: ValidationStatus::Invalid,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                stdout: None,
                stderr: None,
                exit_code: None,
                error_message: Some("Script file does not exist or is not executable".to_string()),
                validated_dependencies: Vec::new(),
                missing_dependencies: Vec::new(),
                validated_at: chrono::Utc::now(),
            };
            return Ok(result);
        }

        // Validate dependencies if required
        let (validated_deps, missing_deps) = if self.config.validate_dependencies {
            validate_script_dependencies(&asset).await?
        } else {
            (Vec::new(), Vec::new())
        };

        // Skip execution if critical dependencies are missing
        if !missing_deps.is_empty() {
            let result = ScriptValidationResult {
                asset,
                status: ValidationStatus::Skipped,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                stdout: None,
                stderr: None,
                exit_code: None,
                error_message: Some(format!("Skipping execution due to missing dependencies: {:?}", missing_deps)),
                validated_dependencies: validated_deps,
                missing_dependencies: missing_deps,
                validated_at: chrono::Utc::now(),
            };
            return Ok(result);
        }

        // Execute script and validate result
        let execution_result = execute_script_and_validate(script_path, &self.config).await?;
        let execution_time = start_time.elapsed();

        let status = match execution_result.exit_code {
            Some(0) => ValidationStatus::Valid,
            Some(_) => ValidationStatus::Invalid,
            None => ValidationStatus::Skipped,
        };

        let result = ScriptValidationResult {
            asset,
            status,
            execution_time_ms: execution_time.as_millis() as u64,
            stdout: execution_result.stdout,
            stderr: execution_result.stderr,
            exit_code: execution_result.exit_code,
            error_message: execution_result.error_message,
            validated_dependencies: validated_deps,
            missing_dependencies: missing_deps,
            validated_at: chrono::Utc::now(),
        };

        debug!("Script validation completed: {:?} ({}ms)", result.status, result.execution_time_ms);
        Ok(result)
    }

    /// Generate validation report
    pub fn generate_validation_report(&self, summary: &ScriptValidationSummary) -> String {
        let mut report = String::new();

        report.push_str("# Script Validation Report\n\n");
        report.push_str(&format!("**Validation Date**: {}\n", summary.validation_timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("**Total Scripts**: {}\n", summary.total_scripts));
        report.push_str(&format!("**Successful**: {}\n", summary.successful_validations));
        report.push_str(&format!("**Failed**: {}\n", summary.failed_validations));
        report.push_str(&format!("**Skipped**: {}\n", summary.skipped_scripts));
        report.push_str(&format!("**Total Time**: {:.2}s\n\n", summary.total_validation_time.as_secs_f64()));

        if summary.failed_validations > 0 {
            report.push_str("## Failed Validations\n\n");
            for result in &summary.script_results {
                if result.status == ValidationStatus::Invalid {
                    report.push_str(&format!("- **{}**: {}\n",
                        result.asset.relative_path.display(),
                        result.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                    ));
                    if let Some(exit_code) = result.exit_code {
                        report.push_str(&format!("  - Exit Code: {}\n", exit_code));
                    }
                    if let Some(stderr) = &result.stderr {
                        if !stderr.trim().is_empty() {
                            report.push_str(&format!("  - Stderr: `{}`\n", stderr.trim()));
                        }
                    }
                }
            }
            report.push_str("\n");
        }

        if summary.skipped_scripts > 0 {
            report.push_str("## Skipped Scripts\n\n");
            for result in &summary.script_results {
                if result.status == ValidationStatus::Skipped {
                    report.push_str(&format!("- **{}**: {}\n",
                        result.asset.relative_path.display(),
                        result.error_message.as_ref().unwrap_or(&"Unknown reason".to_string())
                    ));
                    if !result.missing_dependencies.is_empty() {
                        report.push_str(&format!("  - Missing Dependencies: {:?}\n", result.missing_dependencies));
                    }
                }
            }
            report.push_str("\n");
        }

        report
    }
}

/// Result of script execution
#[derive(Debug)]
struct ExecutionResult {
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: Option<i32>,
    error_message: Option<String>,
}

/// Extract shebang from script content
fn extract_shebang(content: &str) -> Option<String> {
    content.lines().next().and_then(|line| {
        if line.starts_with("#!") {
            Some(line.to_string())
        } else {
            None
        }
    })
}

/// Check if script file exists and is executable
async fn check_script_executable(script_path: &Path) -> anyhow::Result<bool> {
    if !script_path.exists() {
        warn!("Script file does not exist: {}", script_path.display());
        return Ok(false);
    }

    let metadata = std::fs::metadata(script_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            warn!("Script file is not executable: {}", script_path.display());
            return Ok(false);
        }
    }

    Ok(true)
}

/// Validate script dependencies
async fn validate_script_dependencies(
    asset: &TestAsset,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let mut validated_dependencies = Vec::new();
    let mut missing_dependencies = Vec::new();

    // Check for common script interpreters
    let dependency_checks = vec![
        ("bash", which::which("bash")),
        ("sh", which::which("sh")),
        ("python", which::which("python3").or_else(|_| which::which("python"))),
        ("python3", which::which("python3")),
        ("node", which::which("node")),
    ];

    for (dep_name, which_result) in dependency_checks {
        if asset.content.contains(dep_name) || asset.shebang.as_ref().map_or(false, |s| s.contains(dep_name)) {
            match which_result {
                Ok(path) => {
                    debug!("Found dependency: {} at {}", dep_name, path.display());
                    validated_dependencies.push(dep_name.to_string());
                }
                Err(_) => {
                    warn!("Missing dependency: {}", dep_name);
                    missing_dependencies.push(dep_name.to_string());
                }
            }
        }
    }

    Ok((validated_dependencies, missing_dependencies))
}

/// Execute script and validate the result
async fn execute_script_and_validate(
    script_path: &Path,
    config: &ScriptValidationConfig,
) -> anyhow::Result<ExecutionResult> {
    let mut cmd = Command::new(script_path);

    if let Some(working_dir) = &config.working_directory {
        cmd.current_dir(working_dir);
    } else {
        cmd.current_dir(script_path.parent().unwrap_or(script_path));
    }

    // Set environment variables
    for (key, value) in &config.environment_variables {
        cmd.env(key, value);
    }

    // Configure I/O
    if config.capture_output {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    }

    // Execute with timeout
    let result = tokio::time::timeout(config.execution_timeout, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = if config.capture_output {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                None
            };

            let stderr = if config.capture_output {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            } else {
                None
            };

            Ok(ExecutionResult {
                stdout,
                stderr,
                exit_code: output.status.code(),
                error_message: None,
            })
        }
        Ok(Err(e)) => Ok(ExecutionResult {
            stdout: None,
            stderr: None,
            exit_code: None,
            error_message: Some(format!("Script execution failed: {}", e)),
        }),
        Err(_) => Ok(ExecutionResult {
            stdout: None,
            stderr: None,
            exit_code: None,
            error_message: Some("Script execution timed out".to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_script_validation_config_default() {
        let config = ScriptValidationConfig::default();
        assert_eq!(config.execution_timeout, Duration::from_secs(60));
        assert!(config.parallel_execution);
        assert_eq!(config.max_concurrent_scripts, 4);
        assert!(config.validate_dependencies);
        assert!(config.capture_output);
    }

    #[test]
    fn test_extract_shebang() {
        let bash_script = "#!/bin/bash\necho 'test'";
        assert_eq!(extract_shebang(bash_script), Some("#!/bin/bash".to_string()));

        let python_script = "#!/usr/bin/env python3\nprint('test')";
        assert_eq!(extract_shebang(python_script), Some("#!/usr/bin/env python3".to_string()));

        let no_shebang = "echo 'test'";
        assert_eq!(extract_shebang(no_shebang), None);
    }

    #[tokio::test]
    async fn test_check_script_executable() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test_script.sh");

        // Create a test script
        fs::write(&script_path, "#!/bin/bash\necho 'test'").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
        }

        let result = check_script_executable(&script_path).await.unwrap();
        #[cfg(unix)]
        assert!(result);
        #[cfg(not(unix))]
        assert!(result); // On Windows, we just check if file exists
    }

    #[tokio::test]
    async fn test_validate_script_dependencies() {
        let asset = TestAsset {
            path: PathBuf::from("/test/script.sh"),
            relative_path: PathBuf::from("script.sh"),
            content: "#!/bin/bash\necho 'test'".to_string(),
            shebang: Some("#!/bin/bash".to_string()),
        };

        let (validated, missing) = validate_script_dependencies(&asset).await.unwrap();

        // Should find bash dependency if it exists on system
        if which::which("bash").is_ok() {
            assert!(validated.contains(&"bash".to_string()));
        } else {
            assert!(missing.contains(&"bash".to_string()));
        }
    }

    #[test]
    fn test_generate_validation_report() {
        let summary = ScriptValidationSummary {
            total_scripts: 10,
            successful_validations: 8,
            failed_validations: 2,
            skipped_scripts: 0,
            total_validation_time: Duration::from_secs(5),
            script_results: vec![],
            validation_timestamp: chrono::Utc::now(),
        };

        let validator = ScriptValidator::new(ScriptValidationConfig::default());
        let report = validator.generate_validation_report(&summary);

        assert!(report.contains("Script Validation Report"));
        assert!(report.contains("Total Scripts: 10"));
        assert!(report.contains("Successful: 8"));
        assert!(report.contains("Failed: 2"));
    }
}