//! Pattern types for semgrep-style matching
//!
//! This module defines the core pattern types used throughout the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A binding for a metavariable with its value and optional location
#[derive(Debug, Clone)]
pub struct MatchBinding {
    pub value: String,
    pub location: Option<(usize, usize, usize, usize)>,
}

impl MatchBinding {
    pub fn new(value: String) -> Self {
        Self {
            value,
            location: None,
        }
    }

    pub fn with_location(value: String, location: (usize, usize, usize, usize)) -> Self {
        Self {
            value,
            location: Some(location),
        }
    }
}

impl std::fmt::Display for MatchBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::ops::Deref for MatchBinding {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl AsRef<str> for MatchBinding {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<MatchBinding> for String {
    fn from(binding: MatchBinding) -> String {
        binding.value
    }
}

/// Types of patterns supported by semgrep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Simple pattern string
    Simple(String),
    /// Pattern with alternatives (pattern-either)
    Either(Vec<SemgrepPattern>),
    /// Pattern that must be inside another pattern (pattern-inside)
    Inside(Box<SemgrepPattern>),
    /// Pattern that must not be inside another pattern (pattern-not-inside)
    NotInside(Box<SemgrepPattern>),
    /// Pattern that must not match (pattern-not)
    Not(Box<SemgrepPattern>),
    /// Pattern with regex matching (pattern-regex)
    Regex(String),
    /// Pattern with regex that must not match (pattern-not-regex)
    NotRegex(String),
    /// All patterns must match (pattern-all)
    All(Vec<SemgrepPattern>),
    /// Any pattern must match (pattern-any)
    Any(Vec<SemgrepPattern>),
}

/// A semgrep-style pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemgrepPattern {
    pub pattern_type: PatternType,
    pub metavariable_pattern: Option<MetavariablePattern>,
    pub conditions: Vec<Condition>,
    pub focus: Option<Vec<String>>, // Support multiple focus metavariables
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

/// Metavariable analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableAnalysis {
    pub entropy: Option<EntropyAnalysis>,
    pub type_analysis: Option<TypeAnalysis>,
    pub complexity: Option<ComplexityAnalysis>,
}

/// Entropy analysis for detecting secrets/randomness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyAnalysis {
    pub min_entropy: f64,
    pub max_entropy: Option<f64>,
    pub charset: Option<String>, // Expected character set
}

/// Type analysis for metavariables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAnalysis {
    pub expected_types: Vec<String>,
    pub forbidden_types: Vec<String>,
    pub nullable: Option<bool>,
}

/// Complexity analysis for code patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAnalysis {
    pub max_cyclomatic: Option<u32>,
    pub max_nesting_depth: Option<u32>,
    pub max_lines: Option<u32>,
}

/// Condition for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    MetavariableRegex(MetavariableRegex),
    MetavariableComparison(MetavariableComparison),
    MetavariableName(MetavariableName),
    MetavariableAnalysis(MetavariableAnalysisCondition),
    MetavariableType(MetavariableType),
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
        Self {
            metavariable,
            regex,
        }
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
        Self {
            metavariable,
            operator,
            value,
        }
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
        Self {
            metavariable,
            name_pattern,
        }
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
        Self {
            metavariable,
            analysis,
        }
    }
}

/// Metavariable type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetavariableType {
    pub metavariable: String,
    pub var_type: String,
}

impl MetavariableType {
    pub fn new(metavariable: String, var_type: String) -> Self {
        Self {
            metavariable,
            var_type,
        }
    }
}

/// Comparison operators for metavariable conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
    GreaterThan,
    LessThan,
    PythonExpression(String), // Full Python expression support
}

/// Enhanced metavariable comparison with Python expression support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMetavariableComparison {
    pub metavariable: String,
    pub comparison: String,                 // Full Python expression
    pub functions: Vec<ComparisonFunction>, // Available functions
    pub variables: Vec<String>,             // Available variables in scope
}

/// Available functions for metavariable comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonFunction {
    Today,
    Strptime(String), // Format string
    ReMatch(String),  // Regex pattern
    Len,
    Int,
    Float,
    Str,
    Custom(String), // Custom function name
}

