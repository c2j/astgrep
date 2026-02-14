//! Type definitions and helper functions for enhanced analysis

use crate::output::analysis::{Confidence, Severity};
use astgrep_core::Language;
use serde_yaml;
use std::path::PathBuf;

/// Parsed rule structure
#[derive(Debug, Clone)]
pub struct ParsedRule {
    pub id: String,
    pub message: String,
    pub severity: Severity,
    pub languages: Vec<Language>,
    pub patterns: Vec<String>,
    pub fix: Option<String>,
    // Preserve the original YAML value to maintain semantics like pattern-either
    pub raw_rule_value: serde_yaml::Value,
}

/// Embedded SQL snippet extracted from non-SQL sources
#[derive(Clone, Debug)]
pub struct EmbeddedSqlSnippet {
    pub sql: String,
    pub start_line: usize,
    pub context: Option<String>,
}

#[derive(Clone)]
pub struct BasicPattern {
    pub rule_id: String,
    pub pattern: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub fix: Option<String>,
}

/// Determine language from file extension
pub fn determine_language(file_path: &PathBuf) -> anyhow::Result<Language> {
    if let Some(extension) = file_path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "java" => Ok(Language::Java),
            "js" | "jsx" | "ts" | "tsx" => Ok(Language::JavaScript),
            "py" => Ok(Language::Python),
            "sql" => Ok(Language::Sql),
            "sh" | "bash" => Ok(Language::Bash),
            "xml" | "xsd" | "xsl" | "xslt" | "svg" | "pom" => Ok(Language::Xml),
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", ext_str)),
        }
    } else {
        Err(anyhow::anyhow!(
            "File has no extension: {}",
            file_path.display()
        ))
    }
}

/// Simple glob matching implementation
pub fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple glob matching implementation
    // In a real implementation, you'd use a proper glob library
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            // More complex patterns would need proper glob implementation
            text.contains(&pattern.replace('*', ""))
        }
    } else {
        text.contains(pattern)
    }
}
