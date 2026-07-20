//! `validate_rules` MCP tool implementation.
//!
//! Accepts raw YAML rule content, writes it to a temporary file, and runs
//! the astgrep validation engine through `validate_collect`.

use anyhow::{Context, Result};
use rmcp::schemars;
use serde::Deserialize;

use astgrep_cli::validate_enhanced::{validate_collect, ValidationResult};

/// Request parameters for the `validate_rules` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateRulesRequest {
    /// Rule YAML content to validate
    #[schemars(description = "Rule YAML content to validate")]
    pub rule_content: String,
}

/// Execute validation and return the results.
pub async fn handle_validate(req: ValidateRulesRequest) -> Result<Vec<ValidationResult>> {
    // Write rule content to a temporary YAML file.
    let tmp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
    let tmp_file_path = tmp_dir.path().join("rule.yml");
    std::fs::write(&tmp_file_path, &req.rule_content).with_context(|| {
        format!(
            "Failed to write temp rule file: {}",
            tmp_file_path.display()
        )
    })?;

    let results = validate_collect(vec![tmp_file_path], None, false).await?;

    // TempDir is dropped here, cleaning up the temp file.
    Ok(results)
}
