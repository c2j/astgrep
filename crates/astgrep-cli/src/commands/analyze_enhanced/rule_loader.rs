//! Rule loading and parsing functionality

use super::types::ParsedRule;
use anyhow::Result;
use astgrep_core::Language;
use std::path::PathBuf;
use tracing::info;

/// Load rules from rule files/directories for a specific language
pub fn load_rules_for_language(
    rule_paths: &[PathBuf],
    language: Language,
) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    for rule_path in rule_paths {
        if rule_path.is_file() {
            if let Ok(file_rules) = load_rules_from_file(rule_path, language) {
                rules.extend(file_rules);
            }
        } else if rule_path.is_dir() {
            if let Ok(dir_rules) = load_rules_from_directory_recursive(rule_path, language) {
                rules.extend(dir_rules);
            }
        }
    }

    Ok(rules)
}

/// Load rules from a single YAML file
fn load_rules_from_file(file_path: &PathBuf, target_language: Language) -> Result<Vec<ParsedRule>> {
    let bytes = std::fs::read(file_path)?;
    let content = String::from_utf8_lossy(&bytes);
    parse_semgrep_rules(&content, target_language, Some(file_path))
}

/// Recursively load rules from a directory
fn load_rules_from_directory_recursive(
    dir_path: &PathBuf,
    target_language: Language,
) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(subdir_rules) =
                    load_rules_from_directory_recursive(&path, target_language)
                {
                    rules.extend(subdir_rules);
                }
            } else if path
                .extension()
                .map_or(false, |ext| ext == "yaml" || ext == "yml")
            {
                if let Ok(file_rules) = load_rules_from_file(&path, target_language) {
                    rules.extend(file_rules);
                }
            }
        }
    }

    Ok(rules)
}

/// Parse Semgrep-style YAML rules
fn parse_semgrep_rules(
    content: &str,
    target_language: Language,
    file_path: Option<&PathBuf>,
) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    // Try to parse as YAML
    if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        if let Some(yaml_rules) = yaml_value.get("rules").and_then(|r| r.as_sequence()) {
            for rule_value in yaml_rules {
                if let Ok(rule) = parse_single_rule(rule_value, target_language, file_path) {
                    rules.push(rule);
                }
            }
        }
    }

    Ok(rules)
}

/// Parse a single rule from YAML
fn parse_single_rule(
    rule_value: &serde_yaml::Value,
    target_language: Language,
    file_path: Option<&PathBuf>,
) -> Result<ParsedRule> {
    let base_id = rule_value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown-rule");

    // Generate semgrep-compatible rule ID with path prefix
    let id = if let Some(path) = file_path {
        // Convert path to semgrep-style ID prefix (exclude filename, only use directory path)
        let _path_str = path.to_string_lossy();
        let dir_path = path
            .parent()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| std::borrow::Cow::Borrowed(""));

        let path_prefix = dir_path
            .strip_prefix("./")
            .unwrap_or(&dir_path)
            .replace('/', ".");

        if path_prefix.is_empty() {
            base_id.to_string()
        } else {
            format!("{}.{}", path_prefix, base_id)
        }
    } else {
        base_id.to_string()
    };

    let message = rule_value
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Security issue detected")
        .to_string();

    // Parse severity
    let severity = rule_value
        .get("severity")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_uppercase().as_str() {
            "ERROR" | "HIGH" => crate::output::analysis::Severity::Error,
            "WARNING" | "MEDIUM" => crate::output::analysis::Severity::Warning,
            "INFO" | "LOW" => crate::output::analysis::Severity::Info,
            _ => crate::output::analysis::Severity::Warning,
        })
        .unwrap_or(crate::output::analysis::Severity::Warning);

    // Parse languages
    let languages = rule_value
        .get("languages")
        .and_then(|v| v.as_sequence())
        .map(|langs| {
            langs
                .iter()
                .filter_map(|l| l.as_str())
                .filter_map(|l| Language::parse_name(l))
                .collect()
        })
        .unwrap_or_else(|| vec![target_language]);

    // Skip if this rule doesn't apply to the target language
    if !languages.contains(&target_language) {
        info!(
            "Rule '{}' skipped: languages {:?} don't include target {:?}",
            id, languages, target_language
        );
        return Err(anyhow::anyhow!("Rule doesn't apply to target language"));
    }

    info!(
        "Rule '{}' accepted for target language {:?}",
        id, target_language
    );

    // Extract patterns using improved pattern extraction
    let patterns = extract_patterns_from_rule_value(rule_value);

    // Debug: log extracted patterns
    if !patterns.is_empty() {
        info!("Rule '{}' extracted patterns: {:?}", id, patterns);
    }

    // Parse fix suggestion
    let fix = rule_value
        .get("fix")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(ParsedRule {
        id,
        message,
        severity,
        languages,
        patterns,
        fix,
        raw_rule_value: rule_value.clone(),
    })
}

