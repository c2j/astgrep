//! Rule type definitions
//! 
//! This module defines the core types used in the rule system.

use astgrep_core::{Confidence, Finding, Language, Severity, MetavariableAnalysis, ComparisonOperator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub languages: Vec<Language>,
    pub patterns: Vec<Pattern>,
    pub dataflow: Option<DataFlowSpec>,
    pub fix: Option<String>,
    pub fix_regex: Option<FixRegex>,
    pub paths: Option<PathsFilter>,
    pub metadata: HashMap<String, String>,
    pub enabled: bool,
}

impl Rule {
    /// Create a new rule
    pub fn new(
        id: String,
        name: String,
        description: String,
        severity: Severity,
        confidence: Confidence,
        languages: Vec<Language>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            severity,
            confidence,
            languages,
            patterns: Vec::new(),
            dataflow: None,
            fix: None,
            fix_regex: None,
            paths: None,
            metadata: HashMap::new(),
            enabled: true,
        }
    }

    /// Check if this rule applies to the given language
    pub fn applies_to(&self, language: Language) -> bool {
        self.enabled && self.languages.contains(&language)
    }

    /// Add a pattern to this rule
    pub fn add_pattern(mut self, pattern: Pattern) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Add metadata to this rule
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set the fix suggestion
    pub fn with_fix(mut self, fix: String) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Set the dataflow specification
    pub fn with_dataflow(mut self, dataflow: DataFlowSpec) -> Self {
        self.dataflow = Some(dataflow);
        self
    }

    /// Enable or disable this rule
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Pattern matching specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: PatternType,
    pub metavariable_pattern: Option<MetavariablePattern>,
    pub conditions: Vec<Condition>,
    pub focus: Option<Vec<String>>, // Support multiple focus metavariables
}

/// Types of patterns supported by semgrep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Simple pattern string
    Simple(String),
    /// Pattern with alternatives (pattern-either)
    Either(Vec<Pattern>),
    /// Pattern that must be inside another pattern (pattern-inside)
    Inside(Box<Pattern>),
    /// Pattern that must not be inside another pattern (pattern-not-inside)
    NotInside(Box<Pattern>),
    /// Pattern that must not match (pattern-not)
    Not(Box<Pattern>),
    /// Pattern with regex matching (pattern-regex)
    Regex(String),
    /// Pattern with regex that must not match (pattern-not-regex)
    NotRegex(String),
    /// All patterns must match (pattern-all)
    All(Vec<Pattern>),
    /// Any pattern must match (pattern-any)
    Any(Vec<Pattern>),
}

