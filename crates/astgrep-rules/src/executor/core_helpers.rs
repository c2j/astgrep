//! Helper functions for AdvancedRuleExecutor
//!
//! This module contains utility and helper functions extracted from core.rs
//! to improve code organization and maintainability.

use crate::executor::types::TaintMatch;
use crate::types::*;
use astgrep_core::{AstNode, Result, SemgrepMatchResult};
use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// Type Inference Helpers
// ============================================================================

/// Infer the type of a value from its literal representation
pub fn infer_type_from_value(value: &str) -> Option<String> {
    let trimmed = value.trim();

    // String literal: "..." or '...'
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return Some("String".to_string());
    }

    // Integer literal: digits only (possibly with negative sign)
    if trimmed.parse::<i64>().is_ok() {
        return Some("int".to_string());
    }

    // Floating point literal: contains dot and parses as float
    if trimmed.contains('.') && trimmed.parse::<f64>().is_ok() {
        return Some("float".to_string());
    }

    // Boolean literal: true or false
    if trimmed == "true" || trimmed == "false" {
        return Some("boolean".to_string());
    }

    // Null literal
    if trimmed == "null" {
        return Some("null".to_string());
    }

    // Not a recognized literal, probably a variable name
    None
}

/// Check if a value matches a type pattern
pub fn value_matches_type(value: &str, type_name: &str) -> bool {
    match type_name {
        "string" => true, // All values are strings at this level
        "number" => value.parse::<f64>().is_ok(),
        "integer" => value.parse::<i64>().is_ok(),
        "boolean" => value == "true" || value == "false",
        "null" => value == "null" || value == "None" || value == "nil",
        _ => false, // Unknown type
    }
}

// ============================================================================
// Entropy Calculation
// ============================================================================

/// Calculate Shannon entropy of a string
pub fn calculate_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut char_counts = HashMap::new();
    for c in s.chars() {
        *char_counts.entry(c).or_insert(0) += 1;
    }

    let len = s.len() as f64;
    let mut entropy = 0.0;

    for count in char_counts.values() {
        let p = *count as f64 / len;
        entropy -= p * p.log2();
    }

    entropy
}

/// Check if value matches charset
pub fn matches_charset(value: &str, charset: &str) -> bool {
    match charset {
        "alphanumeric" => value.chars().all(|c| c.is_alphanumeric()),
        "alphabetic" => value.chars().all(|c| c.is_alphabetic()),
        "numeric" => value.chars().all(|c| c.is_numeric()),
        "ascii" => value.is_ascii(),
        _ => true, // Unknown charset, assume match
    }
}

// ============================================================================
// Import Map Helpers
// ============================================================================

/// Build a map of imported simple names to their fully qualified names
pub fn build_import_map(full_source: &str) -> HashMap<String, String> {
    let mut import_map = HashMap::new();

    // Parse import statements like "import org.foo.Foo;" or "import org.foo.*;"
    let import_pattern = Regex::new(r"import\s+([\w.]+)(?:\.\*)?;").unwrap();

    for captures in import_pattern.captures_iter(full_source) {
        if let Some(import_match) = captures.get(1) {
            let import_path = import_match.as_str();

            // Extract the simple name (last part after the last dot)
            if let Some(last_dot) = import_path.rfind('.') {
                let simple_name = &import_path[last_dot + 1..];
                let fully_qualified = import_path.to_string();
                import_map.insert(simple_name.to_string(), fully_qualified);
            } else {
                // No dot, the whole import is the simple name
                import_map.insert(import_path.to_string(), import_path.to_string());
            }
        }
    }

    import_map
}

/// Resolve a simple type name to its fully qualified name using import map
pub fn resolve_type_with_imports(
    simple_type: &str,
    import_map: &HashMap<String, String>,
) -> Option<String> {
    // First check if this simple type is in the import map
    if let Some(fully_qualified) = import_map.get(simple_type) {
        return Some(fully_qualified.clone());
    }

    // If not found in imports, return the simple type as-is
    // (it might be a primitive type or in the same package)
    Some(simple_type.to_string())
}

// ============================================================================
// Name Extraction Helpers
// ============================================================================

/// Extract type information for a variable from the match context
pub fn extract_type_info(
    match_result: &SemgrepMatchResult,
    var_name: &str,
    full_source: &str,
    import_map: &HashMap<String, String>,
) -> Option<String> {
    // Pattern 1: Method parameter declarations like "String varName" or "Type varName"
    // Matches: "Type varName" followed by comma, closing paren, or space
    let param_pattern = format!(r"(\w+)\s+{}\s*[,)]", regex::escape(var_name));
    if let Ok(regex) = Regex::new(&param_pattern) {
        if let Some(captures) = regex.captures(full_source) {
            if let Some(type_match) = captures.get(1) {
                let simple_type = type_match.as_str().to_string();
                return resolve_type_with_imports(&simple_type, import_map);
            }
        }
    }

    // Pattern 2: Variable declarations like "Type varName = ...;" or "Type varName;"
    let var_pattern = format!(r"(\w+)\s+{}\s*=[^;]*;", regex::escape(var_name));
    if let Ok(regex) = Regex::new(&var_pattern) {
        if let Some(captures) = regex.captures(full_source) {
            if let Some(type_match) = captures.get(1) {
                let simple_type = type_match.as_str().to_string();
                return resolve_type_with_imports(&simple_type, import_map);
            }
        }
    }

    // Pattern 3: Field declarations like "private Type varName = ...;" or "private Type varName;"
    let field_pattern = format!(
        r"(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(\w+)\s+{}\s*=[^;]*;",
        regex::escape(var_name)
    );
    if let Ok(regex) = Regex::new(&field_pattern) {
        if let Some(captures) = regex.captures(full_source) {
            if let Some(type_match) = captures.get(1) {
                let simple_type = type_match.as_str().to_string();
                return resolve_type_with_imports(&simple_type, import_map);
            }
        }
    }

    None
}

