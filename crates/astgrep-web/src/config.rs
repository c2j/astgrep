//! Web service configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use astgrep_core::constants::{defaults, durations};

/// Web service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Server bind address
    pub bind_address: SocketAddr,
    
    /// Maximum request body size in bytes
    pub max_upload_size: usize,
    
    /// Request timeout duration
    pub request_timeout: Duration,
    
    /// Maximum number of concurrent analysis jobs
    pub max_concurrent_jobs: usize,
    
    /// Job cleanup interval
    pub job_cleanup_interval: Duration,
    
    /// Job retention duration
    pub job_retention_duration: Duration,
    
    /// Rules directory
    pub rules_directory: PathBuf,
    
    /// Temporary files directory
    pub temp_directory: PathBuf,
    
    /// Enable authentication
    pub enable_auth: bool,
    
    /// JWT secret key (for authentication)
    pub jwt_secret: Option<String>,
    
    /// API rate limiting
    pub rate_limit: RateLimitConfig,
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Database configuration (optional)
    #[cfg(feature = "database")]
    pub database: Option<DatabaseConfig>,
    
    /// Metrics configuration
    #[cfg(feature = "metrics")]
    pub metrics: Option<MetricsConfig>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Requests per minute per IP
    pub requests_per_minute: u32,
    
    /// Burst size
    pub burst_size: u32,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    
    /// Max age for preflight requests
    pub max_age: Duration,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Enable request logging
    pub log_requests: bool,
    
    /// Enable response logging
    pub log_responses: bool,
    
    /// Log file path (optional)
    pub log_file: Option<PathBuf>,
}

/// Database configuration
#[cfg(feature = "database")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    
    /// Maximum number of connections
    pub max_connections: u32,
    
    /// Connection timeout
    pub connect_timeout: Duration,
    
    /// Enable migrations
    pub auto_migrate: bool,
}

/// Metrics configuration
#[cfg(feature = "metrics")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    
    /// Metrics endpoint path
    pub endpoint: String,
    
    /// Prometheus metrics port
    pub prometheus_port: Option<u16>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_address: defaults::server::BIND_ADDRESS.parse().unwrap(),
            max_upload_size: defaults::server::MAX_UPLOAD_SIZE,
            request_timeout: durations::request_timeout(),
            max_concurrent_jobs: defaults::server::MAX_CONCURRENT_JOBS,
            job_cleanup_interval: durations::job_cleanup_interval(),
            job_retention_duration: durations::job_retention_duration(),
            rules_directory: PathBuf::from(defaults::server::RULES_DIRECTORY),
            temp_directory: PathBuf::from(defaults::server::TEMP_DIRECTORY),
            enable_auth: false,
            jwt_secret: None,
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            logging: LoggingConfig::default(),
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: defaults::rate_limit::REQUESTS_PER_MINUTE,
            burst_size: defaults::rate_limit::BURST_SIZE,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: defaults::cors::ALLOWED_ORIGINS.iter().map(|s| s.to_string()).collect(),
            allowed_methods: defaults::cors::ALLOWED_METHODS.iter().map(|s| s.to_string()).collect(),
            allowed_headers: defaults::cors::ALLOWED_HEADERS.iter().map(|s| s.to_string()).collect(),
            max_age: durations::cors_max_age(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_requests: true,
            log_responses: false,
            log_file: None,
        }
    }
}

impl WebConfig {
    /// Load configuration from file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_upload_size == 0 {
            return Err(anyhow::anyhow!("max_upload_size must be greater than 0"));
        }
        
        if self.max_concurrent_jobs == 0 {
            return Err(anyhow::anyhow!("max_concurrent_jobs must be greater than 0"));
        }
        
        if !self.rules_directory.exists() {
            return Err(anyhow::anyhow!("rules_directory does not exist: {}", self.rules_directory.display()));
        }
        
        if self.enable_auth && self.jwt_secret.is_none() {
            return Err(anyhow::anyhow!("jwt_secret is required when authentication is enabled"));
        }
        
        Ok(())
    }
    
    /// Create temporary directory if it doesn't exist
    pub fn ensure_temp_directory(&self) -> anyhow::Result<()> {
        if !self.temp_directory.exists() {
            std::fs::create_dir_all(&self.temp_directory)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = WebConfig::default();
        assert_eq!(config.bind_address.port(), 8080);
        assert_eq!(config.max_upload_size, 100 * 1024 * 1024);
        assert!(!config.enable_auth);
    }

    #[test]
    fn test_config_serialization() {
        let config = WebConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: WebConfig = toml::from_str(&serialized).unwrap();
        
        assert_eq!(config.bind_address, deserialized.bind_address);
        assert_eq!(config.max_upload_size, deserialized.max_upload_size);
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config = WebConfig::default();
        config.to_file(&config_path).unwrap();
        
        let loaded_config = WebConfig::from_file(&config_path).unwrap();
        assert_eq!(config.bind_address, loaded_config.bind_address);
    }

    #[test]
    fn test_config_validation() {
        let mut config = WebConfig::default();
        config.rules_directory = PathBuf::from("/non/existent/path");
        assert!(config.validate().is_err()); // rules_directory doesn't exist
        
        config.rules_directory = std::env::temp_dir();
        assert!(config.validate().is_ok());
        
        config.enable_auth = true;
        assert!(config.validate().is_err()); // jwt_secret is None
        
        config.jwt_secret = Some("secret".to_string());
        assert!(config.validate().is_ok());
    }
}
