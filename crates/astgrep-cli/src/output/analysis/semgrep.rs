//! Semgrep-compatible output formatter

use crate::output::analysis::Severity;
use crate::output::analysis::{get_source_line, AnalysisStatistics, Finding, OutputFormatter};
use anyhow::Result;
use std::fmt::Write;
use std::time::Duration;

/// Semgrep-compatible output formatter
pub struct SemgrepFormatter;

impl OutputFormatter for SemgrepFormatter {
    fn format(
        &self,
        findings: &[Finding],
        stats: &AnalysisStatistics,
        _total_time: Duration,
    ) -> Result<String> {
        let mut output = String::new();

        // Semgrep header
        writeln!(&mut output, "┌──── ○○○ ────┐")?;
        writeln!(&mut output, "│ astgrep │")?;
        writeln!(&mut output, "└─────────────┘")?;
        writeln!(&mut output)?;

        // Progress section
        writeln!(
            &mut output,
            "Scanning {} file(s) with {} rule(s):",
            stats.files_analyzed, stats.rules_executed
        )?;
        writeln!(&mut output)?;
        writeln!(&mut output, "  CODE RULES")?;
        writeln!(&mut output, "  Scanning {} file(s).", stats.files_analyzed)?;
        writeln!(&mut output)?;
        writeln!(&mut output, "  PROGRESS")?;
        writeln!(
            &mut output,
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100% 0:00:00"
        )?;
        writeln!(&mut output)?;
        writeln!(&mut output)?;

        // Findings section
        if findings.is_empty() {
            writeln!(&mut output, "┌─────────────────┐")?;
            writeln!(&mut output, "│ 0 Code Findings │")?;
            writeln!(&mut output, "└─────────────────┘")?;
        } else {
            writeln!(&mut output, "┌─────────────────┐")?;
            writeln!(
                &mut output,
                "│ {} Code Finding{} │",
                findings.len(),
                if findings.len() == 1 { "" } else { "s" }
            )?;
            writeln!(&mut output, "└─────────────────┘")?;
            writeln!(&mut output)?;

            // Group findings by file and then by rule
            let mut findings_by_file_and_rule: std::collections::HashMap<
                String,
                std::collections::HashMap<String, Vec<&Finding>>,
            > = std::collections::HashMap::new();

            for finding in findings {
                let file_path = finding.location.file.to_string_lossy().to_string();
                findings_by_file_and_rule
                    .entry(file_path)
                    .or_default()
                    .entry(finding.rule_id.clone())
                    .or_default()
                    .push(finding);
            }

            for (file_path, rules_map) in findings_by_file_and_rule {
                writeln!(&mut output, "    {}", file_path)?;

                for (rule_id, mut rule_findings) in rules_map {
                    // Sort findings by line number
                    rule_findings.sort_by_key(|f| f.location.start_line);

                    // Get the first finding to extract rule info
                    let first_finding = &rule_findings[0];
                    writeln!(&mut output, "   ❯❯❱ {}", rule_id)?;
                    writeln!(&mut output, "          {}", first_finding.message.trim())?;
                    writeln!(&mut output)?;

                    // Display all findings for this rule
                    for (i, finding) in rule_findings.iter().enumerate() {
                        writeln!(
                            &mut output,
                            "           {}┆ {}",
                            finding.location.start_line,
                            get_source_line(&finding.location.file, finding.location.start_line)
                                .unwrap_or_else(|| "<source unavailable>".to_string())
                        )?;

                        // Add separator between findings (except for the last one)
                        if rule_findings.len() > 1 && i < rule_findings.len() - 1 {
                            writeln!(
                                &mut output,
                                "            ⋮┆----------------------------------------"
                            )?;
                        }
                    }
                    writeln!(&mut output)?;
                }
            }
        }

        // Summary section
        writeln!(&mut output, "┌──────────────┐")?;
        writeln!(&mut output, "│ Scan Summary │")?;
        writeln!(&mut output, "└──────────────┘")?;
        writeln!(&mut output, "✅ Scan completed successfully.")?;
        writeln!(
            &mut output,
            " • Findings: {} ({} blocking)",
            findings.len(),
            findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error))
                .count()
        )?;
        writeln!(&mut output, " • Rules run: {}", stats.rules_executed)?;
        writeln!(&mut output, " • Targets scanned: {}", stats.files_analyzed)?;
        writeln!(&mut output, " • Parsed lines: ~100.0%")?;
        writeln!(&mut output, " • No ignore information available")?;
        writeln!(
            &mut output,
            "Ran {} rule{} on {} file{}: {} finding{}.",
            stats.rules_executed,
            if stats.rules_executed == 1 { "" } else { "s" },
            stats.files_analyzed,
            if stats.files_analyzed == 1 { "" } else { "s" },
            findings.len(),
            if findings.len() == 1 { "" } else { "s" }
        )?;

        Ok(output)
    }

    fn content_type(&self) -> &'static str {
        "text/plain"
    }
}
