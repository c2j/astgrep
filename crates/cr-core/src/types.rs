//! Core types for CR-SemService

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Java,
    JavaScript,
    Python,
    Sql,
    Bash,
    Php,
    CSharp,
    C,
}

impl Language {
    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Java => &[".java"],
            Language::JavaScript => &[".js", ".jsx", ".ts", ".tsx"],
            Language::Python => &[".py", ".pyw"],
            Language::Sql => &[".sql", ".ddl", ".dml"],
            Language::Bash => &[".sh", ".bash", ".zsh"],
            Language::Php => &[".php", ".phtml", ".php3", ".php4", ".php5"],
            Language::CSharp => &[".cs", ".csx"],
            Language::C => &[".c", ".h"],
        }
    }

    /// Get language name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Java => "java",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Sql => "sql",
            Language::Bash => "bash",
            Language::Php => "php",
            Language::CSharp => "csharp",
            Language::C => "c",
        }
    }

    /// Parse language from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "java" => Some(Language::Java),
            "javascript" | "js" | "typescript" | "ts" => Some(Language::JavaScript),
            "python" | "py" => Some(Language::Python),
            "sql" => Some(Language::Sql),
            "bash" | "shell" | "sh" => Some(Language::Bash),
            "php" => Some(Language::Php),
            "csharp" | "c#" | "cs" => Some(Language::CSharp),
            "c" => Some(Language::C),
            _ => None,
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = if ext.starts_with('.') { ext } else { &format!(".{}", ext) };

        for &lang in crate::constants::languages::ALL_LANGUAGES {
            if lang.extensions().contains(&ext) {
                return Some(lang);
            }
        }
        None
    }
}

/// Severity levels for analysis findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING", 
            Severity::Error => "ERROR",
            Severity::Critical => "CRITICAL",
        }
    }
}

/// Confidence levels for analysis findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Low => "LOW",
            Confidence::Medium => "MEDIUM",
            Confidence::High => "HIGH",
        }
    }
}

/// Source location information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Location {
    pub fn new(
        file: PathBuf,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            file,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Create a single-point location
    pub fn point(file: PathBuf, line: usize, column: usize) -> Self {
        Self::new(file, line, column, line, column)
    }
}

/// Analysis finding/match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub location: Location,
    pub metadata: HashMap<String, String>,
    pub fix_suggestion: Option<String>,
}

impl Finding {
    pub fn new(
        rule_id: String,
        message: String,
        severity: Severity,
        confidence: Confidence,
        location: Location,
    ) -> Self {
        Self {
            rule_id,
            message,
            severity,
            confidence,
            location,
            metadata: HashMap::new(),
            fix_suggestion: None,
        }
    }

