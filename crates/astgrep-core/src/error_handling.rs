//! Unified error handling utilities for astgrep
//! 
//! This module provides common error handling patterns and utilities to reduce
//! code duplication across the codebase.

use crate::{AnalysisError, Result};
use std::fmt;
use tracing::{error, warn, debug};

/// Error context for providing additional information about errors
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub additional_info: Vec<(String, String)>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            file_path: None,
            line_number: None,
            additional_info: Vec::new(),
        }
    }

    /// Add file path to the context
    pub fn with_file(mut self, file_path: impl Into<String>) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    /// Add line number to the context
    pub fn with_line(mut self, line_number: usize) -> Self {
        self.line_number = Some(line_number);
        self
    }

    /// Add additional information
    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.push((key.into(), value.into()));
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Operation: {}", self.operation)?;
        
        if let Some(file) = &self.file_path {
            write!(f, ", File: {}", file)?;
        }
        
        if let Some(line) = self.line_number {
            write!(f, ", Line: {}", line)?;
        }
        
        for (key, value) in &self.additional_info {
            write!(f, ", {}: {}", key, value)?;
        }
        
        Ok(())
    }
}

/// Common error handling utilities
pub struct ErrorHandler;

impl ErrorHandler {
    /// Handle parsing errors with context
    pub fn handle_parse_error<T>(
        result: std::result::Result<T, Box<dyn std::error::Error>>,
        context: ErrorContext,
    ) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                error!("Parse error: {} - Context: {}", err, context);
                Err(AnalysisError::parse_error(format!(
                    "Failed to parse: {} ({})", err, context
                )))
            }
        }
    }

    /// Handle IO errors with context
    pub fn handle_io_error<T>(
        result: std::io::Result<T>,
        context: ErrorContext,
    ) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                error!("IO error: {} - Context: {}", err, context);
                Err(AnalysisError::io_error(format!(
                    "IO operation failed: {} ({})", err, context
                )))
            }
        }
    }

    /// Handle configuration errors with context
    pub fn handle_config_error<T>(
        result: std::result::Result<T, Box<dyn std::error::Error>>,
        context: ErrorContext,
    ) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                error!("Configuration error: {} - Context: {}", err, context);
                Err(AnalysisError::config_error(format!(
                    "Configuration error: {} ({})", err, context
                )))
            }
        }
    }

    /// Handle timeout errors
    pub fn handle_timeout_error(operation: &str, timeout_ms: u64) -> AnalysisError {
        warn!("Operation '{}' timed out after {}ms", operation, timeout_ms);
        AnalysisError::timeout_error(format!(
            "Operation '{}' timed out after {}ms", operation, timeout_ms
        ))
    }

    /// Handle memory limit errors
    pub fn handle_memory_limit_error(operation: &str, limit_bytes: usize) -> AnalysisError {
        warn!("Operation '{}' exceeded memory limit of {} bytes", operation, limit_bytes);
        AnalysisError::resource_limit_error(format!(
            "Operation '{}' exceeded memory limit of {} bytes", operation, limit_bytes
        ))
    }

    /// Log and convert errors with recovery suggestions
    pub fn log_and_convert_error(
        err: impl std::error::Error,
        context: ErrorContext,
        recovery_suggestion: Option<&str>,
    ) -> AnalysisError {
        let error_msg = format!("Error: {} - Context: {}", err, context);
        
        if let Some(suggestion) = recovery_suggestion {
            warn!("{} - Suggestion: {}", error_msg, suggestion);
            AnalysisError::recoverable_error(format!("{} - Try: {}", error_msg, suggestion))
        } else {
            error!("{}", error_msg);
            AnalysisError::internal_error(error_msg)
        }
    }
}

/// Trait for adding error context to results
pub trait WithErrorContext<T> {
    /// Add error context to a result
    fn with_context(self, context: ErrorContext) -> Result<T>;
    
    /// Add error context with operation name
    fn with_operation(self, operation: impl Into<String>) -> Result<T>;
    
    /// Add error context with file information
    fn with_file_context(self, operation: impl Into<String>, file_path: impl Into<String>) -> Result<T>;
}

impl<T, E> WithErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + 'static,
{
    fn with_context(self, context: ErrorContext) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => {
                let error_msg = format!("Error: {} - Context: {}", err, context);
                debug!("Converting error with context: {}", error_msg);
                Err(AnalysisError::internal_error(error_msg))
            }
        }
    }

    fn with_operation(self, operation: impl Into<String>) -> Result<T> {
        self.with_context(ErrorContext::new(operation))
    }

    fn with_file_context(self, operation: impl Into<String>, file_path: impl Into<String>) -> Result<T> {
        self.with_context(ErrorContext::new(operation).with_file(file_path))
    }
}

