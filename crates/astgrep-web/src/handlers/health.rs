//! Health check handler

use axum::{extract::State, response::Json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    models::{HealthResponse, SystemInfo, DependencyStatus},
    WebConfig, WebResult,
};

/// Health check endpoint
pub async fn health_check(
    State(config): State<Arc<WebConfig>>,
) -> WebResult<Json<HealthResponse>> {
    let start_time = std::time::Instant::now();
    
    // Get system information
    let system_info = get_system_info().await;
    
    // Check dependencies
    let dependencies = check_dependencies(&config).await;
    
    // Determine overall status
    let status = if dependencies.values().all(|dep| dep.healthy) {
        "healthy"
    } else {
        "degraded"
    };
    
    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let response = HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        system: system_info,
        dependencies,
    };
    
    let duration = start_time.elapsed();
    tracing::info!("Health check completed in {:?}", duration);
    
    Ok(Json(response))
}

/// Get system information
async fn get_system_info() -> SystemInfo {
    SystemInfo {
        available_memory_bytes: get_available_memory(),
        cpu_usage_percent: get_cpu_usage(),
        disk_usage_percent: get_disk_usage(),
        active_jobs: get_active_jobs_count(),
    }
}

/// Check health of dependencies
async fn check_dependencies(config: &WebConfig) -> HashMap<String, DependencyStatus> {
    let mut dependencies = HashMap::new();
    
    // Check rules directory
    let rules_status = check_rules_directory(&config.rules_directory).await;
    dependencies.insert("rules_directory".to_string(), rules_status);
    
    // Check temporary directory
    let temp_status = check_temp_directory(&config.temp_directory).await;
    dependencies.insert("temp_directory".to_string(), temp_status);
    
    // Check database if enabled
    #[cfg(feature = "database")]
    if let Some(ref db_config) = config.database {
        let db_status = check_database(db_config).await;
        dependencies.insert("database".to_string(), db_status);
    }
    
    dependencies
}

/// Check rules directory accessibility
async fn check_rules_directory(rules_dir: &std::path::Path) -> DependencyStatus {
    let start_time = std::time::Instant::now();
    
    let result = tokio::fs::metadata(rules_dir).await;
    let duration = start_time.elapsed();
    
    match result {
        Ok(metadata) if metadata.is_dir() => DependencyStatus {
            healthy: true,
            response_time_ms: Some(duration.as_millis() as u64),
            error: None,
        },
        Ok(_) => DependencyStatus {
            healthy: false,
            response_time_ms: Some(duration.as_millis() as u64),
            error: Some("Rules path is not a directory".to_string()),
        },
        Err(e) => DependencyStatus {
            healthy: false,
            response_time_ms: Some(duration.as_millis() as u64),
            error: Some(format!("Cannot access rules directory: {}", e)),
        },
    }
}

/// Check temporary directory accessibility
async fn check_temp_directory(temp_dir: &std::path::Path) -> DependencyStatus {
    let start_time = std::time::Instant::now();
    
    // Try to create the directory if it doesn't exist
    let create_result = tokio::fs::create_dir_all(temp_dir).await;
    
    let duration = start_time.elapsed();
    
    match create_result {
        Ok(()) => {
            // Try to write a test file
            let test_file = temp_dir.join("health_check_test");
            match tokio::fs::write(&test_file, b"test").await {
                Ok(()) => {
                    // Clean up test file
                    let _ = tokio::fs::remove_file(&test_file).await;
                    DependencyStatus {
                        healthy: true,
                        response_time_ms: Some(duration.as_millis() as u64),
                        error: None,
                    }
                }
                Err(e) => DependencyStatus {
                    healthy: false,
                    response_time_ms: Some(duration.as_millis() as u64),
                    error: Some(format!("Cannot write to temp directory: {}", e)),
                },
            }
        }
        Err(e) => DependencyStatus {
            healthy: false,
            response_time_ms: Some(duration.as_millis() as u64),
            error: Some(format!("Cannot create temp directory: {}", e)),
        },
    }
}

/// Check database connectivity
#[cfg(feature = "database")]
async fn check_database(db_config: &crate::config::DatabaseConfig) -> DependencyStatus {
    use sqlx::SqlitePool;
    
    let start_time = std::time::Instant::now();
    
    match SqlitePool::connect(&db_config.url).await {
        Ok(pool) => {
            // Try a simple query
            match sqlx::query("SELECT 1").fetch_one(&pool).await {
                Ok(_) => DependencyStatus {
                    healthy: true,
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    error: None,
                },
                Err(e) => DependencyStatus {
                    healthy: false,
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    error: Some(format!("Database query failed: {}", e)),
                },
            }
        }
        Err(e) => DependencyStatus {
            healthy: false,
            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
            error: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Get available memory (simplified implementation)
fn get_available_memory() -> u64 {
    // This is a simplified implementation
    // In a real application, you would use a system information library
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return a default value
    1024 * 1024 * 1024 // 1GB
}

/// Get CPU usage percentage
fn get_cpu_usage() -> f64 {
    // Get CPU usage from metrics collector
    use crate::handlers::metrics::get_metrics_collector;
    get_metrics_collector().get_cpu_usage()
}

/// Get disk usage percentage
fn get_disk_usage() -> f64 {
    // Get disk usage from system
    // For now, we'll use a simple approximation
    // In a production system, you would use a proper system monitoring library
    use std::fs;

    if let Ok(_metadata) = fs::metadata("/") {
        // This is a simplified calculation
        // In reality, you'd need to check available vs total space
        50.0 // Placeholder - would need proper disk space calculation
    } else {
        0.0
    }
}

/// Get number of active analysis jobs
fn get_active_jobs_count() -> usize {
    // Get active jobs count from metrics collector
    use crate::handlers::metrics::get_metrics_collector;
    get_metrics_collector().get_active_jobs() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            temp_directory: temp_dir.path().join("temp"),
            ..Default::default()
        });

        let result = health_check(State(config)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.status.is_empty());
        assert!(!response.version.is_empty());
    }

    #[tokio::test]
    async fn test_check_rules_directory() {
        let temp_dir = tempdir().unwrap();
        let status = check_rules_directory(temp_dir.path()).await;
        assert!(status.healthy);
        assert!(status.response_time_ms.is_some());
        assert!(status.error.is_none());
    }

    #[tokio::test]
    async fn test_check_temp_directory() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().join("test_temp");
        
        let status = check_temp_directory(&temp_path).await;
        assert!(status.healthy);
        assert!(status.response_time_ms.is_some());
        assert!(status.error.is_none());
        
        // Verify directory was created
        assert!(temp_path.exists());
    }

    #[test]
    fn test_get_system_info() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let system_info = rt.block_on(get_system_info());
        
        assert!(system_info.available_memory_bytes > 0);
        assert!(system_info.cpu_usage_percent >= 0.0);
        assert!(system_info.disk_usage_percent >= 0.0);
    }
}
