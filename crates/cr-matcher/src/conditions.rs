//! Condition evaluation for pattern matching
//! 
//! This module provides functionality for evaluating conditions in pattern matching.

use crate::metavar::MetavarManager;
use cr_core::{AstNode, Result};
use regex::Regex;
use std::collections::HashMap;

/// Condition types for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Metavariable regex match
    MetavarRegex { metavar: String, pattern: String },
    /// Metavariable comparison
    MetavarComparison { metavar: String, operator: ComparisonOp, value: String },
    /// Node type check
    NodeType { expected: String },
    /// Node attribute check
    NodeAttribute { attribute: String, value: String },
    /// Custom condition
    Custom { name: String, params: HashMap<String, String> },
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOp {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    Matches,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
}

impl ComparisonOp {
    /// Parse comparison operator from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" | "eq" => Some(ComparisonOp::Equals),
            "!=" | "ne" => Some(ComparisonOp::NotEquals),
            "contains" => Some(ComparisonOp::Contains),
            "starts_with" => Some(ComparisonOp::StartsWith),
            "ends_with" => Some(ComparisonOp::EndsWith),
            "matches" => Some(ComparisonOp::Matches),
            ">" | "gt" => Some(ComparisonOp::GreaterThan),
            "<" | "lt" => Some(ComparisonOp::LessThan),
            ">=" | "ge" => Some(ComparisonOp::GreaterOrEqual),
            "<=" | "le" => Some(ComparisonOp::LessOrEqual),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ComparisonOp::Equals => "==",
            ComparisonOp::NotEquals => "!=",
            ComparisonOp::Contains => "contains",
            ComparisonOp::StartsWith => "starts_with",
            ComparisonOp::EndsWith => "ends_with",
            ComparisonOp::Matches => "matches",
            ComparisonOp::GreaterThan => ">",
            ComparisonOp::LessThan => "<",
            ComparisonOp::GreaterOrEqual => ">=",
            ComparisonOp::LessOrEqual => "<=",
        }
    }
}

/// Condition evaluator
pub struct ConditionEvaluator {
    custom_evaluators: HashMap<String, Box<dyn Fn(&HashMap<String, String>, &dyn AstNode) -> bool + Send + Sync>>,
}

impl ConditionEvaluator {
    /// Create a new condition evaluator
    pub fn new() -> Self {
        Self {
            custom_evaluators: HashMap::new(),
        }
    }

    /// Add a custom condition evaluator
    pub fn add_custom_evaluator<F>(&mut self, name: String, evaluator: F)
    where
        F: Fn(&HashMap<String, String>, &dyn AstNode) -> bool + Send + Sync + 'static,
    {
        self.custom_evaluators.insert(name, Box::new(evaluator));
    }

    /// Evaluate a condition
    pub fn evaluate(
        &self,
        condition: &ConditionType,
        node: &dyn AstNode,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        match condition {
            ConditionType::MetavarRegex { metavar, pattern } => {
                self.evaluate_metavar_regex(metavar, pattern, metavar_manager)
            }
            ConditionType::MetavarComparison { metavar, operator, value } => {
                self.evaluate_metavar_comparison(metavar, operator, value, metavar_manager)
            }
            ConditionType::NodeType { expected } => {
                Ok(node.node_type() == expected)
            }
            ConditionType::NodeAttribute { attribute, value } => {
                self.evaluate_node_attribute(attribute, value, node)
            }
            ConditionType::Custom { name, params } => {
                self.evaluate_custom_condition(name, params, node, metavar_manager)
            }
        }
    }

