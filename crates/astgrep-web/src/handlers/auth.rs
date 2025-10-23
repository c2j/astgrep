//! Authentication handlers

use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{WebConfig, WebError, WebResult};

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Token validation response
#[derive(Debug, Serialize)]
pub struct TokenValidationResponse {
    pub valid: bool,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Login endpoint
pub async fn login(
    State(config): State<Arc<WebConfig>>,
    Json(request): Json<LoginRequest>,
) -> WebResult<Json<LoginResponse>> {
    // Validate credentials (simplified implementation)
    if !validate_credentials(&request.username, &request.password) {
        return Err(WebError::unauthorized("Invalid credentials"));
    }

    // Generate JWT token
    let token = generate_jwt_token(&request.username, &config)?;
    
    let response = LoginResponse {
        token,
        expires_in: 3600, // 1 hour
        token_type: "Bearer".to_string(),
    };

    tracing::info!("User logged in: {}", request.username);
    Ok(Json(response))
}

/// Token validation endpoint
pub async fn validate_token(
    State(config): State<Arc<WebConfig>>,
    token: String,
) -> WebResult<Json<TokenValidationResponse>> {
    match validate_jwt_token(&token, &config) {
        Ok(claims) => {
            let response = TokenValidationResponse {
                valid: true,
                user_id: Some(claims.sub.clone()),
                username: Some(claims.sub),
                expires_at: Some(chrono::DateTime::from_timestamp(claims.exp as i64, 0).unwrap_or_default()),
            };
            Ok(Json(response))
        }
        Err(_) => {
            let response = TokenValidationResponse {
                valid: false,
                user_id: None,
                username: None,
                expires_at: None,
            };
            Ok(Json(response))
        }
    }
}

/// Validate user credentials (simplified implementation)
fn validate_credentials(username: &str, password: &str) -> bool {
    // This is a simplified implementation
    // In a real application, you would check against a database
    // and use proper password hashing
    
    match username {
        "admin" => password == "admin123",
        "user" => password == "user123",
        _ => false,
    }
}

/// Generate JWT token
fn generate_jwt_token(username: &str, config: &WebConfig) -> WebResult<String> {
    use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let jwt_secret = config.jwt_secret.as_ref()
        .ok_or_else(|| WebError::internal_server_error("JWT secret not configured"))?;

    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::hours(1);

    let claims = Claims {
        sub: username.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let key = EncodingKey::from_secret(jwt_secret.as_bytes());
    let header = Header::new(Algorithm::HS256);

    encode(&header, &claims, &key)
        .map_err(|e| WebError::internal_server_error(format!("Failed to generate token: {}", e)))
}

/// Validate JWT token
fn validate_jwt_token(token: &str, config: &WebConfig) -> WebResult<TokenClaims> {
    use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

    let jwt_secret = config.jwt_secret.as_ref()
        .ok_or_else(|| WebError::internal_server_error("JWT secret not configured"))?;

    let key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<TokenClaims>(token, &key, &validation)
        .map_err(|e| WebError::unauthorized(format!("Invalid token: {}", e)))?;

    Ok(token_data.claims)
}

/// JWT token claims
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_credentials() {
        assert!(validate_credentials("admin", "admin123"));
        assert!(validate_credentials("user", "user123"));
        assert!(!validate_credentials("admin", "wrong"));
        assert!(!validate_credentials("unknown", "password"));
    }

    #[tokio::test]
    async fn test_login_valid_credentials() {
        let config = Arc::new(WebConfig {
            jwt_secret: Some("test-secret".to_string()),
            ..Default::default()
        });

        let request = LoginRequest {
            username: "admin".to_string(),
            password: "admin123".to_string(),
        };

        let result = login(State(config), Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.token.is_empty());
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let config = Arc::new(WebConfig {
            jwt_secret: Some("test-secret".to_string()),
            ..Default::default()
        });

        let request = LoginRequest {
            username: "admin".to_string(),
            password: "wrong".to_string(),
        };

        let result = login(State(config), Json(request)).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_and_validate_jwt_token() {
        let config = WebConfig {
            jwt_secret: Some("test-secret".to_string()),
            ..Default::default()
        };

        // Generate token
        let token = generate_jwt_token("testuser", &config).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = validate_jwt_token(&token, &config).unwrap();
        assert_eq!(claims.sub, "testuser");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_validate_invalid_jwt_token() {
        let config = WebConfig {
            jwt_secret: Some("test-secret".to_string()),
            ..Default::default()
        };

        let result = validate_jwt_token("invalid-token", &config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_endpoint() {
        let config = Arc::new(WebConfig {
            jwt_secret: Some("test-secret".to_string()),
            ..Default::default()
        });

        // Generate a valid token
        let token = generate_jwt_token("testuser", &config).unwrap();

        // Validate the token
        let result = validate_token(State(config), token).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.valid);
        assert_eq!(response.username, Some("testuser".to_string()));
    }
}
