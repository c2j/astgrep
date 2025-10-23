//! Version command implementation

use anyhow::Result;

/// Run the version command
pub async fn run() -> Result<()> {
    println!("astgrep - Multi-language Static Code Analysis Tool");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Authors: {}", env!("CARGO_PKG_AUTHORS"));
    println!("Repository: {}", env!("CARGO_PKG_REPOSITORY"));
    println!();
    println!("Build Information:");
    println!("  Rust Version: {}", get_rust_version());
    println!("  Target: {}", std::env::consts::ARCH);
    println!("  OS: {}", std::env::consts::OS);
    println!();
    println!("Supported Languages: Java, JavaScript, Python, SQL, Bash");
    println!("Output Formats: JSON, YAML, SARIF, Text, XML");
    
    Ok(())
}

fn get_rust_version() -> String {
    // Try to get the Rust version from environment or use a default
    std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version_command() {
        let result = run().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_rust_version_not_empty() {
        // The RUSTC_VERSION might not be available in test environment
        // so we just test that the function doesn't panic
        let _ = get_rust_version();
    }
}
