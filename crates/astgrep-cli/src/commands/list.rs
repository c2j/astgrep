//! List command for showing available rules

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};
use crate::OutputFormatCli;
use astgrep_rules::Rule;
use astgrep_core::{Severity, Confidence, Language};

/// List available rules and their information
pub async fn run(
    rules_dir: Option<PathBuf>,
    language_filter: Option<String>,
    category_filter: Option<String>,
    detailed: bool,
    format: OutputFormatCli,
) -> Result<()> {
    let rules_path = rules_dir.unwrap_or_else(|| PathBuf::from("rules"));
    
    info!("Scanning rules from: {}", rules_path.display());
    
    // Load all rules from the directory
    let rules = load_rules_from_directory(&rules_path).await?;
    
    if rules.is_empty() {
        warn!("No rules found in {}", rules_path.display());
        return Ok(());
    }
    
    // Apply filters
    let filtered_rules = apply_filters(&rules, &language_filter, &category_filter);
    
    // Generate output
    let output = match format {
        OutputFormatCli::Table => generate_table_output(&filtered_rules, detailed),
        OutputFormatCli::Json => generate_json_output(&filtered_rules, detailed)?,
        OutputFormatCli::Markdown => generate_markdown_output(&filtered_rules, detailed),
        OutputFormatCli::Text => generate_text_output(&filtered_rules, detailed),
        _ => generate_table_output(&filtered_rules, detailed),
    };
    
    println!("{}", output);
    Ok(())
}

async fn load_rules_from_directory(dir: &PathBuf) -> Result<Vec<Rule>> {
    let mut rules = Vec::new();

    if !dir.exists() {
        return Ok(rules);
    }

    if dir.is_file() {
        // Single rule file - create a demo rule
        rules.push(Rule::new(
            "demo-rule-001".to_string(),
            "Demo Rule".to_string(),
            "A demonstration rule for testing".to_string(),
            Severity::Warning,
            Confidence::High,
            vec![Language::JavaScript],
        ));
    } else {
        // Directory of rule files
        load_rules_recursively(dir, &mut rules)?;
    }

    Ok(rules)
}

fn load_rules_recursively(dir: &PathBuf, rules: &mut Vec<Rule>) -> Result<()> {
    use std::fs;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            load_rules_recursively(&path, rules)?;
        } else if is_rule_file(&path) {
            // Create a demo rule for each YAML file found
            rules.push(Rule::new(
                format!("rule-{}", rules.len() + 1),
                format!("Rule from {}", path.file_name().unwrap_or_default().to_string_lossy()),
                format!("Rule loaded from {}", path.display()),
                Severity::Warning,
                Confidence::Medium,
                vec![Language::JavaScript],
            ));
        }
    }

    Ok(())
}

fn is_rule_file(path: &PathBuf) -> bool {
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        ext == "yaml" || ext == "yml"
    } else {
        false
    }
}

fn apply_filters<'a>(
    rules: &'a [Rule],
    language_filter: &Option<String>,
    category_filter: &Option<String>,
) -> Vec<&'a Rule> {
    rules.iter()
        .filter(|rule| {
            // Simplified filtering - just check if rule name contains the filter
            if let Some(ref lang) = language_filter {
                if !rule.name.to_lowercase().contains(&lang.to_lowercase()) {
                    return false;
                }
            }

            if let Some(ref category) = category_filter {
                if !rule.description.to_lowercase().contains(&category.to_lowercase()) {
                    return false;
                }
            }

            true
        })
        .collect()
}

fn generate_table_output(rules: &[&Rule], detailed: bool) -> String {
    let mut output = String::new();

    output.push_str(&format!("Found {} rule(s):\n\n", rules.len()));

    if detailed {
        output.push_str("┌─────────────────────┬─────────────────────┬─────────────────────────────────────────────────────────────┐\n");
        output.push_str("│ ID                  │ Name                │ Description                                                     │\n");
        output.push_str("├─────────────────────┼─────────────────────┼─────────────────────────────────────────────────────────────┤\n");

        for rule in rules {
            output.push_str(&format!(
                "│ {:<19} │ {:<19} │ {:<63} │\n",
                truncate_text(&rule.id, 19),
                truncate_text(&rule.name, 19),
                truncate_text(&rule.description, 63)
            ));
        }

        output.push_str("└─────────────────────┴─────────────────────┴─────────────────────────────────────────────────────────────┘\n");
    } else {
        for (i, rule) in rules.iter().enumerate() {
            output.push_str(&format!("{}. {} ({})\n", i + 1, rule.name, rule.id));
            output.push_str(&format!("   Description: {}\n\n", rule.description));
        }
    }

    output
}

fn generate_json_output(rules: &[&Rule], detailed: bool) -> Result<String> {
    use serde_json::json;

    let rules_json: Vec<serde_json::Value> = rules.iter().map(|rule| {
        if detailed {
            json!({
                "id": rule.id,
                "name": rule.name,
                "description": rule.description,
            })
        } else {
            json!({
                "id": rule.id,
                "name": rule.name,
                "description": rule.description,
            })
        }
    }).collect();

    let output = json!({
        "total_rules": rules.len(),
        "rules": rules_json
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

fn generate_markdown_output(rules: &[&Rule], detailed: bool) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("# Available Rules ({} total)\n\n", rules.len()));
    
    for rule in rules {
        output.push_str(&format!("## {}\n\n", rule.name));
        output.push_str(&format!("- **ID:** `{}`\n", rule.id));
        output.push_str(&format!("- **Description:** {}\n", rule.description));
        
        if detailed {
            output.push_str("- **Type:** Static Analysis Rule\n");
            output.push_str("- **Status:** Active\n");
        }
        
        output.push_str("\n");
    }
    
    output
}

fn generate_text_output(rules: &[&Rule], detailed: bool) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Found {} rule(s):\n\n", rules.len()));
    
    for (i, rule) in rules.iter().enumerate() {
        output.push_str(&format!("{}. {} ({})\n", i + 1, rule.name, rule.id));
        output.push_str(&format!("   Description: {}\n", rule.description));
        
        if detailed {
            output.push_str(&format!("   Type: Rule\n"));
            output.push_str(&format!("   Status: Active\n"));
        }
        
        output.push_str("\n");
    }
    
    output
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::{Severity, Confidence, Language};
    
    fn create_test_rule() -> Rule {
        Rule::new(
            "test-rule-001".to_string(),
            "Test Rule".to_string(),
            "A test rule for unit testing".to_string(),
            Severity::Warning,
            Confidence::High,
            vec![Language::Java, Language::Python],
        )
    }
    
    #[test]
    fn test_apply_filters_no_filter() {
        let rules = vec![create_test_rule()];
        let filtered = apply_filters(&rules, &None, &None);
        assert_eq!(filtered.len(), 1);
    }
    
    #[test]
    fn test_apply_filters_language() {
        let rules = vec![create_test_rule()];
        let filtered = apply_filters(&rules, &Some("java".to_string()), &None);
        assert_eq!(filtered.len(), 1);
        
        let filtered = apply_filters(&rules, &Some("javascript".to_string()), &None);
        assert_eq!(filtered.len(), 0);
    }
    
    #[test]
    fn test_apply_filters_category() {
        let rules = vec![create_test_rule()];
        let filtered = apply_filters(&rules, &None, &Some("test".to_string()));
        assert_eq!(filtered.len(), 1);
        
        let filtered = apply_filters(&rules, &None, &Some("security".to_string()));
        assert_eq!(filtered.len(), 0);
    }
    
    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("this is a very long text", 10), "this is...");
    }
}
