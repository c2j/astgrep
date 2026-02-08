//! JSON output formatter

use crate::output::analysis::{AnalysisStatistics, Finding, OutputFormatter};
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

/// JSON output formatter
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        total_time: Duration,
    ) -> Result<String> {
        let output = json!({
            "findings": findings,
            "summary": {
                "total_findings": findings.len(),
                "files_analyzed": stats.files_analyzed,
                "rules_executed": stats.rules_executed,
                "analysis_time_ms": total_time.as_millis(),
            }
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}