impl Pattern {
    /// Create a simple pattern
    pub fn simple(pattern: String) -> Self {
        Self {
            pattern_type: PatternType::Simple(pattern),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern with metavariable constraints
    pub fn with_metavariable(pattern: String, metavar_pattern: MetavariablePattern) -> Self {
        Self {
            pattern_type: PatternType::Simple(pattern),
            metavariable_pattern: Some(metavar_pattern),
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-either
    pub fn either(patterns: Vec<Pattern>) -> Self {
        Self {
            pattern_type: PatternType::Either(patterns),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-inside
    pub fn inside(inner_pattern: Pattern) -> Self {
        Self {
            pattern_type: PatternType::Inside(Box::new(inner_pattern)),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-not-inside
    pub fn not_inside(inner_pattern: Pattern) -> Self {
        Self {
            pattern_type: PatternType::NotInside(Box::new(inner_pattern)),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-not
    pub fn not(inner_pattern: Pattern) -> Self {
        Self {
            pattern_type: PatternType::Not(Box::new(inner_pattern)),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-regex
    pub fn regex(regex: String) -> Self {
        Self {
            pattern_type: PatternType::Regex(regex),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-not-regex
    pub fn not_regex(regex: String) -> Self {
        Self {
            pattern_type: PatternType::NotRegex(regex),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-all
    pub fn all(patterns: Vec<Pattern>) -> Self {
        Self {
            pattern_type: PatternType::All(patterns),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-any
    pub fn any(patterns: Vec<Pattern>) -> Self {
        Self {
            pattern_type: PatternType::Any(patterns),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Get the pattern string for simple patterns
    pub fn get_pattern_string(&self) -> Option<&String> {
        match &self.pattern_type {
            PatternType::Simple(pattern) => Some(pattern),
            PatternType::Regex(pattern) => Some(pattern),
            _ => None,
        }
    }

    /// Add a condition to this pattern
    pub fn add_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the focus for this pattern (single metavariable)
    pub fn with_focus(mut self, focus: String) -> Self {
        self.focus = Some(vec![focus]);
        self
    }

    /// Set multiple focus metavariables for this pattern
    pub fn with_focus_metavariables(mut self, focus_vars: Vec<String>) -> Self {
        self.focus = Some(focus_vars);
        self
    }
}

/// Fix regex specification (Semgrep compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixRegex {
    pub regex: String,
    pub replacement: String,
}

/// Paths filter specification (Semgrep compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsFilter {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
}

/// Metavariable pattern specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariablePattern {
    pub metavariable: String,
    pub patterns: Vec<String>,
    pub regex: Option<String>,
    pub type_constraint: Option<String>,
    pub name_constraint: Option<String>, // metavariable-name support
    pub analysis: Option<MetavariableAnalysis>, // metavariable-analysis support
}

impl MetavariablePattern {
    /// Create a new metavariable pattern
    pub fn new(metavariable: String, patterns: Vec<String>) -> Self {
        Self {
            metavariable,
            patterns,
            regex: None,
            type_constraint: None,
            name_constraint: None,
            analysis: None,
        }
    }

    /// Create a new metavariable pattern with patterns (alias for new)
    pub fn with_patterns(metavariable: String, patterns: Vec<String>) -> Self {
        Self::new(metavariable, patterns)
    }

    /// Add a regex constraint
    pub fn with_regex(mut self, regex: String) -> Self {
        self.regex = Some(regex);
        self
    }

    /// Add a type constraint
    pub fn with_type_constraint(mut self, type_constraint: String) -> Self {
        self.type_constraint = Some(type_constraint);
        self
    }
}

/// Condition for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    MetavariableRegex(MetavariableRegex),
    MetavariableComparison(MetavariableComparison),
    MetavariableName(MetavariableName),
    MetavariableAnalysis(MetavariableAnalysisCondition),
    NodeType(String),
    NodeAttribute(String, String),
    Custom(String),
}

/// Metavariable regex constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableRegex {
    pub metavariable: String,
    pub regex: String,
}

impl MetavariableRegex {
    pub fn new(metavariable: String, regex: String) -> Self {
        Self { metavariable, regex }
    }
}

/// Metavariable comparison constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableComparison {
    pub metavariable: String,
    pub operator: ComparisonOperator,
    pub value: String,
}

impl MetavariableComparison {
    pub fn new(metavariable: String, operator: ComparisonOperator, value: String) -> Self {
        Self { metavariable, operator, value }
    }
}

/// Metavariable name constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableName {
    pub metavariable: String,
    pub name_pattern: String,
}

impl MetavariableName {
    pub fn new(metavariable: String, name_pattern: String) -> Self {
        Self { metavariable, name_pattern }
    }
}

/// Metavariable analysis condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableAnalysisCondition {
    pub metavariable: String,
    pub analysis: MetavariableAnalysis,
}

impl MetavariableAnalysisCondition {
    pub fn new(metavariable: String, analysis: MetavariableAnalysis) -> Self {
        Self { metavariable, analysis }
    }
}

// ComparisonOperator is now imported from cr_core

/// Data flow analysis specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowSpec {
    pub sources: Vec<String>,
    pub sinks: Vec<String>,
    pub sanitizers: Vec<String>,
    pub must_flow: bool,
    pub max_depth: Option<usize>,
}

impl DataFlowSpec {
    /// Create a new data flow specification
    pub fn new(sources: Vec<String>, sinks: Vec<String>) -> Self {
        Self {
            sources,
            sinks,
            sanitizers: Vec::new(),
            must_flow: true,
            max_depth: None,
        }
    }

    /// Add sanitizers
    pub fn with_sanitizers(mut self, sanitizers: Vec<String>) -> Self {
        self.sanitizers = sanitizers;
        self
    }

    /// Set whether flow must exist
    pub fn with_must_flow(mut self, must_flow: bool) -> Self {
        self.must_flow = must_flow;
        self
    }

    /// Set maximum analysis depth
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }
}

/// Rule execution context
#[derive(Debug, Clone)]
pub struct RuleContext {
    pub file_path: String,
    pub language: Language,
    pub source_code: String,
    pub custom_data: HashMap<String, String>,
}

impl RuleContext {
    /// Create a new rule context
    pub fn new(file_path: String, language: Language, source_code: String) -> Self {
        Self {
            file_path,
            language,
            source_code,
            custom_data: HashMap::new(),
        }
    }

    /// Add custom data
    pub fn add_data(mut self, key: String, value: String) -> Self {
        self.custom_data.insert(key, value);
        self
    }

    /// Get custom data
    pub fn get_data(&self, key: &str) -> Option<&String> {
        self.custom_data.get(key)
    }
}

/// Rule execution result
#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_id: String,
    pub findings: Vec<Finding>,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

impl RuleResult {
    /// Create a successful result
    pub fn success(rule_id: String, findings: Vec<Finding>, execution_time_ms: u64) -> Self {
        Self {
            rule_id,
            findings,
            execution_time_ms,
            error: None,
        }
    }

    /// Create an error result
    pub fn error(rule_id: String, error: String, execution_time_ms: u64) -> Self {
        Self {
            rule_id,
            findings: Vec::new(),
            execution_time_ms,
            error: Some(error),
        }
    }

    /// Check if the result is successful
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the number of findings
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(rule.get_metadata("cwe"), Some(&"CWE-89".to_string()));
        assert_eq!(rule.fix, Some("Use PreparedStatement".to_string()));
    }

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::simple("console.log($MSG)".to_string())
            .with_focus("$MSG".to_string());

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

        assert_eq!(pattern.focus, Some(vec!["$PARAM1".to_string(), "$PARAM2".to_string()]));
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
        let dataflow = DataFlowSpec::new(
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
        let success_result = RuleResult::success(
            "test-rule".to_string(),
            vec![],
            100,
        );

        assert!(success_result.is_success());
        assert_eq!(success_result.finding_count(), 0);
        assert_eq!(success_result.execution_time_ms, 100);

        let error_result = RuleResult::error(
            "test-rule".to_string(),
            "Parse error".to_string(),
            50,
        );

        assert!(!error_result.is_success());
        assert_eq!(error_result.error, Some("Parse error".to_string()));
    }
}
