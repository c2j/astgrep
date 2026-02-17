//! Script execution functionality for ASTGreP

use crate::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
};
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