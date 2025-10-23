//! VS Code IDE integration for astgrep
//!
//! This module provides integration with VS Code through the Language Server Protocol (LSP)
//! and diagnostic reporting capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a diagnostic message for VS Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeDiagnostic {
    /// File path
    pub file: String,
    /// Line number (0-based)
    pub line: u32,
    /// Column number (0-based)
    pub column: u32,
    /// Diagnostic message
    pub message: String,
    /// Severity level: "error", "warning", "information", "hint"
    pub severity: String,
    /// Rule ID that triggered this diagnostic
    pub rule_id: String,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

impl VsCodeDiagnostic {
    /// Create a new diagnostic
    pub fn new(
        file: String,
        line: u32,
        column: u32,
        message: String,
        severity: String,
        rule_id: String,
    ) -> Self {
        Self {
            file,
            line,
            column,
            message,
            severity,
            rule_id,
            end_line: line,
            end_column: column + 1,
        }
    }

    /// Set end position
    pub fn with_end_position(mut self, end_line: u32, end_column: u32) -> Self {
        self.end_line = end_line;
        self.end_column = end_column;
        self
    }

    /// Convert to VS Code diagnostic format
    pub fn to_vscode_format(&self) -> serde_json::Value {
        serde_json::json!({
            "range": {
                "start": {
                    "line": self.line,
                    "character": self.column
                },
                "end": {
                    "line": self.end_line,
                    "character": self.end_column
                }
            },
            "message": self.message,
            "severity": self.severity_to_code(),
            "source": "astgrep",
            "code": self.rule_id
        })
    }

    /// Convert severity string to VS Code severity code
    fn severity_to_code(&self) -> u32 {
        match self.severity.as_str() {
            "error" => 1,
            "warning" => 2,
            "information" => 3,
            "hint" => 4,
            _ => 3,
        }
    }
}

/// VS Code extension configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsCodeConfig {
    /// Enable astgrep in VS Code
    pub enabled: bool,
    /// Auto-run analysis on file save
    pub auto_run_on_save: bool,
    /// Show diagnostics inline
    pub show_inline_diagnostics: bool,
    /// Highlight severity level: "error", "warning", "all"
    pub highlight_severity: String,
    /// Rules to enable
    pub enabled_rules: Vec<String>,
    /// Rules to disable
    pub disabled_rules: Vec<String>,
    /// File patterns to analyze
    pub file_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

