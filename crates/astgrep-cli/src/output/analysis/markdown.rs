//! Markdown output formatter

use crate::output::analysis::Severity;
use crate::output::analysis::{AnalysisStatistics, Finding, OutputFormatter};
use anyhow::Result;
use std::time::Duration;

/// Markdown output formatter
pub struct MarkdownFormatter;

impl OutputFormatter for MarkdownFormatter {
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        total_time: Duration,
    ) -> Result<String> {
        let mut md = String::new();

        md.push_str("# astgrep Analysis Report\n\n");
        md.push_str(&format!(
            "**Generated:** {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total findings:** {}\n", findings.len()));
        md.push_str(&format!("- **Files analyzed:** {}\n", stats.files_analyzed));
        md.push_str(&format!("- **Analysis time:** {:?}\n\n", total_time));

        if !findings.is_empty() {
            md.push_str("## Findings\n\n");

            for (i, finding) in findings.iter().enumerate() {
                let severity_emoji = match finding.severity {
                    Severity::Critical => "🔴",
                    Severity::Error => "🔴",
                    Severity::Warning => "🟡",
                    Severity::Info => "🔵",
                };

                md.push_str(&format!(
                    "### {} {}. {}\n\n",
                    severity_emoji,
                    i + 1,
                    finding.message
                ));
                md.push_str(&format!("- **Rule:** `{}`\n", finding.rule_id));
                md.push_str(&format!(
                    "- **File:** `{}:{}:{}`\n",
                    finding.location.file.display(),
                    finding.location.start_line,
                    finding.location.start_column
                ));
                md.push_str(&format!("- **Severity:** {:?}\n", finding.severity));
                md.push_str(&format!("- **Confidence:** {:?}\n", finding.confidence));

                if let Some(ref fix) = finding.fix {
                    md.push_str(&format!("- **Fix:** {}\n", fix));
                }

                md.push('\n');
            }
        }

        Ok(md)
    }

    fn content_type(&self) -> &'static str {
        "text/markdown"
    }
}
