//! Configuration constants for CR-SemService
//! 
//! This module centralizes all configuration constants to eliminate hardcoded values
//! throughout the codebase.

use std::time::Duration;

/// Default configuration constants
pub mod defaults {
    use super::*;

    /// Default server configuration
    pub mod server {
        /// Default bind address
        pub const BIND_ADDRESS: &str = "127.0.0.1:8080";
        
        /// Default maximum upload size (100MB)
        pub const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;
        
        /// Default request timeout (5 minutes)
        pub const REQUEST_TIMEOUT_SECS: u64 = 300;
        
        /// Default maximum concurrent jobs
        pub const MAX_CONCURRENT_JOBS: usize = 10;
        
        /// Default job cleanup interval (1 hour)
        pub const JOB_CLEANUP_INTERVAL_SECS: u64 = 3600;
        
        /// Default job retention duration (24 hours)
        pub const JOB_RETENTION_DURATION_SECS: u64 = 86400;
        
        /// Default temporary directory
        pub const TEMP_DIRECTORY: &str = "/tmp/cr-semservice";
        
        /// Default rules directory
        pub const RULES_DIRECTORY: &str = "rules";
    }

    /// Parser configuration constants
    pub mod parser {
        /// Default parser timeout (30 seconds)
        pub const DEFAULT_TIMEOUT_MS: u64 = 30000;
        
        /// Default maximum file size (10MB)
        pub const DEFAULT_MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
        
        /// Java parser timeout (45 seconds for larger files)
        pub const JAVA_TIMEOUT_MS: u64 = 45000;
        
        /// Java maximum file size (20MB)
        pub const JAVA_MAX_FILE_SIZE: usize = 20 * 1024 * 1024;
        
        /// Python maximum file size (15MB)
        pub const PYTHON_MAX_FILE_SIZE: usize = 15 * 1024 * 1024;
        
        /// SQL parser timeout (20 seconds)
        pub const SQL_TIMEOUT_MS: u64 = 20000;
        
        /// SQL maximum file size (5MB)
        pub const SQL_MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
        
        /// Bash parser timeout (15 seconds)
        pub const BASH_TIMEOUT_MS: u64 = 15000;
        
        /// Bash maximum file size (1MB)
        pub const BASH_MAX_FILE_SIZE: usize = 1 * 1024 * 1024;
    }

    /// Analysis configuration constants
    pub mod analysis {
        /// Default confidence threshold for taint analysis
        pub const CONFIDENCE_THRESHOLD: f64 = 0.5;

        /// Low confidence threshold for sanitization
        pub const LOW_CONFIDENCE_THRESHOLD: f64 = 0.1;

        /// High effectiveness threshold for sanitizers
        pub const HIGH_EFFECTIVENESS_THRESHOLD: f64 = 0.8;

        /// Default maximum analysis depth
        pub const MAX_ANALYSIS_DEPTH: usize = 20;

        /// Default maximum findings per file
        pub const MAX_FINDINGS_PER_FILE: usize = 1000;

        /// Default similarity threshold for pattern matching
        pub const SIMILARITY_THRESHOLD: f64 = 0.8;

        /// Default cache size for pattern matching
        pub const PATTERN_CACHE_SIZE: usize = 1000;

        /// Default maximum metavariable depth
        pub const MAX_METAVAR_DEPTH: usize = 10;

        /// Taint transformation confidence multipliers
        pub mod taint_multipliers {
            /// Encoding transformation confidence multiplier
            pub const ENCODING_MULTIPLIER: f32 = 0.7;

            /// Validation transformation confidence multiplier
            pub const VALIDATION_MULTIPLIER: f32 = 0.5;

            /// Filtering transformation confidence multiplier
            pub const FILTERING_MULTIPLIER: f32 = 0.8;
        }
    }

    /// Rate limiting constants
    pub mod rate_limit {
        /// Default requests per minute
        pub const REQUESTS_PER_MINUTE: u32 = 60;
        
        /// Default burst size
        pub const BURST_SIZE: u32 = 10;
    }

    /// CORS configuration constants
    pub mod cors {
        /// Default max age (1 hour)
        pub const MAX_AGE_SECS: u64 = 3600;
        
        /// Default allowed origins
        pub const ALLOWED_ORIGINS: &[&str] = &["*"];
        
        /// Default allowed methods
        pub const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "OPTIONS"];
        
        /// Default allowed headers
        pub const ALLOWED_HEADERS: &[&str] = &["Content-Type", "Authorization", "X-Request-ID"];
    }
}

/// Performance and optimization constants
pub mod performance {
    /// Default thread pool size (0 = auto-detect)
    pub const DEFAULT_THREAD_COUNT: usize = 0;
    
    /// Maximum thread pool size
    pub const MAX_THREAD_COUNT: usize = 32;
    
    /// Default memory limit per analysis (512MB)
    pub const MEMORY_LIMIT_BYTES: usize = 512 * 1024 * 1024;
    
    /// Default cache TTL (1 hour)
    pub const CACHE_TTL_SECS: u64 = 3600;
    
    /// Default cache size (100MB)
    pub const CACHE_SIZE_BYTES: usize = 100 * 1024 * 1024;
    
    /// Performance monitoring interval (30 seconds)
    pub const MONITORING_INTERVAL_SECS: u64 = 30;
}

/// Security constants
pub mod security {
    /// Default JWT expiration time (24 hours)
    pub const JWT_EXPIRATION_SECS: u64 = 86400;
    
    /// Minimum password length
    pub const MIN_PASSWORD_LENGTH: usize = 8;
    