impl Default for VsCodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_run_on_save: true,
            show_inline_diagnostics: true,
            highlight_severity: "warning".to_string(),
            enabled_rules: Vec::new(),
            disabled_rules: Vec::new(),
            file_patterns: vec![
                "**/*.java".to_string(),
                "**/*.js".to_string(),
                "**/*.py".to_string(),
                "**/*.rb".to_string(),
                "**/*.kt".to_string(),
                "**/*.swift".to_string(),
            ],
            exclude_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/target/**".to_string(),
            ],
        }
    }
}

/// VS Code extension manager
pub struct VsCodeExtension {
    /// Configuration
    config: VsCodeConfig,
    /// Cached diagnostics by file
    diagnostics: HashMap<String, Vec<VsCodeDiagnostic>>,
}

impl VsCodeExtension {
    /// Create a new VS Code extension
    pub fn new() -> Self {
        Self {
            config: VsCodeConfig::default(),
            diagnostics: HashMap::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: VsCodeConfig) -> Self {
        Self {
            config,
            diagnostics: HashMap::new(),
        }
    }

    /// Get configuration
    pub fn get_config(&self) -> &VsCodeConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: VsCodeConfig) {
        self.config = config;
    }

    /// Add a diagnostic
    pub fn add_diagnostic(&mut self, diagnostic: VsCodeDiagnostic) {
        self.diagnostics
            .entry(diagnostic.file.clone())
            .or_insert_with(Vec::new)
            .push(diagnostic);
    }

    /// Get diagnostics for a file
    pub fn get_diagnostics(&self, file: &str) -> Option<&Vec<VsCodeDiagnostic>> {
        self.diagnostics.get(file)
    }

    /// Clear diagnostics for a file
    pub fn clear_diagnostics(&mut self, file: &str) {
        self.diagnostics.remove(file);
    }

    /// Clear all diagnostics
    pub fn clear_all_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    /// Get all diagnostics
    pub fn get_all_diagnostics(&self) -> &HashMap<String, Vec<VsCodeDiagnostic>> {
        &self.diagnostics
    }

    /// Check if file should be analyzed
    pub fn should_analyze_file(&self, file: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check exclude patterns
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(file, pattern) {
                return false;
            }
        }

        // Check file patterns
        if self.config.file_patterns.is_empty() {
            return true;
        }

        for pattern in &self.config.file_patterns {
            if self.matches_pattern(file, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a rule is enabled
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if self.config.disabled_rules.contains(&rule_id.to_string()) {
            return false;
        }

        if !self.config.enabled_rules.is_empty() {
            return self.config.enabled_rules.contains(&rule_id.to_string());
        }

        true
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, file: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Handle patterns like "**/*.java"
            if pattern.starts_with("**/") {
                let suffix = &pattern[3..]; // Remove "**/"
                return file.ends_with(suffix);
            }

            // Handle patterns like "*.java"
            if pattern.starts_with("*.") {
                let suffix = &pattern[1..]; // Remove "*"
                return file.ends_with(suffix);
            }

            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return file.starts_with(prefix) && file.ends_with(suffix);
            }
        }

        file == pattern
    }

    /// Get diagnostic count
    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.values().map(|v| v.len()).sum()
    }

    /// Get diagnostic count for a file
    pub fn diagnostic_count_for_file(&self, file: &str) -> usize {
        self.diagnostics.get(file).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for VsCodeExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vscode_diagnostic_new() {
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            10,
            5,
            "SQL Injection detected".to_string(),
            "error".to_string(),
            "sql_injection".to_string(),
        );

        assert_eq!(diag.file, "test.java");
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 5);
        assert_eq!(diag.message, "SQL Injection detected");
        assert_eq!(diag.severity, "error");
        assert_eq!(diag.rule_id, "sql_injection");
    }

    #[test]
    fn test_vscode_diagnostic_with_end_position() {
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            10,
            5,
            "Error".to_string(),
            "error".to_string(),
            "rule1".to_string(),
        )
        .with_end_position(10, 20);

        assert_eq!(diag.end_line, 10);
        assert_eq!(diag.end_column, 20);
    }

    #[test]
    fn test_vscode_config_default() {
        let config = VsCodeConfig::default();
        assert!(config.enabled);
        assert!(config.auto_run_on_save);
        assert!(config.show_inline_diagnostics);
    }

    #[test]
    fn test_vscode_extension_new() {
        let ext = VsCodeExtension::new();
        assert!(ext.get_config().enabled);
        assert_eq!(ext.diagnostic_count(), 0);
    }

    #[test]
    fn test_vscode_extension_add_diagnostic() {
        let mut ext = VsCodeExtension::new();
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            10,
            5,
            "Error".to_string(),
            "error".to_string(),
            "rule1".to_string(),
        );

        ext.add_diagnostic(diag);
        assert_eq!(ext.diagnostic_count(), 1);
    }

    #[test]
    fn test_vscode_extension_should_analyze_file() {
        let ext = VsCodeExtension::new();
        assert!(ext.should_analyze_file("test.java"));
        assert!(ext.should_analyze_file("test.js"));
        assert!(!ext.should_analyze_file("node_modules/test.js"));
    }

    #[test]
    fn test_vscode_extension_is_rule_enabled() {
        let ext = VsCodeExtension::new();
        assert!(ext.is_rule_enabled("any_rule"));
    }

    #[test]
    fn test_vscode_extension_clear_diagnostics() {
        let mut ext = VsCodeExtension::new();
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            10,
            5,
            "Error".to_string(),
            "error".to_string(),
            "rule1".to_string(),
        );

        ext.add_diagnostic(diag);
        assert_eq!(ext.diagnostic_count(), 1);

        ext.clear_diagnostics("test.java");
        assert_eq!(ext.diagnostic_count(), 0);
    }
}

