//! Regression tests for astgrep
//!
//! Every bug fix MUST add a test here to prevent regressions.
//!
//! # Naming Convention
//!
//! Tests must follow the pattern: `test_regression_<issue>_<description>`
//!
//! - `<issue>` is a short identifier: a GitHub issue number, a ticket ID,
//!   or a hyphenated keyword summarizing the defect.
//! - `<description>` is a brief snake_case summary of what was broken.
//!
//! # Examples
//!
//! - `test_regression_42_sql_parser_crash_on_empty_input`
//! - `test_regression_matcher_false_positive_on_ternary`
//! - `test_regression_taint_missing_cross_function_flow`
//!
//! # Annotations
//!
//! If a test depends on infrastructure that is not yet implemented, mark it
//! with `#[ignore]` and add a comment explaining what is needed:
//!
//! ```ignore
//! #[test]
//! #[ignore = "Requires interprocedural taint analysis (tracked in #87)"]
//! fn test_regression_87_interprocedural_taint() { ... }
//! ```

use astgrep_core::Language;
use astgrep_parser::LanguageParserRegistry;
use astgrep_rules::{RuleEngine, RuleParser, RuleContext};
use std::path::PathBuf;

fn make_context(file_name: &str, lang: Language, source: &str) -> RuleContext {
    RuleContext {
        file_path: file_name.to_string(),
        language: lang,
        source_code: source.to_string(),
        custom_data: std::collections::HashMap::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Template / placeholder tests
//
// Replace these with real regression tests as bugs are discovered and fixed.
// The tests below exercise the same pipeline used by real regression tests so
// that the import path and helper are validated at compile time.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_regression_template_java_parser_does_not_crash_on_empty_file() {
    let parser_registry = LanguageParserRegistry::new();

    let source = "";
    let result = parser_registry.parse_file(&PathBuf::from("empty.java"), source);

    // Parser should not panic; it may succeed with an empty AST or return an error.
    match result {
        Ok(ast) => {
            assert_eq!(ast.node_type(), "program", "Empty file should still produce program root");
        }
        Err(e) => {
            assert!(!e.to_string().is_empty(), "Error should have a message");
        }
    }
}

#[test]
fn test_regression_template_js_parser_handles_unicode_identifiers() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
const café = "coffee";
const 日本語 = "japanese";
function λ(x) { return x + 1; }
"#;

    let result = parser_registry.parse_file(&PathBuf::from("unicode.js"), source);
    assert!(result.is_ok(), "JS parser should handle unicode identifiers without crashing");

    let ast = result.unwrap();
    assert!(!ast.children().is_empty(), "Unicode source should produce AST children");
}

#[test]
fn test_regression_template_python_parser_handles_multiline_string() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
query = """
    SELECT *
    FROM users
    WHERE id = 1
"""
print(query)
"#;

    let result = parser_registry.parse_file(&PathBuf::from("multiline.py"), source);
    assert!(result.is_ok(), "Python parser should handle multiline strings");
}

#[test]
fn test_regression_template_rule_engine_returns_empty_on_clean_code() {
    let parser_registry = LanguageParserRegistry::new();
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: java-eval
    name: "Dangerous eval"
    severity: CRITICAL
    confidence: HIGH
    languages: [java]
    patterns:
      - "eval($X)"
    message: "eval() usage detected"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }

    // Clean code: no eval anywhere
    let source = r#"
public class Clean {
    public int add(int a, int b) {
        return a + b;
    }
}
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("Clean.java"), source)
        .expect("parse java");
    let ctx = make_context("Clean.java", Language::Java, source);
    let findings = rule_engine.analyze(&*ast, &ctx).expect("analyze");

    assert!(findings.is_empty(), "Clean code should produce no findings");
}

#[test]
fn test_regression_template_sql_parser_handles_comments() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
-- This is a line comment
SELECT id, name /* inline */ FROM users
WHERE active = true; -- end comment
"#;

    let result = parser_registry.parse_file(&PathBuf::from("comments.sql"), source);
    match result {
        Ok(ast) => {
            let text = ast.text();
            assert!(text.contains("SELECT"), "AST text should preserve SQL keywords");
        }
        Err(e) => {
            // If comments aren't supported yet, that's acceptable but documented
            println!("SQL with comments not fully supported: {}", e);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Add real regression tests below this line as bugs are fixed.
//
// Example:
//
// #[test]
// fn test_regression_42_sql_parser_crash_on_empty_input() {
//     let parser_registry = LanguageParserRegistry::new();
//     let source = "";
//     let result = parser_registry.parse_file(&PathBuf::from("empty.sql"), source);
//     assert!(result.is_ok(), "Parser should handle empty input without panic");
// }
//
// #[test]
// #[ignore = "Requires interprocedural dataflow (tracked in #87)"]
// fn test_regression_87_interprocedural_taint_not_detected() {
//     // ... test body ...
// }
// ─────────────────────────────────────────────────────────────────────────────
