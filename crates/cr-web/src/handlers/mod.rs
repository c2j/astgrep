//! HTTP request handlers

pub mod analyze;
pub mod common;
pub mod docs;
pub mod health;
pub mod jobs;
pub mod metrics;
pub mod root;
pub mod rules;
pub mod version;

use axum::http::HeaderMap;

/// Extract request ID from headers
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(crate::middleware::REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract user information from request (for authenticated endpoints)
pub fn extract_user_info(_headers: &HeaderMap) -> Option<UserInfo> {
    // This would extract user information from JWT token or session
    // For now, return None (unauthenticated)
    None
}

/// User information extracted from authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
}

/// Common response headers
pub fn add_common_headers(headers: &mut HeaderMap) {
    use axum::http::{header, HeaderValue};
    
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    
    headers.insert(
        "X-API-Version",
        HeaderValue::from_static("v1"),
    );
}
