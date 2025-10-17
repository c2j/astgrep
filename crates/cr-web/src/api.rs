//! API utilities and helpers

use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{WebConfig, WebError, WebResult};

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Request metadata
    pub meta: ResponseMeta,
}

/// Response metadata
#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// API version
    pub version: String,
}

/// Pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-based)
    pub page: Option<u32>,
    /// Number of items per page
    pub per_page: Option<u32>,
    /// Offset (alternative to page)
    pub offset: Option<u32>,
    /// Limit (alternative to per_page)
    pub limit: Option<u32>,
}

/// Pagination metadata
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    /// Current page number
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u32,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there is a next page
    pub has_next: bool,
    /// Whether there is a previous page
    pub has_prev: bool,
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    /// Response data
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationMeta,
    /// Request metadata
    pub meta: ResponseMeta,
}

impl<T> ApiResponse<T> {
    /// Create a new API response
    pub fn new(data: T, request_id: Option<String>) -> Self {
        Self {
            data,
            meta: ResponseMeta {
                request_id,
                timestamp: chrono::Utc::now(),
                version: "v1".to_string(),
            },
        }
    }
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(
        data: Vec<T>,
        pagination: PaginationMeta,
        request_id: Option<String>,
    ) -> Self {
        Self {
            data,
            pagination,
            meta: ResponseMeta {
                request_id,
                timestamp: chrono::Utc::now(),
                version: "v1".to_string(),
            },
        }
    }
}

impl PaginationQuery {
    /// Convert to offset and limit
    pub fn to_offset_limit(&self) -> (u32, u32) {
        // Use explicit offset/limit if provided
        if let (Some(offset), Some(limit)) = (self.offset, self.limit) {
            return (offset, limit.min(100)); // Max 100 items per request
        }
        
        // Use page/per_page if provided
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).min(100); // Default 20, max 100
        
        let offset = (page - 1) * per_page;
        (offset, per_page)
    }
}

impl PaginationMeta {
    /// Create pagination metadata
    pub fn new(page: u32, per_page: u32, total: u32) -> Self {
        let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };
        
        Self {
            page,
            per_page,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
    
    /// Create from offset/limit
    pub fn from_offset_limit(offset: u32, limit: u32, total: u32) -> Self {
        let page = (offset / limit) + 1;
        Self::new(page, limit, total)
    }
}

/// API rate limiting information
#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    /// Requests remaining in current window
    pub remaining: u32,
    /// Total requests allowed per window
    pub limit: u32,
    /// Window reset time
    pub reset_at: chrono::DateTime<chrono::Utc>,
}

/// API status endpoint
pub async fn api_status(
    State(_config): State<Arc<WebConfig>>,
) -> WebResult<Json<ApiResponse<ApiStatusData>>> {
    let status_data = ApiStatusData {
        status: "operational".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime_seconds(),
        endpoints: get_available_endpoints(),
    };
    
    let response = ApiResponse::new(status_data, None);
    Ok(Json(response))
}

/// API status data
#[derive(Debug, Serialize)]
pub struct ApiStatusData {
    /// Service status
    pub status: String,
    /// API version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Available endpoints
    pub endpoints: Vec<EndpointInfo>,
}

/// Endpoint information
#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    /// HTTP method
    pub method: String,
    /// Endpoint path
    pub path: String,
    /// Description
    pub description: String,
}

/// Get service uptime
fn get_uptime_seconds() -> u64 {
    // This is a simplified implementation
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() % 86400 // Mock: uptime within last 24 hours
}

/// Get available endpoints
fn get_available_endpoints() -> Vec<EndpointInfo> {
    vec![
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/health".to_string(),
            description: "Health check and system status".to_string(),
        },
        EndpointInfo {
            method: "POST".to_string(),
            path: "/api/v1/analyze".to_string(),
            description: "Analyze code snippet".to_string(),
        },
        EndpointInfo {
            method: "POST".to_string(),
            path: "/api/v1/analyze/file".to_string(),
            description: "Analyze uploaded file".to_string(),
        },
        EndpointInfo {
            method: "POST".to_string(),
            path: "/api/v1/analyze/archive".to_string(),
            description: "Analyze uploaded archive".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/jobs".to_string(),
            description: "List analysis jobs".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/jobs/{id}".to_string(),
            description: "Get job status".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/rules".to_string(),
            description: "List available rules".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/rules/{id}".to_string(),
            description: "Get rule details".to_string(),
        },
        EndpointInfo {
            method: "POST".to_string(),
            path: "/api/v1/rules/validate".to_string(),
            description: "Validate rule definitions".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/version".to_string(),
            description: "Get version information".to_string(),
        },
        EndpointInfo {
            method: "GET".to_string(),
            path: "/api/v1/metrics".to_string(),
            description: "Get service metrics".to_string(),
        },
    ]
}

