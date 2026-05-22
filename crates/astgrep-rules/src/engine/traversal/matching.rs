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

/// Find pattern spans in source code using semgrep-aware regex conversion
pub(crate) fn find_pattern_spans_in_source(
    pattern: &str,
    source: &str,
    _language: astgrep_core::Language,
    sql_stmt_boundary: bool,
) -> Vec<(usize, usize)> {
    use regex::Regex;

    let mut spans = Vec::new();

    let regex_str = semgrep_pattern_to_regex(pattern);
    let is_multiline = pattern.contains('\n');

    let final_regex = if is_multiline {
        format!("(?s){}", regex_str)
    } else {
        regex_str
    };

    match Regex::new(&final_regex) {
        Ok(re) => {
            if sql_stmt_boundary {
                for stmt in source.split(';') {
                    for mat in re.find_iter(stmt) {
                        spans.push((mat.start(), mat.end()));
                    }
                }
            } else {
                for mat in re.find_iter(source) {
                    spans.push((mat.start(), mat.end()));
                }
            }
        }
        Err(_) => {
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

/// Convert a semgrep pattern to a regex string
/// Handles: ... (wildcard), $VAR (metavar), $...ARGS (named ellipsis), <... ...> (deep match)
fn semgrep_pattern_to_regex(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len() * 2);
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Handle <... ...> deep expression matching
        if i + 4 < len && chars[i] == '<' && chars[i + 1] == '.' && chars[i + 2] == '.' && chars[i + 3] == '.' {
            result.push_str(".*?");
            i += 4;
            if i < len && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }
        if i + 3 < len && chars[i] == '.' && chars[i + 1] == '.' && chars[i + 2] == '.' && chars[i + 3] == '>' {
            result.push_str("");
            i += 4;
            continue;
        }

        // Handle $...NAME (named ellipsis metavariable)
        if chars[i] == '$' && i + 3 < len && chars[i + 1] == '.' && chars[i + 2] == '.' && chars[i + 3] == '.' {
            i += 4;
            let mut name = String::new();
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                name.push(chars[i]);
                i += 1;
            }
            let _ = name;
            result.push_str("[\\s\\S]*?");
            continue;
        }

        // Handle $VAR (metavariable)
        if chars[i] == '$' && i + 1 < len && (chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
            i += 1;
            let mut name = String::new();
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                name.push(chars[i]);
                i += 1;
            }
            let _ = name;
            result.push_str("(?:[a-zA-Z_]\\w*(?:\\.[a-zA-Z_]\\w*)*|\\d+(?:\\.\\d+)?|\"[^\"]*\"|'[^']*')");
            continue;
        }

        // Handle ${$VAR} (bash variable substitution in metavar context)
        if chars[i] == '$' && i + 1 < len && chars[i + 1] == '{' {
            i += 2;
            let mut inner = String::new();
            while i < len && chars[i] != '}' {
                inner.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let inner_regex = semgrep_pattern_to_regex(&inner);
            result.push_str(&format!("\\$\\{{{}\\}}", inner_regex.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")));
            continue;
        }

        // Handle ... (ellipsis wildcard)
        if i + 2 < len && chars[i] == '.' && chars[i + 1] == '.' && chars[i + 2] == '.' {
            result.push_str("[\\s\\S]*?");
            i += 3;
            continue;
        }

        // Handle newlines in pattern → match any whitespace/blank lines
        if chars[i] == '\n' {
            result.push_str("\\s*");
            i += 1;
            continue;
        }

        // Escape and push literal character
        let c = chars[i];
        match c {
            '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                result.push('\\');
                result.push(c);
            }
            _ => {
                result.push(c);
            }
        }
        i += 1;
    }

    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::Language;

    #[test]
    fn test_find_pattern_matches_basic() {
        let source = "hello world\nhello universe";
        let findings = find_pattern_matches("hello", source, Language::Java).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].location.start_line, 1);
        assert_eq!(findings[1].location.start_line, 2);
    }

    #[test]
    fn test_find_pattern_matches_no_match() {
        let source = "foo bar baz";
        let findings = find_pattern_matches("qux", source, Language::Python).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn test_find_pattern_matches_multiline() {
        let source = "line1\nline2\nline3";
        let findings = find_pattern_matches("line2", source, Language::JavaScript).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.start_line, 2);
        assert_eq!(findings[0].location.start_column, 1);
    }

    #[test]
    fn test_find_pattern_spans_in_source_basic() {
        let source = "abc def abc";
        let spans = find_pattern_spans_in_source("abc", source, Language::Java, false);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], (0, 3));
        assert_eq!(spans[1], (8, 11));
    }

    #[test]
    fn test_find_pattern_spans_sql_boundary() {
        let source = "SELECT * FROM users; SELECT * FROM orders";
        let spans = find_pattern_spans_in_source("SELECT", source, Language::Sql, true);
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn test_find_pattern_spans_no_match() {
        let source = "foo bar";
        let spans = find_pattern_spans_in_source("baz", source, Language::Bash, false);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_calculate_line_col() {
        let source = "line1\nline2\nline3";
        assert_eq!(calculate_line_col(source, 0), (1, 1));
        assert_eq!(calculate_line_col(source, 6), (2, 1));
        assert_eq!(calculate_line_col(source, 12), (3, 1));
    }
}
