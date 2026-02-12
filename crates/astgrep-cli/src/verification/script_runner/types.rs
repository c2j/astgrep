use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};

/// Configuration for script execution verification
#[derive(Debug, Clone)]
pub struct ScriptRunnerConfig {
    /// Timeout for script execution
    pub execution_timeout: Duration,
    /// Working directory for script execution
    pub working_directory: Option<PathBuf>,
    /// Environment variables for script execution
    pub environment_variables: HashMap<String, String>,
    /// Whether to capture script output
    pub capture_output: bool,
    /// Whether to verify dependencies before execution
    pub verify_dependencies: bool,
    /// Whether to enable parallel execution
    pub parallel_execution: bool,
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: usize,
    /// Retry attempts for failed executions
    pub max_retries: usize,
    /// Whether to validate script syntax before execution
    pub validate_syntax: bool,
}

impl Default for ScriptRunnerConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(120),
            working_directory: None,
            environment_variables: HashMap::new(),
            capture_output: true,
            verify_dependencies: true,
            parallel_execution: true,
            max_concurrent_executions: 4,
            max_retries: 2,
            validate_syntax: true,
        }
    }
}

/// Result of script execution verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    /// Script that was executed
    pub script_path: PathBuf,
    /// Whether execution was successful
    pub success: bool,
    /// Exit code from script execution
    pub exit_code: Option<i32>,
    /// Standard output (if captured)
    pub stdout: Option<String>,
    /// Standard error (if captured)
    pub stderr: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Error message if execution failed
    pub error_message: Option<String>,
    /// Verification checks performed
    pub verification_checks: Vec<VerificationCheck>,
    /// Script metadata
    pub metadata: ScriptMetadata,
    /// Execution timestamp
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Individual verification check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    /// Type of verification check
    pub check_type: VerificationCheckType,
    /// Description of the check
    pub description: String,
    /// Whether the check passed
    pub passed: bool,
    /// Additional details about the check
    pub details: Option<String>,
    /// Time taken to perform the check in milliseconds
    pub check_time_ms: u64,
}

/// Types of verification checks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationCheckType {
    /// Check if script file exists and is readable
    FileExistence,
    /// Check if script has proper permissions
    FilePermissions,
    /// Check script syntax without executing
    SyntaxValidation,
    /// Check if required dependencies are available
    DependencyCheck,
    /// Check if script can be executed
    Executability,
    /// Validate script output
    OutputValidation,
    /// Check script execution time
    PerformanceCheck,
    /// Verify script integrity (checksum)
    IntegrityCheck,
}

/// Metadata about a script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    /// Script file size in bytes
    pub file_size: u64,
    /// Script file permissions
    pub permissions: Option<u32>,
    /// Script modification timestamp
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Detected script type/language
    pub script_type: String,
    /// Script shebang if present
    pub shebang: Option<String>,
    /// Script dependencies detected
    pub dependencies: Vec<String>,
    /// Script checksum
    pub checksum: Option<String>,
}

/// Overall verification summary
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationSummary {
    /// Total scripts verified
    pub total_scripts: usize,
    /// Successful verifications
    pub successful_verifications: usize,
    /// Failed verifications
    pub failed_verifications: usize,
    /// Total verification time
    pub total_verification_time: Duration,
    /// Individual execution results
    pub execution_results: Vec<ScriptExecutionResult>,
    /// Summary statistics
    pub statistics: VerificationStatistics,
    /// Verification timestamp
    pub verified_at: chrono::DateTime<chrono::Utc>,
}

/// Verification statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationStatistics {
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Fastest execution time in milliseconds
    pub fastest_execution_time_ms: u64,
    /// Slowest execution time in milliseconds
    pub slowest_execution_time_ms: u64,
    /// Most common failure reason
    pub common_failure_reason: Option<String>,
    /// Success rate percentage
    pub success_rate: f64,
    /// Number of scripts with dependencies
    pub scripts_with_dependencies: usize,
    /// Most common script type
    pub most_common_script_type: Option<String>,
}
