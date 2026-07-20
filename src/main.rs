use anyhow::Result;
use std::io;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "mcp") {
        // Intercepted before astgrep_cli::run() to avoid circular dependency
        // between astgrep-cli and astgrep-mcp.
        let rules_dir = extract_mcp_rules_dir(&args);
        std::env::set_var("RUST_LOG", "error");
        return astgrep_mcp::serve_stdio(rules_dir).await;
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting astgrep - Multi-language Static Code Analysis Tool");

    astgrep_cli::run().await
}

fn extract_mcp_rules_dir(args: &[String]) -> Option<PathBuf> {
    let pos = args.iter().position(|a| a == "--rules-dir")?;
    args.get(pos + 1).map(PathBuf::from)
}
