//! Data models for the web API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Analysis request for code snippets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    /// Source code to analyze
    pub code: String,
    
    /// Programming language
    pub language: String,
    
    /// Rules to apply (optional, uses all if not specified)
    pub rules: Option<Vec<String>>,
    
    /// Analysis options
    pub options: Option<AnalysisOptions>,
}

/// Analysis request for file uploads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFileRequest {
    /// File name
    pub filename: String,
    
    /// File content (base64 encoded)
    pub content: String,
    
    /// Programming language (optional, auto-detected if not specified)
    pub language: Option<String>,
    
    /// Rules to apply (optional)
    pub rules: Option<Vec<String>>,
    
    /// Analysis options
    pub options: Option<AnalysisOptions>,
}

/// Analysis request for archive uploads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeArchiveRequest {
    /// Archive content (base64 encoded)
    pub archive: String,
    
    /// Archive format (zip, tar, tar.gz)
    pub format: String,
    
    /// Languages to analyze (optional, auto-detected if not specified)
    pub languages: Option<Vec<String>>,
    
    /// Rules to apply (optional)
    pub rules: Option<Vec<String>>,
    
    /// File patterns to include
    pub include_patterns: Option<Vec<String>>,
    
    /// File patterns to exclude
    pub exclude_patterns: Option<Vec<String>>,
    
    /// Analysis options
    pub options: Option<AnalysisOptions>,
}

/// Analysis options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Minimum severity level to report
    pub min_severity: Option<String>,
    
    /// Minimum confidence level to report
    pub min_confidence: Option<String>,
    
    /// Maximum number of findings to return
    pub max_findings: Option<usize>,
    
    /// Enable data flow analysis
    pub enable_dataflow: Option<bool>,

    /// Enable data flow analysis (alias for enable_dataflow)
    pub enable_dataflow_analysis: Option<bool>,

    /// Enable security-focused analysis
    pub enable_security_analysis: Option<bool>,

    /// Enable performance analysis
    pub enable_performance_analysis: Option<bool>,

    /// Include performance metrics
    pub include_metrics: Option<bool>,

    /// Output format
    pub output_format: Option<String>,
}

/// Analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    /// Job ID for tracking
    pub job_id: Uuid,
    
    /// Analysis status
    pub status: JobStatus,
    
    /// Analysis results (if completed)
    pub results: Option<AnalysisResults>,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Job creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Job completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

/// Analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// List of findings
    pub findings: Vec<Finding>,
    
    /// Analysis summary
    pub summary: AnalysisSummary,
    
    /// Performance metrics (if requested)
    pub metrics: Option<PerformanceMetrics>,
}

/// Code analysis finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Rule ID that triggered this finding
    pub rule_id: String,
    
    /// Finding message
    pub message: String,
    
    /// Severity level
    pub severity: String,
    
    /// Confidence level
    pub confidence: String,
    
    /// Location in the code
    pub location: Location,
    
    /// Suggested fix (optional)
    pub fix: Option<String>,
    
    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// File path
    pub file: String,
    
    /// Start line number (1-based)
    pub start_line: usize,
    
    /// Start column number (1-based)
    pub start_column: usize,
    
    /// End line number (1-based)
    pub end_line: usize,
    
    /// End column number (1-based)
    pub end_column: usize,
    
    /// Code snippet (optional)
    pub snippet: Option<String>,
}

/// Analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Total number of findings
    pub total_findings: usize,
    
    /// Findings by severity
    pub findings_by_severity: HashMap<String, usize>,
    
    /// Findings by confidence
    pub findings_by_confidence: HashMap<String, usize>,
    
    /// Number of files analyzed
    pub files_analyzed: usize,
    
    /// Number of rules executed
    pub rules_executed: usize,
    
    /// Analysis duration in milliseconds
    pub duration_ms: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total analysis time
    pub total_time_ms: u64,
    
    /// Time spent parsing
    pub parse_time_ms: u64,
    
    /// Time spent in rule execution
    pub rule_execution_time_ms: u64,
    
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
}

/// Job status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is pending (not yet queued)
    Pending,

    /// Job is queued for processing
    Queued,
    
    /// Job is currently being processed
    Running,
    
    /// Job completed successfully
    Completed,
    
    /// Job failed with an error
    Failed,
    
    /// Job was cancelled
    Cancelled,
}

/// Job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: Uuid,
    
    /// Job status
    pub status: JobStatus,
    
    /// Job type
    pub job_type: String,
    
    /// Job creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Job start timestamp
    pub started_at: Option<DateTime<Utc>>,
    
    /// Job completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    
    /// Progress percentage (0-100)
    pub progress: u8,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Job metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Rule information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    /// Rule identifier
    pub id: String,
    
    /// Rule name
    pub name: String,
    
    /// Rule description
    pub description: String,
    
    /// Supported languages
    pub languages: Vec<String>,
    
    /// Rule severity
    pub severity: String,
    
    /// Rule confidence
    pub confidence: String,
    
    /// Rule category
    pub category: Option<String>,
    
    /// Rule tags
    pub tags: Vec<String>,
    
    /// Whether the rule is enabled
    pub enabled: bool,
    
    /// Rule metadata
    pub metadata: HashMap<String, String>,
}

/// Rule validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRulesRequest {
    /// Rules content (YAML format)
    pub rules: String,
    
    /// Language to validate against (optional)
    pub language: Option<String>,
    
    /// Enable performance checking
    pub check_performance: Option<bool>,
}

/// Rule validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRulesResponse {
    /// Whether validation passed
    pub valid: bool,
    
    /// Validation errors
    pub errors: Vec<String>,
    
    /// Validation warnings
    pub warnings: Vec<String>,
    
    /// Number of rules validated
    pub rules_count: usize,
    
    /// Performance metrics (if requested)
    pub performance: Option<RulePerformanceMetrics>,
}

/// Rule performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePerformanceMetrics {
    /// Rule loading time in milliseconds
    pub load_time_ms: u64,
    
    /// Average rule complexity
    pub average_complexity: f64,
    
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: u64,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    
    /// Service version
    pub version: String,
    
    /// Uptime in seconds
    pub uptime_seconds: u64,
    
    /// System information
    pub system: SystemInfo,
    
    /// Service dependencies status
    pub dependencies: HashMap<String, DependencyStatus>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Available memory in bytes
    pub available_memory_bytes: u64,
    
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    
    /// Disk usage percentage
    pub disk_usage_percent: f64,
    
    /// Number of active jobs
    pub active_jobs: usize,
}

/// Dependency status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    /// Whether the dependency is healthy
    pub healthy: bool,
    
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    
    /// Error message (if unhealthy)
    pub error: Option<String>,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Service version
    pub version: String,
    
    /// Build timestamp
    pub build_timestamp: String,
    
    /// Git commit hash
    pub git_commit: String,
    
    /// Rust version used for compilation
    pub rust_version: String,
    
    /// Supported features
    pub features: Vec<String>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            min_severity: Some("info".to_string()),
            min_confidence: Some("low".to_string()),
            max_findings: None,
            enable_dataflow: Some(false),
            enable_dataflow_analysis: Some(false),
            enable_security_analysis: Some(false),
            enable_performance_analysis: Some(false),
            include_metrics: Some(false),
            output_format: Some("json".to_string()),
        }
    }
}

impl Default for JobStatus {
    fn default() -> Self {
        Self::Queued
    }
}