impl SemgrepPattern {
    /// Create a simple pattern
    pub fn simple(pattern: String) -> Self {
        Self {
            pattern_type: PatternType::Simple(pattern),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-either
    pub fn either(patterns: Vec<SemgrepPattern>) -> Self {
        Self {
            pattern_type: PatternType::Either(patterns),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-inside
    pub fn inside(inner_pattern: SemgrepPattern) -> Self {
        Self {
            pattern_type: PatternType::Inside(Box::new(inner_pattern)),
            metavariable_pattern: None,
            conditions: Vec::new(),
            focus: None,
        }
    }

    /// Create a pattern-not
    pub fn pattern_not(inner_pattern: SemgrepPattern) -> Self {
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

    /// Add a condition to this pattern
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the metavariable pattern for this pattern
    pub fn with_metavariable_pattern(mut self, metavar_pattern: MetavariablePattern) -> Self {
        self.metavariable_pattern = Some(metavar_pattern);
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

    /// Get the pattern string for simple patterns
    pub fn get_pattern_string(&self) -> Option<&String> {
        match &self.pattern_type {
            PatternType::Simple(pattern) => Some(pattern),
            PatternType::Regex(pattern) => Some(pattern),
            _ => None,
        }
    }
}

impl MetavariablePattern {
    /// Create a new metavariable pattern
    pub fn new(metavariable: String) -> Self {
        Self {
            metavariable,
            patterns: Vec::new(),
            regex: None,
            type_constraint: None,
            name_constraint: None,
            analysis: None,
        }
    }

    /// Create a new metavariable pattern with patterns
    pub fn with_patterns(metavariable: String, patterns: Vec<String>) -> Self {
        Self {
            metavariable,
            patterns,
            regex: None,
            type_constraint: None,
            name_constraint: None,
            analysis: None,
        }
    }

    /// Add a pattern to this metavariable pattern
    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Set the regex constraint
    pub fn with_regex(mut self, regex: String) -> Self {
        self.regex = Some(regex);
        self
    }

    /// Set the type constraint
    pub fn with_type_constraint(mut self, type_constraint: String) -> Self {
        self.type_constraint = Some(type_constraint);
        self
    }
}

/// Result of a semgrep-style pattern match
pub struct SemgrepMatchResult {
    pub node: Box<dyn crate::AstNode>,
    pub bindings: HashMap<String, MatchBinding>,
    pub confidence: f64,
}

impl SemgrepMatchResult {
    pub fn new(node: Box<dyn crate::AstNode>, bindings: HashMap<String, MatchBinding>) -> Self {
        Self {
            node,
            bindings,
            confidence: 1.0,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }
}

impl std::fmt::Debug for SemgrepMatchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let binding_values: HashMap<_, _> =
            self.bindings.iter().map(|(k, v)| (k, &v.value)).collect();
        f.debug_struct("SemgrepMatchResult")
            .field("node_type", &self.node.node_type())
            .field("bindings", &binding_values)
            .field("confidence", &self.confidence)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_binding_new() {
        let binding = MatchBinding::new("hello".to_string());
        assert_eq!(binding.value, "hello");
        assert_eq!(binding.location, None);
    }

    #[test]
    fn test_match_binding_new_empty() {
        let binding = MatchBinding::new("".to_string());
        assert_eq!(binding.value, "");
        assert_eq!(binding.location, None);
    }

    #[test]
    fn test_match_binding_with_location() {
        let binding = MatchBinding::with_location("world".to_string(), (1, 2, 3, 4));
        assert_eq!(binding.value, "world");
        assert_eq!(binding.location, Some((1, 2, 3, 4)));
    }

    #[test]
    fn test_match_binding_display() {
        let binding = MatchBinding::new("display_test".to_string());
        assert_eq!(format!("{}", binding), "display_test");
    }

    #[test]
    fn test_match_binding_deref() {
        let binding = MatchBinding::new("deref_test".to_string());
        assert_eq!(&*binding, "deref_test");
        assert!(binding.starts_with("deref"));
    }

    #[test]
    fn test_match_binding_as_ref() {
        let binding = MatchBinding::new("as_ref_test".to_string());
        assert_eq!(binding.as_ref(), "as_ref_test");
    }

    #[test]
    fn test_match_binding_into_string() {
        let binding = MatchBinding::new("into_test".to_string());
        let s: String = binding.into();
        assert_eq!(s, "into_test");
    }

    #[test]
    fn test_pattern_type_simple() {
        let simple = PatternType::Simple("foo".to_string());
        assert!(matches!(simple, PatternType::Simple(_)));
    }

    #[test]
    fn test_pattern_type_either() {
        let either = PatternType::Either(vec![]);
        assert!(matches!(either, PatternType::Either(_)));
    }

    #[test]
    fn test_pattern_type_inside() {
        let inside = PatternType::Inside(Box::new(SemgrepPattern::simple("x".to_string())));
        assert!(matches!(inside, PatternType::Inside(_)));
    }

    #[test]
    fn test_pattern_type_not_inside() {
        let not_inside = PatternType::NotInside(Box::new(SemgrepPattern::simple("y".to_string())));
        assert!(matches!(not_inside, PatternType::NotInside(_)));
    }

    #[test]
    fn test_pattern_type_not() {
        let not = PatternType::Not(Box::new(SemgrepPattern::simple("z".to_string())));
        assert!(matches!(not, PatternType::Not(_)));
    }

    #[test]
    fn test_pattern_type_regex() {
        let regex = PatternType::Regex("r.*".to_string());
        assert!(matches!(regex, PatternType::Regex(_)));
    }

    #[test]
    fn test_pattern_type_not_regex() {
        let not_regex = PatternType::NotRegex("n.*".to_string());
        assert!(matches!(not_regex, PatternType::NotRegex(_)));
    }

    #[test]
    fn test_pattern_type_all() {
        let all = PatternType::All(vec![]);
        assert!(matches!(all, PatternType::All(_)));
    }

    #[test]
    fn test_pattern_type_any() {
        let any = PatternType::Any(vec![]);
        assert!(matches!(any, PatternType::Any(_)));
    }

    #[test]
    fn test_semgrep_pattern_simple() {
        let pattern = SemgrepPattern::simple("$X + $Y".to_string());
        assert_eq!(pattern.get_pattern_string().map(|s| s.as_str()), Some("$X + $Y"));
        assert!(pattern.conditions.is_empty());
        assert!(pattern.focus.is_none());
        assert!(pattern.metavariable_pattern.is_none());
    }

    #[test]
    fn test_semgrep_pattern_either() {
        let p1 = SemgrepPattern::simple("a".to_string());
        let p2 = SemgrepPattern::simple("b".to_string());
        let pattern = SemgrepPattern::either(vec![p1, p2]);
        assert!(matches!(pattern.pattern_type, PatternType::Either(_)));
        assert!(pattern.get_pattern_string().is_none());
    }

    #[test]
    fn test_semgrep_pattern_inside() {
        let inner = SemgrepPattern::simple("inner".to_string());
        let pattern = SemgrepPattern::inside(inner);
        assert!(matches!(pattern.pattern_type, PatternType::Inside(_)));
        assert!(pattern.get_pattern_string().is_none());
    }

    #[test]
    fn test_semgrep_pattern_pattern_not() {
        let inner = SemgrepPattern::simple("inner".to_string());
        let pattern = SemgrepPattern::pattern_not(inner);
        assert!(matches!(pattern.pattern_type, PatternType::Not(_)));
        assert!(pattern.get_pattern_string().is_none());
    }

    #[test]
    fn test_semgrep_pattern_regex() {
        let pattern = SemgrepPattern::regex("foo.*bar".to_string());
        assert_eq!(pattern.get_pattern_string().map(|s| s.as_str()), Some("foo.*bar"));
    }

    #[test]
    fn test_semgrep_pattern_with_condition() {
        let pattern = SemgrepPattern::simple("$X".to_string())
            .with_condition(Condition::NodeType("identifier".to_string()));
        assert_eq!(pattern.conditions.len(), 1);
        assert!(matches!(&pattern.conditions[0], Condition::NodeType(s) if s == "identifier"));
    }

    #[test]
    fn test_semgrep_pattern_with_multiple_conditions() {
        let pattern = SemgrepPattern::simple("$X".to_string())
            .with_condition(Condition::NodeType("identifier".to_string()))
            .with_condition(Condition::Custom("check".to_string()));
        assert_eq!(pattern.conditions.len(), 2);
    }

    #[test]
    fn test_semgrep_pattern_with_metavariable_pattern() {
        let mv = MetavariablePattern::new("$X".to_string());
        let pattern = SemgrepPattern::simple("$X".to_string())
            .with_metavariable_pattern(mv);
        assert!(pattern.metavariable_pattern.is_some());
        assert_eq!(pattern.metavariable_pattern.as_ref().unwrap().metavariable, "$X");
    }

    #[test]
    fn test_semgrep_pattern_with_focus() {
        let pattern = SemgrepPattern::simple("$X".to_string())
            .with_focus("$X".to_string());
        assert_eq!(pattern.focus, Some(vec!["$X".to_string()]));
    }

    #[test]
    fn test_semgrep_pattern_with_focus_metavariables() {
        let pattern = SemgrepPattern::simple("$X".to_string())
            .with_focus_metavariables(vec!["$X".to_string(), "$Y".to_string()]);
        assert_eq!(pattern.focus, Some(vec!["$X".to_string(), "$Y".to_string()]));
    }

    #[test]
    fn test_metavariable_pattern_new() {
        let mv = MetavariablePattern::new("$VAR".to_string());
        assert_eq!(mv.metavariable, "$VAR");
        assert!(mv.patterns.is_empty());
        assert!(mv.regex.is_none());
        assert!(mv.type_constraint.is_none());
        assert!(mv.name_constraint.is_none());
        assert!(mv.analysis.is_none());
    }

    #[test]
    fn test_metavariable_pattern_with_patterns() {
        let mv = MetavariablePattern::with_patterns("$VAR".to_string(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(mv.metavariable, "$VAR");
        assert_eq!(mv.patterns.len(), 2);
    }

    #[test]
    fn test_metavariable_pattern_with_pattern_builder() {
        let mv = MetavariablePattern::new("$VAR".to_string())
            .with_pattern("a".to_string())
            .with_pattern("b".to_string());
        assert_eq!(mv.patterns, vec!["a", "b"]);
    }

    #[test]
    fn test_metavariable_pattern_with_regex() {
        let mv = MetavariablePattern::new("$VAR".to_string()).with_regex(".*".to_string());
        assert_eq!(mv.regex, Some(".*".to_string()));
    }

    #[test]
    fn test_metavariable_pattern_with_type_constraint() {
        let mv = MetavariablePattern::new("$VAR".to_string()).with_type_constraint("String".to_string());
        assert_eq!(mv.type_constraint, Some("String".to_string()));
    }

    #[test]
    fn test_metavariable_pattern_full_builder() {
        let mv = MetavariablePattern::new("$X".to_string())
            .with_pattern("a".to_string())
            .with_regex("[0-9]+".to_string())
            .with_type_constraint("int".to_string());
        assert_eq!(mv.patterns.len(), 1);
        assert_eq!(mv.regex, Some("[0-9]+".to_string()));
        assert_eq!(mv.type_constraint, Some("int".to_string()));
    }

    #[test]
    fn test_entropy_analysis_fields() {
        let analysis = EntropyAnalysis {
            min_entropy: 3.5,
            max_entropy: Some(8.0),
            charset: Some("base64".to_string()),
        };
        assert_eq!(analysis.min_entropy, 3.5);
        assert_eq!(analysis.max_entropy, Some(8.0));
        assert_eq!(analysis.charset, Some("base64".to_string()));
    }

    #[test]
    fn test_entropy_analysis_minimal() {
        let analysis = EntropyAnalysis {
            min_entropy: 0.0,
            max_entropy: None,
            charset: None,
        };
        assert_eq!(analysis.min_entropy, 0.0);
        assert!(analysis.max_entropy.is_none());
        assert!(analysis.charset.is_none());
    }

    #[test]
    fn test_type_analysis_fields() {
        let analysis = TypeAnalysis {
            expected_types: vec!["String".to_string(), "int".to_string()],
            forbidden_types: vec!["null".to_string()],
            nullable: Some(false),
        };
        assert_eq!(analysis.expected_types.len(), 2);
        assert_eq!(analysis.forbidden_types, vec!["null"]);
        assert_eq!(analysis.nullable, Some(false));
    }

    #[test]
    fn test_complexity_analysis_fields() {
        let analysis = ComplexityAnalysis {
            max_cyclomatic: Some(10),
            max_nesting_depth: Some(3),
            max_lines: Some(100),
        };
        assert_eq!(analysis.max_cyclomatic, Some(10));
        assert_eq!(analysis.max_nesting_depth, Some(3));
        assert_eq!(analysis.max_lines, Some(100));
    }

    #[test]
    fn test_metavariable_analysis_all_fields() {
        let analysis = MetavariableAnalysis {
            entropy: Some(EntropyAnalysis {
                min_entropy: 4.0,
                max_entropy: None,
                charset: None,
            }),
            type_analysis: Some(TypeAnalysis {
                expected_types: vec!["String".to_string()],
                forbidden_types: vec![],
                nullable: None,
            }),
            complexity: Some(ComplexityAnalysis {
                max_cyclomatic: Some(5),
                max_nesting_depth: None,
                max_lines: None,
            }),
        };
        assert!(analysis.entropy.is_some());
        assert!(analysis.type_analysis.is_some());
        assert!(analysis.complexity.is_some());
    }

    #[test]
    fn test_condition_metavariable_regex() {
        let cond = Condition::MetavariableRegex(MetavariableRegex::new("$X".to_string(), ".*".to_string()));
        assert!(matches!(cond, Condition::MetavariableRegex(_)));
    }

    #[test]
    fn test_condition_metavariable_comparison() {
        let cond = Condition::MetavariableComparison(MetavariableComparison::new(
            "$X".to_string(),
            ComparisonOperator::Equals,
            "val".to_string(),
        ));
        assert!(matches!(cond, Condition::MetavariableComparison(_)));
    }

    #[test]
    fn test_condition_metavariable_name() {
        let cond = Condition::MetavariableName(MetavariableName::new("$X".to_string(), "foo_.*".to_string()));
        assert!(matches!(cond, Condition::MetavariableName(_)));
    }

    #[test]
    fn test_condition_metavariable_analysis() {
        let analysis = MetavariableAnalysis {
            entropy: None,
            type_analysis: None,
            complexity: None,
        };
        let cond = Condition::MetavariableAnalysis(MetavariableAnalysisCondition::new("$X".to_string(), analysis));
        assert!(matches!(cond, Condition::MetavariableAnalysis(_)));
    }

    #[test]
    fn test_condition_metavariable_type() {
        let cond = Condition::MetavariableType(MetavariableType::new("$X".to_string(), "String".to_string()));
        assert!(matches!(cond, Condition::MetavariableType(_)));
    }

    #[test]
    fn test_condition_node_type() {
        let cond = Condition::NodeType("identifier".to_string());
        assert!(matches!(cond, Condition::NodeType(_)));
    }

    #[test]
    fn test_condition_node_attribute() {
        let cond = Condition::NodeAttribute("key".to_string(), "value".to_string());
        assert!(matches!(cond, Condition::NodeAttribute(_, _)));
    }

    #[test]
    fn test_condition_custom() {
        let cond = Condition::Custom("custom".to_string());
        assert!(matches!(cond, Condition::Custom(_)));
    }

    #[test]
    fn test_metavariable_regex_new() {
        let mvr = MetavariableRegex::new("$X".to_string(), "^foo$".to_string());
        assert_eq!(mvr.metavariable, "$X");
        assert_eq!(mvr.regex, "^foo$");
    }

    #[test]
    fn test_metavariable_comparison_new() {
        let mvc = MetavariableComparison::new("$X".to_string(), ComparisonOperator::Contains, "foo".to_string());
        assert_eq!(mvc.metavariable, "$X");
        assert!(matches!(mvc.operator, ComparisonOperator::Contains));
        assert_eq!(mvc.value, "foo");
    }

    #[test]
    fn test_metavariable_name_new() {
        let mvn = MetavariableName::new("$X".to_string(), "name_.*".to_string());
        assert_eq!(mvn.metavariable, "$X");
        assert_eq!(mvn.name_pattern, "name_.*");
    }

    #[test]
    fn test_metavariable_type_new() {
        let mvt = MetavariableType::new("$X".to_string(), "int".to_string());
        assert_eq!(mvt.metavariable, "$X");
        assert_eq!(mvt.var_type, "int");
    }

    #[test]
    fn test_metavariable_analysis_condition_new() {
        let analysis = MetavariableAnalysis {
            entropy: Some(EntropyAnalysis {
                min_entropy: 3.0,
                max_entropy: None,
                charset: None,
            }),
            type_analysis: None,
            complexity: None,
        };
        let cond = MetavariableAnalysisCondition::new("$X".to_string(), analysis);
        assert_eq!(cond.metavariable, "$X");
        assert!(cond.analysis.entropy.is_some());
    }

    #[test]
    fn test_comparison_operator_all_variants() {
        let ops: Vec<ComparisonOperator> = vec![
            ComparisonOperator::Equals,
            ComparisonOperator::NotEquals,
            ComparisonOperator::Contains,
            ComparisonOperator::StartsWith,
            ComparisonOperator::EndsWith,
            ComparisonOperator::Matches,
            ComparisonOperator::GreaterThan,
            ComparisonOperator::LessThan,
            ComparisonOperator::PythonExpression("x > 0".to_string()),
        ];
        assert_eq!(ops.len(), 9);
        assert!(matches!(ops[0], ComparisonOperator::Equals));
        assert!(matches!(ops[8], ComparisonOperator::PythonExpression(_)));
    }

    #[test]
    fn test_enhanced_metavariable_comparison() {
        let enhanced = EnhancedMetavariableComparison {
            metavariable: "$X".to_string(),
            comparison: "int($X) > 100".to_string(),
            functions: vec![ComparisonFunction::Int],
            variables: vec!["$X".to_string()],
        };
        assert_eq!(enhanced.metavariable, "$X");
        assert_eq!(enhanced.comparison, "int($X) > 100");
        assert_eq!(enhanced.functions.len(), 1);
        assert_eq!(enhanced.variables.len(), 1);
    }

    #[test]
    fn test_comparison_function_variants() {
        let funcs = vec![
            ComparisonFunction::Today,
            ComparisonFunction::Strptime("%Y-%m-%d".to_string()),
            ComparisonFunction::ReMatch(".*".to_string()),
            ComparisonFunction::Len,
            ComparisonFunction::Int,
            ComparisonFunction::Float,
            ComparisonFunction::Str,
            ComparisonFunction::Custom("my_func".to_string()),
        ];
        assert_eq!(funcs.len(), 8);
        assert!(matches!(funcs[0], ComparisonFunction::Today));
        assert!(matches!(funcs[7], ComparisonFunction::Custom(_)));
    }

    #[test]
    fn test_semgrep_match_result_new() {
        use crate::AstNode;
        struct DummyNode;
        impl AstNode for DummyNode {
            fn node_type(&self) -> &str { "dummy" }
            fn child_count(&self) -> usize { 0 }
            fn child(&self, _index: usize) -> Option<&dyn AstNode> { None }
            fn location(&self) -> Option<(usize, usize, usize, usize)> { None }
            fn text(&self) -> Option<&str> { None }
            fn clone_node(&self) -> Box<dyn AstNode> { Box::new(DummyNode) }
        }

        let node: Box<dyn AstNode> = Box::new(DummyNode);
        let bindings = HashMap::from([("$X".to_string(), MatchBinding::new("value".to_string()))]);
        let result = SemgrepMatchResult::new(node, bindings);
        assert_eq!(result.confidence, 1.0);
        assert_eq!(result.bindings.len(), 1);
    }

    #[test]
    fn test_semgrep_match_result_with_confidence() {
        use crate::AstNode;
        struct DummyNode;
        impl AstNode for DummyNode {
            fn node_type(&self) -> &str { "dummy" }
            fn child_count(&self) -> usize { 0 }
            fn child(&self, _index: usize) -> Option<&dyn AstNode> { None }
            fn location(&self) -> Option<(usize, usize, usize, usize)> { None }
            fn text(&self) -> Option<&str> { None }
            fn clone_node(&self) -> Box<dyn AstNode> { Box::new(DummyNode) }
        }

        let node: Box<dyn AstNode> = Box::new(DummyNode);
        let result = SemgrepMatchResult::new(node, HashMap::new()).with_confidence(0.85);
        assert_eq!(result.confidence, 0.85);
        assert!(result.bindings.is_empty());
    }

    #[test]
    fn test_semgrep_match_result_debug() {
        use crate::AstNode;
        struct DummyNode;
        impl AstNode for DummyNode {
            fn node_type(&self) -> &str { "dummy" }
            fn child_count(&self) -> usize { 0 }
            fn child(&self, _index: usize) -> Option<&dyn AstNode> { None }
            fn location(&self) -> Option<(usize, usize, usize, usize)> { None }
            fn text(&self) -> Option<&str> { None }
            fn clone_node(&self) -> Box<dyn AstNode> { Box::new(DummyNode) }
        }

        let node: Box<dyn AstNode> = Box::new(DummyNode);
        let result = SemgrepMatchResult::new(node, HashMap::new());
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("SemgrepMatchResult"));
        assert!(debug_str.contains("dummy"));
    }

    #[test]
    fn test_pattern_type_serde_roundtrip() {
        let pattern = PatternType::Simple("foo($X)".to_string());
        let json = serde_json::to_string(&pattern).expect("serialize");
        let deserialized: PatternType = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, PatternType::Simple(_)));
    }

    #[test]
    fn test_comparison_operator_serde_roundtrip() {
        let op = ComparisonOperator::Contains;
        let json = serde_json::to_string(&op).expect("serialize");
        let deserialized: ComparisonOperator = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, ComparisonOperator::Contains));
    }
}
