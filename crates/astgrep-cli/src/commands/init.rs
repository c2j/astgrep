//! Init command for creating configuration files

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};
use astgrep_core::constants::{defaults, paths};

/// Initialize a new configuration file
pub async fn run(output: PathBuf, template: String, force: bool) -> Result<()> {
    if output.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            output.display()
        ));
    }

    info!("Creating configuration file: {}", output.display());

    let config_content = match template.as_str() {
        "default" => generate_default_config(),
        "minimal" => generate_minimal_config(),
        "comprehensive" => generate_comprehensive_config(),
        "security" => generate_security_focused_config(),
        "performance" => generate_performance_focused_config(),
        _ => {
            warn!("Unknown template: {}, using default", template);
            generate_default_config()
        }
    };

    std::fs::write(&output, config_content)?;
    info!("Configuration file created successfully");

    println!("✅ Configuration file created: {}", output.display());
    println!("📝 Edit the file to customize your analysis settings");
    println!("🚀 Run analysis with: astgrep analyze --config {}", output.display());

    Ok(())
}

fn generate_default_config() -> String {
    format!(
        "# astgrep Configuration File\n\
        # This file configures the static analysis behavior\n\
        \n\
        [general]\n\
        verbose = false\n\
        threads = 0\n\
        profile = false\n\
        \n\
        [analysis]\n\
        languages = [\"java\", \"javascript\", \"python\", \"sql\", \"bash\"]\n\
        output_format = \"json\"\n\
        include_metrics = true\n\
        enable_dataflow = false\n\
        max_findings = 0\n\
        fail_on_findings = false\n\
        \n\
        [filtering]\n\
        min_severity = \"info\"\n\
        min_confidence = \"low\"\n\
        exclude_patterns = [\n\
            \"*.test.java\",\n\
            \"*.spec.js\",\n\
            \"**/test/**\",\n\
            \"**/tests/**\",\n\
            \"**/node_modules/**\",\n\
            \"**/target/**\",\n\
            \"**/build/**\",\n\
            \"**/.git/**\"\n\
        ]\n\
        \n\
        [rules]\n\
        rules_directory = \"rules\"\n\
        rule_files = []\n\
        enabled_categories = [\"security\", \"best-practice\", \"performance\"]\n\
        disabled_categories = [\"experimental\"]\n\
        \n\
        [output]\n\
        output_directory = \"reports\"\n\
        generate_html = false\n\
        generate_sarif = true\n\
        generate_baseline = false\n\
        \n\
        [integrations]\n\
        [integrations.github]\n\
        enabled = false\n\
        \n\
        [integrations.jira]\n\
        enabled = false\n\
        \n\
        [integrations.slack]\n\
        enabled = false\n"
    )
}

fn generate_minimal_config() -> String {
    format!(
        "# Minimal astgrep Configuration\n\
        \n\
        [general]\n\
        verbose = false\n\
        \n\
        [analysis]\n\
        languages = [\"java\", \"javascript\", \"python\"]\n\
        output_format = \"text\"\n\
        \n\
        [filtering]\n\
        min_severity = \"warning\"\n\
        exclude_patterns = [\n\
            \"**/test/**\",\n\
            \"**/node_modules/**\"\n\
        ]\n\
        \n\
        [rules]\n\
        rules_directory = \"rules\"\n"
    )
}

fn generate_comprehensive_config() -> String {
    format!(
        "# Comprehensive astgrep Configuration\n\
        \n\
        [general]\n\
        verbose = true\n\
        threads = 4\n\
        profile = true\n\
        \n\
        [analysis]\n\
        languages = [\"java\", \"javascript\", \"python\", \"sql\", \"bash\"]\n\
        output_format = \"json\"\n\
        include_metrics = true\n\
        enable_dataflow = true\n\
        max_findings = 1000\n\
        fail_on_findings = true\n\
        \n\
        [filtering]\n\
        min_severity = \"info\"\n\
        min_confidence = \"low\"\n\
        \n\
        [rules]\n\
        rules_directory = \"rules\"\n\
        enabled_categories = [\"security\", \"best-practice\", \"performance\"]\n\
        \n\
        [output]\n\
        output_directory = \"reports\"\n\
        generate_html = true\n\
        generate_sarif = true\n\
        generate_baseline = true\n"
    )
}

fn generate_security_focused_config() -> String {
    format!(
        "# Security-Focused astgrep Configuration\n\
        \n\
        [general]\n\
        verbose = true\n\
        threads = 0\n\
        profile = false\n\
        \n\
        [analysis]\n\
        languages = [\"java\", \"javascript\", \"python\", \"sql\", \"bash\"]\n\
        output_format = \"sarif\"\n\
        include_metrics = true\n\
        enable_dataflow = true\n\
        max_findings = 0\n\
        fail_on_findings = true\n\
        \n\
        [filtering]\n\
        min_severity = \"warning\"\n\
        min_confidence = \"medium\"\n\
        \n\
        [rules]\n\
        rules_directory = \"rules\"\n\
        enabled_categories = [\"security\", \"vulnerability\", \"injection\"]\n\
        disabled_categories = [\"style\", \"formatting\", \"experimental\"]\n\
        \n\
        [output]\n\
        output_directory = \"security-reports\"\n\
        generate_html = true\n\
        generate_sarif = true\n\
        generate_baseline = true\n"
    )
}

fn generate_performance_focused_config() -> String {
    format!(
        "# Performance-Focused astgrep Configuration\n\
        \n\
        [general]\n\
        verbose = false\n\
        threads = 0\n\
        profile = true\n\
        \n\
        [analysis]\n\
        languages = [\"java\", \"javascript\", \"python\"]\n\
        output_format = \"json\"\n\
        include_metrics = true\n\
        enable_dataflow = false\n\
        max_findings = 100\n\
        fail_on_findings = false\n\
        \n\
        [filtering]\n\
        min_severity = \"warning\"\n\
        min_confidence = \"high\"\n\
        \n\
        [rules]\n\
        rules_directory = \"rules\"\n\
        enabled_categories = [\"security\", \"best-practice\", \"performance\"]\n\
        disabled_categories = [\"style\", \"formatting\", \"experimental\"]\n\
        \n\
        [output]\n\
        output_directory = \"reports\"\n\
        generate_html = false\n\
        generate_sarif = true\n\
        generate_baseline = false\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_init_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.toml");

        let result = run(config_path.clone(), "default".to_string(), false).await;
        assert!(result.is_ok());
        assert!(config_path.exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[general]"));
        assert!(content.contains("[analysis]"));
        assert!(content.contains("[filtering]"));
    }

    #[tokio::test]
    async fn test_init_existing_file_without_force() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("existing-config.toml");

        // Create existing file
        std::fs::write(&config_path, "existing content").unwrap();

        let result = run(config_path, "default".to_string(), false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_init_existing_file_with_force() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("existing-config.toml");

        // Create existing file
        std::fs::write(&config_path, "existing content").unwrap();

        let result = run(config_path.clone(), "default".to_string(), true).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[general]"));
        assert!(!content.contains("existing content"));
    }

    #[test]
    fn test_generate_minimal_config() {
        let config = generate_minimal_config();
        assert!(config.contains("[general]"));
        assert!(config.contains("verbose = false"));
        assert!(config.contains("languages = [\"java\", \"javascript\", \"python\"]"));
    }

    #[test]
    fn test_generate_security_config() {
        let config = generate_security_focused_config();
        assert!(config.contains("security"));
        assert!(config.contains("fail_on_findings = true"));
        assert!(config.contains("enable_dataflow = true"));
    }
}
