//! Error types for CR-SemService

use thiserror::Error;

/// Result type alias for CR-SemService operations
pub type Result<T> = std::result::Result<T, AnalysisError>;

/// Main error type for the analysis engine
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Rule validation error: {message}")]
    RuleValidationError { message: String },

    #[error("Pattern matching error: {message}")]
    PatternMatchError { message: String },

    #[error("Data flow analysis error: {message}")]
    DataFlowError { message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Language not supported: {language}")]
    UnsupportedLanguage { language: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },

    #[error("Timeout error: {message}")]
    TimeoutError { message: String },

    #[error("Resource limit error: {message}")]
    ResourceLimitError { message: String },

    #[error("Recoverable error: {message}")]
    RecoverableError { message: String },
}

impl AnalysisError {
    /// Create a new parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
        }
    }

    /// Create a new rule validation error
    pub fn rule_validation_error(message: impl Into<String>) -> Self {
        Self::RuleValidationError {
            message: message.into(),
        }
    }

    /// Create a new pattern match error
    pub fn pattern_match_error(message: impl Into<String>) -> Self {
        Self::PatternMatchError {
            message: message.into(),
        }
    }

    /// Create a new data flow error
    pub fn data_flow_error(message: impl Into<String>) -> Self {
        Self::DataFlowError {
            message: message.into(),
        }
    }

    /// Create a new configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// Create a new unsupported language error
    pub fn unsupported_language(language: impl Into<String>) -> Self {
        Self::UnsupportedLanguage {
            language: language.into(),
        }
    }

    /// Create a new internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Create a new IO error
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::IoError(std::io::Error::new(std::io::ErrorKind::Other, message.into()))
    }

    /// Create a new timeout error
    pub fn timeout_error(message: impl Into<String>) -> Self {
        Self::TimeoutError {
            message: message.into(),
        }
    }

    /// Create a new resource limit error
    pub fn resource_limit_error(message: impl Into<String>) -> Self {
        Self::ResourceLimitError {
            message: message.into(),
        }
    }

    /// Create a new recoverable error
    pub fn recoverable_error(message: impl Into<String>) -> Self {
        Self::RecoverableError {
            message: message.into(),
        }
    }

    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::ParseError { .. } => "parsing",
            Self::RuleValidationError { .. } => "rule_validation",
            Self::PatternMatchError { .. } => "pattern_matching",
            Self::DataFlowError { .. } => "data_flow",
            Self::IoError(_) => "io",
            Self::SerializationError(_) => "serialization",
            Self::JsonError(_) => "json",
            Self::ConfigError { .. } => "config",
            Self::UnsupportedLanguage { .. } => "unsupported_language",
            Self::InternalError { .. } => "internal",
            Self::TimeoutError { .. } => "timeout",
            Self::ResourceLimitError { .. } => "resource_limit",
            Self::RecoverableError { .. } => "recoverable",
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::ParseError { .. } => false,
            Self::RuleValidationError { .. } => true,
            Self::PatternMatchError { .. } => true,
            Self::DataFlowError { .. } => true,
            Self::IoError(_) => false,
            Self::SerializationError(_) => false,
            Self::JsonError(_) => false,
            Self::ConfigError { .. } => true,
            Self::UnsupportedLanguage { .. } => false,
            Self::InternalError { .. } => false,
            Self::TimeoutError { .. } => true,
            Self::ResourceLimitError { .. } => true,
            Self::RecoverableError { .. } => true,
        }
    }

    /// Get suggested action for error
    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::ParseError { .. } => "Check source code syntax",
            Self::RuleValidationError { .. } => "Check rule configuration",
            Self::PatternMatchError { .. } => "Review pattern syntax",
            Self::DataFlowError { .. } => "Check data flow configuration",
            Self::IoError(_) => "Check file permissions and paths",
            Self::SerializationError(_) => "Check YAML format",
            Self::JsonError(_) => "Check JSON format",
            Self::ConfigError { .. } => "Review configuration settings",
            Self::UnsupportedLanguage { .. } => "Use a supported language",
            Self::InternalError { .. } => "Report this as a bug",
            Self::TimeoutError { .. } => "Increase timeout or reduce complexity",
            Self::ResourceLimitError { .. } => "Increase resource limits or reduce input size",
            Self::RecoverableError { .. } => "Follow the suggested recovery action",
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::ParseError { .. } => ErrorSeverity::High,
            Self::RuleValidationError { .. } => ErrorSeverity::Medium,
            Self::PatternMatchError { .. } => ErrorSeverity::Medium,
            Self::DataFlowError { .. } => ErrorSeverity::Medium,
            Self::IoError(_) => ErrorSeverity::High,
            Self::SerializationError(_) => ErrorSeverity::Medium,
            Self::JsonError(_) => ErrorSeverity::Medium,
            Self::ConfigError { .. } => ErrorSeverity::Medium,
            Self::UnsupportedLanguage { .. } => ErrorSeverity::Low,
            Self::InternalError { .. } => ErrorSeverity::Critical,
            Self::TimeoutError { .. } => ErrorSeverity::Medium,
            Self::ResourceLimitError { .. } => ErrorSeverity::Medium,
            Self::RecoverableError { .. } => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Low => "low",
            ErrorSeverity::Medium => "medium",
            ErrorSeverity::High => "high",
            ErrorSeverity::Critical => "critical",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let parse_err = AnalysisError::parse_error("test parse error");
        assert!(matches!(parse_err, AnalysisError::ParseError { .. }));

        let rule_err = AnalysisError::rule_validation_error("test rule error");
        assert!(matches!(rule_err, AnalysisError::RuleValidationError { .. }));

        let pattern_err = AnalysisError::pattern_match_error("test pattern error");
        assert!(matches!(pattern_err, AnalysisError::PatternMatchError { .. }));

        let dataflow_err = AnalysisError::data_flow_error("test dataflow error");
        assert!(matches!(dataflow_err, AnalysisError::DataFlowError { .. }));

        let config_err = AnalysisError::config_error("test config error");
        assert!(matches!(config_err, AnalysisError::ConfigError { .. }));

        let lang_err = AnalysisError::unsupported_language("unknown");
        assert!(matches!(lang_err, AnalysisError::UnsupportedLanguage { .. }));

        let internal_err = AnalysisError::internal_error("test internal error");
        assert!(matches!(internal_err, AnalysisError::InternalError { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = AnalysisError::parse_error("syntax error");
        assert_eq!(err.to_string(), "Parse error: syntax error");

        let err = AnalysisError::unsupported_language("cobol");
        assert_eq!(err.to_string(), "Language not supported: cobol");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let analysis_err: AnalysisError = io_err.into();
        assert!(matches!(analysis_err, AnalysisError::IoError(_)));
    }
}
