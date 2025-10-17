//! Error handling for the web service

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Web service result type
pub type WebResult<T> = Result<T, WebError>;

/// Web service error types
#[derive(Error, Debug)]
pub enum WebError {
    /// Bad request error
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    /// Unauthorized error
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    /// Forbidden error
    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    /// Not found error
    #[error("Not found: {message}")]
    NotFound { message: String },

    /// Conflict error
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// Unprocessable entity error
    #[error("Unprocessable entity: {message}")]
    UnprocessableEntity { message: String },

    /// Too many requests error
    #[error("Too many requests: {message}")]
    TooManyRequests { message: String },

    /// Internal server error
    #[error("Internal server error: {message}")]
    InternalServerError { message: String },

    /// Service unavailable error
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// Analysis error
    #[error("Analysis error: {source}")]
    AnalysisError {
        #[from]
        source: anyhow::Error,
    },

    /// Validation error
    #[error("Validation error: {message}")]
    ValidationError { message: String },

    /// File processing error
    #[error("File processing error: {message}")]
    FileProcessingError { message: String },

    /// Database error
    #[cfg(feature = "database")]
    #[error("Database error: {source}")]
    DatabaseError {
        #[from]
        source: sqlx::Error,
    },

    /// JSON serialization error
    #[error("JSON error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    /// IO error
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// JWT error
    #[error("JWT error: {source}")]
    JwtError {
        #[from]
        source: jsonwebtoken::errors::Error,
    },
}

/// Error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
    /// Additional details (optional)
    pub details: Option<serde_json::Value>,
    /// Request ID for tracking
    pub request_id: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl WebError {
    /// Create a bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Create an unauthorized error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// Create a forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Create an unprocessable entity error
    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::UnprocessableEntity {
            message: message.into(),
        }
    }

    /// Create a too many requests error
    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::TooManyRequests {
            message: message.into(),
        }
    }

    /// Create an internal server error
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::InternalServerError {
            message: message.into(),
        }
    }

    /// Create a service unavailable error
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create a file processing error
    pub fn file_processing_error(message: impl Into<String>) -> Self {
        Self::FileProcessingError {
            message: message.into(),
        }
    }

    /// Create an analysis error
    pub fn analysis_error(message: impl Into<String>) -> Self {
        Self::AnalysisError {
            source: anyhow::anyhow!(message.into()),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::AnalysisError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ValidationError { .. } => StatusCode::BAD_REQUEST,
            Self::FileProcessingError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            #[cfg(feature = "database")]
            Self::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::JsonError { .. } => StatusCode::BAD_REQUEST,
            Self::IoError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::JwtError { .. } => StatusCode::UNAUTHORIZED,
        }
    }

    /// Get the error code string
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::BadRequest { .. } => "BAD_REQUEST",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::UnprocessableEntity { .. } => "UNPROCESSABLE_ENTITY",
            Self::TooManyRequests { .. } => "TOO_MANY_REQUESTS",
            Self::InternalServerError { .. } => "INTERNAL_SERVER_ERROR",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            Self::AnalysisError { .. } => "ANALYSIS_ERROR",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
            Self::FileProcessingError { .. } => "FILE_PROCESSING_ERROR",
            #[cfg(feature = "database")]
            Self::DatabaseError { .. } => "DATABASE_ERROR",
            Self::JsonError { .. } => "JSON_ERROR",
            Self::IoError { .. } => "IO_ERROR",
            Self::JwtError { .. } => "JWT_ERROR",
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            details: None,
            request_id: None, // This would be set by middleware
            timestamp: chrono::Utc::now(),
        };

        tracing::error!("Web error: {} - {}", status_code, self);

        (status_code, Json(error_response)).into_response()
    }
}

// From traits are automatically generated by thiserror for #[from] fields

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(WebError::bad_request("test").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(WebError::unauthorized("test").status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(WebError::not_found("test").status_code(), StatusCode::NOT_FOUND);
        assert_eq!(WebError::internal_server_error("test").status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(WebError::bad_request("test").error_code(), "BAD_REQUEST");
        assert_eq!(WebError::unauthorized("test").error_code(), "UNAUTHORIZED");
        assert_eq!(WebError::not_found("test").error_code(), "NOT_FOUND");
    }

    #[test]
    fn test_error_display() {
        let error = WebError::bad_request("Invalid input");
        assert_eq!(error.to_string(), "Bad request: Invalid input");
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "TEST_ERROR".to_string(),
            message: "Test message".to_string(),
            details: None,
            request_id: Some("req-123".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
    }
}
