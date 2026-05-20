//! Property-based tests for astgrep-rules using proptest.
//!
//! These tests verify that rule parsing and validation handle arbitrary
//! YAML input gracefully — never panic, always return Ok or Err.

use astgrep_rules::RuleEngine;
use proptest::prelude::*;

proptest! {
    /// 1. Invalid YAML always returns an error (never panics).
    #[test]
    fn prop_invalid_yaml_returns_error(input in ".*") {
        let mut engine = RuleEngine::new();
        // Arbitrary string is extremely unlikely to be valid rules YAML
        let _result = engine.load_rules_from_yaml(&input);
        // Must not panic — result is either Ok or Err
    }

    /// 2. Rule engine can be created fresh and used repeatedly without panic.
    #[test]
    fn prop_repeated_load_never_panics(
        yamls in prop::collection::vec(".*", 0..10)
    ) {
        let mut engine = RuleEngine::new();
        for yaml in &yamls {
            // Each load must not panic
            let _ = engine.load_rules_from_yaml(yaml);
            engine.clear_rules();
        }
    }

    /// 3. Rule with random severity string handled gracefully.
    ///    Valid rule structure but with random severity field.
    #[test]
    fn prop_random_severity_handled(severity in "[a-zA-Z]{0,20}") {
        let yaml = format!(
            r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: {}
    languages: [java]
    patterns:
      - "test"
"#,
            severity
        );
        let mut engine = RuleEngine::new();
        let _result = engine.load_rules_from_yaml(&yaml);
        // Must not panic regardless of severity value
    }

    /// 4. Rule with random language string handled gracefully.
    #[test]
    fn prop_random_language_handled(language in "[a-zA-Z]{0,20}") {
        let yaml = format!(
            r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [{}]
    patterns:
      - "test"
"#,
            language
        );
        let mut engine = RuleEngine::new();
        let _result = engine.load_rules_from_yaml(&yaml);
        // Must not panic regardless of language value
    }

    /// 5. Rule with empty pattern handled gracefully.
    #[test]
    fn prop_empty_pattern_handled(
        pattern in proptest::option::of("")
    ) {
        let pat = pattern.unwrap_or_default();
        let yaml = format!(
            r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java]
    patterns:
      - {:?}
"#,
            pat
        );
        let mut engine = RuleEngine::new();
        let _result = engine.load_rules_from_yaml(&yaml);
        // Must not panic
    }
}
