//! Java language parser and adapter
//! 
//! This module provides Java-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter, BaseParser};
use cr_ast::{AstBuilder, UniversalNode, NodeType};
use cr_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Java AST adapter
pub struct JavaAdapter;

impl JavaAdapter {
    /// Create a new Java adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse Java-specific constructs
    fn parse_java_construct(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // Simplified Java parsing - in reality would use tree-sitter-java
        let trimmed = source.trim();

        // For multi-line source, try to identify the main construct
        if trimmed.contains("class ") {
            self.parse_class_declaration(source, context)
        } else if trimmed.starts_with("package ") {
            self.parse_package_declaration(source, context)
        } else if trimmed.starts_with("import ") {
            self.parse_import_declaration(source, context)
        } else if trimmed.contains("public ") || trimmed.contains("private ") || trimmed.contains("protected ") {
            self.parse_method_or_field(source, context)
        } else {
            // Default to program with the source as content
            Ok(AstBuilder::program(vec![
                AstBuilder::expression_statement(
                    AstBuilder::string_literal(trimmed)
                        .with_text(trimmed.to_string())
                )
            ]).with_text(source.to_string()))
        }
    }

    /// Parse package declaration
    fn parse_package_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let package_line = source.lines().next().unwrap_or("").trim();
        if let Some(package_name) = package_line.strip_prefix("package ").and_then(|s| s.strip_suffix(";")) {
            Ok(AstBuilder::package_declaration(package_name.trim()))
        } else {
            Err(cr_core::AnalysisError::parse_error("Invalid package declaration"))
        }
    }

    /// Parse import declaration
    fn parse_import_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let import_line = source.lines().next().unwrap_or("").trim();
        if let Some(import_path) = import_line.strip_prefix("import ").and_then(|s| s.strip_suffix(";")) {
            let is_static = import_path.starts_with("static ");
            let path = if is_static {
                import_path.strip_prefix("static ").unwrap_or(import_path)
            } else {
                import_path
            };
            
            Ok(AstBuilder::import_declaration(path.trim(), is_static))
        } else {
            Err(cr_core::AnalysisError::parse_error("Invalid import declaration"))
        }
    }

    /// Parse class declaration
    fn parse_class_declaration(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        // Very simplified class parsing
        let lines: Vec<&str> = source.lines().collect();
        let mut class_name = "UnknownClass";
        let mut is_public = false;
        let mut is_abstract = false;
        let mut extends_class = None;
        let mut implements_interfaces = Vec::new();

        // Find class declaration line
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.contains("class ") {
                is_public = trimmed.contains("public ");
                is_abstract = trimmed.contains("abstract ");
                
                // Extract class name (simplified)
                if let Some(class_start) = trimmed.find("class ") {
                    let after_class = &trimmed[class_start + 6..];
                    if let Some(name_end) = after_class.find(|c: char| c.is_whitespace() || c == '{' || c == '<') {
                        class_name = &after_class[..name_end];
                    } else {
                        class_name = after_class.trim_end_matches('{').trim();
                    }
                }

                // Check for extends
                if let Some(extends_start) = trimmed.find(" extends ") {
                    let after_extends = &trimmed[extends_start + 9..];
                    if let Some(extends_end) = after_extends.find(|c: char| c.is_whitespace() || c == '{' || c == '<') {
                        extends_class = Some(after_extends[..extends_end].to_string());
                    }
                }

                // Check for implements
                if let Some(implements_start) = trimmed.find(" implements ") {
                    let after_implements = &trimmed[implements_start + 12..];
                    let interfaces_str = after_implements.split('{').next().unwrap_or("").trim();
                    implements_interfaces = interfaces_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                break;
            }
        }

        // Create class node
        let mut class_node = AstBuilder::simple_class_declaration(class_name);
        
        if is_public {
            class_node = class_node.with_modifier("public");
        }
        if is_abstract {
            class_node = class_node.with_modifier("abstract");
        }
        if let Some(parent) = extends_class {
            class_node = class_node.with_parent(parent);
        }
        for interface in implements_interfaces {
            class_node = class_node.with_interface(interface);
        }

        Ok(class_node.with_text(source.to_string()))
    }

    /// Parse method or field declaration
    fn parse_method_or_field(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        // Very simplified method/field parsing
        let trimmed = source.trim();
        
        if trimmed.contains('(') && trimmed.contains(')') {
            // Likely a method
            self.parse_method_declaration(trimmed)
        } else {
            // Likely a field
            self.parse_field_declaration(trimmed)
        }
    }

    /// Parse method declaration
    fn parse_method_declaration(&self, source: &str) -> Result<UniversalNode> {
        let mut method_name = "unknownMethod";
        let mut return_type = "void";
        let mut is_public = source.contains("public ");
        let mut is_private = source.contains("private ");
        let mut is_static = source.contains("static ");

        // Extract method name (simplified)
        if let Some(paren_pos) = source.find('(') {
            let before_paren = &source[..paren_pos];
            if let Some(name_start) = before_paren.rfind(' ') {
                method_name = before_paren[name_start + 1..].trim();
                
                // Extract return type
                let before_name = &before_paren[..name_start];
                if let Some(type_start) = before_name.rfind(' ') {
                    return_type = before_name[type_start + 1..].trim();
                }
            }
        }

        let mut method_node = UniversalNode::new(NodeType::MethodDeclaration)
            .with_identifier(method_name.to_string())
            .with_attribute("return_type".to_string(), return_type.to_string());
        
        if is_public {
            method_node = method_node.with_modifier("public");
        }
        if is_private {
            method_node = method_node.with_modifier("private");
        }
        if is_static {
            method_node = method_node.with_modifier("static");
        }

        Ok(method_node.with_text(source.to_string()))
    }

    /// Parse field declaration
    fn parse_field_declaration(&self, source: &str) -> Result<UniversalNode> {
        let mut field_name = "unknownField";
        let mut field_type = "Object";
        let is_public = source.contains("public ");
        let is_private = source.contains("private ");
        let is_static = source.contains("static ");
        let is_final = source.contains("final ");

        // Extract field name and type (simplified)
        let parts: Vec<&str> = source.split_whitespace().collect();
        if parts.len() >= 2 {
            // Find type and name
            let mut type_index = 0;
            for (i, part) in parts.iter().enumerate() {
                if !["public", "private", "protected", "static", "final"].contains(part) {
                    type_index = i;
                    break;
                }
            }
            
            if type_index < parts.len() - 1 {
                field_type = parts[type_index];
                field_name = parts[type_index + 1].trim_end_matches(';').trim_end_matches('=');
                if let Some(eq_pos) = field_name.find('=') {
                    field_name = &field_name[..eq_pos].trim();
                }
            }
        }

        let mut field_node = AstBuilder::field_declaration(field_name, field_type);
        
        if is_public {
            field_node = field_node.with_modifier("public");
        }
        if is_private {
            field_node = field_node.with_modifier("private");
        }
        if is_static {
            field_node = field_node.with_modifier("static");
        }
        if is_final {
            field_node = field_node.with_modifier("final");
        }

        Ok(field_node.with_text(source.to_string()))
    }
}

