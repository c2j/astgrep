// Regex pattern matching functions

use super::super::types::ParsedRule;
use crate::output::analysis::Confidence;
use crate::output::analysis::{Finding, Location};
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Apply a regex pattern to source code and return findings
pub fn apply_regex_pattern(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    if let Ok(regex) = regex::Regex::new(pattern) {
        for (line_num, line) in source_code.lines().enumerate() {
            // Skip lines that are comments
            let trimmed_line = line.trim();
            if trimmed_line.starts_with('#')
                || trimmed_line.starts_with("//")
                || trimmed_line.starts_with("/*")
            {
                info!("Skipping comment line {}: '{}'", line_num + 1, line);
                continue;
            }

            // Use captures_iter to get all matches with capture groups
            for captures in regex.captures_iter(line) {
                if let Some(mat) = captures.get(0) {
                    // Check if the match is inside a comment on the same line
                    if let Some(comment_pos) = line.find('#') {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }
                    if let Some(comment_pos) = line.find("//") {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }

                    // Additional check: if the line starts with # (comment), skip the match entirely
                    if line.trim_start().starts_with('#') {
                        continue; // Skip matches in comment lines
                    }

                    // Replace capture groups in the message
                    let message = replace_capture_groups(&rule.message, &captures);

                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message,
                        severity: rule.severity.clone(),
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            end_line: line_num + 1,
                            start_column: mat.start() + 1,
                            end_column: mat.end() + 1,
                        },
                        confidence: Confidence::High,
                        fix: None,
                    };
                    findings.push(finding);
                }
            }
        }
    }

    Ok(findings)
}

/// Replace capture groups in message template with actual captured values
pub fn replace_capture_groups(message: &str, captures: &regex::Captures) -> String {
    let mut result = message.to_string();

    // Replace numbered capture groups: ${1}, ${2}, etc.
    for i in 1..captures.len() {
        if let Some(captured) = captures.get(i) {
            let placeholder = format!("${{{}}}", i);
            result = result.replace(&placeholder, captured.as_str());
        }
    }

    result
}
