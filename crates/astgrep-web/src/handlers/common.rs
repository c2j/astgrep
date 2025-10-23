//! Common handler utilities and patterns
//!
//! This module provides reusable patterns and utilities to reduce code duplication
//! across different HTTP handlers.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    api::PaginatedResponse,
    models::{JobStatus, AnalysisResponse, AnalysisResults},
    WebConfig, WebError, WebResult,
};

/// Common pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Number of items to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl PaginationQuery {
    /// Get limit with default value
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(50).min(1000) // Cap at 1000
    }

    /// Get offset with default value
    pub fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }
}

/// Common response builder for analysis operations
pub struct AnalysisResponseBuilder {
    job_id: Uuid,
    status: JobStatus,
}

impl AnalysisResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        Self {
            job_id: Uuid::new_v4(),
            status: JobStatus::Pending,
        }
    }

    /// Set job status
    pub fn with_status(mut self, status: JobStatus) -> Self {
        self.status = status;
        self
    }

    /// Build successful response with results
    pub fn success(self, results: AnalysisResults) -> AnalysisResponse {
        AnalysisResponse {
            job_id: self.job_id,
            status: JobStatus::Completed,
            results: Some(results),
            error: None,
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        }
    }

    /// Build error response
    pub fn error(self, error_message: String) -> AnalysisResponse {
        AnalysisResponse {
            job_id: self.job_id,
            status: JobStatus::Failed,
            results: None,
            error: Some(error_message),
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        }
    }
}

/// Common request validation patterns
pub struct RequestValidator;

impl RequestValidator {
    /// Validate that a string field is not empty
    pub fn validate_non_empty(field: &str, field_name: &str) -> WebResult<()> {
        if field.trim().is_empty() {
            return Err(WebError::bad_request(format!("{} cannot be empty", field_name)));
        }
        Ok(())
    }

    /// Validate file content size
    pub fn validate_content_size(content: &[u8], max_size: usize) -> WebResult<()> {
        if content.len() > max_size {
            return Err(WebError::bad_request(format!(
                "Content size {} exceeds maximum allowed size {}",
                content.len(),
                max_size
            )));
        }
        Ok(())
    }

    /// Validate base64 content and decode
    pub fn validate_and_decode_base64(content: &str) -> WebResult<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};
        
        general_purpose::STANDARD
            .decode(content)
            .map_err(|e| WebError::bad_request(format!("Invalid base64 content: {}", e)))
    }

    /// Validate UTF-8 content
    pub fn validate_utf8(content: &[u8]) -> WebResult<String> {
        String::from_utf8(content.to_vec())
            .map_err(|e| WebError::bad_request(format!("Invalid UTF-8 content: {}", e)))
    }
}

/// Common logging patterns
pub struct HandlerLogger;

impl HandlerLogger {
    /// Log request start
    pub fn log_request_start(operation: &str, details: &str) {
        info!("Starting {}: {}", operation, details);
    }

    /// Log request completion
    pub fn log_request_completion(operation: &str, job_id: Uuid, duration: std::time::Duration) {
        info!(
            "Completed {} - job_id: {}, duration: {:?}",
            operation, job_id, duration
        );
    }

    /// Log request error
    pub fn log_request_error(operation: &str, error: &WebError) {
        error!("Failed {}: {}", operation, error);
    }

    /// Log validation warning
    pub fn log_validation_warning(operation: &str, warning: &str) {
        warn!("{} validation warning: {}", operation, warning);
    }
}

/// Common error handling patterns
pub struct ErrorHandler;

impl ErrorHandler {
    /// Handle parsing errors with context
    pub fn handle_parse_error<T>(
        result: Result<T, impl std::fmt::Display>,
        operation: &str,
    ) -> WebResult<T> {
        result.map_err(|e| {
            let error_msg = format!("Failed to parse during {}: {}", operation, e);
            HandlerLogger::log_request_error(operation, &WebError::bad_request(&error_msg));
            WebError::bad_request(error_msg)
        })
    }

    /// Handle analysis errors with context
    pub fn handle_analysis_error<T>(
        result: Result<T, impl std::fmt::Display>,
        operation: &str,
    ) -> WebResult<T> {
        result.map_err(|e| {
            let error_msg = format!("Analysis failed during {}: {}", operation, e);
            HandlerLogger::log_request_error(operation, &WebError::analysis_error(&error_msg));
            WebError::analysis_error(error_msg)
        })
    }
}

/// Macro to reduce boilerplate in handler functions
#[macro_export]
macro_rules! handler_with_logging {
    ($operation:expr, $details:expr, $body:block) => {{
        use crate::handlers::common::{HandlerLogger, AnalysisResponseBuilder};
        
        let start_time = std::time::Instant::now();
        HandlerLogger::log_request_start($operation, $details);
        
        let result = async move $body.await;
        
        match &result {
            Ok(_) => {
                let duration = start_time.elapsed();
                HandlerLogger::log_request_completion($operation, uuid::Uuid::new_v4(), duration);
            }
            Err(e) => {
                HandlerLogger::log_request_error($operation, e);
            }
        }
        
        result
    }};
}

/// Common pagination helper
pub fn paginate_results<T>(
    items: Vec<T>,
    query: &PaginationQuery,
) -> (Vec<T>, crate::api::PaginationMeta) {
    let total = items.len();
    let offset = query.offset();
    let limit = query.limit();
    
    let paginated_items = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let page = (offset / limit) + 1;
    let total_pages = (total + limit - 1) / limit; // Ceiling division

    let pagination = crate::api::PaginationMeta {
        page: page as u32,
        per_page: limit as u32,
        total: total as u32,
        total_pages: total_pages as u32,
        has_next: offset + limit < total,
        has_prev: offset > 0,
    };
    
    (paginated_items, pagination)
}