/// Find method name by line number in source
pub fn find_method_name_by_line(source_text: &str, line_num: usize) -> Option<String> {
    let lines: Vec<&str> = source_text.lines().collect();
    if line_num == 0 || line_num > lines.len() {
        return None;
    }

    // Search backwards from the current line to find method declaration
    for i in (0..line_num).rev() {
        let line = lines[i];

        // Look for method declaration patterns
        // Pattern 1: "public/protected/private ... methodName("
        let method_pattern = Regex::new(
            r"(?:public|protected|private)?\s*(?:static\s+)?(?:\w+(?:<[^>]+>)?)\s+(\w+)\s*\(",
        )
        .unwrap();
        if let Some(captures) = method_pattern.captures(line) {
            if let Some(method_name) = captures.get(1) {
                let name = method_name.as_str();
                // Filter out keywords
                if ![
                    "if",
                    "for",
                    "while",
                    "switch",
                    "catch",
                    "class",
                    "interface",
                ]
                .contains(&name)
                {
                    return Some(name.to_string());
                }
            }
        }

        // Don't go too far back (max 50 lines)
        if line_num - i > 50 {
            break;
        }
    }

    None
}

/// Extract method body from source by method name
pub fn extract_method_body(source_text: &str, method_name: &str) -> Option<String> {
    // Find the method declaration
    let pattern = format!(
        r"(?:public|protected|private)?\s*(?:static\s+)?(?:\w+(?:<[^>]+>)?)\s+{}\s*\([^)]*\)\s*(?:throws\s+[\w,\s]+)?\s*\{{",
        regex::escape(method_name)
    );
    let regex = Regex::new(&pattern).ok()?;

    if let Some(m) = regex.find(source_text) {
        let start = m.end();

        // Find matching closing brace
        let mut brace_count = 1;
        let mut end = start;

        for (i, c) in source_text[start..].char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end = start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if end > start {
            return Some(source_text[start..end].to_string());
        }
    }

    None
}

// ============================================================================
// Pattern Simplification
// ============================================================================

/// Simplify a fully qualified pattern by extracting just the class and method
/// e.g., "org.apache.ibatis.session.SqlSessionFactory.openSession(...)" -> "SqlSessionFactory.openSession(...)"
pub fn simplify_fully_qualified_pattern(pattern: &str) -> Option<String> {
    // Find the last two dots before '('
    let paren_pos = pattern.find('(')?;
    let before_paren = &pattern[..paren_pos];

    // Find last two dots
    let mut last_dot = None;
    let mut second_last_dot = None;

    for (i, c) in before_paren.char_indices() {
        if c == '.' {
            second_last_dot = last_dot;
            last_dot = Some(i);
        }
    }

    if let (Some(second), Some(last)) = (second_last_dot, last_dot) {
        let simplified = format!("{}{}", &pattern[second + 1..], &pattern[paren_pos..]);
        Some(simplified)
    } else {
        None
    }
}

/// Extract the last call arguments from nested method calls
/// e.g., "sink1("Abc", w)" -> returns "Abc", w"
pub fn extract_last_call_args(text: &str) -> Option<&str> {
    // Find the last '(' and its matching ')'
    let last_paren = text.rfind('(')?;
    let rest = &text[last_paren + 1..];

    // Find matching closing paren
    let mut paren_count = 1;
    for (i, c) in rest.char_indices() {
        match c {
            '(' => paren_count += 1,
            ')' => {
                paren_count -= 1;
                if paren_count == 0 {
                    return Some(&rest[..i]);
                }
            }
            _ => {}
        }
    }

    None
}

/// Parse an ellipsis pattern like "x(). ... .z()" or "$X(). ... .z()"
/// Returns (start_method, end_method) if successful
pub fn parse_ellipsis_pattern(pattern_str: &str) -> Option<(String, String)> {
    // Remove whitespace for easier parsing
    let pattern = pattern_str.replace(" ", "");

    // Pattern format: something(). ... .something()
    // Find "()" at the start
    let start_paren = pattern.find("()")?;
    let start_method = if start_paren > 0 {
        pattern[..start_paren].to_string()
    } else {
        return None;
    };

    // Find "...()" sequence
    let ellipsis_idx = pattern.find("...")?;
    let after_ellipsis = &pattern[ellipsis_idx + 3..];

    // Skip one dot, then find the final "()" for end method
    if !after_ellipsis.starts_with('.') {
        return None;
    }

    let remaining = &after_ellipsis[1..];
    let end_paren = remaining.find("()")?;
    // Remove any leading dots from end_method
    let end_method = remaining[..end_paren].trim_start_matches('.').to_string();

    Some((start_method, end_method))
}
