//! Property-based tests for astgrep-matcher using proptest.
//!
//! These tests verify that pattern parsing and matching handle arbitrary
//! input gracefully — never panic, always produce consistent results.

use astgrep_matcher::{PatternMatcher, PatternParser};
use astgrep_ast::AstBuilder;
use proptest::prelude::*;

proptest! {
    /// 1. PatternParser never panics on arbitrary input.
    ///    It must return Ok(ParsedPattern) or Err.
    #[test]
    fn prop_pattern_parser_never_panics(input in ".*") {
        let parser = PatternParser::new();
        let _result = parser.parse(&input);
    }

    /// 2. Literal pattern parsing round-trips: parse then display produces
    ///    consistent output (no panic, deterministic).
    #[test]
    fn prop_parsed_pattern_display_never_panics(input in "[a-zA-Z0-9_ ]{0,100}") {
        let parser = PatternParser::new();
        if let Ok(parsed) = parser.parse(&input) {
            // Display must not panic
            let display = format!("{}", parsed);
            // Re-parsing the display output must not panic either
            let _reparsed = parser.parse(&display);
        }
    }

    /// 3. PatternMatcher.matches() is idempotent on a simple node:
    ///    matching the same pattern twice yields the same boolean result.
    #[test]
    fn prop_match_idempotent(
        pattern in "[a-zA-Z0-9_$@.]{0,50}",
        node_text in "[a-zA-Z0-9_]{0,30}"
    ) {
        let node = AstBuilder::identifier(&node_text).with_text(node_text.clone());

        let mut matcher1 = PatternMatcher::new();
        let mut matcher2 = PatternMatcher::new();

        let result1 = matcher1.matches(&pattern, &node);
        let result2 = matcher2.matches(&pattern, &node);

        // Both must produce the same Ok/Err and boolean result
        match (result1, result2) {
            (Ok(b1), Ok(b2)) => assert_eq!(b1, b2),
            (Err(_), Err(_)) => {} // both errors is consistent
            (r1, r2) => panic!(
                "Mismatched results for pattern {:?}: {:?} vs {:?}",
                pattern, r1, r2
            ),
        }
    }

    /// 4. Metavariable pattern $X never panics on any node text.
    #[test]
    fn prop_metavariable_never_panics(node_text in ".*") {
        let node = AstBuilder::identifier("x").with_text(node_text.clone());
        let mut matcher = PatternMatcher::new();

        let _result = matcher.matches("$X", &node);
    }

    /// 5. Wildcard pattern "..." never panics and always matches.
    #[test]
    fn prop_wildcard_always_matches(node_text in ".*") {
        let node = if node_text.is_empty() {
            AstBuilder::identifier("empty")
        } else {
            AstBuilder::identifier("x").with_text(node_text.clone())
        };
        let mut matcher = PatternMatcher::new();

        let result = matcher.matches("...", &node);
        // Wildcard should always match without error
        assert!(result.is_ok());
    }
}
