//! Advanced pattern matcher
//! 
//! This module provides the advanced pattern matching functionality with full support
//! for metavariables, conditions, and complex patterns.

use crate::{conditions::{ConditionEvaluator, ConditionType}, metavar::*, PatternParser, ParsedPattern};
use astgrep_core::{AstNode, Result};
use std::collections::HashMap;

/// Advanced pattern matcher with full feature support
pub struct AdvancedPatternMatcher {
    parser: PatternParser,
    metavar_manager: MetavarManager,
    condition_evaluator: ConditionEvaluator,
    case_sensitive: bool,
    max_depth: Option<usize>,
    debug_mode: bool,
}

impl AdvancedPatternMatcher {
    /// Create a new advanced pattern matcher
    pub fn new() -> Self {
        Self {
            parser: PatternParser::new(),
            metavar_manager: MetavarManager::new(),
            condition_evaluator: ConditionEvaluator::new(),
            case_sensitive: true,
            max_depth: None,
            debug_mode: false,
        }
    }

    /// Create a case-insensitive matcher
    pub fn case_insensitive() -> Self {
        Self {
            parser: PatternParser::new(),
            metavar_manager: MetavarManager::new(),
            condition_evaluator: ConditionEvaluator::new(),
            case_sensitive: false,
            max_depth: None,
            debug_mode: false,
        }
    }