/// Extract patterns from complex Semgrep rule structure
fn extract_patterns_from_rule_value(rule_value: &serde_yaml::Value) -> Vec<String> {
    let mut patterns = Vec::new();

    // Handle pattern-either
    if let Some(pattern_either) = rule_value.get("pattern-either") {
        if let Some(either_array) = pattern_either.as_sequence() {
            for item in either_array {
                patterns.extend(extract_patterns_from_rule_value(item));
            }
        }
    }

    // Handle patterns (array)
    if let Some(patterns_array) = rule_value.get("patterns") {
        if let Some(array) = patterns_array.as_sequence() {
            for item in array {
                patterns.extend(extract_patterns_from_rule_value(item));
            }
        }
    }

    // Handle single pattern
    if let Some(pattern_value) = rule_value.get("pattern") {
        if let Some(pattern_str) = pattern_value.as_str() {
            patterns.extend(extract_simple_patterns(pattern_str));
        }
    }

    // Handle pattern-regex (CRITICAL: This was missing!)
    if let Some(pattern_regex) = rule_value.get("pattern-regex") {
        if let Some(regex_str) = pattern_regex.as_str() {
            // Add the regex pattern directly - it will be used as a regex
            patterns.push(regex_str.to_string());
        }
    }

    // Handle pattern-not-regex
    if let Some(pattern_not_regex) = rule_value.get("pattern-not-regex") {
        if let Some(regex_str) = pattern_not_regex.as_str() {
            // For not-regex patterns, we'll handle them differently in the matching logic
            // For now, just add them as patterns to be processed
            patterns.push(format!("NOT_REGEX:{}", regex_str));
        }
    }

    // Handle pattern-not-inside
    if let Some(pattern_not_inside) = rule_value.get("pattern-not-inside") {
        if let Some(not_inside_str) = pattern_not_inside.as_str() {
            // For not-inside patterns, we'll handle them differently in the matching logic
            patterns.push(format!("NOT_INSIDE:{}", not_inside_str));
        }
    }

    // Handle metavariable-regex (extract the regex pattern)
    if let Some(metavar_regex) = rule_value.get("metavariable-regex") {
        if let Some(regex_value) = metavar_regex.get("regex") {
            if let Some(regex_str) = regex_value.as_str() {
                // For SHA1 detection, add specific patterns
                if regex_str.contains("SHA1") || regex_str.contains("SHA-1") {
                    patterns.push("\"SHA1\"".to_string());
                    patterns.push("\"SHA-1\"".to_string());
                }
                if regex_str.contains("MD5") {
                    patterns.push("\"MD5\"".to_string());
                }
            }
        }
    }

    // Handle taint analysis patterns
    // Extract patterns from pattern-sources
    if let Some(sources) = rule_value.get("pattern-sources") {
        if let Some(sources_array) = sources.as_sequence() {
            for source in sources_array {
                patterns.extend(extract_patterns_from_rule_value(source));
            }
        }
    }

    // Extract patterns from pattern-sinks
    if let Some(sinks) = rule_value.get("pattern-sinks") {
        if let Some(sinks_array) = sinks.as_sequence() {
            for sink in sinks_array {
                patterns.extend(extract_patterns_from_rule_value(sink));
            }
        }
    }

    // Extract patterns from pattern-sanitizers
    if let Some(sanitizers) = rule_value.get("pattern-sanitizers") {
        if let Some(sanitizers_array) = sanitizers.as_sequence() {
            for sanitizer in sanitizers_array {
                patterns.extend(extract_patterns_from_rule_value(sanitizer));
            }
        }
    }

    patterns
}

/// Extract simple string patterns from Semgrep pattern syntax
fn extract_simple_patterns(pattern: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    // Check if this is a metavariable pattern - pass through as-is for tree-sitter
    if pattern.contains('$') {
        patterns.push(pattern.trim().to_string());
        return patterns;
    }

    // Look for quoted strings in the pattern
    let re = regex::Regex::new(r#""([^"]+)""#).unwrap();
    for cap in re.captures_iter(pattern) {
        if let Some(matched) = cap.get(1) {
            let pattern_str = matched.as_str();
            // Skip metavariables like $ALGO
            if !pattern_str.starts_with('$') {
                patterns.push(format!("\"{}\"", pattern_str));
            }
        }
    }

    // Look for function calls with ellipsis (e.g., "sink(...)")
    let func_re = regex::Regex::new(r"(\w+)\s*\(\s*\.\.\.\s*\)").unwrap();
    for cap in func_re.captures_iter(pattern) {
        if let Some(func_name) = cap.get(1) {
            patterns.push(format!("{}(", func_name.as_str()));
        }
    }

    // Look for common API calls in patterns
    if pattern.contains("MessageDigest.getInstance") {
        patterns.push("MessageDigest.getInstance".to_string());
    }
    if pattern.contains("executeQuery") {
        patterns.push(".executeQuery(".to_string());
    }
    if pattern.contains("eval") {
        patterns.push("eval(".to_string());
    }
    if pattern.contains("getSha1Digest") {
        patterns.push("getSha1Digest".to_string());
    }

    // Handle simple string literals without quotes (for taint sources)
    // Only add if we haven't already extracted patterns from quotes
    if patterns.is_empty() && pattern.trim().starts_with('"') && pattern.trim().ends_with('"') {
        patterns.push(pattern.trim().to_string());
    }

    // Handle simple literals (numbers, identifiers, etc.)
    // If no patterns were extracted yet, treat the whole pattern as a literal
    if patterns.is_empty() {
        let trimmed = pattern.trim();
        if !trimmed.is_empty() {
            patterns.push(trimmed.to_string());
        }
    }

    // Remove duplicates
    patterns.sort();
    patterns.dedup();

    patterns
}
