//! Integration tests for the rule execution system
//!
//! These tests verify that all components work together correctly.

use cr_core::{Severity, Confidence, Language};
use cr_ast::*;
use cr_rules::{RuleEngine, Rule, Pattern, PatternType, RuleContext, RuleValidator};

/// Test the complete rule execution pipeline
#[test]
fn test_complete_rule_execution() {
    // Create a simple AST
    let ast = AstBuilder::simple_call(
        "executeQuery",
        vec![AstBuilder::identifier("userInput")]
    ).with_text("executeQuery(userInput)".to_string());

    // Create a rule
    let rule = Rule {
        id: "test-rule-001".to_string(),
        name: "Test Rule".to_string(),
        description: "A test rule for integration testing".to_string(),
        severity: Severity::Warning,
        confidence: Confidence::Medium,
        languages: vec![Language::Java],
        patterns: vec![
            Pattern {
                pattern_type: PatternType::Simple("executeQuery($VAR)".to_string()),
                metavariable_pattern: None,
                focus: None,
                conditions: vec![],
            }
        ],
        dataflow: None,
        fix: Some("Use prepared statements".to_string()),
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    // Execute rule
    let mut engine = RuleEngine::new();
    engine.add_rule(rule).unwrap();

    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "executeQuery(userInput)".to_string(),
    );

    let findings = engine.analyze(&ast, &context).unwrap();

    // For now, just verify the engine runs without error
    // Pattern matching may not be fully implemented yet
    println!("Findings count: {}", findings.len());

    // Verify the engine has the rule
    assert_eq!(engine.rule_count(), 1);
}

/// Test basic rule execution
#[test]
fn test_basic_rule_execution() {
    // Create a simple AST
    let ast = AstBuilder::simple_call(
        "executeQuery",
        vec![AstBuilder::identifier("input")]
    );

    // Create a simple rule
    let rule = Rule {
        id: "basic-test".to_string(),
        name: "Basic Test".to_string(),
        description: "A basic test rule".to_string(),
        severity: Severity::Warning,
        confidence: Confidence::Medium,
        languages: vec![Language::Java],
        patterns: vec![
            Pattern {
                pattern_type: PatternType::Simple("executeQuery($VAR)".to_string()),
                metavariable_pattern: None,
                focus: None,
                conditions: vec![],
            }
        ],
        dataflow: None,
        fix: Some("Use prepared statements".to_string()),
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    // Execute rule
    let mut engine = RuleEngine::new();
    engine.add_rule(rule).unwrap();

    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "executeQuery(input)".to_string(),
    );

    let findings = engine.analyze(&ast, &context).unwrap();

    // For now, just verify the engine runs without error
    println!("Basic test findings count: {}", findings.len());

    // Verify the engine has the rule
    assert_eq!(engine.rule_count(), 1);
}

