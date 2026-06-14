//! Utility functions for constant propagation
//!
//! This module contains helper functions for context detection,
//! name extraction, and value parsing.

use crate::constant_propagation::state::{ConstantValue, SourceLocation};
use astgrep_core::AstNode;
use std::collections::HashMap;

/// Get the tree-sitter node kind, falling back to the universal node type.
fn ts_kind(node: &dyn AstNode) -> String {
    node.get_attribute("ts_kind")
        .map(String::from)
        .unwrap_or_else(|| node.node_type().to_string())
}

/// Get the source location of a node
pub fn get_node_location(node: &dyn AstNode) -> Option<SourceLocation> {
    node.location()
        .map(|(start_line, start_col, _, _)| SourceLocation::new(start_line, start_col))
}

/// Check if this is a static block context
pub fn is_static_block_context(node: &dyn AstNode) -> bool {
    // Check for static block patterns:
    // 1. The node itself contains "static {" pattern
    // 2. Or it's inside a static initialization block
    if let Some(text) = node.text() {
        let text_trimmed = text.trim();
        // Match patterns like "static {" or "static{"
        if text_trimmed.starts_with("static") && text_trimmed.contains("{") {
            return true;
        }
    }

    // Check children recursively for static block patterns
    // This handles cases where static keyword and block are separate children
    let mut has_static = false;
    let mut has_block = false;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let child_type = child.node_type();

            // Look for static keyword
            if child_type == "static" || child.text().map(|t| t.trim() == "static").unwrap_or(false)
            {
                has_static = true;
            }

            // Look for block
            if child_type == "block_statement" || child_type == "block" {
                has_block = true;
            }

            // If we see both static and a block in the same node context, it's likely a static block
            if has_static && has_block {
                return true;
            }
        }
    }

    // Also check if parent node is a static block
    let kind = ts_kind(node);
    if kind == "static_initializer" || kind == "static_block" || kind == "static_initialization" {
        return true;
    }

    false
}

/// Check if this is a constructor declaration.
///
/// Handles two forms:
/// 1. Tree-sitter-java `constructor_declaration` nodes (detected via ts_kind)
/// 2. Other grammars that wrap constructors as `declaration_statement`
///    with 4 children: [modifiers, identifier(class_name), params, body]
pub fn is_constructor_declaration(node: &dyn AstNode, class_name: Option<&str>) -> bool {
    // Check raw tree-sitter kind first (handles constructor_declaration nodes
    // that are mapped to FunctionDeclaration in the universal type system)
    if ts_kind(node) == "constructor_declaration" {
        if let Some(class) = class_name {
            // Verify the identifier matches the expected class name
            return node.child_count() > 0
                && (0..node.child_count()).any(|i| {
                    node.child(i).is_some_and(|c| {
                        c.node_type() == "identifier" && c.text().is_some_and(|t| t == class)
                    })
                });
        }
        return true;
    }

    // Fallback: declaration_statement wrapping (other tree-sitter grammars)
    if node.node_type() != "declaration_statement" {
        return false;
    }

    if node.child_count() != 4 {
        return false;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.node_type() == "identifier" {
                if let Some(name) = child.text() {
                    if let Some(class) = class_name {
                        if name == class {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if this is a method declaration
/// Java methods appear as declaration_statement with:
/// - 5 children: [modifiers, return_type, identifier(method_name), params, body]
pub fn is_method_declaration(node: &dyn AstNode) -> bool {
    // Must be a declaration_statement
    if node.node_type() != "declaration_statement" {
        return false;
    }

    // Methods have 5 children (with return type)
    // Check if the 3rd child (index 2) is an identifier (method name)
    if node.child_count() == 5 {
        // Check if the 2nd child (after modifiers) is NOT an identifier
        // (it would be the return type for methods)
        if let Some(child) = node.child(2) {
            if child.node_type() == "identifier" {
                // Check if the 2nd child (index 1) is NOT an identifier
                // (it would be the return type for methods)
                if let Some(second_child) = node.child(1) {
                    if second_child.node_type() != "identifier" {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Extract variable name from assignment target
/// Handles: identifier, field_access (this.field), etc.
pub fn extract_variable_name_from_assignment_target(node: &dyn AstNode) -> Option<String> {
    match node.node_type() {
        "identifier" => {
            // Direct variable: x = ...
            node.text().map(|t| t.to_string())
        }
        "field_access" => {
            // Field access: this.field = ... or obj.field = ...
            // Extract the field name (last identifier child)
            let mut field_name = None;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.node_type() == "identifier" {
                        field_name = child.text().map(|t| t.to_string());
                    }
                }
            }
            field_name
        }
        _ => {
            // Try to find identifier in other node types
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if let Some(name) = extract_variable_name_from_assignment_target(child) {
                        return Some(name);
                    }
                }
            }
            None
        }
    }
}

/// Extract constant value from expression node
pub fn extract_constant_from_expression(
    node: &dyn AstNode,
    constants: &HashMap<String, ConstantValue>,
) -> Option<ConstantValue> {
    let kind = ts_kind(node);
    match kind.as_str() {
        "literal"
        | "decimal_integer_literal"
        | "integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal" => {
            if let Some(text) = node.text() {
                let trimmed = text.trim();
                if trimmed.starts_with('"') || trimmed.starts_with('\'') {
                    Some(ConstantValue::String(
                        trimmed.trim_matches(|c| c == '"' || c == '\'').to_string(),
                    ))
                } else if let Ok(i) = trimmed.parse::<i64>() {
                    Some(ConstantValue::Integer(i))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "string_literal" | "literal_string" => {
            if let Some(text) = node.text() {
                Some(ConstantValue::String(
                    text.trim_matches(|c| c == '"' || c == '\'').to_string(),
                ))
            } else {
                None
            }
        }
        "true" | "false" => {
            // Boolean literal
            node.text().map(|t| t == "true").map(ConstantValue::Boolean)
        }
        "null_literal" | "null" => Some(ConstantValue::Null),
        "identifier" => {
            // If identifier refers to a known constant, propagate it
            if let Some(var_name) = node.text() {
                constants.get(var_name).cloned()
            } else {
                None
            }
        }
        "method_invocation" | "call_expression" => {
            // For known string-producing methods, return a string constant
            if let Some(text) = node.text() {
                let t = text.trim();
                if t.contains("String.format")
                    || t.contains("String.valueOf")
                    || t.contains(".concat(")
                    || t.contains(".toString(")
                    || t.contains("StringBuilder")
                    || t.contains("StringBuffer")
                {
                    return Some(ConstantValue::String(String::new()));
                }
            }
            // Fall through to check children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if let Some(value) = extract_constant_from_expression(child, constants) {
                        return Some(value);
                    }
                }
            }
            None
        }
        _ => {
            // For other node types, check children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if let Some(value) = extract_constant_from_expression(child, constants) {
                        return Some(value);
                    }
                }
            }
            None
        }
    }
}
