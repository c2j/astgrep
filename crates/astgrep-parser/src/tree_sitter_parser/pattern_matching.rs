//! Pattern matching traversal
//!
//! This module provides pattern matching traversal functionality
//! including recursive matching and specific match detection.

use super::{integration::TreeSitterParser, integration::MetaVariableBindings, integration::PatternType};
use astgrep_core::Result;
use tree_sitter::Node;

impl TreeSitterParser {
    /// Find nodes matching a pattern in AST
    pub fn find_pattern_matches<'a>(&self, tree: &'a tree_sitter::Tree, source: &str, pattern: &str) -> Result<Vec<Node<'a>>> {
        let mut matches = Vec::new();
        let root = tree.root_node();
        self.find_matches_recursive(&root, source, pattern, &mut matches)?;
        Ok(matches)
    }

    /// Recursively find pattern matches
    pub fn find_matches_recursive<'a>(
        &self,
        node: &Node<'a>,
        source: &str,
        pattern: &str,
        matches: &mut Vec<Node<'a>>
    ) -> Result<()> {
        // Check if current node matches pattern
        if self.node_matches_pattern(node, source, pattern)? {
            // Only add this match if it's a leaf match or most specific match
            if self.is_most_specific_match(node, source, pattern)? {
                matches.push(*node);
            }
            // Don't recurse into children if we found a match at this level
            // This prevents duplicate matches for same semantic construct
            return Ok(());
        }

        // Check children only if current node doesn't match
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_matches_recursive(&child, source, pattern, matches)?;
            }
        }

        Ok(())
    }

    /// Check if this is most specific match for pattern
    fn is_most_specific_match(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        // For function calls, we want call_expression node, not its children
        if pattern.ends_with("(...)") && matches!(node.kind(), "call_expression" | "call") {
            return Ok(true);
        }

        // For string literals, we want string node itself
        if pattern.starts_with('"') && pattern.ends_with('"') && matches!(node.kind(), "string" | "string_literal") {
            return Ok(true);
        }

        // For numeric literals, we want number node itself
        if pattern.chars().all(|c| c.is_ascii_digit() || c == '.') &&
           matches!(node.kind(), "integer" | "number" | "integer_literal" | "float" | "decimal_literal") {
            return Ok(true);
        }

        // For import statements, we want import statement node
        if pattern.starts_with("import ") && matches!(node.kind(), "import_statement" | "import_from_statement") {
            return Ok(true);
        }

        // For other patterns, check if any children also match
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if self.node_matches_pattern(&child, source, pattern)? {
                    // If a child also matches, this is not most specific
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Check if a node matches a pattern using AST-based matching
    pub fn node_matches_pattern(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        let mut bindings = MetaVariableBindings::new();
        self.node_matches_pattern_with_bindings(node, source, pattern, &mut bindings)
    }

    /// Check if a node matches a pattern with metavariable bindings
    fn node_matches_pattern_with_bindings(
        &self,
        node: &Node,
        source: &str,
        pattern: &str,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // Use AST-based pattern matching instead of text matching
        match self.classify_pattern(pattern) {
            PatternType::StringLiteral(content) => {
                self.match_string_literal(node, source, &content)
            }
            PatternType::NumericLiteral(value) => {
                self.match_numeric_literal(node, source, &value)
            }
            PatternType::FunctionCall(func_name) => {
                self.match_function_call(node, source, &func_name)
            }
            PatternType::ImportStatement(import_spec) => {
                self.match_import_statement(node, source, &import_spec)
            }
            PatternType::MethodCall(object, method) => {
                self.match_method_call(node, source, &object, &method)
            }
            PatternType::Identifier(name) => {
                self.match_identifier(node, source, &name)
            }
            PatternType::MetaVariable(var_name) => {
                self.match_metavariable(node, source, &var_name, bindings)
            }
            PatternType::MetaFunctionCall(func_name, args) => {
                self.match_meta_function_call(node, source, &func_name, &args, bindings)
            }
            PatternType::PatternEither(patterns) => {
                self.match_pattern_either(node, source, &patterns, bindings)
            }
            PatternType::PatternNot(pattern) => {
                self.match_pattern_not(node, source, &pattern, bindings)
            }
            PatternType::PatternInside(inner, outer) => {
                self.match_pattern_inside(node, source, &inner, &outer, bindings)
            }
            PatternType::PatternWhere(pattern, condition) => {
                self.match_pattern_where(node, source, &pattern, &condition, bindings)
            }
            PatternType::Generic(text) => {
                // Fallback for unrecognized patterns - but be more selective
                self.match_generic_pattern(node, source, &text)
            }
        }
    }

    /// Helper to match a PatternType
    fn match_pattern_type(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        match pattern {
            PatternType::StringLiteral(content) => self.match_string_literal(node, source, content),
            PatternType::NumericLiteral(value) => self.match_numeric_literal(node, source, value),
            PatternType::FunctionCall(func_name) => self.match_function_call(node, source, func_name),
            PatternType::ImportStatement(import_spec) => self.match_import_statement(node, source, import_spec),
            PatternType::MethodCall(object, method) => self.match_method_call(node, source, object, method),
            PatternType::Identifier(name) => self.match_identifier(node, source, name),
            PatternType::MetaVariable(var_name) => self.match_metavariable(node, source, var_name, bindings),
            PatternType::MetaFunctionCall(func_name, args) => self.match_meta_function_call(node, source, func_name, args, bindings),
            PatternType::PatternEither(patterns) => self.match_pattern_either(node, source, patterns, bindings),
            PatternType::PatternNot(pattern) => self.match_pattern_not(node, source, pattern, bindings),
            PatternType::PatternInside(inner, outer) => self.match_pattern_inside(node, source, inner, outer, bindings),
            PatternType::PatternWhere(pattern, condition) => self.match_pattern_where(node, source, pattern, condition, bindings),
            PatternType::Generic(text) => self.match_generic_pattern(node, source, text),
        }
    }

    /// Match pattern-either (OR logic)
    fn match_pattern_either(
        &self,
        node: &Node,
        source: &str,
        patterns: &[PatternType],
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        for pattern in patterns {
            let mut temp_bindings = bindings.clone();
            if self.match_pattern_type(node, source, pattern, &mut temp_bindings)? {
                *bindings = temp_bindings;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Match pattern-not (NOT logic)
    fn match_pattern_not(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        let mut temp_bindings = bindings.clone();
        let matches = self.match_pattern_type(node, source, pattern, &mut temp_bindings)?;
        Ok(!matches)
    }

    /// Match pattern-inside (context matching)
    fn match_pattern_inside(
        &self,
        node: &Node,
        source: &str,
        inner: &PatternType,
        outer: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // First, match inner pattern to capture metavariables
        let mut inner_bindings = bindings.clone();
        if self.match_pattern_type(node, source, inner, &mut inner_bindings)? {
            // Check if we're inside an outer pattern, preserving inner bindings
            let mut current = node.parent();
            while let Some(parent) = current {
                // Use inner_bindings (with captured metavariables from inner) for outer matching
                let mut combined_bindings = inner_bindings.clone();
                if self.match_pattern_type(&parent, source, outer, &mut combined_bindings)? {
                    // Merge both inner and outer bindings
                    *bindings = combined_bindings;
                    return Ok(true);
                }
                current = parent.parent();
            }
        }
        Ok(false)
    }

    /// Match pattern-where (conditional matching)
    fn match_pattern_where(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        condition: &str,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // First check if pattern matches
        if self.match_pattern_type(node, source, pattern, bindings)? {
            // Then evaluate condition (simplified implementation)
            self.evaluate_where_condition(node, source, condition, bindings)
        } else {
            Ok(false)
        }
    }

    /// Evaluate where condition (simplified)
    fn evaluate_where_condition(
        &self,
        _node: &Node,
        _source: &str,
        condition: &str,
        bindings: &MetaVariableBindings
    ) -> Result<bool> {
        // Simplified condition evaluation
        // In a full implementation, this would parse and evaluate complex conditions
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();

                let left_value = if left.starts_with('$') {
                    bindings.get(left).map(|s| s.as_str()).unwrap_or("")
                } else {
                    left
                };

                let right_value = if right.starts_with('$') {
                    bindings.get(right).map(|s| s.as_str()).unwrap_or("")
                } else {
                    right.trim_matches('"')
                };

                return Ok(left_value == right_value);
            }
        }

        // Default to true for unrecognized conditions
        Ok(true)
    }

    /// Generic pattern matching (fallback)
    fn match_generic_pattern(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        // Only match on specific node types to avoid matching entire file
        if matches!(node.kind(), "module" | "program" | "source_file") {
            return Ok(false);
        }

        let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
        Ok(node_text.contains(pattern))
    }
}