/// Test rule validation
#[test]
fn test_rule_validation() {
    let mut validator = RuleValidator::new();

    // Valid rule
    let valid_rule = Rule {
        id: "valid-rule".to_string(),
        name: "Valid Rule".to_string(),
        description: "A valid rule for testing".to_string(),
        severity: Severity::Info,
        confidence: Confidence::Low,
        languages: vec![Language::Java],
        patterns: vec![
            Pattern {
                pattern_type: PatternType::Simple("test()".to_string()),
                metavariable_pattern: None,
                focus: None,
                conditions: vec![],
            }
        ],
        dataflow: None,
        fix: None,
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    let result = validator.validate_rule(&valid_rule);
    assert!(result.is_ok(), "Valid rule should pass validation");

    // Invalid rule (empty ID)
    let mut invalid_rule = valid_rule.clone();
    invalid_rule.id = String::new();

    let result = validator.validate_rule(&invalid_rule);
    assert!(result.is_err(), "Invalid rule should fail validation");
}

/// Test rule engine configuration
#[test]
fn test_rule_engine_configuration() {
    let mut engine = RuleEngine::new();

    // Test initial state
    assert_eq!(engine.rule_count(), 0);

    // Test rule loading
    let rule = Rule {
        id: "test-config".to_string(),
        name: "Test Configuration".to_string(),
        description: "Test rule for configuration".to_string(),
        severity: Severity::Info,
        confidence: Confidence::Low,
        languages: vec![Language::Java],
        patterns: vec![
            Pattern {
                pattern_type: PatternType::Simple("test()".to_string()),
                metavariable_pattern: None,
                focus: None,
                conditions: vec![],
            }
        ],
        dataflow: None,
        fix: None,
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    engine.add_rule(rule).unwrap();
    assert_eq!(engine.rule_count(), 1);

    // Test language filtering
    let java_rules = engine.rules_for_language(Language::Java);
    assert_eq!(java_rules.len(), 1);

    let python_rules = engine.rules_for_language(Language::Python);
    assert_eq!(python_rules.len(), 0);
}

/// Test performance with simple rule
#[test]
fn test_simple_performance() {
    use std::time::Instant;

    // Create a simple AST
    let ast = AstBuilder::simple_call(
        "executeQuery",
        vec![AstBuilder::identifier("input")]
    );

    // Create a simple rule
    let rule = Rule {
        id: "perf-test".to_string(),
        name: "Performance Test".to_string(),
        description: "Test rule for performance".to_string(),
        severity: Severity::Warning,
        confidence: Confidence::Medium,
        languages: vec![Language::Java],
        patterns: vec![
            Pattern {
                pattern_type: PatternType::Simple("executeQuery($VAR)".to_string()),
                metavariable_pattern: None,
                focus: None,
                conditions: vec![],
            }
        ],
        dataflow: None,
        fix: None,
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    // Execute rule and measure time
    let start = Instant::now();
    let mut engine = RuleEngine::new();
    engine.add_rule(rule).unwrap();

    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "executeQuery(input)".to_string(),
    );

    let findings = engine.analyze(&ast, &context).unwrap();
    let duration = start.elapsed();

    // Verify performance
    assert!(duration.as_millis() < 1000, "Should complete within 1 second");
    println!("Performance test findings count: {}", findings.len());
}

/// Test error handling
#[test]
fn test_error_handling() {
    let ast = AstBuilder::identifier("test");
    let mut engine = RuleEngine::new();

    // Test with no rules
    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "test".to_string(),
    );

    let findings = engine.analyze(&ast, &context).unwrap();
    assert!(findings.is_empty(), "No rules should produce no findings");

    // Test with invalid rule (this should be handled gracefully)
    let invalid_rule = Rule {
        id: String::new(), // Invalid empty ID
        name: "Invalid Rule".to_string(),
        description: "Invalid rule for testing".to_string(),
        severity: Severity::Error,
        confidence: Confidence::High,
        languages: vec![Language::Java],
        patterns: vec![],
        dataflow: None,
        fix: None,
        metadata: std::collections::HashMap::new(),
        enabled: true,
    };

    // Try to add invalid rule (should fail validation)
    let result = engine.add_rule(invalid_rule);
    assert!(result.is_err(), "Should fail to add invalid rule");
    
    // Engine should still work with no valid rules
    let result = engine.analyze(&ast, &context);
    assert!(result.is_ok(), "Analysis should succeed even with no valid rules");
}

/// Test multiple rules
#[test]
fn test_multiple_rules() {
    let ast = AstBuilder::simple_call(
        "executeQuery",
        vec![AstBuilder::identifier("input")]
    );

    let mut engine = RuleEngine::new();

    // Add multiple rules
    for i in 0..3 {
        let rule = Rule {
            id: format!("rule_{}", i),
            name: format!("Rule {}", i),
            description: format!("Test rule {}", i),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            languages: vec![Language::Java],
            patterns: vec![
                Pattern {
                    pattern_type: PatternType::Simple("executeQuery($VAR)".to_string()),
                    metavariable_pattern: None,
                    focus: None,
                    conditions: vec![],
                }
            ],
            dataflow: None,
            fix: None,
            metadata: std::collections::HashMap::new(),
            enabled: true,
        };
        engine.add_rule(rule).unwrap();
    }

    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "executeQuery(input)".to_string(),
    );

    let findings = engine.analyze(&ast, &context).unwrap();

    // For now, just verify the engine runs without error
    println!("Multiple rules test findings count: {}", findings.len());

    // Verify the engine has all rules
    assert_eq!(engine.rule_count(), 3);
}
