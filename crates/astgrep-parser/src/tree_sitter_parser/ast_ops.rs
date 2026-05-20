//! AST operations and transformations
//!
//! This module provides AST-based pattern matching operations
//! for various node types and patterns.

use super::integration::{MetaVariableBindings, TreeSitterParser};
use astgrep_core::Result;
use tree_sitter::Node;

impl TreeSitterParser {
    /// Match string literal nodes
    pub(super) fn match_string_literal(
        &self,
        node: &Node,
        source: &str,
        content: &str,
    ) -> Result<bool> {
        if matches!(node.kind(), "string" | "string_literal") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            // Remove quotes and check content
            let cleaned_text = node_text.trim_matches('"').trim_matches('\'');
            Ok(cleaned_text == content)
        } else {
            Ok(false)
        }
    }

    /// Match numeric literal nodes
    pub(super) fn match_numeric_literal(
        &self,
        node: &Node,
        source: &str,
        value: &str,
    ) -> Result<bool> {
        if matches!(
            node.kind(),
            "integer" | "number" | "integer_literal" | "float" | "decimal_literal"
        ) {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            Ok(node_text == value)
        } else {
            Ok(false)
        }
    }

    /// Match function call nodes
    pub(super) fn match_function_call(
        &self,
        node: &Node,
        source: &str,
        func_name: &str,
    ) -> Result<bool> {
        if matches!(node.kind(), "call_expression" | "call") {
            // Check if function name matches
            if let Some(function_node) = node.child_by_field_name("function") {
                let func_text = function_node.utf8_text(source.as_bytes()).unwrap_or("");
                Ok(func_text == func_name)
            } else {
                // Fallback: check first child
                if let Some(first_child) = node.child(0) {
                    let func_text = first_child.utf8_text(source.as_bytes()).unwrap_or("");
                    Ok(func_text == func_name)
                } else {
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Match import statement nodes
    pub(super) fn match_import_statement(
        &self,
        node: &Node,
        source: &str,
        import_spec: &str,
    ) -> Result<bool> {
        if matches!(node.kind(), "import_statement" | "import_from_statement") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");

            // Extract module path from import specification
            if let Some(module_path) = self.extract_module_path_from_pattern(import_spec) {
                // Check if import statement contains this module path
                Ok(node_text.contains(&module_path))
            } else {
                // Fallback to exact match
                Ok(node_text.trim() == import_spec.trim())
            }
        } else {
            Ok(false)
        }
    }

    /// Extract module path from import pattern (e.g., "foo.bar" from "import foo.bar")
    fn extract_module_path_from_pattern(&self, pattern: &str) -> Option<String> {
        let pattern = pattern.trim();
        if let Some(module_part) = pattern.strip_prefix("import ") {
            let module_path = module_part.split_whitespace().next().unwrap_or("");
            if !module_path.is_empty() {
                Some(module_path.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Match method call nodes (e.g., System.out.println or $obj.method)
    pub(super) fn match_method_call(
        &self,
        node: &Node,
        source: &str,
        object: &str,
        method: &str,
    ) -> Result<bool> {
        // Check if this is a method invocation node (Java)
        if node.kind() == "method_invocation" {
            // For Java method invocations, check method name
            if let Some(name_node) = node.child_by_field_name("name") {
                let method_name = name_node.utf8_text(source.as_bytes()).unwrap_or("");

                // If method doesn't match, return false
                if method_name != method {
                    return Ok(false);
                }

                // If object is a metavariable, any object matches
                if object.starts_with('$') {
                    return Ok(true);
                }

                // Otherwise, check if object matches
                if let Some(object_node) = node.child_by_field_name("object") {
                    let object_text = object_node.utf8_text(source.as_bytes()).unwrap_or("");
                    return Ok(object_text == object);
                }

                return Ok(false);
            }
        }

        // For other languages (Python, JavaScript, etc.)
        if matches!(node.kind(), "call_expression" | "call") {
            // Check if this is a method call on specified object
            if let Some(function_node) = node.child_by_field_name("function") {
                if matches!(
                    function_node.kind(),
                    "attribute" | "member_expression" | "field_expression"
                ) {
                    let func_text = function_node.utf8_text(source.as_bytes()).unwrap_or("");

                    // If object is a metavariable, match any object with specified method
                    if object.starts_with('$') {
                        // Extract method name from function text
                        if let Some(dot_pos) = func_text.rfind('.') {
                            let actual_method = &func_text[dot_pos + 1..];
                            return Ok(actual_method == method);
                        }
                        return Ok(false);
                    }

                    // Otherwise, exact match
                    let expected = format!("{}.{}", object, method);
                    Ok(func_text == expected)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Match identifier nodes
    pub(super) fn match_identifier(&self, node: &Node, source: &str, name: &str) -> Result<bool> {
        if matches!(node.kind(), "identifier") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            Ok(node_text == name)
        } else {
            Ok(false)
        }
    }

    /// Match metavariable patterns
    pub(super) fn match_metavariable(
        &self,
        node: &Node,
        source: &str,
        var_name: &str,
        bindings: &mut MetaVariableBindings,
    ) -> Result<bool> {
        let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");

        // Try to bind metavariable
        if bindings.bind(var_name, node_text) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Match meta function call patterns
    pub(super) fn match_meta_function_call(
        &self,
        node: &Node,
        source: &str,
        func_name: &str,
        args: &[String],
        bindings: &mut MetaVariableBindings,
    ) -> Result<bool> {
        if !matches!(
            node.kind(),
            "call_expression" | "call" | "method_invocation"
        ) {
            return Ok(false);
        }

        // Get function name - handle different node types
        let actual_func_text = if node.kind() == "method_invocation" {
            // For Java method invocations, we need to get full method chain
            // e.g., System.err.print -> we want full chain
            let mut parts = Vec::new();

            // Get object part (e.g., System.err)
            if let Some(object_node) = node.child_by_field_name("object") {
                parts.push(object_node.utf8_text(source.as_bytes()).unwrap_or(""));
            }

            // Get method name
            if let Some(name_node) = node.child_by_field_name("name") {
                parts.push(name_node.utf8_text(source.as_bytes()).unwrap_or(""));
            }

            if parts.len() == 2 {
                format!("{}.{}", parts[0], parts[1])
            } else {
                // Fallback: get entire node text up to opening parenthesis
                let full_text = node.utf8_text(source.as_bytes()).unwrap_or("");
                if let Some(paren_pos) = full_text.find('(') {
                    full_text[..paren_pos].to_string()
                } else {
                    full_text.to_string()
                }
            }
        } else {
            // For other call types, use function field
            if let Some(function_node) = node.child_by_field_name("function") {
                function_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string()
            } else {
                "".to_string()
            }
        };

        // If func_name is a metavariable, try to bind it
        if func_name.starts_with('$') {
            if !bindings.bind(func_name, &actual_func_text) {
                return Ok(false);
            }
        } else if actual_func_text != func_name {
            return Ok(false);
        }

        // Match arguments if specified
        if !args.is_empty() {
            return self.match_function_arguments(node, source, args, bindings);
        }

        Ok(true)
    }

    /// Match function arguments
    fn match_function_arguments(
        &self,
        node: &Node,
        source: &str,
        expected_args: &[String],
        bindings: &mut MetaVariableBindings,
    ) -> Result<bool> {
        // Get arguments node
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut actual_args = Vec::new();

            // Collect actual arguments
            for i in 0..args_node.child_count() {
                if let Some(arg_node) = args_node.child(i) {
                    if arg_node.kind() != "," && arg_node.kind() != "(" && arg_node.kind() != ")" {
                        // Skip separators
                        let arg_text = arg_node.utf8_text(source.as_bytes()).unwrap_or("");
                        actual_args.push(arg_text);
                    }
                }
            }

            // Check if argument count matches
            if actual_args.len() != expected_args.len() {
                return Ok(false);
            }

            // Match each argument
            for (expected, actual) in expected_args.iter().zip(actual_args.iter()) {
                if expected.starts_with('$') {
                    // Metavariable argument
                    if !bindings.bind(expected, actual) {
                        return Ok(false);
                    }
                } else if expected != actual {
                    return Ok(false);
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