    /// Maximum login attempts
    pub const MAX_LOGIN_ATTEMPTS: u32 = 5;
    
    /// Account lockout duration (15 minutes)
    pub const LOCKOUT_DURATION_SECS: u64 = 900;
}

/// File and path constants
pub mod paths {
    /// Default configuration file name
    pub const CONFIG_FILE: &str = "cr-semservice.toml";

    /// Default web configuration file name
    pub const WEB_CONFIG_FILE: &str = "cr-web.toml";

    /// Default log file name
    pub const LOG_FILE: &str = "cr-semservice.log";

    /// Default exclude patterns
    pub const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
        "*.test.java",
        "*.spec.js",
        "**/test/**",
        "**/tests/**",
        "**/node_modules/**",
        "**/target/**",
        "**/build/**",
        "**/.git/**",
    ];
}

/// Language configuration constants
pub mod languages {
    use crate::Language;

    /// All supported languages in order of preference
    pub const ALL_LANGUAGES: &[Language] = &[
        Language::Java,
        Language::JavaScript,
        Language::Python,
        Language::Php,
        Language::Sql,
        Language::Bash,
        Language::CSharp,
        Language::C,
    ];

    /// Default languages for analysis
    pub const DEFAULT_LANGUAGES: &[Language] = ALL_LANGUAGES;
}

/// Helper functions for creating Duration objects
pub mod durations {
    use super::*;
    
    /// Create default request timeout duration
    pub fn request_timeout() -> Duration {
        Duration::from_secs(defaults::server::REQUEST_TIMEOUT_SECS)
    }
    
    /// Create default job cleanup interval duration
    pub fn job_cleanup_interval() -> Duration {
        Duration::from_secs(defaults::server::JOB_CLEANUP_INTERVAL_SECS)
    }
    
    /// Create default job retention duration
    pub fn job_retention_duration() -> Duration {
        Duration::from_secs(defaults::server::JOB_RETENTION_DURATION_SECS)
    }
    
    /// Create CORS max age duration
    pub fn cors_max_age() -> Duration {
        Duration::from_secs(defaults::cors::MAX_AGE_SECS)
    }
    
    /// Create cache TTL duration
    pub fn cache_ttl() -> Duration {
        Duration::from_secs(performance::CACHE_TTL_SECS)
    }
}

/// Version and metadata constants
pub mod metadata {
    /// Application name
    pub const APP_NAME: &str = "CR-SemService";

    /// Application description
    pub const APP_DESCRIPTION: &str = "Static Code Analysis Service";

    /// Default user agent
    pub const USER_AGENT: &str = "CR-SemService/1.0";

    /// API version
    pub const API_VERSION: &str = "v1";
}

/// Rule and finding constants
pub mod rules {
    /// Default location values for mock findings
    pub mod location {
        pub const DEFAULT_START_LINE: u32 = 1;
        pub const DEFAULT_START_COLUMN: u32 = 1;
        pub const DEFAULT_END_LINE: u32 = 1;
        pub const DEFAULT_END_COLUMN: u32 = 1;
    }

    /// Common metadata keys
    pub mod metadata_keys {
        pub const ANALYSIS_TYPE: &str = "analysis_type";
        pub const VULNERABILITY_TYPE: &str = "vulnerability_type";
        pub const CATEGORY: &str = "category";
        pub const IMPACT: &str = "impact";
        pub const CWE: &str = "cwe";
        pub const OWASP: &str = "owasp";
    }

    /// Common metadata values
    pub mod metadata_values {
        pub const DATAFLOW: &str = "dataflow";
        pub const SECURITY: &str = "security";
        pub const PERFORMANCE: &str = "performance";
        pub const SQL_INJECTION: &str = "sql_injection";
        pub const XSS: &str = "xss";
        pub const SECRETS: &str = "secrets";
        pub const CODE_INJECTION: &str = "code_injection";
        pub const MEMORY_CPU: &str = "memory_cpu";
        pub const RENDERING: &str = "rendering";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_reasonable() {
        // Test that constants have reasonable values
        assert!(defaults::server::MAX_UPLOAD_SIZE > 0);
        assert!(defaults::parser::DEFAULT_TIMEOUT_MS > 0);
        assert!(defaults::analysis::CONFIDENCE_THRESHOLD >= 0.0);
        assert!(defaults::analysis::CONFIDENCE_THRESHOLD <= 1.0);
        assert!(defaults::analysis::SIMILARITY_THRESHOLD >= 0.0);
        assert!(defaults::analysis::SIMILARITY_THRESHOLD <= 1.0);
    }

    #[test]
    fn test_duration_helpers() {
        let timeout = durations::request_timeout();
        assert!(timeout.as_secs() > 0);
        
        let cleanup = durations::job_cleanup_interval();
        assert!(cleanup.as_secs() > 0);
    }

    #[test]
    fn test_file_size_constants() {
        // Ensure file size constants are in ascending order where appropriate
        assert!(defaults::parser::BASH_MAX_FILE_SIZE < defaults::parser::SQL_MAX_FILE_SIZE);
        assert!(defaults::parser::SQL_MAX_FILE_SIZE < defaults::parser::DEFAULT_MAX_FILE_SIZE);
        assert!(defaults::parser::DEFAULT_MAX_FILE_SIZE < defaults::parser::PYTHON_MAX_FILE_SIZE);
        assert!(defaults::parser::PYTHON_MAX_FILE_SIZE < defaults::parser::JAVA_MAX_FILE_SIZE);
    }
}
