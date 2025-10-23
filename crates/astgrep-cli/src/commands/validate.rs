//! Validate command implementation

use anyhow::Result;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Run the validate command
pub async fn run(rule_files: Vec<PathBuf>) -> Result<()> {
    if rule_files.is_empty() {
        return Err(anyhow::anyhow!("No rule files specified for validation"));
    }

    info!("Validating {} rule file(s)", rule_files.len());

    let mut validation_errors = 0;
    let mut total_rules = 0;

    for rule_file in &rule_files {
        info!("Validating rule file: {:?}", rule_file);

        if !rule_file.exists() {
            error!("Rule file does not exist: {:?}", rule_file);
            validation_errors += 1;
            continue;
        }

        match validate_rule_file(rule_file).await {
            Ok(rule_count) => {
                info!("✓ Rule file {:?} is valid ({} rules)", rule_file, rule_count);
                total_rules += rule_count;
            }
            Err(e) => {
                error!("✗ Rule file {:?} validation failed: {}", rule_file, e);
                validation_errors += 1;
            }
        }
    }

    if validation_errors > 0 {
        error!(
            "Validation completed with {} error(s) out of {} file(s)",
            validation_errors,
            rule_files.len()
        );
        return Err(anyhow::anyhow!(
            "Validation failed with {} error(s)",
            validation_errors
        ));
    } else {
        info!(
            "✓ All {} rule file(s) are valid (total {} rules)",
            rule_files.len(),
            total_rules
        );
    }

    Ok(())
}

async fn validate_rule_file(rule_file: &PathBuf) -> Result<usize> {
    let content = std::fs::read_to_string(rule_file)?;

    // Basic YAML syntax validation
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("YAML syntax error: {}", e))?;

    // Check if it's a rules file with the expected structure
    let rules = yaml_value
        .get("rules")
        .ok_or_else(|| anyhow::anyhow!("Missing 'rules' key in rule file"))?;

    let rules_array = rules
        .as_sequence()
        .ok_or_else(|| anyhow::anyhow!("'rules' must be an array"))?;

    if rules_array.is_empty() {
        warn!("Rule file contains no rules");
        return Ok(0);
    }

    // Validate each rule
    for (index, rule) in rules_array.iter().enumerate() {
        validate_rule(rule, index)?;
    }

    Ok(rules_array.len())
}

fn validate_rule(rule: &serde_yaml::Value, index: usize) -> Result<()> {
    let rule_obj = rule
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("Rule {} is not an object", index))?;

    // Check required fields (note: name and description are optional and auto-generated if missing)
    let required_fields = ["id", "severity", "languages"];
    for field in &required_fields {
        if !rule_obj.contains_key(&serde_yaml::Value::String(field.to_string())) {
            return Err(anyhow::anyhow!(
                "Rule {} missing required field: {}",
                index,
                field
            ));
        }
    }

    // Check for message field (required for semgrep compatibility)
    if !rule_obj.contains_key(&serde_yaml::Value::String("message".to_string())) {
        return Err(anyhow::anyhow!(
            "Rule {} missing required field: message",
            index
        ));
    }

    // Validate severity
    if let Some(severity) = rule_obj.get("severity") {
        if let Some(severity_str) = severity.as_str() {
            match severity_str.to_uppercase().as_str() {
                "INFO" | "WARNING" | "ERROR" | "CRITICAL" => {}
                _ => {
                    return Err(anyhow::anyhow!(
                        "Rule {} has invalid severity: {}",
                        index,
                        severity_str
                    ));
                }
            }
        }
    }

    // Validate languages
    if let Some(languages) = rule_obj.get("languages") {
        if let Some(lang_array) = languages.as_sequence() {
            for lang in lang_array {
                if let Some(lang_str) = lang.as_str() {
                    match lang_str.to_lowercase().as_str() {
                        "java" | "javascript" | "python" | "sql" | "bash" => {}
                        _ => {
                            warn!(
                                "Rule {} references unsupported language: {}",
                                index, lang_str
                            );
                        }
                    }
                }
            }
        }
    }

    // Check for patterns
    if !rule_obj.contains_key(&serde_yaml::Value::String("patterns".to_string())) {
        warn!("Rule {} has no patterns defined", index);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_validate_empty_rule_files() {
        let result = run(vec![]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No rule files specified"));
    }

    #[tokio::test]
    async fn test_validate_nonexistent_file() {
        let result = run(vec![PathBuf::from("/nonexistent/rules.yml")]).await;
        // Should return an error for nonexistent files
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_valid_rule_file() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("valid_rules.yml");
        
        let valid_rules = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "test"
"#;
        
        std::fs::write(&rule_file, valid_rules).unwrap();
        
        let result = validate_rule_file(&rule_file).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_validate_invalid_yaml() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("invalid_yaml.yml");
        
        let invalid_yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: "Unclosed quote
"#;
        
        std::fs::write(&rule_file, invalid_yaml).unwrap();
        
        let result = validate_rule_file(&rule_file).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("YAML syntax error"));
    }

    #[tokio::test]
    async fn test_validate_missing_required_field() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("missing_field.yml");
        
        let missing_field_rules = r#"
rules:
  - id: test-rule
    name: Test Rule
    # Missing description, severity, and languages
"#;
        
        std::fs::write(&rule_file, missing_field_rules).unwrap();
        
        let result = validate_rule_file(&rule_file).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required field"));
    }

    #[tokio::test]
    async fn test_validate_invalid_severity() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("invalid_severity.yml");
        
        let invalid_severity_rules = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    severity: INVALID
    languages: [java]
"#;
        
        std::fs::write(&rule_file, invalid_severity_rules).unwrap();
        
        let result = validate_rule_file(&rule_file).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid severity"));
    }

    #[tokio::test]
    async fn test_validate_empty_rules() {
        let temp_dir = tempdir().unwrap();
        let rule_file = temp_dir.path().join("empty_rules.yml");
        
        let empty_rules = r#"
rules: []
"#;
        
        std::fs::write(&rule_file, empty_rules).unwrap();
        
        let result = validate_rule_file(&rule_file).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
