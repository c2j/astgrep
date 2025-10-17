//! Pattern matching engine for CR-SemService
//!
//! This crate provides pattern matching functionality for rules.

pub mod matcher;
pub mod parser;
pub mod metavar;
pub mod conditions;
pub mod advanced_matcher;
pub mod precise_matcher;

pub use matcher::*;
pub use parser::*;
pub use advanced_matcher::*;
pub use precise_matcher::*;
pub use metavar::{MetavarBinding, MetavarConstraint, MetavarManager};
pub use conditions::{ConditionEvaluator, ConditionType, ComparisonOp};

use cr_core::{AstNode, Result};
use std::collections::HashMap;

/// Main pattern matcher interface
pub struct PatternMatcher {
    metavar_bindings: HashMap<String, String>,
    case_sensitive: bool,
    max_depth: Option<usize>,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self {
            metavar_bindings: HashMap::new(),
            case_sensitive: true,
            max_depth: None,
        }
    }

    /// Create a case-insensitive pattern matcher
    pub fn case_insensitive() -> Self {
        Self {
            metavar_bindings: HashMap::new(),
            case_sensitive: false,
            max_depth: None,
        }
    }

    /// Set maximum matching depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Match a pattern against an AST node
    pub fn matches(&mut self, pattern: &str, node: &dyn AstNode) -> Result<bool> {
        self.metavar_bindings.clear();
        let parsed_pattern = PatternParser::new().parse(pattern)?;
        self.match_pattern(&parsed_pattern, node, 0)
    }

    /// Get metavariable bindings from the last match
    pub fn get_bindings(&self) -> &HashMap<String, String> {
        &self.metavar_bindings
    }

    /// Reset the matcher state
    pub fn reset(&mut self) {
        self.metavar_bindings.clear();
    }

    /// Internal pattern matching logic
    fn match_pattern(&mut self, pattern: &ParsedPattern, node: &dyn AstNode, depth: usize) -> Result<bool> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth >= max_depth {
                return Ok(false);
            }
        }

        match pattern {
            ParsedPattern::Literal(literal) => self.match_literal(literal, node),
            ParsedPattern::Metavariable(metavar) => self.match_metavariable(metavar, node),
            ParsedPattern::EllipsisMetavariable(metavar) => self.match_ellipsis_metavariable(metavar, node),
            ParsedPattern::NodeType(node_type) => self.match_node_type(node_type, node),
            ParsedPattern::Sequence(patterns) => self.match_sequence(patterns, node, depth),
            ParsedPattern::Alternative(patterns) => self.match_alternative(patterns, node, depth),
            ParsedPattern::Wildcard => Ok(true),
        }
    }

    /// Match literal text
    fn match_literal(&self, literal: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if self.case_sensitive {
                Ok(text.contains(literal))
            } else {
                Ok(text.to_lowercase().contains(&literal.to_lowercase()))
            }
        } else {
            Ok(false)
        }
    }

    /// Match metavariable
    fn match_metavariable(&mut self, metavar: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            // Check if this metavariable is already bound
            if let Some(existing_binding) = self.metavar_bindings.get(metavar) {
                Ok(existing_binding == text)
            } else {
                // Bind the metavariable
                self.metavar_bindings.insert(metavar.to_string(), text.to_string());
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    /// Match ellipsis metavariable (can match zero or more nodes)
    fn match_ellipsis_metavariable(&mut self, metavar: &str, node: &dyn AstNode) -> Result<bool> {
        // For now, treat ellipsis metavariables like regular metavariables
        // In a full implementation, this would handle variable-length matching
        if let Some(text) = node.text() {
            // Check if this metavariable is already bound
            if let Some(existing_binding) = self.metavar_bindings.get(metavar) {
                Ok(existing_binding == text)
            } else {
                // Bind the metavariable
                self.metavar_bindings.insert(metavar.to_string(), text.to_string());
                Ok(true)
            }
        } else {
            // Ellipsis can match empty content
            self.metavar_bindings.insert(metavar.to_string(), "".to_string());
            Ok(true)
        }
    }

    /// Match node type
    fn match_node_type(&self, expected_type: &str, node: &dyn AstNode) -> Result<bool> {
        Ok(node.node_type() == expected_type)
    }

    /// Match sequence of patterns
    fn match_sequence(&mut self, patterns: &[ParsedPattern], node: &dyn AstNode, depth: usize) -> Result<bool> {
        if patterns.is_empty() {
            return Ok(true);
        }

        // Try to match the sequence against the node and its children
        if patterns.len() == 1 {
            return self.match_pattern(&patterns[0], node, depth + 1);
        }

        // For multiple patterns, try to match against children
        if node.child_count() >= patterns.len() {
            for i in 0..=node.child_count() - patterns.len() {
                let temp_bindings = self.metavar_bindings.clone();
                let mut all_match = true;

                for (j, pattern) in patterns.iter().enumerate() {
                    if let Some(child) = node.child(i + j) {
                        if !self.match_pattern(pattern, child, depth + 1)? {
                            all_match = false;
                            break;
                        }
                    } else {
                        all_match = false;
                        break;
                    }
                }

                if all_match {
                    return Ok(true);
                } else {
                    // Restore bindings if match failed
                    self.metavar_bindings = temp_bindings;
                }
            }
        }

        Ok(false)
    }

    /// Match alternative patterns (OR)
    fn match_alternative(&mut self, patterns: &[ParsedPattern], node: &dyn AstNode, depth: usize) -> Result<bool> {
        for pattern in patterns {
            let temp_bindings = self.metavar_bindings.clone();
            if self.match_pattern(pattern, node, depth + 1)? {
                return Ok(true);
            }
            // Restore bindings if match failed
            self.metavar_bindings = temp_bindings;
        }
        Ok(false)
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_ast::AstBuilder;

    #[test]
    fn test_pattern_matcher_creation() {
        let matcher = PatternMatcher::new();
        assert!(matcher.get_bindings().is_empty());
        assert!(matcher.case_sensitive);
        assert!(matcher.max_depth.is_none());
    }

    #[test]
    fn test_pattern_matcher_case_insensitive() {
        let matcher = PatternMatcher::case_insensitive();
        assert!(!matcher.case_sensitive);
    }

    #[test]
    fn test_pattern_matcher_with_max_depth() {
        let matcher = PatternMatcher::new().with_max_depth(5);
        assert_eq!(matcher.max_depth, Some(5));
    }

    #[test]
    fn test_simple_literal_match() {
        let mut matcher = PatternMatcher::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let result = matcher.matches("test", &node).unwrap();
        assert!(result);

        let result = matcher.matches("other", &node).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_metavariable_match() {
        let mut matcher = PatternMatcher::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let result = matcher.matches("$VAR", &node).unwrap();
        assert!(result);

        let bindings = matcher.get_bindings();
        assert_eq!(bindings.get("VAR"), Some(&"test_var".to_string()));
    }

    #[test]
    fn test_consistent_metavariable_binding() {
        let mut matcher = PatternMatcher::new();

        // Test with a simple node first
        let node = AstBuilder::identifier("variable_a").with_text("variable_a".to_string());

        // First binding
        let result1 = matcher.matches("$VAR", &node).unwrap();
        assert!(result1);

        // Second binding with same value should work
        matcher.reset();
        let result2 = matcher.matches("$VAR", &node).unwrap();
        assert!(result2);

        let bindings = matcher.get_bindings();
        assert_eq!(bindings.get("VAR"), Some(&"variable_a".to_string()));
    }

    #[test]
    fn test_inconsistent_metavariable_binding() {
        let mut matcher = PatternMatcher::new();

        // Create a binary expression: a + b
        let left = AstBuilder::identifier("variable_a").with_text("variable_a".to_string());
        let right = AstBuilder::identifier("variable_b").with_text("variable_b".to_string());
        let expr = AstBuilder::binary_expression(
            cr_ast::BinaryOperator::Add,
            left,
            right,
        );

        // This should not match because the variables are different
        let result = matcher.matches("$VAR + $VAR", &expr).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_wildcard_match() {
        let mut matcher = PatternMatcher::new();
        let node = AstBuilder::identifier("anything");

        let result = matcher.matches("...", &node).unwrap();
        assert!(result);
    }

    #[test]
    fn test_reset_matcher() {
        let mut matcher = PatternMatcher::new();
        let node = AstBuilder::identifier("test").with_text("test".to_string());

        matcher.matches("$VAR", &node).unwrap();
        assert!(!matcher.get_bindings().is_empty());

        matcher.reset();
        assert!(matcher.get_bindings().is_empty());
    }

    #[test]
    fn test_case_insensitive_matching() {
        let mut matcher = PatternMatcher::case_insensitive();
        let node = AstBuilder::identifier("TEST_VAR").with_text("TEST_VAR".to_string());

        let result = matcher.matches("test", &node).unwrap();
        assert!(result);
    }

    #[test]
    fn test_max_depth_limit() {
        let mut matcher = PatternMatcher::new().with_max_depth(1);
        let node = AstBuilder::identifier("test").with_text("test".to_string());

        // Should match at depth 1
        let result = matcher.matches("test", &node).unwrap();
        assert!(result);

        // Test with depth 0 - should not match
        let mut matcher_depth_0 = PatternMatcher::new().with_max_depth(0);
        let result = matcher_depth_0.matches("test", &node).unwrap();
        assert!(!result);
    }
}
