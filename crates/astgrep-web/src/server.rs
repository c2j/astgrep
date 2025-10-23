//! Web server implementation

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

use crate::{WebConfig, WebResult, WebError};

/// Web server instance
pub struct WebServer {
    app: Router,
    config: Arc<WebConfig>,
    listener: TcpListener,
}

impl WebServer {
    /// Create a new web server instance
    pub async fn new(app: Router, config: Arc<WebConfig>) -> WebResult<Self> {
        // Ensure temp directory exists
        config.ensure_temp_directory()
            .map_err(|e| WebError::internal_server_error(format!("Failed to create temp directory: {}", e)))?;

        // Validate configuration
        config.validate()
            .map_err(|e| WebError::internal_server_error(format!("Invalid configuration: {}", e)))?;

        // Bind to the configured address
        let listener = TcpListener::bind(&config.bind_address).await
            .map_err(|e| WebError::internal_server_error(format!("Failed to bind to {}: {}", config.bind_address, e)))?;

        info!("Server bound to {}", config.bind_address);

        Ok(Self {
            app,
            config,
            listener,
        })
    }

    /// Start the web server
    pub async fn serve(self) -> WebResult<()> {
        let addr = self.listener.local_addr()
            .map_err(|e| WebError::internal_server_error(format!("Failed to get local address: {}", e)))?;

        info!("🚀 astgrep web server starting on {}", addr);
        info!("📖 API documentation available at http://{}/docs", addr);
        info!("❤️  Health check available at http://{}/api/v1/health", addr);

        // Start background tasks
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            start_background_tasks(config_clone).await;
        });

        // Serve the application
        axum::serve(self.listener, self.app).await
            .map_err(|e| {
                error!("Server error: {}", e);
                WebError::internal_server_error(format!("Server error: {}", e))
            })?;

        Ok(())
    }

    /// Get the server's bind address
    pub fn bind_address(&self) -> SocketAddr {
        self.config.bind_address
    }

    /// Get the server configuration
    pub fn config(&self) -> &WebConfig {
        &self.config
    }
}

/// Start background tasks
async fn start_background_tasks(config: Arc<WebConfig>) {
    info!("Starting background tasks");

    // Job cleanup task
    let cleanup_config = config.clone();
    tokio::spawn(async move {
        job_cleanup_task(cleanup_config).await;
    });

    // Metrics collection task (if enabled)
    #[cfg(feature = "metrics")]
    if config.metrics.as_ref().map_or(false, |m| m.enabled) {
        let metrics_config = config.clone();
        tokio::spawn(async move {
            metrics_collection_task(metrics_config).await;
        });
    }

    // Health check task
    let health_config = config.clone();
    tokio::spawn(async move {
        health_check_task(health_config).await;
    });
}

/// Background task for cleaning up old jobs
async fn job_cleanup_task(config: Arc<WebConfig>) {
    let mut interval = tokio::time::interval(config.job_cleanup_interval);
    
    loop {
        interval.tick().await;
        
        match cleanup_old_jobs(&config).await {
            Ok(cleaned_count) => {
                if cleaned_count > 0 {
                    info!("Cleaned up {} old jobs", cleaned_count);
                }
            }
            Err(e) => {
                error!("Failed to cleanup old jobs: {}", e);
            }
        }
    }
}

/// Background task for metrics collection
#[cfg(feature = "metrics")]
async fn metrics_collection_task(config: Arc<WebConfig>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Collect every minute
    
    loop {
        interval.tick().await;
        
        if let Err(e) = collect_metrics(&config).await {
            error!("Failed to collect metrics: {}", e);
        }
    }
}

/// Background task for periodic health checks
async fn health_check_task(config: Arc<WebConfig>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Check every 5 minutes
    
    loop {
        interval.tick().await;
        
        if let Err(e) = perform_health_check(&config).await {
            error!("Health check failed: {}", e);
        }
    }
}

/// Clean up old jobs
async fn cleanup_old_jobs(config: &WebConfig) -> WebResult<usize> {
    // This is a simplified implementation
    // In a real application, you would query the job storage and remove old jobs
    
    let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(config.job_retention_duration)
        .map_err(|e| WebError::internal_server_error(format!("Invalid retention duration: {}", e)))?;
    
    // Mock cleanup - in reality, you would query and delete from storage
    let cleaned_count = 0; // Mock: no jobs to clean
    
    if cleaned_count > 0 {
        info!("Cleaned up {} jobs older than {}", cleaned_count, cutoff_time);
    }
    
    Ok(cleaned_count)
}

/// Collect metrics
#[cfg(feature = "metrics")]
async fn collect_metrics(_config: &WebConfig) -> WebResult<()> {
    // This is a simplified implementation
    // In a real application, you would collect and store metrics
    
    // Mock metrics collection
    info!("Collecting metrics");
    
    Ok(())
}

/// Perform periodic health check
async fn perform_health_check(config: &WebConfig) -> WebResult<()> {
    // Check critical dependencies
    
    // Check rules directory
    if !config.rules_directory.exists() {
        return Err(WebError::internal_server_error("Rules directory not accessible"));
    }
    
    // Check temp directory
    if let Err(e) = tokio::fs::create_dir_all(&config.temp_directory).await {
        return Err(WebError::internal_server_error(format!("Cannot access temp directory: {}", e)));
    }
    
    // Check database if enabled
    #[cfg(feature = "database")]
    if let Some(ref _db_config) = config.database {
        // In a real implementation, you would check database connectivity
    }
    
    Ok(())
}

/// Graceful shutdown handler
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_web_server_creation() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            bind_address: "127.0.0.1:0".parse().unwrap(), // Use port 0 for testing
            rules_directory: temp_dir.path().to_path_buf(),
            temp_directory: temp_dir.path().join("temp"),
            ..Default::default()
        });

        let app = Router::new();
        let server = WebServer::new(app, config).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_old_jobs() {
        let temp_dir = tempdir().unwrap();
        let config = WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            temp_directory: temp_dir.path().join("temp"),
            job_retention_duration: std::time::Duration::from_secs(3600),
            ..Default::default()
        };

        let result = cleanup_old_jobs(&config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No jobs to clean in mock
    }

    #[tokio::test]
    async fn test_perform_health_check() {
        let temp_dir = tempdir().unwrap();
        let config = WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            temp_directory: temp_dir.path().join("temp"),
            ..Default::default()
        };

        let result = perform_health_check(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_perform_health_check_missing_rules_dir() {
        let config = WebConfig {
            rules_directory: "/nonexistent/path".into(),
            temp_directory: "/tmp/test".into(),
            ..Default::default()
        };

        let result = perform_health_check(&config).await;
        assert!(result.is_err());
    }
}