    /// Evaluate multiple conditions with AND logic
    pub fn evaluate_all(
        &self,
        conditions: &[ConditionType],
        node: &dyn AstNode,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        for condition in conditions {
            if !self.evaluate(condition, node, metavar_manager)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate multiple conditions with OR logic
    pub fn evaluate_any(
        &self,
        conditions: &[ConditionType],
        node: &dyn AstNode,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        for condition in conditions {
            if self.evaluate(condition, node, metavar_manager)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Evaluate metavariable regex condition
    fn evaluate_metavar_regex(
        &self,
        metavar: &str,
        pattern: &str,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        if let Some(binding) = metavar_manager.get_binding(metavar) {
            let regex = Regex::new(pattern)
                .map_err(|e| cr_core::AnalysisError::pattern_match_error(format!("Invalid regex: {}", e)))?;
            Ok(regex.is_match(&binding.value))
        } else {
            Ok(false)
        }
    }

    /// Evaluate metavariable comparison condition
    fn evaluate_metavar_comparison(
        &self,
        metavar: &str,
        operator: &ComparisonOp,
        value: &str,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        if let Some(binding) = metavar_manager.get_binding(metavar) {
            Ok(self.compare_values(&binding.value, operator, value))
        } else {
            Ok(false)
        }
    }

    /// Compare two values using the given operator
    fn compare_values(&self, left: &str, operator: &ComparisonOp, right: &str) -> bool {
        match operator {
            ComparisonOp::Equals => left == right,
            ComparisonOp::NotEquals => left != right,
            ComparisonOp::Contains => left.contains(right),
            ComparisonOp::StartsWith => left.starts_with(right),
            ComparisonOp::EndsWith => left.ends_with(right),
            ComparisonOp::Matches => {
                if let Ok(regex) = Regex::new(right) {
                    regex.is_match(left)
                } else {
                    false
                }
            }
            ComparisonOp::GreaterThan => {
                if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    l > r
                } else {
                    left > right
                }
            }
            ComparisonOp::LessThan => {
                if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    l < r
                } else {
                    left < right
                }
            }
            ComparisonOp::GreaterOrEqual => {
                if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    l >= r
                } else {
                    left >= right
                }
            }
            ComparisonOp::LessOrEqual => {
                if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    l <= r
                } else {
                    left <= right
                }
            }
        }
    }

    /// Evaluate node attribute condition
    fn evaluate_node_attribute(&self, attribute: &str, expected_value: &str, node: &dyn AstNode) -> Result<bool> {
        match attribute {
            "type" => Ok(node.node_type() == expected_value),
            "text" => {
                if let Some(text) = node.text() {
                    Ok(text == expected_value)
                } else {
                    Ok(false)
                }
            }
            "child_count" => {
                if let Ok(expected_count) = expected_value.parse::<usize>() {
                    Ok(node.child_count() == expected_count)
                } else {
                    Ok(false)
                }
            }
            _ => {
                // For other attributes, we would need to extend the AstNode trait
                // For now, return false for unknown attributes
                Ok(false)
            }
        }
    }

