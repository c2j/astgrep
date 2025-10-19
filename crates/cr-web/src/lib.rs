//! CR-SemService Web API
//! 
//! This crate provides a RESTful web service interface for CR-SemService,
//! enabling remote code analysis and integration with CI/CD pipelines.

pub mod api;
pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod server;
pub mod storage;

pub use config::WebConfig;
pub use error::{WebError, WebResult};
pub use server::WebServer;

use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

/// Create the main application router
pub fn create_app(config: Arc<WebConfig>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(Any);

    let middleware_stack = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(DefaultBodyLimit::max(config.max_upload_size))
        .layer(axum_middleware::from_fn_with_state(
            config.clone(),
            middleware::request_id,
        ));

    let api_routes = Router::new()
        .route("/analyze", post(handlers::analyze::analyze_code))
        .route("/analyze/sarif", post(handlers::analyze::analyze_code_sarif))
        .route("/analyze/file", post(handlers::analyze::analyze_file))
        .route("/analyze/archive", post(handlers::analyze::analyze_archive))
        .route("/jobs/:id", get(handlers::jobs::get_job_status))
        .route("/jobs", get(handlers::jobs::list_jobs))
        .route("/rules", get(handlers::rules::list_rules))
        .route("/rules/:id", get(handlers::rules::get_rule))
        .route("/rules/validate", post(handlers::rules::validate_rules))
        .route("/health", get(handlers::health::health_check))
        .route("/metrics", get(handlers::metrics::get_metrics))
        .route("/version", get(handlers::version::get_version));

    let app = Router::new()
        .nest("/api/v1", api_routes)
        .route("/", get(handlers::root::root))
        .route("/docs", get(handlers::docs::api_docs))
        .route("/playground", get(handlers::playground::playground))
        .layer(middleware_stack)
        .with_state(config);

    info!("Web application router created");
    app
}

/// Initialize the web service
pub async fn init_web_service(config: WebConfig) -> Result<WebServer> {
    info!("Initializing CR-SemService web service");
    
    let config = Arc::new(config);
    let app = create_app(config.clone());
    
    let server = WebServer::new(app, config).await?;
    
    info!("Web service initialized successfully");
    Ok(server)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_create_app() {
        let config = Arc::new(WebConfig::default());
        let app = create_app(config);
        
        let server = TestServer::new(app).unwrap();
        
        // Test root endpoint
        let response = server.get("/").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Test health endpoint
        let response = server.get("/api/v1/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_headers() {
        let config = Arc::new(WebConfig::default());
        let app = create_app(config);
        
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .method(axum_test::http::Method::OPTIONS, "/api/v1/health")
            .add_header("Origin", "http://localhost:3000")
            .add_header("Access-Control-Request-Method", "GET")
            .await;
            
        assert_eq!(response.status_code(), StatusCode::OK);
        assert!(response.headers().contains_key("access-control-allow-origin"));
    }
}
