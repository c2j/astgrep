use cr_web::{create_app, WebConfig};
use cr_web::handlers::metrics::init_metrics_collector;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize metrics collector
    init_metrics_collector();

    // Load configuration
    let config = Arc::new(WebConfig::default());
    
    info!("Starting CR Web Service");
    info!("Configuration: {:?}", config);

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        error!("Current rules_directory: {:?}", config.rules_directory);
        error!("Please ensure the rules directory exists or update the configuration");
        std::process::exit(1);
    }

    // Create the application
    let app = create_app(config.clone());

    // Bind to address
    let addr = config.bind_address.clone();
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server listening on {}", addr);
    info!("API documentation available at http://{}/docs", addr);
    info!("Health check available at http://{}/health", addr);

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
