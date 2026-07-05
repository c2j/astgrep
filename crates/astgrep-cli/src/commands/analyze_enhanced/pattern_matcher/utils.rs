// Utility functions for pattern matching

use super::super::types::ParsedRule;
use super::core::{convert_pattern_to_regex, find_pattern_matches};
use crate::output::analysis::{Confidence, Finding, Location};
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Simple metavariable pattern matching for basic cases
pub fn apply_simple_metavariable_pattern(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Check if this is a NOT_REGEX pattern
    if pattern.starts_with("NOT_REGEX:") {
        // For now, skip NOT_REGEX patterns in simple matching
        // They should be handled by the enhanced pattern matching
        return Ok(findings);
    }

    // Check if this is a NOT_INSIDE pattern
    if pattern.starts_with("NOT_INSIDE:") {
        // For now, skip NOT_INSIDE patterns in simple matching
        // They should be handled by the enhanced pattern matching
        return Ok(findings);
    }

    // Check if this is a direct regex pattern
    // Allow patterns with escaped dollar signs (like \\$[a-zA-Z_])
    let is_regex_pattern = (!pattern.contains('$') || pattern.contains("\\$"))
        && (pattern.contains('[')
            || pattern.contains('*')
            || pattern.contains('+')
            || pattern.contains('?')
            || pattern.contains('^')
            || pattern.contains('\\')
            || pattern.contains('(')
            || pattern.contains('|'));

    info!(
        "Pattern '{}' - is_regex_pattern: {}, contains '$': {}, contains '(': {}, contains '|': {}",
        pattern,
        is_regex_pattern,
        pattern.contains('$'),
        pattern.contains('('),
        pattern.contains('|')
    );

    if is_regex_pattern {
        // Handle as direct regex - first fix double escaping from YAML
        let fixed_pattern = pattern
            .replace("\\\\s", "\\s")
            .replace("\\\\d", "\\d")
            .replace("\\\\w", "\\w")
            .replace("\\\\b", "\\b")
            .replace("\\\\t", "\\t")
            .replace("\\\\n", "\\n")
            .replace("\\\\r", "\\r")
            .replace("\\\\$", "\\$");

        info!("Attempting to compile regex pattern: '{}'", fixed_pattern);

        if let Ok(regex) = regex::Regex::new(&fixed_pattern) {
            info!("Regex compiled successfully, searching for matches...");
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

                if line.contains('$') {
                    info!("Checking line {}: '{}'", line_num + 1, line);
                }
                for mat in regex.find_iter(line) {
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

                    info!(
                        "Found regex match: '{}' at line {} position {}-{}",
                        &line[mat.start()..mat.end()],
                        line_num + 1,
                        mat.start(),
                        mat.end()
                    );
                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity,
                        confidence: Confidence::High,
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: mat.start() + 1,
                            end_line: line_num + 1,
                            end_column: mat.end() + 1,
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        } else {
            // If regex compilation fails, try as literal pattern
            for (line_num, line) in source_code.lines().enumerate() {
                // Skip lines that are comments
                let trimmed_line = line.trim();
                if trimmed_line.starts_with('#')
                    || trimmed_line.starts_with("//")
                    || trimmed_line.starts_with("/*")
                {
                    continue;
                }

                if line.contains(pattern) {
                    if let Some(pos) = line.find(pattern) {
                        // Check if the match is inside a comment on the same line
                        if let Some(comment_pos) = line.find('#') {
                            if pos >= comment_pos {
                                continue; // Skip matches inside inline comments
                            }
                        }
                        if let Some(comment_pos) = line.find("//") {
                            if pos >= comment_pos {
                                continue; // Skip matches inside inline comments
                            }
                        }

                        // Additional check: if the line starts with # (comment), skip the match entirely
                        if line.trim_start().starts_with('#') {
                            continue; // Skip matches in comment lines
                        }

                        let finding = Finding {
                            rule_id: rule.id.clone(),
                            message: rule.message.clone(),
                            severity: rule.severity,
                            confidence: Confidence::Medium,
                            location: Location {
                                file: file_path.clone(),
                                start_line: line_num + 1,
                                start_column: pos + 1,
                                end_line: line_num + 1,
                                end_column: pos + pattern.len() + 1,
                            },
                            fix: rule.fix.clone(),
                        };
                        findings.push(finding);
                    }
                }
            }
        }
    } else {
        // Convert pattern to a regex-like pattern for matching
        let regex_pattern = convert_pattern_to_regex(pattern);

        for (line_num, line) in source_code.lines().enumerate() {
            // Skip lines that are comments
            let trimmed_line = line.trim();
            if trimmed_line.starts_with('#')
                || trimmed_line.starts_with("//")
                || trimmed_line.starts_with("/*")
            {
                continue;
            }

            if let Some(matches) = find_pattern_matches(&regex_pattern, line) {
                for match_pos in matches {
                    // Check if the match is inside a comment on the same line
                    if let Some(comment_pos) = line.find('#') {
                        if match_pos >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }
                    if let Some(comment_pos) = line.find("//") {
                        if match_pos >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }

                    // Additional check: if the line starts with # (comment), skip the match entirely
                    if line.trim_start().starts_with('#') {
                        continue; // Skip matches in comment lines
                    }

                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity,
                        confidence: Confidence::High,
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: match_pos + 1,
                            end_line: line_num + 1,
                            end_column: match_pos + pattern.len(),
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        }
    }

    // Handle the most common case: $X (matches any expression)
    if pattern.trim() == "$X" {
        // Find all expressions in the code (simplified heuristic)
        for (line_num, line) in source_code.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("def ")
                && !trimmed.starts_with("class ")
                && !trimmed.starts_with("import ")
                && !trimmed.starts_with("from ")
            {
                // Look for expressions (assignments, function calls, etc.)
                if trimmed.contains('=')
                    || trimmed.contains('(')
                    || trimmed.contains('[')
                    || (trimmed.chars().any(|c| c.is_alphanumeric())
                        && !trimmed.starts_with("if ")
                        && !trimmed.starts_with("for ")
                        && !trimmed.starts_with("while "))
                {
                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity,
                        confidence: Confidence::Low, // Lower confidence for heuristic matching
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: 1,
                            end_line: line_num + 1,
                            end_column: line.len() + 1,
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        }
    }

    Ok(findings)
}
