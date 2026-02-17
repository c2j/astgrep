//! SARIF (Static Analysis Results Interchange Format) output formatter

use crate::output::analysis::{AnalysisStatistics, Finding, OutputFormatter};
use crate::output::analysis::Severity;
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

/// SARIF output formatter
pub struct SarifFormatter;

impl OutputFormatter for SarifFormatter {
    fn format(
        &self,
        findings: &[Finding],
        _stats: &AnalysisStatistics,
        _total_time: Duration,
    ) -> Result<String> {
        let sarif = json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "astgrep",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/your-org/astgrep"
                    }
                },
                "results": findings.iter().map(|finding| {
                    json!({
                        "ruleId": finding.rule_id,
                        "message": {
                            "text": finding.message
                        },
                        "level": severity_to_sarif_level(finding.severity),
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": finding.location.file.to_string_lossy()
                                },
                                "region": {
                                    "startLine": finding.location.start_line,
                                    "startColumn": finding.location.start_column,
                                    "endLine": finding.location.end_line,
                                    "endColumn": finding.location.end_column
                                }
                            }
                        }]
                    })
                }).collect::<Vec<_>>()
            }]
        });

        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    fn content_type(&self) -> &'static str {
        "application/sarif+json"
    }
}

fn severity_to_sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "error",
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}
