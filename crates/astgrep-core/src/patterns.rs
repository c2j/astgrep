//! Pattern types for semgrep-style matching
//!
//! This module defines the core pattern types used throughout the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub comparison: String, // Full Python expression
    pub functions: Vec<ComparisonFunction>, // Available functions
    pub variables: Vec<String>, // Available variables in scope
}

/// Available functions for metavariable comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonFunction {
    Today,
    Strptime(String), // Format string
    ReMatch(String), // Regex pattern
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
    pub fn not(inner_pattern: SemgrepPattern) -> Self {
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
    pub bindings: HashMap<String, String>,
    pub confidence: f64,
}

impl SemgrepMatchResult {
    pub fn new(node: Box<dyn crate::AstNode>, bindings: HashMap<String, String>) -> Self {
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
        f.debug_struct("SemgrepMatchResult")
            .field("node_type", &self.node.node_type())
            .field("bindings", &self.bindings)
            .field("confidence", &self.confidence)
            .finish()
    }
}
