//! HTML output formatter

use crate::output::analysis::Severity;
use crate::output::analysis::{AnalysisStatistics, Finding, OutputFormatter};
use anyhow::Result;
use std::time::Duration;

/// HTML output formatter
pub struct HtmlFormatter;

impl OutputFormatter for HtmlFormatter {
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        total_time: Duration,
    ) -> Result<String> {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>astgrep Analysis Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str(".finding { border: 1px solid #ddd; margin: 10px 0; padding: 10px; }\n");
        html.push_str(".error { border-left: 5px solid #f44336; }\n");
        html.push_str(".warning { border-left: 5px solid #ff9800; }\n");
        html.push_str(".info { border-left: 5px solid #2196f3; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str("<h1>astgrep Analysis Report</h1>\n");
        html.push_str(&format!(
            "<p>Generated on: {}</p>\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        html.push_str("<h2>Summary</h2>\n");
        html.push_str(&format!("<p>Total findings: {}</p>\n", findings.len()));
        html.push_str(&format!(
            "<p>Files analyzed: {}</p>\n",
            stats.files_analyzed
        ));
        html.push_str(&format!("<p>Analysis time: {:?}</p>\n", total_time));

        if !findings.is_empty() {
            html.push_str("<h2>Findings</h2>\n");

            for finding in findings {
                let severity_class = match finding.severity {
                    Severity::Critical => "error",
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };

                html.push_str(&format!("<div class=\"finding {}\">\n", severity_class));
                html.push_str(&format!("<h3>{}</h3>\n", finding.message));
                html.push_str(&format!(
                    "<p><strong>Rule:</strong> {}</p>\n",
                    finding.rule_id
                ));
                html.push_str(&format!(
                    "<p><strong>File:</strong> {}:{}:{}</p>\n",
                    finding.location.file.display(),
                    finding.location.start_line,
                    finding.location.start_column
                ));
                html.push_str(&format!(
                    "<p><strong>Severity:</strong> {:?}</p>\n",
                    finding.severity
                ));
                html.push_str(&format!(
                    "<p><strong>Confidence:</strong> {:?}</p>\n",
                    finding.confidence
                ));

                if let Some(ref fix) = finding.fix {
                    html.push_str(&format!("<p><strong>Fix:</strong> {}</p>\n", fix));
                }

                html.push_str("</div>\n");
            }
        }

        html.push_str("</body>\n</html>\n");

        Ok(html)
    }

    fn content_type(&self) -> &'static str {
        "text/html"
    }
}