/// Validate request size
pub fn validate_request_size(content_length: Option<usize>, max_size: usize) -> WebResult<()> {
    if let Some(length) = content_length {
        if length > max_size {
            return Err(WebError::bad_request(format!(
                "Request too large: {} bytes (max: {} bytes)",
                length, max_size
            )));
        }
    }
    Ok(())
}

/// Extract and validate language parameter
pub fn validate_language(language: &str) -> WebResult<()> {
    const SUPPORTED_LANGUAGES: &[&str] = &[
        "java", "javascript", "typescript", "python", "sql", "bash", "c", "cpp", "csharp", "go", "rust", "php", "ruby"
    ];
    
    if !SUPPORTED_LANGUAGES.contains(&language.to_lowercase().as_str()) {
        return Err(WebError::bad_request(format!(
            "Unsupported language: {}. Supported languages: {}",
            language,
            SUPPORTED_LANGUAGES.join(", ")
        )));
    }
    
    Ok(())
}

/// Extract and validate severity parameter
pub fn validate_severity(severity: &str) -> WebResult<()> {
    const VALID_SEVERITIES: &[&str] = &["info", "warning", "error", "critical"];
    
    if !VALID_SEVERITIES.contains(&severity.to_lowercase().as_str()) {
        return Err(WebError::bad_request(format!(
            "Invalid severity: {}. Valid severities: {}",
            severity,
            VALID_SEVERITIES.join(", ")
        )));
    }
    
    Ok(())
}

/// Extract and validate confidence parameter
pub fn validate_confidence(confidence: &str) -> WebResult<()> {
    const VALID_CONFIDENCES: &[&str] = &["low", "medium", "high"];
    
    if !VALID_CONFIDENCES.contains(&confidence.to_lowercase().as_str()) {
        return Err(WebError::bad_request(format!(
            "Invalid confidence: {}. Valid confidences: {}",
            confidence,
            VALID_CONFIDENCES.join(", ")
        )));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_query_to_offset_limit() {
        // Test with page/per_page
        let query = PaginationQuery {
            page: Some(2),
            per_page: Some(10),
            offset: None,
            limit: None,
        };
        assert_eq!(query.to_offset_limit(), (10, 10));
        
        // Test with offset/limit
        let query = PaginationQuery {
            page: None,
            per_page: None,
            offset: Some(20),
            limit: Some(15),
        };
        assert_eq!(query.to_offset_limit(), (20, 15));
        
        // Test defaults
        let query = PaginationQuery {
            page: None,
            per_page: None,
            offset: None,
            limit: None,
        };
        assert_eq!(query.to_offset_limit(), (0, 20));
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(2, 10, 25);
        assert_eq!(meta.page, 2);
        assert_eq!(meta.per_page, 10);
        assert_eq!(meta.total, 25);
        assert_eq!(meta.total_pages, 3);
        assert!(meta.has_next);
        assert!(meta.has_prev);
        
        let meta = PaginationMeta::from_offset_limit(20, 10, 25);
        assert_eq!(meta.page, 3);
        assert!(!meta.has_next);
        assert!(meta.has_prev);
    }

    #[test]
    fn test_validate_language() {
        assert!(validate_language("java").is_ok());
        assert!(validate_language("JavaScript").is_ok()); // Case insensitive
        assert!(validate_language("unknown").is_err());
    }

    #[test]
    fn test_validate_severity() {
        assert!(validate_severity("warning").is_ok());
        assert!(validate_severity("ERROR").is_ok()); // Case insensitive
        assert!(validate_severity("invalid").is_err());
    }

    #[test]
    fn test_validate_confidence() {
        assert!(validate_confidence("medium").is_ok());
        assert!(validate_confidence("HIGH").is_ok()); // Case insensitive
        assert!(validate_confidence("invalid").is_err());
    }

    #[test]
    fn test_validate_request_size() {
        assert!(validate_request_size(Some(1000), 2000).is_ok());
        assert!(validate_request_size(Some(3000), 2000).is_err());
        assert!(validate_request_size(None, 2000).is_ok());
    }

    #[tokio::test]
    async fn test_api_status() {
        let config = Arc::new(WebConfig::default());
        let result = api_status(State(config)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap().0;
        assert_eq!(response.data.status, "operational");
        assert!(!response.data.endpoints.is_empty());
    }
}
