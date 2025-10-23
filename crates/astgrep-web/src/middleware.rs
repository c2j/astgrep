//! Middleware for the web service

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{WebConfig, WebError, WebResult};

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Add request ID to all requests
pub async fn request_id(
    State(_config): State<Arc<WebConfig>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Generate or extract request ID
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add request ID to request headers
    request.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap(),
    );

    // Log the request
    info!(
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        "Incoming request"
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap(),
    );

    response
}

/// Authentication middleware
pub async fn auth(
    State(config): State<Arc<WebConfig>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> WebResult<Response> {
    // Skip authentication if disabled
    if !config.enable_auth {
        return Ok(next.run(request).await);
    }

    // Extract authorization header
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| WebError::unauthorized("Missing authorization header"))?;

    // Validate JWT token
    if !auth_header.starts_with("Bearer ") {
        return Err(WebError::unauthorized("Invalid authorization header format"));
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix
    
    // Validate the token (simplified implementation)
    validate_jwt_token(token, &config)?;

    Ok(next.run(request).await)
}

/// Rate limiting middleware
pub async fn rate_limit(
    State(config): State<Arc<WebConfig>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> WebResult<Response> {
    // Skip rate limiting if disabled
    if !config.rate_limit.enabled {
        return Ok(next.run(request).await);
    }

    // Extract client IP
    let client_ip = extract_client_ip(&headers)
        .unwrap_or_else(|| "unknown".to_string());

    // Check rate limit (simplified implementation)
    if is_rate_limited(&client_ip, &config).await {
        return Err(WebError::too_many_requests("Rate limit exceeded"));
    }

    Ok(next.run(request).await)
}

/// Request logging middleware
pub async fn request_logging(
    State(config): State<Arc<WebConfig>>,
    request: Request,
    next: Next,
) -> Response {
    if !config.logging.log_requests {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    let start_time = std::time::Instant::now();
    
    info!(
        method = %method,
        uri = %uri,
        user_agent = ?headers.get(header::USER_AGENT),
        "Request started"
    );

    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status();
    
    info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

/// CORS middleware (handled by tower-http, but this is for custom logic)
pub async fn cors_custom(
    State(config): State<Arc<WebConfig>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*");

    // Check if origin is allowed
    let allowed = config.cors.allowed_origins.contains(&"*".to_string()) 
        || config.cors.allowed_origins.contains(&origin.to_string());

    if !allowed {
        warn!(origin = %origin, "CORS: Origin not allowed");
    }

    let mut response = next.run(request).await;

    // Add custom CORS headers if needed
    if allowed {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_str(origin).unwrap(),
        );
    }

    response
}

/// Security headers middleware
pub async fn security_headers(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Add security headers
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );
    
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    response
}

/// Validate JWT token (simplified implementation)
fn validate_jwt_token(token: &str, config: &WebConfig) -> WebResult<()> {
    use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let jwt_secret = config.jwt_secret.as_ref()
        .ok_or_else(|| WebError::internal_server_error("JWT secret not configured"))?;

    let key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);

    decode::<Claims>(token, &key, &validation)
        .map_err(|e| WebError::unauthorized(format!("Invalid token: {}", e)))?;

    Ok(())
}

/// Extract client IP from headers
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // Try various headers in order of preference
    let ip_headers = [
        "x-forwarded-for",
        "x-real-ip",
        "x-client-ip",
        "cf-connecting-ip",
    ];

    for header_name in &ip_headers {
        if let Some(value) = headers.get(*header_name) {
            if let Ok(ip_str) = value.to_str() {
                // Take the first IP if there are multiple (comma-separated)
                let ip = ip_str.split(',').next().unwrap_or(ip_str).trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    None
}

/// Check if client is rate limited (simplified implementation)
async fn is_rate_limited(client_ip: &str, config: &WebConfig) -> bool {
    // This is a simplified implementation
    // In a real application, you would use a proper rate limiting library
    // like tower-governor or implement your own with Redis/in-memory store
    
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static RATE_LIMIT_STORE: std::sync::LazyLock<Mutex<HashMap<String, (Instant, u32)>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut store = RATE_LIMIT_STORE.lock().unwrap();
    let now = Instant::now();
    let window = Duration::from_secs(60); // 1 minute window

    match store.get_mut(client_ip) {
        Some((last_reset, count)) => {
            if now.duration_since(*last_reset) > window {
                // Reset the window
                *last_reset = now;
                *count = 1;
                false
            } else {
                *count += 1;
                *count > config.rate_limit.requests_per_minute
            }
        }
        None => {
            store.insert(client_ip.to_string(), (now, 1));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn test_extract_client_ip() {
        let mut headers = HeaderMap::new();
        
        // Test x-forwarded-for header
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.1, 10.0.0.1"));
        assert_eq!(extract_client_ip(&headers), Some("192.168.1.1".to_string()));
        
        // Test x-real-ip header
        headers.clear();
        headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.2"));
        assert_eq!(extract_client_ip(&headers), Some("192.168.1.2".to_string()));
        
        // Test no headers
        headers.clear();
        assert_eq!(extract_client_ip(&headers), None);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = WebConfig {
            rate_limit: crate::config::RateLimitConfig {
                enabled: true,
                requests_per_minute: 2,
                burst_size: 1,
            },
            ..Default::default()
        };

        // First request should pass
        assert!(!is_rate_limited("test-ip", &config).await);
        
        // Second request should pass
        assert!(!is_rate_limited("test-ip", &config).await);
        
        // Third request should be rate limited
        assert!(is_rate_limited("test-ip", &config).await);
    }
}
