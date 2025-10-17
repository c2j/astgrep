//! Base adapter implementation to reduce code duplication
//! 
//! This module provides a generic base adapter that can be used by language-specific
//! adapters to reduce boilerplate code.

use crate::adapters::{AstAdapter, AdapterContext, AdapterMetadata};
use cr_ast::{UniversalNode, NodeType, AstBuilder};
use cr_core::{Language, Result, constants::metadata};
use std::collections::HashMap;

/// Generic base adapter that provides common functionality
pub struct BaseAdapter {
    language: Language,
    name: String,
    version: String,
    description: String,
    features: Vec<String>,
    extensions: Vec<String>,
}

impl BaseAdapter {
    /// Create a new base adapter for a specific language
    pub fn new(language: Language) -> Self {
        let (name, description, extensions) = match language {
            Language::Java => (
                "Java Adapter",
                "Adapter for parsing Java source code",
                vec!["java".to_string()],
            ),
            Language::JavaScript => (
                "JavaScript Adapter", 
                "Adapter for parsing JavaScript source code",
                vec!["js".to_string(), "jsx".to_string(), "mjs".to_string()],
            ),
            Language::Python => (
                "Python Adapter",
                "Adapter for parsing Python source code", 
                vec!["py".to_string(), "pyw".to_string()],
            ),
            Language::Php => (
                "PHP Adapter",
                "Adapter for parsing PHP source code",
                vec!["php".to_string(), "phtml".to_string()],
            ),
            Language::Sql => (
                "SQL Adapter",
                "Adapter for parsing SQL source code",
                vec!["sql".to_string()],
            ),
            Language::Bash => (
                "Bash Adapter", 
                "Adapter for parsing Bash shell scripts",
                vec!["sh".to_string(), "bash".to_string()],
            ),
            Language::CSharp => (
                "C# Adapter",
                "Adapter for parsing C# source code",
                vec!["cs".to_string()],
            ),
            Language::C => (
                "C Adapter",
                "Adapter for parsing C source code", 
                vec!["c".to_string(), "h".to_string()],
            ),
        };

        Self {
            language,
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            features: vec![
                "basic_parsing".to_string(),
                "ast_conversion".to_string(),
            ],
            extensions,
        }
    }

    /// Create a base adapter with custom metadata
    pub fn with_metadata(
        language: Language,
        name: String,
        version: String,
        description: String,
        features: Vec<String>,
        extensions: Vec<String>,
    ) -> Self {
        Self {
            language,
            name,
            version,
            description,
            features,
            extensions,
        }
    }

    /// Parse source code into a basic AST structure
    /// This provides a default implementation that can be overridden
    pub fn parse_basic_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // Create a basic AST structure based on the language
        let mut root = UniversalNode::new(NodeType::Program)
            .with_text(source.to_string())
            .with_location(1, 1, source.lines().count(), source.len());

        // Add basic parsing based on language patterns
        match self.language {
            Language::Java | Language::CSharp | Language::C => {
                self.parse_c_style_language(source, &mut root)?;
            }
            Language::JavaScript => {
                self.parse_javascript_style(source, &mut root)?;
            }
            Language::Python => {
                self.parse_python_style(source, &mut root)?;
            }
            Language::Php => {
                self.parse_php_style(source, &mut root)?;
            }
            Language::Sql => {
                self.parse_sql_style(source, &mut root)?;
            }
            Language::Bash => {
                self.parse_bash_style(source, &mut root)?;
            }
        }

