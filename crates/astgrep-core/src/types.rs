//! Core types for astgrep

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
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
    Xml,
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
            Language::Xml => &[".xml", ".xsd", ".xsl", ".xslt", ".svg", ".pom"],
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
            Language::Xml => "xml",
        }
    }

    /// Parse language from string
    pub fn parse_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "java" => Some(Language::Java),
            "javascript" | "js" | "typescript" | "ts" => Some(Language::JavaScript),
            "python" | "py" => Some(Language::Python),
            "sql" => Some(Language::Sql),
            "bash" | "shell" | "sh" => Some(Language::Bash),
            "xml" => Some(Language::Xml),
            _ => None,
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = if ext.starts_with('.') {
            ext
        } else {
            &format!(".{}", ext)
        };

        crate::constants::languages::ALL_LANGUAGES
            .iter()
            .find(|&&lang| lang.extensions().contains(&ext))
            .copied()
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
    pub metadata: HashMap<String, Value>,
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
        self.metadata.insert(key, Value::String(value));
        self
    }

    /// Add metadata with any YAML value type
    pub fn with_metadata_value(mut self, key: String, value: Value) -> Self {
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
    #[serde(default)]
    pub sql_dialect: Option<SqlDialect>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        use crate::constants::{languages, paths};

        Self {
            target_paths: vec![PathBuf::from(".")],
            exclude_patterns: paths::DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            languages: languages::DEFAULT_LANGUAGES.to_vec(),
            rule_files: vec![],
            output_format: OutputFormat::Json,
            parallel: true,
            max_threads: Some(crate::constants::performance::DEFAULT_THREAD_COUNT),
            sql_dialect: None,
        }
    }
}

/// SQL 方言枚举。新增方言时必须保留向前兼容（已有 match 不会因新 variant 编译失败）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SqlDialect {
    /// 通用 SQL（tree-sitter-sequel）
    Standard,
    /// 华为 GaussDB 集中式
    #[serde(rename = "gaussdb")]
    GaussDB,
    /// 开源 OpenGauss（默认集中式，可切换分布式）
    #[serde(rename = "opengauss")]
    OpenGauss,
    /// 阿里 PolarDB MySQL 兼容版
    #[serde(rename = "polardb-mysql")]
    PolarDBMySQL,
}

impl SqlDialect {
    /// 返回该方言使用的底层 parser 家族，用于派发器选择路径。
    pub fn parser_family(&self) -> SqlParserFamily {
        match self {
            SqlDialect::Standard => SqlParserFamily::TreeSitterSequel,
            SqlDialect::GaussDB | SqlDialect::OpenGauss => SqlParserFamily::Ogsql,
            SqlDialect::PolarDBMySQL => SqlParserFamily::Sqlparser,
        }
    }

    /// 从字符串解析方言。未知字符串返回 `None`，由调用方决定 fallback 策略。
    // from_str conflicts with std::str::FromStr which returns Result instead of Option
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "standard" | "sql" => Some(Self::Standard),
            "gaussdb" | "gauss" => Some(Self::GaussDB),
            "opengauss" | "og" => Some(Self::OpenGauss),
            "polardb-mysql" | "polardb" => Some(Self::PolarDBMySQL),
            _ => None,
        }
    }
}

