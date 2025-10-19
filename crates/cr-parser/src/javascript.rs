//! JavaScript language parser and adapter
//! 
//! This module provides JavaScript-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter, BaseParser};
use cr_ast::{AstBuilder, UniversalNode};
use cr_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// JavaScript AST adapter
pub struct JavaScriptAdapter;

impl JavaScriptAdapter {
    /// Create a new JavaScript adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse JavaScript-specific constructs
    fn parse_js_construct(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        let trimmed = source.trim();
        
        if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
            self.parse_module_statement(trimmed, context)
        } else if trimmed.starts_with("function ") || trimmed.contains(" function ") {
            self.parse_function_declaration(trimmed, context)
        } else if trimmed.starts_with("class ") {
            self.parse_class_declaration(trimmed, context)
        } else if trimmed.starts_with("const ") || trimmed.starts_with("let ") || trimmed.starts_with("var ") {
            self.parse_variable_declaration(trimmed, context)
        } else if trimmed.contains("=>") {
            self.parse_arrow_function(trimmed, context)
        } else {
            // Default to expression statement
            Ok(AstBuilder::expression_statement(
                AstBuilder::string_literal(trimmed)
                    .with_text(trimmed.to_string())
            ))
        }
    }

    /// Parse import/export statements
    fn parse_module_statement(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        if source.starts_with("import ") {
            self.parse_import_statement(source)
        } else if source.starts_with("export ") {
            self.parse_export_statement(source)
        } else {
            Err(cr_core::AnalysisError::parse_error("Invalid module statement"))
        }
    }

    /// Parse import statement
    fn parse_import_statement(&self, source: &str) -> Result<UniversalNode> {
        let import_line = source.trim_end_matches(';');
        
        if let Some(from_pos) = import_line.find(" from ") {
            let import_part = &import_line[6..from_pos].trim(); // Skip "import "
            let module_part = &import_line[from_pos + 6..].trim(); // Skip " from "
            
            // Remove quotes from module path
            let module_path = module_part.trim_matches('"').trim_matches('\'');
            
            // Parse import specifiers (simplified)
            let mut import_node = AstBuilder::import_declaration(module_path, false);
            
            if import_part.starts_with('{') && import_part.ends_with('}') {
                // Named imports: import { a, b } from 'module'
                let specifiers = &import_part[1..import_part.len()-1];
                for spec in specifiers.split(',') {
                    let spec = spec.trim();
                    if !spec.is_empty() {
                        import_node = import_node.with_specifier(spec.to_string());
                    }
                }
            } else if import_part.contains(" as ") {
                // Namespace import: import * as name from 'module'
                import_node = import_node.with_namespace(import_part.to_string());
            } else {
                // Default import: import name from 'module'
                import_node = import_node.with_default(import_part.to_string());
            }
            
            Ok(import_node.with_text(source.to_string()))
        } else {
            // Side-effect import: import 'module'
            let module_path = import_line[6..].trim().trim_matches('"').trim_matches('\'');
            Ok(AstBuilder::import_declaration(module_path, false)
                .with_text(source.to_string()))
        }
    }

    /// Parse export statement
    fn parse_export_statement(&self, source: &str) -> Result<UniversalNode> {
        let export_line = source.trim_end_matches(';');
        
        if export_line.starts_with("export default ") {
            // Default export
            let exported = &export_line[15..]; // Skip "export default "
            Ok(AstBuilder::export_declaration(exported, true)
                .with_text(source.to_string()))
        } else if export_line.starts_with("export {") {
            // Named exports
            let end_brace = export_line.find('}').unwrap_or(export_line.len());
            let specifiers = &export_line[8..end_brace]; // Skip "export {"
            
            let mut export_node = AstBuilder::export_declaration("", false);
            for spec in specifiers.split(',') {
                let spec = spec.trim();
                if !spec.is_empty() {
                    export_node = export_node.with_specifier(spec.to_string());
                }
            }
            
            Ok(export_node.with_text(source.to_string()))
        } else {
            // Export declaration
            let exported = &export_line[7..]; // Skip "export "
            Ok(AstBuilder::export_declaration(exported, false)
                .with_text(source.to_string()))
        }
    }

    /// Parse function declaration
    fn parse_function_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let mut function_name = "anonymous";
        let mut is_async = source.contains("async ");
        let mut is_generator = source.contains("function*");
        
        // Find function name
        if let Some(func_pos) = source.find("function") {
            let after_function = &source[func_pos + 8..]; // Skip "function"
            if is_generator {
                let after_star = after_function.trim_start_matches('*');
                if let Some(paren_pos) = after_star.find('(') {
                    function_name = after_star[..paren_pos].trim();
                }
            } else {
                if let Some(paren_pos) = after_function.find('(') {
                    function_name = after_function[..paren_pos].trim();
                }
            }
        }
        
        if function_name.is_empty() {
            function_name = "anonymous";
        }

        let mut func_node = AstBuilder::simple_function_declaration(function_name);
        
        if is_async {
            func_node = func_node.with_modifier("async");
        }
        if is_generator {
            func_node = func_node.with_modifier("generator");
        }

        Ok(func_node.with_text(source.to_string()))
    }

    /// Parse class declaration
    fn parse_class_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let mut class_name = "UnknownClass";
        let mut extends_class = None;
        
        // Find class name
        if let Some(class_pos) = source.find("class ") {
            let after_class = &source[class_pos + 6..];
            if let Some(space_or_brace) = after_class.find(|c: char| c.is_whitespace() || c == '{') {
                class_name = &after_class[..space_or_brace];
            }
        }

        // Check for extends
        if let Some(extends_pos) = source.find(" extends ") {
            let after_extends = &source[extends_pos + 9..];
            if let Some(space_or_brace) = after_extends.find(|c: char| c.is_whitespace() || c == '{') {
                extends_class = Some(after_extends[..space_or_brace].to_string());
            }
        }

        let mut class_node = AstBuilder::simple_class_declaration(class_name);
        
        if let Some(parent) = extends_class {
            class_node = class_node.with_parent(parent);
        }

        Ok(class_node.with_text(source.to_string()))
    }

    /// Parse variable declaration
    fn parse_variable_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let mut var_type = "var";
        let mut var_name = "unknown";
        
        if source.starts_with("const ") {
            var_type = "const";
            let after_const = &source[6..];
            if let Some(eq_pos) = after_const.find('=') {
                var_name = after_const[..eq_pos].trim();
            } else if let Some(space_pos) = after_const.find(' ') {
                var_name = after_const[..space_pos].trim();
            }
        } else if source.starts_with("let ") {
            var_type = "let";
            let after_let = &source[4..];
            if let Some(eq_pos) = after_let.find('=') {
                var_name = after_let[..eq_pos].trim();
            } else if let Some(space_pos) = after_let.find(' ') {
                var_name = after_let[..space_pos].trim();
            }
        } else if source.starts_with("var ") {
            var_type = "var";
            let after_var = &source[4..];
            if let Some(eq_pos) = after_var.find('=') {
                var_name = after_var[..eq_pos].trim();
            } else if let Some(space_pos) = after_var.find(' ') {
                var_name = after_var[..space_pos].trim();
            }
        }

        Ok(AstBuilder::variable_declaration(var_name, None)
            .with_attribute("var_type".to_string(), var_type.to_string())
            .with_text(source.to_string()))
    }

    /// Parse arrow function
    fn parse_arrow_function(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let arrow_pos = source.find("=>").unwrap_or(0);
        let params_part = source[..arrow_pos].trim();
        
        // Extract parameters (simplified)
        let params = if params_part.starts_with('(') && params_part.ends_with(')') {
            &params_part[1..params_part.len()-1]
        } else {
            params_part
        };

        let mut arrow_func = AstBuilder::arrow_function();
        
        if !params.is_empty() {
            for param in params.split(',') {
                let param = param.trim();
                if !param.is_empty() {
                    arrow_func = arrow_func.with_parameter(param.to_string());
                }
            }
        }

        Ok(arrow_func.with_text(source.to_string()))
    }
}