        Ok(root)
    }

    /// Parse C-style languages (Java, C#, C)
    fn parse_c_style_language(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                continue;
            }

            // Detect method calls
            if let Some(call_node) = self.parse_method_call(line, line_num + 1) {
                *root = root.clone().add_child(call_node);
            }

            // Detect variable declarations
            if let Some(var_node) = self.parse_variable_declaration(line, line_num + 1) {
                *root = root.clone().add_child(var_node);
            }
        }
        Ok(())
    }

    /// Parse JavaScript-style syntax
    fn parse_javascript_style(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                continue;
            }

            // Detect function calls
            if let Some(call_node) = self.parse_function_call(line, line_num + 1) {
                *root = root.clone().add_child(call_node);
            }

            // Detect variable assignments
            if let Some(assign_node) = self.parse_assignment(line, line_num + 1) {
                *root = root.clone().add_child(assign_node);
            }
        }
        Ok(())
    }

    /// Parse Python-style syntax
    fn parse_python_style(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            // Detect function calls
            if let Some(call_node) = self.parse_function_call(line, line_num + 1) {
                *root = root.clone().add_child(call_node);
            }

            // Detect assignments
            if let Some(assign_node) = self.parse_assignment(line, line_num + 1) {
                *root = root.clone().add_child(assign_node);
            }
        }
        Ok(())
    }

    /// Parse PHP-style syntax
    fn parse_php_style(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
                continue;
            }

            // Detect function calls
            if let Some(call_node) = self.parse_function_call(line, line_num + 1) {
                *root = root.clone().add_child(call_node);
            }

            // Detect variable assignments
            if let Some(assign_node) = self.parse_assignment(line, line_num + 1) {
                *root = root.clone().add_child(assign_node);
            }
        }
        Ok(())
    }

    /// Parse SQL-style syntax
    fn parse_sql_style(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("--") {
                continue;
            }

            // Detect SQL statements
            if let Some(stmt_node) = self.parse_sql_statement(line, line_num + 1) {
                *root = root.clone().add_child(stmt_node);
            }
        }
        Ok(())
    }

    /// Parse Bash-style syntax
    fn parse_bash_style(&self, source: &str, root: &mut UniversalNode) -> Result<()> {
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            // Detect command calls
            if let Some(cmd_node) = self.parse_command(line, line_num + 1) {
                *root = root.clone().add_child(cmd_node);
            }
        }
        Ok(())
    }

    /// Parse method calls (Java, C#, C style)
    fn parse_method_call(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        if line.contains("(") && line.contains(")") {
            Some(
                UniversalNode::new(NodeType::CallExpression)
                    .with_text(line.to_string())
                    .with_location(line_num, 1, line_num, line.len())
            )
        } else {
            None
        }
    }

    /// Parse function calls (JavaScript, Python, PHP style)
    fn parse_function_call(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        if line.contains("(") && line.contains(")") {
            Some(
                UniversalNode::new(NodeType::CallExpression)
                    .with_text(line.to_string())
                    .with_location(line_num, 1, line_num, line.len())
            )
        } else {
            None
        }
    }

    /// Parse variable declarations
    fn parse_variable_declaration(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        let keywords = match self.language {
            Language::Java | Language::CSharp => vec!["int", "String", "var", "final"],
            Language::C => vec!["int", "char", "float", "double", "void"],
            _ => vec![],
        };

        for keyword in keywords {
            if line.starts_with(keyword) {
                return Some(
                    UniversalNode::new(NodeType::VariableDeclaration)
                        .with_text(line.to_string())
                        .with_location(line_num, 1, line_num, line.len())
                );
            }
        }
        None
    }

    /// Parse assignments
    fn parse_assignment(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        if line.contains("=") && !line.contains("==") && !line.contains("!=") {
            Some(
                UniversalNode::new(NodeType::AssignmentExpression)
                    .with_text(line.to_string())
                    .with_location(line_num, 1, line_num, line.len())
            )
        } else {
            None
        }
    }

    /// Parse SQL statements
    fn parse_sql_statement(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        let sql_keywords = ["SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER"];
        let upper_line = line.to_uppercase();
        
        for keyword in sql_keywords {
            if upper_line.starts_with(keyword) {
                return Some(
                    UniversalNode::new(NodeType::ExpressionStatement)
                        .with_text(line.to_string())
                        .with_location(line_num, 1, line_num, line.len())
                );
            }
        }
        None
    }

    /// Parse shell commands
    fn parse_command(&self, line: &str, line_num: usize) -> Option<UniversalNode> {
        if !line.is_empty() {
            Some(
                UniversalNode::new(NodeType::CallExpression)
                    .with_text(line.to_string())
                    .with_location(line_num, 1, line_num, line.len())
            )
        } else {
            None
        }
    }
}

impl AstAdapter for BaseAdapter {
    fn language(&self) -> Language {
        self.language
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
            supported_features: self.features.clone(),
        }
    }

    fn adapt_node(&self, _node: &dyn std::any::Any, _context: &AdapterContext) -> Result<UniversalNode> {
        // Simple implementation for now
        Ok(UniversalNode::new(NodeType::Program))
    }

    fn parse_to_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        self.parse_basic_ast(source, context)
    }
}

/// Macro to create language-specific adapters with minimal boilerplate
#[macro_export]
macro_rules! create_language_adapter {
    ($name:ident, $language:expr) => {
        pub struct $name {
            base: BaseAdapter,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    base: BaseAdapter::new($language),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl AstAdapter for $name {
            fn language(&self) -> Language {
                self.base.language()
            }

            fn metadata(&self) -> AdapterMetadata {
                self.base.metadata()
            }

            fn parse_to_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
                self.base.parse_to_ast(source, context)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_adapter_creation() {
        let adapter = BaseAdapter::new(Language::Java);
        assert_eq!(adapter.language(), Language::Java);
        assert_eq!(adapter.metadata().name, "Java Adapter");
    }

    #[test]
    fn test_base_adapter_parsing() {
        let adapter = BaseAdapter::new(Language::Java);
        let context = AdapterContext::new(
            "test.java".to_string(),
            "System.out.println(\"Hello\");".to_string(),
            Language::Java,
        );
        
        let result = adapter.parse_to_ast("System.out.println(\"Hello\");", &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_macro_generated_adapter() {
        create_language_adapter!(TestJavaAdapter, Language::Java);
        
        let adapter = TestJavaAdapter::new();
        assert_eq!(adapter.language(), Language::Java);
    }
}