impl AstAdapter for JavaAdapter {
    fn adapt_node(&self, _node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        // In a real implementation, this would convert tree-sitter Java nodes
        // For now, we'll parse the source directly
        self.parse_java_construct(&context.source_code, context)
    }

    fn language(&self) -> Language {
        Language::Java
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "JavaAdapter".to_string(),
            "1.0.0".to_string(),
            "Java AST adapter using simplified parsing".to_string(),
        )
        .with_feature("package_declarations".to_string())
        .with_feature("import_declarations".to_string())
        .with_feature("class_declarations".to_string())
        .with_feature("method_declarations".to_string())
        .with_feature("field_declarations".to_string())
        .with_feature("modifiers".to_string())
    }
}

/// Java language parser
pub struct JavaParser {
    adapter: JavaAdapter,
}

impl JavaParser {
    /// Create a new Java parser
    pub fn new() -> Self {
        Self {
            adapter: JavaAdapter::new(),
        }
    }

    /// Parse Java source code
    fn parse_java_source(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Java,
        );

        let universal_node = self.adapter.parse_java_construct(source, &context)?;
        Ok(Box::new(universal_node))
    }
}

impl LanguageParser for JavaParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        self.parse_java_source(source, file_path)
    }

    fn language(&self) -> Language {
        Language::Java
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("java"))
            .unwrap_or(false)
    }
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_parser_creation() {
        let parser = JavaParser::new();
        assert_eq!(parser.language(), Language::Java);
    }

    #[test]
    fn test_java_parser_supports_file() {
        let parser = JavaParser::new();
        assert!(parser.supports_file(Path::new("Test.java")));
        assert!(parser.supports_file(Path::new("com/example/Test.java")));
        assert!(!parser.supports_file(Path::new("test.py")));
        assert!(!parser.supports_file(Path::new("test.js")));
    }

    #[test]
    fn test_parse_package_declaration() {
        let adapter = JavaAdapter::new();
        let context = AdapterContext::new(
            "Test.java".to_string(),
            "package com.example;".to_string(),
            Language::Java,
        );

        let result = adapter.parse_package_declaration("package com.example;", &context);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "package_declaration");
    }

    #[test]
    fn test_parse_import_declaration() {
        let adapter = JavaAdapter::new();
        let context = AdapterContext::new(
            "Test.java".to_string(),
            "import java.util.List;".to_string(),
            Language::Java,
        );

        let result = adapter.parse_import_declaration("import java.util.List;", &context);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");
    }

    #[test]
    fn test_parse_static_import() {
        let adapter = JavaAdapter::new();
        let context = AdapterContext::new(
            "Test.java".to_string(),
            "import static java.lang.Math.PI;".to_string(),
            Language::Java,
        );

        let result = adapter.parse_import_declaration("import static java.lang.Math.PI;", &context);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "import_declaration");
    }

    #[test]
    fn test_parse_simple_class() {
        let adapter = JavaAdapter::new();
        let context = AdapterContext::new(
            "Test.java".to_string(),
            "public class Test {}".to_string(),
            Language::Java,
        );

        let result = adapter.parse_class_declaration("public class Test {}", &context);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");
    }

    #[test]
    fn test_parse_class_with_extends() {
        let adapter = JavaAdapter::new();
        let source = "public class Child extends Parent {}";
        let context = AdapterContext::new(
            "Child.java".to_string(),
            source.to_string(),
            Language::Java,
        );

        let result = adapter.parse_class_declaration(source, &context);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "class_declaration");
    }

    #[test]
    fn test_parse_method_declaration() {
        let adapter = JavaAdapter::new();
        let source = "public void testMethod() {}";

        let result = adapter.parse_method_declaration(source);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "method_declaration");
    }

    #[test]
    fn test_parse_field_declaration() {
        let adapter = JavaAdapter::new();
        let source = "private String name;";

        let result = adapter.parse_field_declaration(source);
        assert!(result.is_ok());
        
        let node = result.unwrap();
        assert_eq!(node.node_type(), "field_declaration");
    }

    #[test]
    fn test_java_adapter_metadata() {
        let adapter = JavaAdapter::new();
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "JavaAdapter");
        assert!(metadata.supported_features.contains(&"class_declarations".to_string()));
        assert!(metadata.supported_features.contains(&"method_declarations".to_string()));
    }

    #[test]
    fn test_full_java_parsing() {
        let parser = JavaParser::new();
        let source = r#"
package com.example;

import java.util.List;

public class Test {
    private String name;
    
    public void setName(String name) {
        this.name = name;
    }
}
"#;

        let result = parser.parse(source, Path::new("Test.java"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert!(ast.text().is_some());
    }
}
