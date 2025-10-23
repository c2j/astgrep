//! astgrep Web Server Binary
//! 
//! This binary starts the astgrep web server with REST API endpoints
//! for code analysis, job management, and system monitoring.

use anyhow::Result;
use clap::Parser;
use astgrep_web::{init_web_service, WebConfig};
use std::path::PathBuf;
use tracing::{info, error};

/// astgrep Web Server
#[derive(Parser)]
#[command(name = "astgrep-web-server")]
#[command(about = "astgrep REST API Server for static code analysis")]
#[command(version)]
#[command(long_about = "astgrep Web Server provides a REST API for static code analysis.\n\n\
USAGE:\n    \
astgrep-web-server [OPTIONS]\n\n\
EXAMPLES:\n    \
# Start with default configuration\n    \
astgrep-web-server\n\n    \
# Start with custom configuration file\n    \
astgrep-web-server --config /etc/astgrep/config.toml\n\n    \
# Override bind address and port\n    \
astgrep-web-server --bind 0.0.0.0 --port 9090\n\n    \
# Generate default configuration file\n    \
astgrep-web-server --generate-config\n\n    \
# Enable verbose logging\n    \
astgrep-web-server --verbose\n\n\
CONFIGURATION:\n    \
Configuration can be provided via TOML file or command-line arguments.\n    \
Command-line arguments override configuration file settings.\n\n\
ENDPOINTS:\n    \
- GET  /api/v1/health              - Health check\n    \
- POST /api/v1/analyze             - Analyze code\n    \
- GET  /api/v1/jobs/{id}           - Get job status\n    \
- GET  /api/v1/jobs/{id}/result    - Get job result\n    \
- GET  /docs                        - API documentation\n    \
- GET  /playground                 - Interactive playground")]
struct Args {
    /// Configuration file path (default: astgrep-web.toml)
    #[arg(short, long, default_value = "astgrep-web.toml",
          value_name = "FILE",
          help = "Path to TOML configuration file")]
    config: PathBuf,

    /// Server bind address (e.g., 127.0.0.1, 0.0.0.0)
    #[arg(short, long,
          value_name = "ADDR",
          help = "Server bind address (overrides config file)")]
    bind: Option<String>,

    /// Port to bind to (1-65535)
    #[arg(short, long,
          value_name = "PORT",
          help = "Server port (overrides config file)")]
    port: Option<u16>,

    /// Rules directory path
    #[arg(short, long,
          value_name = "DIR",
          help = "Directory containing analysis rules (overrides config file)")]
    rules: Option<PathBuf>,

    /// Enable verbose logging (debug level)
    #[arg(short, long,
          help = "Enable verbose logging output")]
    verbose: bool,

    /// Generate default configuration file and exit
    #[arg(long,
          help = "Generate default configuration file at specified path")]
    generate_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose)?;

    info!("🚀 Starting astgrep Web Server");

    // Generate config if requested
    if args.generate_config {
        return generate_default_config(&args.config);
    }

    // Load configuration
    let mut config = load_config(&args.config).await?;

    // Override config with command line arguments
    if let Some(bind_addr) = args.bind {
        if let Some(port) = args.port {
            config.bind_address = format!("{}:{}", bind_addr, port).parse()?;
        } else {
            config.bind_address = bind_addr.parse()?;
        }
    } else if let Some(port) = args.port {
        config.bind_address.set_port(port);
    }

    if let Some(rules_dir) = args.rules {
        config.rules_directory = rules_dir;
    }

    // Initialize and start the web service
    let server = init_web_service(config).await?;
    
    info!("🌐 Web server listening on {}", server.bind_address());
    info!("📖 API documentation: http://{}/docs", server.bind_address());
    info!("❤️  Health check: http://{}/api/v1/health", server.bind_address());

    // Start the server
    if let Err(e) = server.serve().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Initialize logging
fn init_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
            .with_target(false)
            .with_thread_ids(verbose)
            .with_file(verbose)
            .with_line_number(verbose)
            .finish(),
    )?;

    Ok(())
}

/// Load configuration from file
async fn load_config(config_path: &PathBuf) -> Result<WebConfig> {
    if config_path.exists() {
        info!("Loading configuration from: {}", config_path.display());
        WebConfig::from_file(config_path)
    } else {
        info!("Configuration file not found, using defaults");
        Ok(WebConfig::default())
    }
}

/// Generate default configuration file
fn generate_default_config(config_path: &PathBuf) -> Result<()> {
    let config = WebConfig::default();
    config.to_file(config_path)?;
    
    println!("✅ Generated default configuration file: {}", config_path.display());
    println!("📝 Edit the file to customize settings, then run the server again.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_args_parsing() {
        let args = Args::try_parse_from(&[
            "astgrep-web-server",
            "--config", "test.toml",
            "--bind", "0.0.0.0",
            "--port", "9090",
            "--verbose"
        ]).unwrap();

        assert_eq!(args.config, PathBuf::from("test.toml"));
        assert_eq!(args.bind, Some("0.0.0.0".to_string()));
        assert_eq!(args.port, Some(9090));
        assert!(args.verbose);
    }

    #[tokio::test]
    async fn test_load_config_default() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");
        
        let config = load_config(&config_path).await.unwrap();
        assert_eq!(config.bind_address.port(), 8080); // Default port
    }

    #[test]
    fn test_generate_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.toml");
        
        generate_default_config(&config_path).unwrap();
        assert!(config_path.exists());
        
        // Verify we can load it back
        let loaded_config = WebConfig::from_file(&config_path).unwrap();
        assert_eq!(loaded_config.bind_address.port(), 8080);
    }
}
