//! Script execution functionality for ASTGreP

use crate::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use tokio::process::Command;

/// Configuration for script execution
#[derive(Debug)]
pub struct ExecutionConfig {
    /// Timeout for script execution
    pub timeout: std::time::Duration,
    /// Working directory for script execution
    pub working_directory: Option<PathBuf>,
    /// Environment variables for script execution
    pub environment_variables: HashMap<String, String>,
    /// Whether to capture script output
    pub capture_output: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(60),
            working_directory: None,
            environment_variables: HashMap::new(),
            capture_output: true,
        }
    }
}

/// Context for script execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Path to the script to execute
    pub script_path: PathBuf,
    /// Arguments to pass to the script
    pub args: Vec<String>,
    /// Working directory for execution
    pub working_directory: PathBuf,
    /// Environment variables for execution
    pub environment_variables: HashMap<String, String>,
}

/// Result of script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Standard output
    pub stdout: Option<String>,
    /// Standard error
    pub stderr: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Whether execution timed out
    pub timed_out: bool,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Script executor for running test scripts
pub struct ScriptExecutor {
    config: ExecutionConfig,
}

impl ScriptExecutor {
    /// Create a new script executor
    pub fn new(config: ExecutionConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Execute a script with the given context
    pub async fn execute_script(&self, context: &ExecutionContext) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new(&context.script_path);
        cmd.args(&context.args)
            .current_dir(&context.working_directory);

        // Set environment variables
        for (key, value) in &context.environment_variables {
            cmd.env(key, value);
        }

        // Configure I/O
        if self.config.capture_output {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        // Execute with timeout
        let result = tokio::time::timeout(self.config.timeout, cmd.output()).await;

        let execution_result = match result {
            Ok(Ok(output)) => {
                let stdout = if self.config.capture_output {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    None
                };

                let stderr = if self.config.capture_output {
                    Some(String::from_utf8_lossy(&output.stderr).to_string())
                } else {
                    None
                };

                ExecutionResult {
                    stdout,
                    stderr,
                    exit_code: output.status.code(),
                    timed_out: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                }
            }
            Ok(Err(e)) => ExecutionResult {
                stdout: None,
                stderr: Some(format!("Failed to execute script: {}", e)),
                exit_code: None,
                timed_out: false,
                duration_ms: start_time.elapsed().as_millis() as u64,
            },
            Err(_) => ExecutionResult {
                stdout: None,
                stderr: Some("Script execution timed out".to_string()),
                exit_code: None,
                timed_out: true,
                duration_ms: self.config.timeout.as_millis() as u64,
            },
        };

        Ok(execution_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_config_default_values() {
        let config = ExecutionConfig::default();
        assert_eq!(config.timeout, std::time::Duration::from_secs(60));
        assert_eq!(config.working_directory, None);
        assert!(config.environment_variables.is_empty());
        assert_eq!(config.capture_output, true);
    }

    #[test]
    fn test_execution_config_custom_values() {
        let config = ExecutionConfig {
            timeout: std::time::Duration::from_secs(120),
            working_directory: Some(PathBuf::from("/workspace")),
            environment_variables: HashMap::from([("KEY".to_string(), "VALUE".to_string())]),
            capture_output: false,
        };
        assert_eq!(config.timeout, std::time::Duration::from_secs(120));
        assert_eq!(config.working_directory, Some(PathBuf::from("/workspace")));
        assert_eq!(config.environment_variables.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(config.capture_output, false);
    }

    #[test]
    fn test_execution_context_construction() {
        let script_path = PathBuf::from("/scripts/test.sh");
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let working_directory = PathBuf::from("/workspace");
        let env_vars = HashMap::from([("KEY".to_string(), "VALUE".to_string())]);

        let context = ExecutionContext {
            script_path: script_path.clone(),
            args: args.clone(),
            working_directory: working_directory.clone(),
            environment_variables: env_vars.clone(),
        };

        assert_eq!(context.script_path, script_path);
        assert_eq!(context.args, args);
        assert_eq!(context.working_directory, working_directory);
        assert_eq!(context.environment_variables, env_vars);
    }

    #[test]
    fn test_execution_context_empty_args() {
        let context = ExecutionContext {
            script_path: PathBuf::from("/scripts/run.sh"),
            args: vec![],
            working_directory: PathBuf::from("/workspace"),
            environment_variables: HashMap::new(),
        };
        assert!(context.args.is_empty());
        assert!(context.environment_variables.is_empty());
    }

    #[test]
    fn test_execution_result_successful() {
        let result = ExecutionResult {
            stdout: Some("output".to_string()),
            stderr: None,
            exit_code: Some(0),
            timed_out: false,
            duration_ms: 150,
        };
        assert_eq!(result.stdout.as_ref().unwrap(), "output");
        assert_eq!(result.stderr, None);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.timed_out, false);
        assert_eq!(result.duration_ms, 150);
    }

    #[test]
    fn test_execution_result_timed_out() {
        let result = ExecutionResult {
            stdout: None,
            stderr: Some("Script execution timed out".to_string()),
            exit_code: None,
            timed_out: true,
            duration_ms: 60000,
        };
        assert_eq!(result.stdout, None);
        assert_eq!(result.stderr.as_ref().unwrap(), "Script execution timed out");
        assert_eq!(result.exit_code, None);
        assert_eq!(result.timed_out, true);
        assert_eq!(result.duration_ms, 60000);
    }

    #[test]
    fn test_execution_result_error() {
        let result = ExecutionResult {
            stdout: None,
            stderr: Some("Failed to execute script: permission denied".to_string()),
            exit_code: None,
            timed_out: false,
            duration_ms: 5,
        };
        assert_eq!(result.stdout, None);
        assert_eq!(result.timed_out, false);
        assert!(result.stderr.as_ref().unwrap().contains("permission denied"));
    }

    #[test]
    fn test_script_executor_new_succeeds() {
        let config = ExecutionConfig::default();
        let executor = ScriptExecutor::new(config);
        assert!(executor.is_ok());
    }

    #[test]
    fn test_script_executor_new_with_custom_config() {
        let config = ExecutionConfig {
            timeout: std::time::Duration::from_secs(30),
            working_directory: Some(PathBuf::from("/custom")),
            environment_variables: HashMap::new(),
            capture_output: false,
        };
        let executor = ScriptExecutor::new(config);
        assert!(executor.is_ok());
    }
}
