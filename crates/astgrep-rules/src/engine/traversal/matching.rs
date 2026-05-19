//! Pattern matching utilities
//!
//! This module provides functions for finding pattern matches in source code.

use astgrep_core::{Confidence, Finding, Location, Result, Severity};
use regex::Regex;
use std::path::PathBuf;

/// Find pattern matches in source code
pub fn find_pattern_matches(
    pattern: &str,
    source: &str,
    _language: astgrep_core::Language,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Simple pattern matching using regex
    // This is a simplified implementation - in a real implementation,
    // you would use the AST-based matcher
    let escaped_pattern = regex::escape(pattern);
    let pattern_regex = Regex::new(&escaped_pattern).ok();

    if let Some(regex) = pattern_regex {
        for mat in regex.find_iter(source) {
            let start = mat.start();
            let end = mat.end();

            // Calculate line and column
            let (start_line, start_col) = calculate_line_col(source, start);
            let (end_line, end_col) = calculate_line_col(source, end);

            let finding = Finding::new(
                "pattern_match".to_string(),
                "Pattern match".to_string(),
                Severity::Warning,
                Confidence::Medium,
                Location::new(PathBuf::from("."), start_line, start_col, end_line, end_col),
            );

            findings.push(finding);
        }
    }

    Ok(findings)
}

/// Find pattern spans in source code
pub(crate) fn find_pattern_spans_in_source(
    pattern: &str,
    source: &str,
    _language: astgrep_core::Language,
    sql_stmt_boundary: bool,
) -> Vec<(usize, usize)> {
    use regex::Regex;

    let mut spans = Vec::new();

    // Escape special regex characters but keep the pattern as literal
    let escaped = regex::escape(pattern);

    // Try to compile as regex
    match Regex::new(&escaped) {
        Ok(re) => {
            if sql_stmt_boundary {
                // For SQL, split by statements and match within each
                for stmt in source.split(';') {
                    for mat in re.find_iter(stmt) {
                        spans.push((mat.start(), mat.end()));
                    }
                }
            } else {
                // Normal matching
                for mat in re.find_iter(source) {
                    spans.push((mat.start(), mat.end()));
                }
            }
        }
        Err(_) => {
            // If regex fails, do simple string search
            let mut start = 0;
            while let Some(pos) = source[start..].find(pattern) {
                let match_start = start + pos;
                let match_end = match_start + pattern.len();
                spans.push((match_start, match_end));
                start = match_end;
            }
        }
    }

    spans
}

/// Calculate line and column from byte offset
fn calculate_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, c) in source.chars().enumerate() {
        if i >= offset {
            break;
        }

        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}
