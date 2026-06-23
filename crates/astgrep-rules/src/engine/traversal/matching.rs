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

/// Detect whether a pattern looks like an already-valid regex rather than plain text.
///
/// Patterns containing regex-specific constructs (shorthand classes like `\s`, `\d`,
/// character classes `[...]`, anchors `^`/`$`, non-capturing groups, lookaheads)
/// are treated as raw regex and will NOT be escaped.
fn looks_like_raw_regex(pattern: &str) -> bool {
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();

    // Check for regex shorthand classes: \s, \d, \w, \b, \S, \D, \W, \B, \A, \z, \n, \t, \r, etc.
    for i in 0..len.saturating_sub(1) {
        if chars[i] == '\\' && chars[i + 1].is_ascii_alphabetic() {
            return true;
        }
    }

    // Check for character classes with regex-typical content: ranges like [a-z], [0-9], or shorthand [\\d]
    if let Some(open) = pattern.find('[') {
        if let Some(close) = pattern[open..].find(']') {
            let inner = &pattern[open + 1..open + close];
            if inner.contains('-')
                || inner.contains("\\d")
                || inner.contains("\\w")
                || inner.contains("\\s")
            {
                return true;
            }
        }
    }

    // Check for anchors at typical positions
    if pattern.starts_with('^') || pattern.ends_with('$') {
        return true;
    }

    // Check for non-capturing groups / lookahead / lookbehind
    if pattern.contains("(?:")
        || pattern.contains("(?=")
        || pattern.contains("(?!")
        || pattern.contains("(?<=")
        || pattern.contains("(?<!")
    {
        return true;
    }

    false
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

    // Strip outer quotes if present (pattern may be wrapped in "" or '')
    let inner_pattern = if pattern.len() >= 2 {
        let chars: Vec<char> = pattern.chars().collect();
        if chars[0] == '"' && chars[chars.len() - 1] == '"' {
            &pattern[1..pattern.len() - 1]
        } else if chars[0] == '\'' && chars[chars.len() - 1] == '\'' {
            &pattern[1..pattern.len() - 1]
        } else {
            pattern
        }
    } else {
        pattern
    };

    let regex_str = if inner_pattern.starts_with("=~/")
        && inner_pattern.ends_with('/')
        && inner_pattern.len() > 4
    {
        // Semgrep =~/regex/ syntax: match regex against string content
        inner_pattern[3..inner_pattern.len() - 1].to_string()
    } else if looks_like_raw_regex(inner_pattern) {
        // Pattern is already a regex — pass through without escaping
        inner_pattern.to_string()
    } else {
        semgrep_pattern_to_regex(inner_pattern)
    };
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

pub(crate) fn semgrep_pattern_to_regex(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len() * 2);
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Handle <... ...> deep expression matching
        if i + 4 < len
            && chars[i] == '<'
            && chars[i + 1] == '.'
            && chars[i + 2] == '.'
            && chars[i + 3] == '.'
        {
            result.push_str(".*?");
            i += 4;
            if i < len && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }
        if i + 3 < len
            && chars[i] == '.'
            && chars[i + 1] == '.'
            && chars[i + 2] == '.'
            && chars[i + 3] == '>'
        {
            result.push_str("");
            i += 4;
            continue;
        }

        // Handle $...NAME (named ellipsis metavariable)
        if chars[i] == '$'
            && i + 3 < len
            && chars[i + 1] == '.'
            && chars[i + 2] == '.'
            && chars[i + 3] == '.'
        {
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
            result.push_str(
                "(?:[a-zA-Z_]\\w*(?:\\.[a-zA-Z_]\\w*)*|\\d+(?:\\.\\d+)?|\"[^\"]*\"|'[^']*')",
            );
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
            result.push_str(&format!(
                "\\$\\{{{}\\}}",
                inner_regex
                    .replace("\\", "\\\\")
                    .replace("(", "\\(")
                    .replace(")", "\\)")
            ));
            continue;
        }

        // Handle ... (ellipsis wildcard)
        if i + 2 < len && chars[i] == '.' && chars[i + 1] == '.' && chars[i + 2] == '.' {
            result.push_str("[\\s\\S]*?");
            i += 3;
            continue;
        }

        // Handle newlines in pattern → match actual newlines with optional surrounding whitespace
        if chars[i] == '\n' {
            result.push_str("\\s*\\n\\s*");
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
mod tests_multiline_newline {
    use super::*;

    #[test]
    fn test_newline_should_match_actual_newline() {
        let regex_str = semgrep_pattern_to_regex("foo\nbar");
        let re = regex::Regex::new(&regex_str).unwrap();
        assert!(
            re.is_match("foo\nbar"),
            "pattern with \\n should match actual newline"
        );
    }

    #[test]
    fn test_newline_should_not_match_same_line_spaces() {
        let regex_str = semgrep_pattern_to_regex("foo\nbar");
        let re = regex::Regex::new(&regex_str).unwrap();
        // This is the FP bug: \n converted to \s* matches spaces on the same line
        assert!(
            !re.is_match("foo    bar"),
            "pattern with \\n should NOT match same-line spaces"
        );
    }

    #[test]
    fn test_newline_should_match_with_surrounding_whitespace() {
        let regex_str = semgrep_pattern_to_regex("foo\nbar");
        let re = regex::Regex::new(&regex_str).unwrap();
        // \n should also match \r\n and surrounding whitespace
        assert!(
            re.is_match("foo \n bar"),
            "pattern with \\n should match newline with surrounding spaces"
        );
        assert!(
            re.is_match("foo\r\nbar"),
            "pattern with \\n should match CRLF"
        );
    }
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

    #[test]
    fn test_raw_regex_shorthand_class() {
        let source = "foo bar\tbaz";
        let spans = find_pattern_spans_in_source("\\s+", source, Language::Java, false);
        assert_eq!(spans.len(), 2);
        assert_eq!(&source[spans[0].0..spans[0].1], " ");
        assert_eq!(&source[spans[1].0..spans[1].1], "\t");
    }

    #[test]
    fn test_raw_regex_character_class() {
        let source = "abc123def";
        let spans = find_pattern_spans_in_source("[0-9]+", source, Language::Java, false);
        assert_eq!(spans.len(), 1);
        assert_eq!(&source[spans[0].0..spans[0].1], "123");
    }

    #[test]
    fn test_raw_regex_anchor() {
        let source = "hello world hello";
        let spans = find_pattern_spans_in_source("^hello", source, Language::Java, false);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0], (0, 5));
    }

    #[test]
    fn test_plain_text_escaping_still_works() {
        let source = "arr[0] = arr[1]";
        let spans = find_pattern_spans_in_source("arr[0]", source, Language::Java, false);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0], (0, 6));
    }

    #[test]
    fn test_raw_regex_alternation() {
        let source = "foo bar baz";
        let spans = find_pattern_spans_in_source("(?:foo|baz)", source, Language::Java, false);
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn test_looks_like_raw_regex() {
        assert!(looks_like_raw_regex("\\s+"));
        assert!(looks_like_raw_regex("\\d{1,3}"));
        assert!(looks_like_raw_regex("[a-z]+"));
        assert!(looks_like_raw_regex("^hello"));
        assert!(looks_like_raw_regex("world$"));
        assert!(looks_like_raw_regex("(?:foo|bar)"));
        assert!(looks_like_raw_regex("(?=error)"));
        assert!(looks_like_raw_regex("(?!warning)"));
        assert!(!looks_like_raw_regex("hello"));
        assert!(!looks_like_raw_regex("arr[0]"));
        assert!(!looks_like_raw_regex("foo.bar"));
    }
}
