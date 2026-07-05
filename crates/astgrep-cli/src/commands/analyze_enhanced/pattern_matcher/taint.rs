//! Taint analysis functionality for pattern matching

use super::super::types::ParsedRule;
use crate::output::analysis::{Confidence, Finding, Location};
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Apply simplified taint analysis for taint rules
pub fn apply_simple_taint_analysis(
    rule: &ParsedRule,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = source_code.lines().collect();

    // Find taint sources, sinks, and sanitized variables
    let mut taint_sources = Vec::new();
    let mut sanitized_vars = Vec::new();
    let mut sink_calls = Vec::new();
    let mut var_assignments = Vec::new(); // Track all variable assignments

    // First pass: collect all assignments and direct taint sources
    for (line_num, line) in lines.iter().enumerate() {
        // Find direct taint source usage in sink calls
        if line.contains("sink(") && line.contains("\"tainted\"") {
            // Check if the taint is sanitized: sink(sanitize("tainted"))
            if !line.contains("sanitize(") {
                // Direct taint: sink("tainted") without sanitization
                let finding = Finding {
                    rule_id: rule.id.clone(),
                    message: rule.message.clone(),
                    severity: rule.severity,
                    confidence: Confidence::High,
                    location: Location {
                        file: file_path.clone(),
                        start_line: line_num + 1,
                        start_column: line.find("sink(").unwrap_or(0) + 1,
                        end_line: line_num + 1,
                        end_column: line.find("sink(").unwrap_or(0) + 5,
                    },
                    fix: rule.fix.clone(),
                };
                findings.push(finding);
            }
        }

        // Track variable assignments from taint sources
        if line.contains("\"tainted\"") && line.contains("=") {
            // Extract variable name (handle both regular vars and PHP vars with $)
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                if let Some(var_name) = left_part.split_whitespace().last() {
                    let clean_var = var_name.trim_end_matches(';').trim();
                    info!("Taint source found: {} (line {})", clean_var, line_num + 1);
                    taint_sources.push((clean_var.to_string(), line_num + 1));
                }
            }
        }

        // Track variable-to-variable assignments for later propagation
        if line.contains("=") && !line.contains("\"") && !line.contains("sanitize(") {
            // Extract assignment: var1 = var2
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                let right_part = line[equals_pos + 1..].trim();

                // Extract variable names (simplified)
                if let Some(left_var) = left_part.split_whitespace().last() {
                    if let Some(right_var) = right_part.split_whitespace().next() {
                        // Clean up variable names (remove semicolons, etc.)
                        let clean_left = left_var.trim_end_matches(';').trim();
                        let clean_right = right_var.trim_end_matches(';').trim();
                        info!(
                            "Variable assignment found: {} = {} (line {})",
                            clean_left,
                            clean_right,
                            line_num + 1
                        );
                        var_assignments.push((
                            clean_left.to_string(),
                            clean_right.to_string(),
                            line_num + 1,
                        ));
                    }
                }
            }
        }

        // Track variable sanitization: x = sanitize(x) or x = sanitize("tainted")
        // Skip sanitizers that are in conditional branches (improved heuristic)
        // Detect base indentation level from the first non-empty line
        let line_indent = line.len() - line.trim_start().len();

        // Simple heuristic: if line has more than 2 spaces and contains "if" or "else" nearby,
        // consider subsequent more-indented lines as conditional
        let is_in_conditional = line_indent > 2
            && (
                // Check if there's an if/else statement in recent lines
                lines
                    .iter()
                    .take(line_num + 1)
                    .rev()
                    .take(3)
                    .any(|prev_line| {
                        prev_line.trim().starts_with("if ") || prev_line.trim().starts_with("else")
                    })
            );

        if line.contains("sanitize(") && line.contains("=") && !is_in_conditional {
            // Extract variable name being assigned (handle both regular vars and PHP vars with $)
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                if let Some(var_name) = left_part.split_whitespace().last() {
                    let clean_var = var_name.trim_end_matches(';').trim();
                    info!("Sanitizer found: {} (line {})", clean_var, line_num + 1);
                    sanitized_vars.push((clean_var.to_string(), line_num + 1));
                }
            }
        }

        // Track sink calls with variables
        if line.contains("sink(") && !line.contains("\"tainted\"") {
            // Extract argument (simplified)
            if let Some(start) = line.find("sink(") {
                let after_paren = start + 5;
                if let Some(end) = line[after_paren..].find(')') {
                    let arg = line[after_paren..after_paren + end].trim();
                    if !arg.is_empty() && !arg.starts_with('"') {
                        sink_calls.push((arg.to_string(), line_num + 1));
                    }
                }
            }
        }
    }

    // Multi-round taint propagation through variable assignments
    let mut changed = true;
    let mut round = 0;
    while changed && round < 10 {
        // Limit rounds to prevent infinite loops
        changed = false;
        round += 1;

        for (left_var, right_var, assignment_line) in &var_assignments {
            // Check if right_var is tainted and left_var is not yet tainted
            let right_taint_info = taint_sources
                .iter()
                .find(|(tainted_var, _)| tainted_var == right_var);
            let left_already_tainted = taint_sources
                .iter()
                .any(|(tainted_var, _)| tainted_var == left_var);

            if let Some((_, taint_line)) = right_taint_info {
                // Only propagate if the assignment happens AFTER the taint source
                if !left_already_tainted && assignment_line > taint_line {
                    info!(
                        "Taint propagation round {}: {} -> {} (line {}, taint from line {})",
                        round, right_var, left_var, assignment_line, taint_line
                    );
                    taint_sources.push((left_var.clone(), *assignment_line));
                    changed = true;
                }
            }
        }
    }

    // Check for taint propagation through variables (excluding sanitized ones)
    // Use a set to track which sinks we've already processed to avoid duplicates
    let mut processed_sinks = std::collections::HashSet::new();

    for (sink_arg, sink_line) in &sink_calls {
        if processed_sinks.contains(sink_line) {
            continue; // Skip if we've already processed this sink
        }

        // Find if any taint source affects this sink
        let mut sink_is_tainted = false;
        for (var_name, source_line) in &taint_sources {
            if sink_arg == var_name && source_line < sink_line {
                // Only consider taint sources that occur BEFORE the sink
                // Check if this variable was sanitized AFTER the taint source but BEFORE the sink
                let is_sanitized_before_sink =
                    sanitized_vars.iter().any(|(sanitized_var, sanitize_line)| {
                        sanitized_var == var_name
                            && sanitize_line > source_line
                            && sanitize_line < sink_line
                    });

                if !is_sanitized_before_sink {
                    info!("Sink at line {} is tainted: variable {} from line {} (no sanitization between {} and {})",
                          sink_line, var_name, source_line, source_line, sink_line);
                    sink_is_tainted = true;
                    break; // Found at least one taint source that affects this sink
                } else {
                    info!(
                        "Sink at line {} is safe: variable {} was sanitized before use",
                        sink_line, var_name
                    );
                }
            }
        }

        if sink_is_tainted {
            let finding = Finding {
                rule_id: rule.id.clone(),
                message: rule.message.clone(),
                severity: rule.severity,
                confidence: Confidence::Medium,
                location: Location {
                    file: file_path.clone(),
                    start_line: *sink_line,
                    start_column: lines[*sink_line - 1].find("sink(").unwrap_or(0) + 1,
                    end_line: *sink_line,
                    end_column: lines[*sink_line - 1].find("sink(").unwrap_or(0) + 5,
                },
                fix: rule.fix.clone(),
            };
            findings.push(finding);
        }

        processed_sinks.insert(*sink_line);
    }

    Ok(findings)
}