/// SQL parser 家族分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlParserFamily {
    TreeSitterSequel,
    Ogsql,
    Sqlparser,
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

    pub fn parse_name(s: &str) -> Option<Self> {
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
        assert_eq!(
            Language::JavaScript.extensions(),
            &[".js", ".jsx", ".ts", ".tsx"]
        );
        assert_eq!(Language::Python.extensions(), &[".py", ".pyw"]);
        assert_eq!(Language::Sql.extensions(), &[".sql", ".ddl", ".dml"]);
        assert_eq!(Language::Bash.extensions(), &[".sh", ".bash", ".zsh"]);
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::parse_name("java"), Some(Language::Java));
        assert_eq!(
            Language::parse_name("JavaScript"),
            Some(Language::JavaScript)
        );
        assert_eq!(Language::parse_name("python"), Some(Language::Python));
        assert_eq!(Language::parse_name("sql"), Some(Language::Sql));
        assert_eq!(Language::parse_name("bash"), Some(Language::Bash));
        assert_eq!(Language::parse_name("unknown"), None);
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension(".java"), Some(Language::Java));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension(".py"), Some(Language::Python));
        assert_eq!(Language::from_extension(".sql"), Some(Language::Sql));
        assert_eq!(Language::from_extension(".sh"), Some(Language::Bash));
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

        assert_eq!(
            finding.metadata.get("cwe"),
            Some(&serde_yaml::Value::String("CWE-89".to_string()))
        );
        assert_eq!(
            finding.fix_suggestion,
            Some("Use prepared statements".to_string())
        );
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::parse_name("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse_name("YAML"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::parse_name("sarif"), Some(OutputFormat::Sarif));
        assert_eq!(OutputFormat::parse_name("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::parse_name("xml"), Some(OutputFormat::Xml));
        assert_eq!(OutputFormat::parse_name("unknown"), None);
    }

    #[test]
    fn test_analysis_config_default() {
        let config = AnalysisConfig::default();
        assert_eq!(config.target_paths, vec![PathBuf::from(".")]);
        assert!(!config.exclude_patterns.is_empty());
        assert_eq!(config.languages.len(), 6);
        assert!(config.parallel);
        assert_eq!(config.output_format, OutputFormat::Json);
    }

    #[test]
    fn test_sql_dialect_from_str_standard() {
        assert_eq!(SqlDialect::from_str("standard"), Some(SqlDialect::Standard));
    }

    #[test]
    fn test_sql_dialect_from_str_sql_alias() {
        assert_eq!(SqlDialect::from_str("sql"), Some(SqlDialect::Standard));
    }

    #[test]
    fn test_sql_dialect_from_str_gaussdb() {
        assert_eq!(SqlDialect::from_str("gaussdb"), Some(SqlDialect::GaussDB));
    }

    #[test]
    fn test_sql_dialect_from_str_gaussdb_case_insensitive() {
        assert_eq!(SqlDialect::from_str("GAUSSDB"), Some(SqlDialect::GaussDB));
    }

    #[test]
    fn test_sql_dialect_from_str_opengauss() {
        assert_eq!(
            SqlDialect::from_str("opengauss"),
            Some(SqlDialect::OpenGauss)
        );
    }

    #[test]
    fn test_sql_dialect_from_str_og_alias() {
        assert_eq!(SqlDialect::from_str("og"), Some(SqlDialect::OpenGauss));
    }

    #[test]
    fn test_sql_dialect_from_str_polardb_mysql() {
        assert_eq!(
            SqlDialect::from_str("polardb-mysql"),
            Some(SqlDialect::PolarDBMySQL)
        );
    }

    #[test]
    fn test_sql_dialect_from_str_polardb_alias() {
        assert_eq!(
            SqlDialect::from_str("polardb"),
            Some(SqlDialect::PolarDBMySQL)
        );
    }

    #[test]
    fn test_sql_dialect_from_str_unknown() {
        assert_eq!(SqlDialect::from_str("oracle"), None);
    }

    #[test]
    fn test_sql_dialect_from_str_empty() {
        assert_eq!(SqlDialect::from_str(""), None);
    }

    #[test]
    fn test_sql_dialect_parser_family() {
        assert_eq!(
            SqlDialect::Standard.parser_family(),
            SqlParserFamily::TreeSitterSequel
        );
        assert_eq!(SqlDialect::GaussDB.parser_family(), SqlParserFamily::Ogsql);
        assert_eq!(
            SqlDialect::OpenGauss.parser_family(),
            SqlParserFamily::Ogsql
        );
        assert_eq!(
            SqlDialect::PolarDBMySQL.parser_family(),
            SqlParserFamily::Sqlparser
        );
    }

    #[test]
    fn test_sql_dialect_serde_roundtrip() {
        let serialized = serde_json::to_string(&SqlDialect::GaussDB).unwrap();
        assert_eq!(serialized, "\"gaussdb\"");
        let deserialized: SqlDialect = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, SqlDialect::GaussDB);
    }

    #[test]
    fn test_analysis_config_default_sql_dialect() {
        let config = AnalysisConfig::default();
        assert_eq!(config.sql_dialect, None);
    }
}