    /// Enable debug mode for detailed matching information
    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }

    /// Set maximum matching depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Add a metavariable constraint
    pub fn add_metavar_constraint(&mut self, metavar: String, constraint: MetavarConstraint) {
        self.metavar_manager.add_constraint(metavar, constraint);
    }

    /// Add a custom condition evaluator
    pub fn add_custom_condition<F>(&mut self, name: String, evaluator: F)
    where
        F: Fn(&HashMap<String, String>, &dyn AstNode) -> bool + Send + Sync + 'static,
    {
        self.condition_evaluator.add_custom_evaluator(name, evaluator);
    }

    /// Match a pattern against an AST node
    pub fn matches(&mut self, pattern: &str, node: &dyn AstNode) -> Result<bool> {
        self.reset();
        let parsed_pattern = self.parser.parse(pattern)?;
        
        if self.debug_mode {
            eprintln!("Parsed pattern: {}", parsed_pattern);
        }
        
        self.match_pattern(&parsed_pattern, node, 0)
    }

    /// Match a pattern with additional conditions
    pub fn matches_with_conditions(
        &mut self,
        pattern: &str,
        node: &dyn AstNode,
        conditions: &[ConditionType],
    ) -> Result<bool> {
        if self.matches(pattern, node)? {
            self.condition_evaluator.evaluate_all(conditions, node, &self.metavar_manager)
        } else {
            Ok(false)
        }
    }

    /// Find all nodes that match a pattern
    pub fn find_matches(&mut self, pattern: &str, root: &dyn AstNode) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        self.find_matches_recursive(pattern, root, &mut matches, 0)?;
        Ok(matches)
    }

    /// Find matches with conditions
    pub fn find_matches_with_conditions(
        &mut self,
        pattern: &str,
        root: &dyn AstNode,
        conditions: &[ConditionType],
    ) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        self.find_matches_with_conditions_recursive(pattern, root, conditions, &mut matches, 0)?;
        Ok(matches)
    }

    /// Get metavariable bindings from the last match
    pub fn get_bindings(&self) -> HashMap<String, String> {
        self.metavar_manager.get_binding_values()
    }

    /// Reset the matcher state
    pub fn reset(&mut self) {
        self.metavar_manager.clear_bindings();
    }

    /// Internal pattern matching logic
    fn match_pattern(&mut self, pattern: &ParsedPattern, node: &dyn AstNode, depth: usize) -> Result<bool> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth > max_depth {
                return Ok(false);
            }
        }

        if self.debug_mode {
            eprintln!("Matching pattern {:?} against node type: {}", pattern, node.node_type());
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
            self.metavar_manager.bind(metavar.to_string(), text.to_string(), node)
        } else {
            Ok(false)
        }
    }

    /// Match ellipsis metavariable (can match zero or more nodes)
    fn match_ellipsis_metavariable(&mut self, metavar: &str, node: &dyn AstNode) -> Result<bool> {
        // For now, treat ellipsis metavariables like regular metavariables
        // In a full implementation, this would handle variable-length matching
        if let Some(text) = node.text() {
            self.metavar_manager.bind(metavar.to_string(), text.to_string(), node)
        } else {
            // Ellipsis can match empty content
            self.metavar_manager.bind(metavar.to_string(), "".to_string(), node)
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

        if patterns.len() == 1 {
            return self.match_pattern(&patterns[0], node, depth + 1);
        }

        // Try to match the sequence against the node and its children
        if node.child_count() >= patterns.len() {
            for i in 0..=node.child_count() - patterns.len() {
                let snapshot = self.metavar_manager.snapshot();
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
                    self.metavar_manager.restore(snapshot);
                }
            }
        }

        Ok(false)
    }

    /// Match alternative patterns (OR)
    fn match_alternative(&mut self, patterns: &[ParsedPattern], node: &dyn AstNode, depth: usize) -> Result<bool> {
        for pattern in patterns {
            let snapshot = self.metavar_manager.snapshot();
            if self.match_pattern(pattern, node, depth + 1)? {
                return Ok(true);
            }
            // Restore bindings if match failed
            self.metavar_manager.restore(snapshot);
        }
        Ok(false)
    }

    /// Recursively find matches in the AST
    fn find_matches_recursive(
        &mut self,
        pattern: &str,
        node: &dyn AstNode,
        matches: &mut Vec<MatchResult>,
        depth: usize,
    ) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }

        // Try to match at current node
        let snapshot = self.metavar_manager.snapshot();
        if self.matches(pattern, node)? {
            let bindings = self.get_bindings();
            matches.push(MatchResult::new(node.clone_node(), bindings));
        }
        self.metavar_manager.restore(snapshot);

        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_matches_recursive(pattern, child, matches, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Recursively find matches with conditions
    fn find_matches_with_conditions_recursive(
        &mut self,
        pattern: &str,
        node: &dyn AstNode,
        conditions: &[ConditionType],
        matches: &mut Vec<MatchResult>,
        depth: usize,
    ) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }

        // Try to match at current node
        let snapshot = self.metavar_manager.snapshot();
        if self.matches_with_conditions(pattern, node, conditions)? {
            let bindings = self.get_bindings();
            matches.push(MatchResult::new(node.clone_node(), bindings));
        }
        self.metavar_manager.restore(snapshot);

        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_matches_with_conditions_recursive(pattern, child, conditions, matches, depth + 1)?;
            }
        }

        Ok(())
    }
}

impl Default for AdvancedPatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a pattern match
pub struct MatchResult {
    pub node: Box<dyn AstNode>,
    pub bindings: HashMap<String, String>,
}

impl MatchResult {
    /// Create a new match result
    pub fn new(node: Box<dyn AstNode>, bindings: HashMap<String, String>) -> Self {
        Self { node, bindings }
    }

    /// Get the matched node
    pub fn node(&self) -> &dyn AstNode {
        self.node.as_ref()
    }

    /// Get metavariable bindings
    pub fn bindings(&self) -> &HashMap<String, String> {
        &self.bindings
    }

    /// Get a specific binding value
    pub fn get_binding(&self, name: &str) -> Option<&String> {
        self.bindings.get(name)
    }

    /// Check if a metavariable is bound
    pub fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_ast::AstBuilder;

