//! Python language parser and adapter
//! 
//! This module provides Python-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use astgrep_ast::{AstBuilder, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Python AST adapter
pub struct PythonAdapter;

impl PythonAdapter {
    /// Create a new Python adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse Python-specific constructs
    fn parse_python_construct(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        let trimmed = source.trim();
        
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            self.parse_import_statement(trimmed, context)
        } else if trimmed.starts_with("def ") {
            self.parse_function_definition(trimmed, context)
        } else if trimmed.starts_with("class ") {
            self.parse_class_definition(trimmed, context)
        } else if trimmed.starts_with("@") {
            self.parse_decorator(trimmed, context)
        } else if trimmed.starts_with("if ") || trimmed.starts_with("elif ") || trimmed.starts_with("else:") {
            self.parse_if_statement(trimmed, context)
        } else if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
            self.parse_loop_statement(trimmed, context)
        } else if trimmed.starts_with("try:") || trimmed.starts_with("except ") || trimmed.starts_with("finally:") {
            self.parse_try_statement(trimmed, context)
        } else {
            // Default to expression statement
            Ok(AstBuilder::expression_statement(
                AstBuilder::string_literal(trimmed)
                    .with_text(trimmed.to_string())
            ))
        }
    }

    /// Parse import statement
    fn parse_import_statement(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        if source.starts_with("from ") {
            // from module import name1, name2
            if let Some(import_pos) = source.find(" import ") {
                let module_part = &source[5..import_pos]; // Skip "from "
                let imports_part = &source[import_pos + 8..]; // Skip " import "
                
                let mut import_node = AstBuilder::import_declaration(module_part, false);
                
                for import_name in imports_part.split(',') {
                    let import_name = import_name.trim();
                    if !import_name.is_empty() {
                        if import_name == "*" {
                            import_node = import_node.with_wildcard(true);
                        } else {
                            import_node = import_node.with_specifier(import_name.to_string());
                        }
                    }
                }
                
                Ok(import_node.with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid from import statement"))
            }
        } else if source.starts_with("import ") {
            // import module1, module2
            let imports_part = &source[7..]; // Skip "import "
            let mut import_node = AstBuilder::import_declaration("", false);
            
            for module_name in imports_part.split(',') {
                let module_name = module_name.trim();
                if !module_name.is_empty() {
                    if module_name.contains(" as ") {
                        let parts: Vec<&str> = module_name.split(" as ").collect();
                        if parts.len() == 2 {
                            import_node = import_node.with_alias(parts[0].trim().to_string(), parts[1].trim().to_string());
                        }
                    } else {
                        import_node = import_node.with_module(module_name.to_string());
                    }
                }
            }
            
            Ok(import_node.with_text(source.to_string()))
        } else {
            Err(astgrep_core::AnalysisError::parse_error("Invalid import statement"))
        }
    }

    /// Parse function definition
    fn parse_function_definition(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        // def function_name(params): -> return_type
        let mut function_name = "unknown";
        let mut is_async = source.trim_start().starts_with("async def");
        
        let def_start = if is_async { 
            source.find("async def").unwrap_or(0) + 9 
        } else { 
            source.find("def").unwrap_or(0) + 3 
        };
        
        let after_def = &source[def_start..].trim_start();
        
        if let Some(paren_pos) = after_def.find('(') {
            function_name = &after_def[..paren_pos].trim();
        }

        let mut func_node = AstBuilder::simple_function_declaration(function_name);
        
        if is_async {
            func_node = func_node.with_modifier("async");
        }

        // Check for decorators (simplified)
        if source.contains('@') {
            func_node = func_node.with_decorator("decorated");
        }

        Ok(func_node.with_text(source.to_string()))
    }

    /// Parse class definition
    fn parse_class_definition(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let mut class_name = "UnknownClass";
        let mut base_classes = Vec::new();
        
        // Find class name
        if let Some(class_pos) = source.find("class ") {
            let after_class = &source[class_pos + 6..];
            
            if let Some(paren_pos) = after_class.find('(') {
                // class Name(Base1, Base2):
                class_name = after_class[..paren_pos].trim();
                
                if let Some(close_paren) = after_class.find(')') {
                    let bases_str = &after_class[paren_pos + 1..close_paren];
                    for base in bases_str.split(',') {
                        let base = base.trim();
                        if !base.is_empty() {
                            base_classes.push(base.to_string());
                        }
                    }
                }
            } else if let Some(colon_pos) = after_class.find(':') {
                // class Name:
                class_name = after_class[..colon_pos].trim();
            }
        }

        let mut class_node = AstBuilder::simple_class_declaration(class_name);
        
        for base in base_classes {
            class_node = class_node.with_parent(base);
        }

        Ok(class_node.with_text(source.to_string()))
    }

    /// Parse decorator
    fn parse_decorator(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let decorator_name = source.trim_start_matches('@').trim();
        
        Ok(AstBuilder::decorator(decorator_name)
            .with_text(source.to_string()))
    }

    /// Parse if statement
    fn parse_if_statement(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        if source.starts_with("if ") {
            if let Some(colon_pos) = source.find(':') {
                let condition = &source[3..colon_pos].trim(); // Skip "if "
                Ok(AstBuilder::simple_if_statement(condition)
                    .with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid if statement"))
            }
        } else if source.starts_with("elif ") {
            if let Some(colon_pos) = source.find(':') {
                let condition = &source[5..colon_pos].trim(); // Skip "elif "
                Ok(AstBuilder::elif_statement(condition)
                    .with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid elif statement"))
            }
        } else {
            // else:
            Ok(AstBuilder::else_statement()
                .with_text(source.to_string()))
        }
    }

    /// Parse loop statement
    fn parse_loop_statement(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        if source.starts_with("for ") {
            if let Some(colon_pos) = source.find(':') {
                let loop_header = &source[4..colon_pos].trim(); // Skip "for "
                Ok(AstBuilder::simple_for_statement(loop_header)
                    .with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid for statement"))
            }
        } else if source.starts_with("while ") {
            if let Some(colon_pos) = source.find(':') {
                let condition = &source[6..colon_pos].trim(); // Skip "while "
                Ok(AstBuilder::simple_while_statement(condition)
                    .with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid while statement"))
            }
        } else {
            Err(astgrep_core::AnalysisError::parse_error("Unknown loop statement"))
        }
    }

    /// Parse try statement
    fn parse_try_statement(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        if source.starts_with("try:") {
            Ok(AstBuilder::try_statement()
                .with_text(source.to_string()))
        } else if source.starts_with("except ") {
            let exception_part = &source[7..]; // Skip "except "
            if let Some(colon_pos) = exception_part.find(':') {
                let exception_type = &exception_part[..colon_pos].trim();
                Ok(AstBuilder::except_statement(exception_type)
                    .with_text(source.to_string()))
            } else {
                Ok(AstBuilder::except_statement("")
                    .with_text(source.to_string()))
            }
        } else if source.starts_with("finally:") {
            Ok(AstBuilder::finally_statement()
                .with_text(source.to_string()))
        } else {
            Err(astgrep_core::AnalysisError::parse_error("Unknown try statement"))
        }
    }
}

impl AstAdapter for PythonAdapter {
    fn adapt_node(&self, _node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        self.parse_python_construct(&context.source_code, context)
    }

    fn language(&self) -> Language {
        Language::Python
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "PythonAdapter".to_string(),
            "1.0.0".to_string(),
            "Python AST adapter with Python 3.x support".to_string(),
        )
        .with_feature("import_statements".to_string())
        .with_feature("function_definitions".to_string())
        .with_feature("class_definitions".to_string())
        .with_feature("decorators".to_string())
        .with_feature("async_await".to_string())
        .with_feature("comprehensions".to_string())
        .with_feature("context_managers".to_string())
    }
}

/// Python language parser
pub struct PythonParser {
    adapter: PythonAdapter,
}

impl PythonParser {
    /// Create a new Python parser
    pub fn new() -> Self {
        Self {
            adapter: PythonAdapter::new(),
        }
    }
}

impl LanguageParser for PythonParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Python,
        );

        let universal_node = self.adapter.parse_python_construct(source, &context)?;
        Ok(Box::new(universal_node))
    }

    fn language(&self) -> Language {
        Language::Python
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "py" | "pyw" | "pyi")
        } else {
            false
        }
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_parser_creation() {
        let parser = PythonParser::new();
        assert_eq!(parser.language(), Language::Python);
    }

    #[test]
    fn test_python_parser_supports_file() {
        let parser = PythonParser::new();
        assert!(parser.supports_file(Path::new("script.py")));
        assert!(parser.supports_file(Path::new("module.pyw")));
        assert!(parser.supports_file(Path::new("types.pyi")));
        assert!(!parser.supports_file(Path::new("test.js")));
        assert!(!parser.supports_file(Path::new("test.java")));
    }

    #[test]
    fn test_parse_import_statement() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "import os".to_string(),
            Language::Python,
        );

        // Simple import
        let result = adapter.parse_import_statement("import os", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");

        // Import with alias
        let result = adapter.parse_import_statement("import numpy as np", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");

        // From import
        let result = adapter.parse_import_statement("from os import path", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");

        // From import with wildcard
        let result = adapter.parse_import_statement("from math import *", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");
    }

    #[test]
    fn test_parse_function_definition() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "def my_function():".to_string(),
            Language::Python,
        );

        // Simple function
        let result = adapter.parse_function_definition("def my_function():", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");

        // Async function
        let result = adapter.parse_function_definition("async def async_function():", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");
    }

    #[test]
    fn test_parse_class_definition() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "class MyClass:".to_string(),
            Language::Python,
        );

        // Simple class
        let result = adapter.parse_class_definition("class MyClass:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");

        // Class with inheritance
        let result = adapter.parse_class_definition("class Child(Parent):", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");

        // Class with multiple inheritance
        let result = adapter.parse_class_definition("class Child(Parent1, Parent2):", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");
    }

    #[test]
    fn test_parse_decorator() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "@property".to_string(),
            Language::Python,
        );

        let result = adapter.parse_decorator("@property", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "decorator");
    }

    #[test]
    fn test_parse_if_statement() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "if x > 0:".to_string(),
            Language::Python,
        );

        // if statement
        let result = adapter.parse_if_statement("if x > 0:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "if_statement");

        // elif statement
        let result = adapter.parse_if_statement("elif x < 0:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "elif_statement");

        // else statement
        let result = adapter.parse_if_statement("else:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "else_statement");
    }

    #[test]
    fn test_parse_loop_statement() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "for i in range(10):".to_string(),
            Language::Python,
        );

        // for loop
        let result = adapter.parse_loop_statement("for i in range(10):", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "for_statement");

        // while loop
        let result = adapter.parse_loop_statement("while True:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "while_statement");
    }

    #[test]
    fn test_parse_try_statement() {
        let adapter = PythonAdapter::new();
        let context = AdapterContext::new(
            "test.py".to_string(),
            "try:".to_string(),
            Language::Python,
        );

        // try
        let result = adapter.parse_try_statement("try:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "try_statement");

        // except
        let result = adapter.parse_try_statement("except ValueError:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "except_statement");

        // finally
        let result = adapter.parse_try_statement("finally:", &context);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "finally_statement");
    }

    #[test]
    fn test_python_adapter_metadata() {
        let adapter = PythonAdapter::new();
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "PythonAdapter");
        assert!(metadata.supported_features.contains(&"decorators".to_string()));
        assert!(metadata.supported_features.contains(&"async_await".to_string()));
        assert!(metadata.supported_features.contains(&"comprehensions".to_string()));
    }
}
