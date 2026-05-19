use anyhow::Result;
use std::io;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting astgrep - Multi-language Static Code Analysis Tool");

    astgrep_cli::run().await
}