/// Macro for creating error contexts quickly
#[macro_export]
macro_rules! error_context {
    ($operation:expr) => {
        ErrorContext::new($operation)
    };
    ($operation:expr, file: $file:expr) => {
        ErrorContext::new($operation).with_file($file)
    };
    ($operation:expr, file: $file:expr, line: $line:expr) => {
        ErrorContext::new($operation).with_file($file).with_line($line)
    };
    ($operation:expr, $($key:expr => $value:expr),+) => {
        {
            let mut ctx = ErrorContext::new($operation);
            $(
                ctx = ctx.with_info($key, $value);
            )+
            ctx
        }
    };
}

/// Macro for handling common error patterns
#[macro_export]
macro_rules! handle_error {
    (parse: $result:expr, $context:expr) => {
        ErrorHandler::handle_parse_error($result, $context)
    };
    (io: $result:expr, $context:expr) => {
        ErrorHandler::handle_io_error($result, $context)
    };
    (config: $result:expr, $context:expr) => {
        ErrorHandler::handle_config_error($result, $context)
    };
}

/// Recovery strategies for different types of errors
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry the operation with different parameters
    Retry { max_attempts: u32, delay_ms: u64 },
    /// Use a fallback implementation
    Fallback { description: String },
    /// Skip the problematic item and continue
    Skip { reason: String },
    /// Fail fast - no recovery possible
    FailFast,
}

impl RecoveryStrategy {
    /// Get a human-readable description of the recovery strategy
    pub fn description(&self) -> String {
        match self {
            RecoveryStrategy::Retry { max_attempts, delay_ms } => {
                format!("Retry up to {} times with {}ms delay", max_attempts, delay_ms)
            }
            RecoveryStrategy::Fallback { description } => {
                format!("Use fallback: {}", description)
            }
            RecoveryStrategy::Skip { reason } => {
                format!("Skip item: {}", reason)
            }
            RecoveryStrategy::FailFast => {
                "No recovery possible - fail immediately".to_string()
            }
        }
    }
}

/// Error recovery utilities
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Attempt to recover from an error using the specified strategy
    pub fn attempt_recovery<T, F>(
        operation: F,
        strategy: RecoveryStrategy,
        context: ErrorContext,
    ) -> Result<Option<T>>
    where
        F: Fn() -> Result<T>,
    {
        match strategy {
            RecoveryStrategy::Retry { max_attempts, delay_ms } => {
                for attempt in 1..=max_attempts {
                    match operation() {
                        Ok(result) => return Ok(Some(result)),
                        Err(err) => {
                            if attempt < max_attempts {
                                warn!(
                                    "Attempt {}/{} failed for {}: {} - Retrying in {}ms",
                                    attempt, max_attempts, context.operation, err, delay_ms
                                );
                                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                            } else {
                                error!(
                                    "All {} attempts failed for {}: {}",
                                    max_attempts, context.operation, err
                                );
                                return Err(err);
                            }
                        }
                    }
                }
                Ok(None)
            }
            RecoveryStrategy::Skip { reason } => {
                warn!("Skipping operation {}: {}", context.operation, reason);
                Ok(None)
            }
            RecoveryStrategy::Fallback { description } => {
                warn!("Using fallback for {}: {}", context.operation, description);
                // Fallback implementation would be provided by the caller
                Ok(None)
            }
            RecoveryStrategy::FailFast => {
                // Just execute once and return the result
                operation().map(Some)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let ctx = ErrorContext::new("test_operation")
            .with_file("test.rs")
            .with_line(42)
            .with_info("key", "value");

        assert_eq!(ctx.operation, "test_operation");
        assert_eq!(ctx.file_path, Some("test.rs".to_string()));
        assert_eq!(ctx.line_number, Some(42));
        assert_eq!(ctx.additional_info.len(), 1);
    }

    #[test]
    fn test_error_context_display() {
        let ctx = ErrorContext::new("test_operation")
            .with_file("test.rs")
            .with_line(42);

        let display = format!("{}", ctx);
        assert!(display.contains("test_operation"));
        assert!(display.contains("test.rs"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_with_error_context() {
        let result: std::result::Result<i32, std::io::Error> = 
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "test error"));
        
        let context_result = result.with_operation("test_operation");
        assert!(context_result.is_err());
    }

    #[test]
    fn test_recovery_strategy_description() {
        let retry = RecoveryStrategy::Retry { max_attempts: 3, delay_ms: 1000 };
        assert!(retry.description().contains("3 times"));
        
        let fallback = RecoveryStrategy::Fallback { description: "use default".to_string() };
        assert!(fallback.description().contains("use default"));
    }
}
