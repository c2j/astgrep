//! Analysis output formatting module for astgrep
//!
//! This module provides various output formatters for analysis findings,
//! including JSON, SARIF, HTML, Markdown, Text, and Semgrep-compatible formats.

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

mod html;
mod json;
mod markdown;
mod sarif;
mod semgrep;
mod text;

pub use html::HtmlFormatter;
pub use json::JsonFormatter;
pub use markdown::MarkdownFormatter;
pub use sarif::SarifFormatter;
pub use semgrep::SemgrepFormatter;
pub use text::TextFormatter;

/// Analysis finding representation
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub location: Location,
    pub fix: Option<String>,
}

/// Location of a finding in source code
#[derive(Debug, Clone, serde::Serialize)]
pub struct Location {
    #[serde(serialize_with = "serialize_pathbuf")]
    pub file: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

fn serialize_pathbuf<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&path.to_string_lossy())
}

// Re-export Severity and Confidence from astgrep_core to ensure consistency
pub use astgrep_core::{Confidence, Severity};

/// Analysis statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisStatistics {
    pub files_analyzed: usize,
    pub rules_executed: usize,
    pub parse_errors: usize,
    pub analysis_errors: usize,
    pub dataflow_analyses: usize,
}

impl AnalysisStatistics {
    pub fn new() -> Self {
        Self {
            files_analyzed: 0,
            rules_executed: 0,
            parse_errors: 0,
            analysis_errors: 0,
            dataflow_analyses: 0,
        }
    }
}

impl Default for AnalysisStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for output formatters
pub trait OutputFormatter {
    /// Format findings into the target output format
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        total_time: Duration,
    ) -> Result<String>;

    /// Get the content type (MIME type) for this format
    fn content_type(&self) -> &'static str;
}

/// Factory for creating output formatters
pub struct OutputFactory;

impl OutputFactory {
    /// Create a formatter for the specified output format
    pub fn create(format: OutputFormat) -> Box<dyn OutputFormatter> {
        match format {
            OutputFormat::Json => Box::new(JsonFormatter),
            OutputFormat::Sarif => Box::new(SarifFormatter),
            OutputFormat::Text => Box::new(TextFormatter),
            OutputFormat::Html => Box::new(HtmlFormatter),
            OutputFormat::Markdown => Box::new(MarkdownFormatter),
        }
    }

    /// Create a formatter for the specified compatibility mode
    pub fn create_compatible(mode: &str) -> Option<Box<dyn OutputFormatter>> {
        match mode.to_lowercase().as_str() {
            "semgrep" => Some(Box::new(SemgrepFormatter)),
            _ => None,
        }
    }
}

/// Output format types
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Sarif,
    Text,
    Html,
    Markdown,
}

/// Helper function to get source line from file
pub fn get_source_line(file_path: &std::path::Path, line_number: usize) -> Option<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    for (current_line, line) in reader.lines().enumerate() {
        if current_line + 1 == line_number {
            return line.ok().map(|l| l.trim().to_string());
        }
    }

    None
}