impl AstAdapter for JavaScriptAdapter {
    fn adapt_node(&self, _node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        self.parse_js_construct(&context.source_code, context)
    }

    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "JavaScriptAdapter".to_string(),
            "1.0.0".to_string(),
            "JavaScript AST adapter with ES6+ support".to_string(),
        )
        .with_feature("import_export".to_string())
        .with_feature("arrow_functions".to_string())
        .with_feature("classes".to_string())
        .with_feature("async_await".to_string())
        .with_feature("destructuring".to_string())
        .with_feature("template_literals".to_string())
    }
}

/// JavaScript language parser
pub struct JavaScriptParser {
    adapter: JavaScriptAdapter,
}

impl JavaScriptParser {
    /// Create a new JavaScript parser
    pub fn new() -> Self {
        Self {
            adapter: JavaScriptAdapter::new(),
        }
    }
}

impl LanguageParser for JavaScriptParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        // Try to use tree-sitter parser first for better AST structure
        if let Ok(mut ts_parser) = crate::tree_sitter_parser::TreeSitterParser::new() {
            if let Ok(Some(tree)) = ts_parser.parse(source, Language::JavaScript) {
                if let Ok(universal_node) = ts_parser.tree_to_universal_ast(&tree, source) {
                    return Ok(Box::new(universal_node));
                }
            }
        }

        // Fallback to simple adapter-based parsing
        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::JavaScript,
        );

        let universal_node = self.adapter.parse_js_construct(source, &context)?;
        Ok(Box::new(universal_node))
    }

    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs")
        } else {
            false
        }
    }
}