    /// Evaluate custom condition
    fn evaluate_custom_condition(
        &self,
        name: &str,
        params: &HashMap<String, String>,
        node: &dyn AstNode,
        metavar_manager: &MetavarManager,
    ) -> Result<bool> {
        if let Some(evaluator) = self.custom_evaluators.get(name) {
            let bindings = metavar_manager.get_binding_values();
            Ok(evaluator(params, node) || evaluator(&bindings, node))
        } else {
            // Unknown custom condition - return false
            Ok(false)
        }
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for condition handling
pub mod utils {
    use super::*;

    /// Create a metavariable regex condition
    pub fn metavar_regex(metavar: &str, pattern: &str) -> ConditionType {
        ConditionType::MetavarRegex {
            metavar: metavar.to_string(),
            pattern: pattern.to_string(),
        }
    }

    /// Create a metavariable comparison condition
    pub fn metavar_comparison(metavar: &str, operator: ComparisonOp, value: &str) -> ConditionType {
        ConditionType::MetavarComparison {
            metavar: metavar.to_string(),
            operator,
            value: value.to_string(),
        }
    }

    /// Create a node type condition
    pub fn node_type(expected: &str) -> ConditionType {
        ConditionType::NodeType {
            expected: expected.to_string(),
        }
    }

    /// Create a node attribute condition
    pub fn node_attribute(attribute: &str, value: &str) -> ConditionType {
        ConditionType::NodeAttribute {
            attribute: attribute.to_string(),
            value: value.to_string(),
        }
    }

    /// Create a custom condition
    pub fn custom(name: &str, params: HashMap<String, String>) -> ConditionType {
        ConditionType::Custom {
            name: name.to_string(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metavar::MetavarManager;
    use cr_ast::AstBuilder;

    #[test]
    fn test_comparison_op_from_str() {
        assert_eq!(ComparisonOp::from_str("=="), Some(ComparisonOp::Equals));
        assert_eq!(ComparisonOp::from_str("!="), Some(ComparisonOp::NotEquals));
        assert_eq!(ComparisonOp::from_str("contains"), Some(ComparisonOp::Contains));
        assert_eq!(ComparisonOp::from_str("unknown"), None);
    }

    #[test]
    fn test_comparison_op_as_str() {
        assert_eq!(ComparisonOp::Equals.as_str(), "==");
        assert_eq!(ComparisonOp::Contains.as_str(), "contains");
        assert_eq!(ComparisonOp::GreaterThan.as_str(), ">");
    }

    #[test]
    fn test_evaluate_node_type_condition() {
        let evaluator = ConditionEvaluator::new();
        let metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test");

        let condition = utils::node_type("identifier");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::node_type("literal");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_node_attribute_condition() {
        let evaluator = ConditionEvaluator::new();
        let metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let condition = utils::node_attribute("text", "test_var");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::node_attribute("text", "other_var");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(!result);

        let condition = utils::node_attribute("child_count", "0");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_metavar_regex_condition() {
        let evaluator = ConditionEvaluator::new();
        let mut metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        // Bind metavariable
        metavar_manager.bind("VAR".to_string(), "test_123".to_string(), &node).unwrap();

        let condition = utils::metavar_regex("VAR", r"test_\d+");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::metavar_regex("VAR", r"fail_\d+");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_metavar_comparison_condition() {
        let evaluator = ConditionEvaluator::new();
        let mut metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        // Bind metavariable
        metavar_manager.bind("VAR".to_string(), "hello_world".to_string(), &node).unwrap();

        let condition = utils::metavar_comparison("VAR", ComparisonOp::Contains, "world");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::metavar_comparison("VAR", ComparisonOp::StartsWith, "hello");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::metavar_comparison("VAR", ComparisonOp::Equals, "goodbye");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_numeric_comparison() {
        let evaluator = ConditionEvaluator::new();
        let mut metavar_manager = MetavarManager::new();
        let node = AstBuilder::integer_literal(42);

        // Bind metavariable
        metavar_manager.bind("NUM".to_string(), "42".to_string(), &node).unwrap();

        let condition = utils::metavar_comparison("NUM", ComparisonOp::GreaterThan, "30");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::metavar_comparison("NUM", ComparisonOp::LessThan, "50");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);

        let condition = utils::metavar_comparison("NUM", ComparisonOp::Equals, "42");
        let result = evaluator.evaluate(&condition, &node, &metavar_manager).unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_all_conditions() {
        let evaluator = ConditionEvaluator::new();
        let mut metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        metavar_manager.bind("VAR".to_string(), "test_var".to_string(), &node).unwrap();

        let conditions = vec![
            utils::node_type("identifier"),
            utils::metavar_comparison("VAR", ComparisonOp::StartsWith, "test"),
            utils::node_attribute("child_count", "0"),
        ];

        let result = evaluator.evaluate_all(&conditions, &node, &metavar_manager).unwrap();
        assert!(result);

        // Add a failing condition
        let conditions_with_failure = vec![
            utils::node_type("identifier"),
            utils::metavar_comparison("VAR", ComparisonOp::StartsWith, "fail"),
        ];

        let result = evaluator.evaluate_all(&conditions_with_failure, &node, &metavar_manager).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_any_conditions() {
        let evaluator = ConditionEvaluator::new();
        let metavar_manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        let conditions = vec![
            utils::node_type("literal"), // false
            utils::node_type("identifier"), // true
            utils::node_attribute("text", "wrong"), // false
        ];

        let result = evaluator.evaluate_any(&conditions, &node, &metavar_manager).unwrap();
        assert!(result);

        let all_false_conditions = vec![
            utils::node_type("literal"),
            utils::node_attribute("text", "wrong"),
        ];

        let result = evaluator.evaluate_any(&all_false_conditions, &node, &metavar_manager).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_custom_condition_evaluator() {
        let mut evaluator = ConditionEvaluator::new();
        evaluator.add_custom_evaluator(
            "is_long_identifier".to_string(),
            |_params, node| {
                if let Some(text) = node.text() {
                    text.len() > 10
                } else {
                    false
                }
            },
        );

        let metavar_manager = MetavarManager::new();
        let short_node = AstBuilder::identifier("short").with_text("short".to_string());
        let long_node = AstBuilder::identifier("very_long_identifier").with_text("very_long_identifier".to_string());

        let condition = utils::custom("is_long_identifier", HashMap::new());

        let result = evaluator.evaluate(&condition, &short_node, &metavar_manager).unwrap();
        assert!(!result);

        let result = evaluator.evaluate(&condition, &long_node, &metavar_manager).unwrap();
        assert!(result);
    }
}