    #[test]
    fn test_advanced_matcher_simple_pattern() {
        let mut matcher = AdvancedPatternMatcher::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let result = matcher.matches("test", &node).unwrap();
        assert!(result);

        let result = matcher.matches("other", &node).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_advanced_matcher_metavariable() {
        let mut matcher = AdvancedPatternMatcher::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let result = matcher.matches("$VAR", &node).unwrap();
        assert!(result);

        let bindings = matcher.get_bindings();
        assert_eq!(bindings.get("VAR"), Some(&"test_var".to_string()));
    }

    #[test]
    fn test_advanced_matcher_node_type() {
        let mut matcher = AdvancedPatternMatcher::new();
        let node = AstBuilder::identifier("test_var");

        let result = matcher.matches("@identifier", &node).unwrap();
        assert!(result);

        let result = matcher.matches("@literal", &node).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_advanced_matcher_sequence() {
        let mut matcher = AdvancedPatternMatcher::new();
        let call_node = AstBuilder::call_expression(
            AstBuilder::identifier("println").with_text("println".to_string()),
            vec![AstBuilder::string_literal("hello").with_text("\"hello\"".to_string())],
        );

        let result = matcher.matches("println $ARG", &call_node).unwrap();
        assert!(result);

        let bindings = matcher.get_bindings();
        assert!(bindings.contains_key("ARG"));
    }

    #[test]
    fn test_advanced_matcher_alternative() {
        let mut matcher = AdvancedPatternMatcher::new();
        let node1 = AstBuilder::identifier("hello").with_text("hello".to_string());
        let node2 = AstBuilder::identifier("world").with_text("world".to_string());

        let result1 = matcher.matches("hello | world", &node1).unwrap();
        assert!(result1);

        let result2 = matcher.matches("hello | world", &node2).unwrap();
        assert!(result2);

        let node3 = AstBuilder::identifier("other").with_text("other".to_string());
        let result3 = matcher.matches("hello | world", &node3).unwrap();
        assert!(!result3);
    }

    #[test]
    fn test_advanced_matcher_with_conditions() {
        let mut matcher = AdvancedPatternMatcher::new();
        let node = AstBuilder::identifier("test_var").with_text("test_var".to_string());

        let conditions = vec![
            crate::conditions::utils::node_type("identifier"),
            crate::conditions::utils::metavar_regex("VAR", r"test_.*"),
        ];

        let result = matcher.matches_with_conditions("$VAR", &node, &conditions).unwrap();
        assert!(result);

        let failing_conditions = vec![
            crate::conditions::utils::node_type("literal"),
        ];

        let result = matcher.matches_with_conditions("$VAR", &node, &failing_conditions).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_advanced_matcher_find_matches() {
        let mut matcher = AdvancedPatternMatcher::new();
        
        let root = AstBuilder::program(vec![
            AstBuilder::expression_statement(
                AstBuilder::identifier("test1").with_text("test1".to_string())
            ),
            AstBuilder::expression_statement(
                AstBuilder::identifier("test2").with_text("test2".to_string())
            ),
            AstBuilder::expression_statement(
                AstBuilder::identifier("other").with_text("other".to_string())
            ),
        ]);

        let matches = matcher.find_matches("test", &root).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_advanced_matcher_case_insensitive() {
        let mut matcher = AdvancedPatternMatcher::case_insensitive();
        let node = AstBuilder::identifier("TEST_VAR").with_text("TEST_VAR".to_string());

        let result = matcher.matches("test", &node).unwrap();
        assert!(result);
    }

    #[test]
    fn test_advanced_matcher_max_depth() {
        let mut matcher = AdvancedPatternMatcher::new().with_max_depth(1);
        
        let deep_node = AstBuilder::program(vec![
            AstBuilder::expression_statement(
                AstBuilder::call_expression(
                    AstBuilder::identifier("deep").with_text("deep".to_string()),
                    vec![],
                )
            ),
        ]);

        let matches = matcher.find_matches("deep", &deep_node).unwrap();
        // Should not find the deep node due to depth limit
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_result() {
        let node = AstBuilder::identifier("test");
        let mut bindings = HashMap::new();
        bindings.insert("VAR".to_string(), "test".to_string());

        let result = MatchResult::new(Box::new(node), bindings);
        
        assert_eq!(result.node().node_type(), "identifier");
        assert!(result.has_binding("VAR"));
        assert_eq!(result.get_binding("VAR"), Some(&"test".to_string()));
        assert!(!result.has_binding("OTHER"));
    }
}
