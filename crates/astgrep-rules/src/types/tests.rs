use super::*;
use astgrep_core::{Confidence, Language, Severity};

#[test]
fn test_rule_creation() {
    let rule = Rule::new(
        "test-rule".to_string(),
        "Test Rule".to_string(),
        "A test rule".to_string(),
        Severity::Error,
        Confidence::High,
        vec![Language::Java],
    );

    assert_eq!(rule.id, "test-rule");
    assert_eq!(rule.name, "Test Rule");
    assert_eq!(rule.severity, Severity::Error);
    assert_eq!(rule.confidence, Confidence::High);
    assert!(rule.applies_to(Language::Java));
    assert!(!rule.applies_to(Language::Python));
    assert!(rule.enabled);
}

#[test]
fn test_rule_builder_pattern() {
    let rule = Rule::new(
        "sql-injection".to_string(),
        "SQL Injection".to_string(),
        "Detects SQL injection".to_string(),
        Severity::Critical,
        Confidence::High,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("$STMT.execute($QUERY)".to_string()))
    .add_metadata("cwe".to_string(), "CWE-89".to_string())
    .with_fix("Use PreparedStatement".to_string());

    assert_eq!(rule.patterns.len(), 1);
    assert_eq!(
        rule.get_metadata("cwe"),
        Some(&serde_yaml::Value::String("CWE-89".to_string()))
    );
    assert_eq!(rule.fix, Some("Use PreparedStatement".to_string()));
}

#[test]
fn test_pattern_creation() {
    let pattern = Pattern::simple("console.log($MSG)".to_string()).with_focus("$MSG".to_string());

    if let PatternType::Simple(pattern_str) = &pattern.pattern_type {
        assert_eq!(pattern_str, "console.log($MSG)");
    } else {
        panic!("Expected Simple pattern type");
    }
    assert_eq!(pattern.focus, Some(vec!["$MSG".to_string()]));
}

#[test]
fn test_pattern_not_inside() {
    let inner_pattern = Pattern::simple("class $CLASS:".to_string());
    let pattern = Pattern::not_inside(inner_pattern);

    if let PatternType::NotInside(inner) = &pattern.pattern_type {
        if let PatternType::Simple(pattern_str) = &inner.pattern_type {
            assert_eq!(pattern_str, "class $CLASS:");
        } else {
            panic!("Expected Simple inner pattern type");
        }
    } else {
        panic!("Expected NotInside pattern type");
    }
}

#[test]
fn test_pattern_not_regex() {
    let pattern = Pattern::not_regex("test_.*".to_string());

    if let PatternType::NotRegex(regex_str) = &pattern.pattern_type {
        assert_eq!(regex_str, "test_.*");
    } else {
        panic!("Expected NotRegex pattern type");
    }
}

#[test]
fn test_multiple_focus_metavariables() {
    let pattern = Pattern::simple("function $FUNC($PARAM1, $PARAM2) {}".to_string())
        .with_focus_metavariables(vec!["$PARAM1".to_string(), "$PARAM2".to_string()]);

    assert_eq!(
        pattern.focus,
        Some(vec!["$PARAM1".to_string(), "$PARAM2".to_string()])
    );
}

#[test]
fn test_metavariable_pattern() {
    let metavar = MetavariablePattern::new(
        "$QUERY".to_string(),
        vec!["$STR + $INPUT".to_string()],
    )
    .with_regex(r"SELECT.*FROM.*".to_string())
    .with_type_constraint("String".to_string());

    assert_eq!(metavar.metavariable, "$QUERY");
    assert_eq!(metavar.patterns.len(), 1);
    assert!(metavar.regex.is_some());
    assert!(metavar.type_constraint.is_some());
}

#[test]
fn test_dataflow_spec() {
    let dataflow = DataFlowSpec::from_strings(
        vec!["request.getParameter(...)".to_string()],
        vec!["Statement.execute(...)".to_string()],
    )
    .with_sanitizers(vec!["sanitize(...)".to_string()])
    .with_max_depth(10);

    assert_eq!(dataflow.sources.len(), 1);
    assert_eq!(dataflow.sinks.len(), 1);
    assert_eq!(dataflow.sanitizers.len(), 1);
    assert_eq!(dataflow.max_depth, Some(10));
    assert!(dataflow.must_flow);
}

#[test]
fn test_rule_context() {
    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "public class Test {}".to_string(),
    )
    .add_data("project".to_string(), "my-project".to_string());

    assert_eq!(context.file_path, "test.java");
    assert_eq!(context.language, Language::Java);
    assert_eq!(context.get_data("project"), Some(&"my-project".to_string()));
}

#[test]
fn test_rule_result() {
    let success_result = RuleResult::success("test-rule".to_string(), vec![], 100);

    assert!(success_result.is_success());
    assert_eq!(success_result.finding_count(), 0);
    assert_eq!(success_result.execution_time_ms, 100);

    let error_result = RuleResult::error("test-rule".to_string(), "Parse error".to_string(), 50);

    assert!(!error_result.is_success());
    assert_eq!(error_result.error, Some("Parse error".to_string()));
}
