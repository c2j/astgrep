//! Plain text output formatter

use crate::output::analysis::{get_source_line, AnalysisStatistics, Finding, OutputFormatter};
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
            output.push_str("No issues found.\n\n");
        } else {
            output.push_str(&format!("Found {} issue(s):\n\n", findings.len()));

            let cwd = std::env::current_dir().unwrap_or_default();

            for (i, finding) in findings.iter().enumerate() {
                let file = finding.location.file.display().to_string();
                let short_path = file
                    .strip_prefix(cwd.to_string_lossy().as_ref())
                    .unwrap_or(&file)
                    .trim_start_matches('/');
                let rule_label = if finding.message != finding.rule_id {
                    format!("{} ({})", finding.message, finding.rule_id)
                } else {
                    finding.rule_id.clone()
                };

                output.push_str(&format!(
                    "{}. {}  {}:{}:{}\n",
                    i + 1,
                    rule_label,
                    short_path,
                    finding.location.start_line,
                    finding.location.start_column,
                ));
                output.push_str(&format!(
                    "   Severity: {:?}, Confidence: {:?}\n",
                    finding.severity, finding.confidence
                ));

                // Show source line with position marker
                if let Some(src_line) =
                    get_source_line(&finding.location.file, finding.location.start_line)
                {
                    output.push_str(&format!(
                        "   {} | {}\n",
                        finding.location.start_line, src_line
                    ));
                    let indent = format!("   {} | ", finding.location.start_line);
                    let padding = " ".repeat(finding.location.start_column.saturating_sub(1));
                    output.push_str(&format!("{}{}^\n", indent, padding));
                }

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
        if stats.parse_errors > 0 || stats.analysis_errors > 0 {
            output.push_str(&format!("Parse errors: {}\n", stats.parse_errors));
            output.push_str(&format!("Analysis errors: {}\n", stats.analysis_errors));
        }

        Ok(output)
    }

    fn content_type(&self) -> &'static str {
        "text/plain"
    }
}
