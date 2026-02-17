//! Plain text output formatter

use crate::output::analysis::{AnalysisStatistics, Finding, OutputFormatter};
use anyhow::Result;
use std::time::Duration;

/// Plain text output formatter
pub struct TextFormatter;

impl OutputFormatter for TextFormatter {
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        total_time: Duration,
    ) -> Result<String> {
        let mut output = String::new();

        output.push_str("=== astgrep Analysis Results ===\n\n");

        if findings.is_empty() {
            output.push_str("✅ No issues found!\n\n");
        } else {
            output.push_str(&format!("Found {} issue(s):\n\n", findings.len()));

            for (i, finding) in findings.iter().enumerate() {
                output.push_str(&format!(
                    "{}. {} ({})",
                    i + 1,
                    finding.message,
                    finding.rule_id
                ));
                output.push_str(&format!(
                    "   File: {}:{}:{}\n",
                    finding.location.file.display(),
                    finding.location.start_line,
                    finding.location.start_column
                ));
                output.push_str(&format!(
                    "   Severity: {:?}, Confidence: {:?}\n",
                    finding.severity, finding.confidence
                ));
                if let Some(ref fix) = finding.fix {
                    output.push_str(&format!("   Fix: {}\n", fix));
                }
                output.push_str("\n");
            }
        }

        // Summary
        output.push_str("=== Summary ===\n");
        output.push_str(&format!("Files analyzed: {}\n", stats.files_analyzed));
        output.push_str(&format!("Rules executed: {}\n", stats.rules_executed));
        output.push_str(&format!("Analysis time: {:?}\n", total_time));
        output.push_str(&format!("Parse errors: {}\n", stats.parse_errors));
        output.push_str(&format!("Analysis errors: {}\n", stats.analysis_errors));

        Ok(output)
    }

    fn content_type(&self) -> &'static str {
        "text/plain"
    }
}