    /// Add metadata to the finding
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add fix suggestion to the finding
    pub fn with_fix(mut self, fix: String) -> Self {
        self.fix_suggestion = Some(fix);
        self
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub target_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub languages: Vec<Language>,
    pub rule_files: Vec<PathBuf>,
    pub output_format: OutputFormat,
    pub parallel: bool,
    pub max_threads: Option<usize>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        use crate::constants::{defaults, paths, languages};

        Self {
            target_paths: vec![PathBuf::from(".")],
            exclude_patterns: paths::DEFAULT_EXCLUDE_PATTERNS.iter().map(|s| s.to_string()).collect(),
            languages: languages::DEFAULT_LANGUAGES.to_vec(),
            rule_files: vec![],
            output_format: OutputFormat::Json,
            parallel: true,
            max_threads: Some(crate::constants::performance::DEFAULT_THREAD_COUNT),
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Yaml,
    Sarif,
    Text,
    Xml,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Yaml => "yaml",
            OutputFormat::Sarif => "sarif",
            OutputFormat::Text => "text",
            OutputFormat::Xml => "xml",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(OutputFormat::Json),
            "yaml" | "yml" => Some(OutputFormat::Yaml),
            "sarif" => Some(OutputFormat::Sarif),
            "text" | "txt" => Some(OutputFormat::Text),
            "xml" => Some(OutputFormat::Xml),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::Java.extensions(), &[".java"]);
        assert_eq!(Language::JavaScript.extensions(), &[".js", ".jsx", ".ts", ".tsx"]);
        assert_eq!(Language::Python.extensions(), &[".py", ".pyw"]);
        assert_eq!(Language::Sql.extensions(), &[".sql", ".ddl", ".dml"]);
        assert_eq!(Language::Bash.extensions(), &[".sh", ".bash", ".zsh"]);
        assert_eq!(Language::Php.extensions(), &[".php", ".phtml", ".php3", ".php4", ".php5"]);
        assert_eq!(Language::CSharp.extensions(), &[".cs", ".csx"]);
        assert_eq!(Language::C.extensions(), &[".c", ".h"]);
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("java"), Some(Language::Java));
        assert_eq!(Language::from_str("JavaScript"), Some(Language::JavaScript));
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("sql"), Some(Language::Sql));
        assert_eq!(Language::from_str("bash"), Some(Language::Bash));
        assert_eq!(Language::from_str("php"), Some(Language::Php));
        assert_eq!(Language::from_str("csharp"), Some(Language::CSharp));
        assert_eq!(Language::from_str("c#"), Some(Language::CSharp));
        assert_eq!(Language::from_str("c"), Some(Language::C));
        assert_eq!(Language::from_str("unknown"), None);
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension(".java"), Some(Language::Java));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension(".py"), Some(Language::Python));
        assert_eq!(Language::from_extension(".sql"), Some(Language::Sql));
        assert_eq!(Language::from_extension(".sh"), Some(Language::Bash));
        assert_eq!(Language::from_extension(".php"), Some(Language::Php));
        assert_eq!(Language::from_extension(".cs"), Some(Language::CSharp));
        assert_eq!(Language::from_extension(".c"), Some(Language::C));
        assert_eq!(Language::from_extension(".unknown"), None);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_confidence_ordering() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
    }

    #[test]
    fn test_location_creation() {
        let file = PathBuf::from("test.java");
        let loc = Location::new(file.clone(), 1, 5, 1, 10);
        assert_eq!(loc.file, file);
        assert_eq!(loc.start_line, 1);
        assert_eq!(loc.start_column, 5);
        assert_eq!(loc.end_line, 1);
        assert_eq!(loc.end_column, 10);

        let point_loc = Location::point(file.clone(), 5, 10);
        assert_eq!(point_loc.start_line, 5);
        assert_eq!(point_loc.end_line, 5);
        assert_eq!(point_loc.start_column, 10);
        assert_eq!(point_loc.end_column, 10);
    }

    #[test]
    fn test_finding_creation() {
        let location = Location::point(PathBuf::from("test.java"), 1, 1);
        let finding = Finding::new(
            "test-rule".to_string(),
            "Test message".to_string(),
            Severity::Error,
            Confidence::High,
            location,
        );

        assert_eq!(finding.rule_id, "test-rule");
        assert_eq!(finding.message, "Test message");
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.confidence, Confidence::High);
        assert!(finding.metadata.is_empty());
        assert!(finding.fix_suggestion.is_none());
    }

    #[test]
    fn test_finding_with_metadata_and_fix() {
        let location = Location::point(PathBuf::from("test.java"), 1, 1);
        let finding = Finding::new(
            "test-rule".to_string(),
            "Test message".to_string(),
            Severity::Error,
            Confidence::High,
            location,
        )
        .with_metadata("cwe".to_string(), "CWE-89".to_string())
        .with_fix("Use prepared statements".to_string());

        assert_eq!(finding.metadata.get("cwe"), Some(&"CWE-89".to_string()));
        assert_eq!(finding.fix_suggestion, Some("Use prepared statements".to_string()));
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("YAML"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::from_str("sarif"), Some(OutputFormat::Sarif));
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("xml"), Some(OutputFormat::Xml));
        assert_eq!(OutputFormat::from_str("unknown"), None);
    }

    #[test]
    fn test_analysis_config_default() {
        let config = AnalysisConfig::default();
        assert_eq!(config.target_paths, vec![PathBuf::from(".")]);
        assert!(config.exclude_patterns.is_empty());
        assert_eq!(config.languages.len(), 5);
        assert!(config.parallel);
        assert_eq!(config.output_format, OutputFormat::Json);
    }
}