impl Default for JavaScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_parser_creation() {
        let parser = JavaScriptParser::new();
        assert_eq!(parser.language(), Language::JavaScript);
    }

    #[test]
    fn test_javascript_parser_supports_file() {
        let parser = JavaScriptParser::new();
        assert!(parser.supports_file(Path::new("app.js")));
        assert!(parser.supports_file(Path::new("component.jsx")));
        assert!(parser.supports_file(Path::new("types.ts")));
        assert!(parser.supports_file(Path::new("component.tsx")));
        assert!(!parser.supports_file(Path::new("test.py")));
        assert!(!parser.supports_file(Path::new("test.java")));
    }

    #[test]
    fn test_parse_import_statement() {
        let adapter = JavaScriptAdapter::new();
        
        // Named import
        let result = adapter.parse_import_statement("import { useState, useEffect } from 'react';");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");

        // Default import
        let result = adapter.parse_import_statement("import React from 'react';");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");

        // Namespace import
        let result = adapter.parse_import_statement("import * as utils from './utils';");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");
    }

    #[test]
    fn test_parse_export_statement() {
        let adapter = JavaScriptAdapter::new();
        
        // Default export
        let result = adapter.parse_export_statement("export default MyComponent;");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "export_declaration");

        // Named export
        let result = adapter.parse_export_statement("export { useState, useEffect };");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "export_declaration");
    }

    #[test]
    fn test_parse_function_declaration() {
        let adapter = JavaScriptAdapter::new();
        let context = AdapterContext::new(
            "test.js".to_string(),
            "function myFunction() {}".to_string(),
            Language::JavaScript,
        );

        let result = adapter.parse_function_declaration("function myFunction() {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");

        // Async function
        let result = adapter.parse_function_declaration("async function fetchData() {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");

        // Generator function
        let result = adapter.parse_function_declaration("function* generator() {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");
    }

    #[test]
    fn test_parse_class_declaration() {
        let adapter = JavaScriptAdapter::new();
        let context = AdapterContext::new(
            "test.js".to_string(),
            "class MyClass {}".to_string(),
            Language::JavaScript,
        );

        let result = adapter.parse_class_declaration("class MyClass {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");

        // Class with extends
        let result = adapter.parse_class_declaration("class Child extends Parent {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");
    }

    #[test]
    fn test_parse_variable_declaration() {
        let adapter = JavaScriptAdapter::new();
        let context = AdapterContext::new(
            "test.js".to_string(),
            "const x = 5;".to_string(),
            Language::JavaScript,
        );

        // const
        let result = adapter.parse_variable_declaration("const x = 5;", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "variable_declaration");

        // let
        let result = adapter.parse_variable_declaration("let y = 10;", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "variable_declaration");

        // var
        let result = adapter.parse_variable_declaration("var z;", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "variable_declaration");
    }

    #[test]
    fn test_parse_arrow_function() {
        let adapter = JavaScriptAdapter::new();
        let context = AdapterContext::new(
            "test.js".to_string(),
            "const fn = () => {}".to_string(),
            Language::JavaScript,
        );

        // No parameters
        let result = adapter.parse_arrow_function("() => {}", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "arrow_function");

        // Single parameter
        let result = adapter.parse_arrow_function("x => x * 2", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "arrow_function");

        // Multiple parameters
        let result = adapter.parse_arrow_function("(a, b) => a + b", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "arrow_function");
    }

    #[test]
    fn test_javascript_adapter_metadata() {
        let adapter = JavaScriptAdapter::new();
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "JavaScriptAdapter");
        assert!(metadata.supported_features.contains(&"arrow_functions".to_string()));
        assert!(metadata.supported_features.contains(&"import_export".to_string()));
        assert!(metadata.supported_features.contains(&"classes".to_string()));
    }
}
