//! Java language parser and adapter
//!
//! This module provides Java-specific parsing using tree-sitter-java for
//! full AST generation, replacing the previous simplified manual parsing.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::tree_sitter_parser::TreeSitterParser;
use astgrep_ast::UniversalNode;
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Java AST adapter — provides metadata and tree-sitter-based node adaptation.
pub struct JavaAdapter;

impl Default for JavaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaAdapter {
    /// Create a new Java adapter.
    pub fn new() -> Self {
        Self
    }

    /// Parse Java source code using tree-sitter-java.
    fn parse_with_tree_sitter(&self, source: &str) -> Result<UniversalNode> {
        let ts_parser = TreeSitterParser::new()?;
        let tree = ts_parser.parse(source, Language::Java)?.ok_or_else(|| {
            astgrep_core::AnalysisError::parse_error("Java parser returned no tree")
        })?;
        ts_parser.tree_to_universal_ast(&tree, source)
    }
}

impl AstAdapter for JavaAdapter {
    fn adapt_node(
        &self,
        _node: &dyn std::any::Any,
        context: &AdapterContext,
    ) -> Result<UniversalNode> {
        self.parse_with_tree_sitter(&context.source_code)
    }

    fn language(&self) -> Language {
        Language::Java
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "JavaAdapter".to_string(),
            "1.0.0".to_string(),
            "Java AST adapter using tree-sitter-java".to_string(),
        )
        .with_feature("package_declarations".to_string())
        .with_feature("import_declarations".to_string())
        .with_feature("class_declarations".to_string())
        .with_feature("method_declarations".to_string())
        .with_feature("field_declarations".to_string())
        .with_feature("modifiers".to_string())
    }
}

/// Java language parser backed by tree-sitter-java.
///
/// Produces a full `UniversalNode` AST via `TreeSitterParser`,
/// replacing the previous simplified manual parsing approach.
pub struct JavaParser {
    ts_parser: TreeSitterParser,
}

impl JavaParser {
    /// Create a new Java parser.
    ///
    /// # Panics
    ///
    /// Panics if the tree-sitter-java grammar cannot be initialized.
    /// This should never happen in practice since grammars are statically linked.
    pub fn new() -> Self {
        Self {
            ts_parser: TreeSitterParser::new().expect(
                "TreeSitterParser initialization failed: \
                 tree-sitter grammars are statically linked and should always initialize",
            ),
        }
    }
}

impl LanguageParser for JavaParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let tree = self
            .ts_parser
            .parse(source, Language::Java)?
            .ok_or_else(|| {
                astgrep_core::AnalysisError::parse_error("Java parser returned no tree")
            })?;
        let universal = self.ts_parser.tree_to_universal_ast(&tree, source)?;
        Ok(Box::new(universal))
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
    fn test_java_adapter_metadata() {
        let adapter = JavaAdapter::new();
        let metadata = adapter.metadata();

        assert_eq!(metadata.name, "JavaAdapter");
        assert!(metadata
            .supported_features
            .contains(&"class_declarations".to_string()));
        assert!(metadata
            .supported_features
            .contains(&"method_declarations".to_string()));
    }

    #[test]
    fn test_parse_simple_class() {
        let parser = JavaParser::new();
        let source = "public class Test {}";

        let result = parser.parse(source, Path::new("Test.java"));
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(ast.text().is_some());
    }

    #[test]
    fn test_parse_class_with_method() {
        let parser = JavaParser::new();
        let source = r#"
public class Test {
    public void hello() {
        System.out.println("world");
    }
}
"#;

        let result = parser.parse(source, Path::new("Test.java"));
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(ast.text().is_some());
    }

    #[test]
    fn test_parse_method_invocation() {
        let parser = JavaParser::new();
        // tree-sitter-java can parse standalone expressions via statement wrappers
        let source = "class Wrapper { void m() { stmt.execute(query); } }";

        let result = parser.parse(source, Path::new("Wrapper.java"));
        assert!(result.is_ok());

        let ast = result.unwrap();
        let text = ast.text().unwrap_or_default();
        assert!(text.contains("execute"));
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
