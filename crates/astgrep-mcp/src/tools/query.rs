//! `list_rules` and `list_languages` MCP tool implementations.

use std::path::Path;

use serde::Serialize;

use astgrep_core::Language;

/// Metadata about a single rule, returned by `list_rules`.
#[derive(Debug, Clone, Serialize)]
pub struct RuleInfo {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub languages: Vec<String>,
    pub description: String,
}

/// Language descriptor returned by `list_languages`.
#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
}

/// Scan a directory for YAML rule files and extract rule metadata.
pub fn list_rules_from_dir(dir: &Path) -> Vec<RuleInfo> {
    let mut rules = Vec::new();

    if !dir.exists() {
        return rules;
    }

    scan_rules_dir(dir, &mut rules);
    rules
}

fn scan_rules_dir(dir: &Path, rules: &mut Vec<RuleInfo>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rules_dir(&path, rules);
        } else if is_yaml_file(&path) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                extract_rules_from_yaml(&content, rules);
            }
        }
    }
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "yaml" || ext == "yml")
}

fn extract_rules_from_yaml(content: &str, rules: &mut Vec<RuleInfo>) {
    // Parse the YAML document as a generic value
    let value: serde_yaml::Value = match serde_yaml::from_str(content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let rules_array = match value.get("rules") {
        Some(serde_yaml::Value::Sequence(arr)) => arr,
        _ => return,
    };

    for rule_val in rules_array {
        let id = rule_val
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = rule_val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity = rule_val
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("INFO")
            .to_string();
        let description = rule_val
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let languages: Vec<String> = rule_val
            .get("languages")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        rules.push(RuleInfo {
            id,
            name,
            severity,
            languages,
            description,
        });
    }
}

/// Build a static list of all supported languages and their extensions.
pub fn list_supported_languages() -> Vec<LanguageInfo> {
    let all_langs = [
        Language::Java,
        Language::JavaScript,
        Language::Python,
        Language::Sql,
        Language::Bash,
        Language::Xml,
        Language::Text,
    ];

    all_langs
        .iter()
        .map(|lang| {
            let extensions: Vec<String> = lang.extensions().iter().map(|e| e.to_string()).collect();
            LanguageInfo {
                name: lang.as_str().to_string(),
                extensions,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_supported_languages() {
        let langs = list_supported_languages();
        assert!(!langs.is_empty(), "Should return at least one language");
        let java = langs.iter().find(|l| l.name == "java").unwrap();
        assert!(java.extensions.contains(&".java".to_string()));
    }

    #[test]
    fn test_extract_rules_from_yaml() {
        let yaml = r#"
rules:
  - id: my-rule
    name: My Rule
    severity: ERROR
    languages: [java, python]
    description: A test rule
"#;
        let mut rules = Vec::new();
        extract_rules_from_yaml(yaml, &mut rules);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "my-rule");
        assert_eq!(rules[0].severity, "ERROR");
        assert_eq!(rules[0].languages, vec!["java", "python"]);
    }

    #[test]
    fn test_extract_rules_from_yaml_no_rules_key() {
        let yaml = "some_key: value";
        let mut rules = Vec::new();
        extract_rules_from_yaml(yaml, &mut rules);
        assert!(rules.is_empty());
    }

    #[test]
    fn test_is_yaml_file() {
        assert!(is_yaml_file(Path::new("rule.yaml")));
        assert!(is_yaml_file(Path::new("rule.yml")));
        assert!(!is_yaml_file(Path::new("rule.json")));
        assert!(!is_yaml_file(Path::new("rule")));
    }

    #[test]
    fn test_list_rules_from_dir_nonexistent() {
        let rules = list_rules_from_dir(Path::new("/nonexistent/path/for/testing"));
        assert!(rules.is_empty());
    }
}
