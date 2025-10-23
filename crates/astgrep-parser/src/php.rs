//! PHP language parser for astgrep

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use crate::base_adapter::BaseAdapter;
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// PHP-specific AST adapter using the base adapter
pub struct PhpAdapter {
    base: BaseAdapter,
}

impl PhpAdapter {
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(Language::Php),
        }
    }
}

impl Default for PhpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstAdapter for PhpAdapter {
    fn language(&self) -> Language {
        self.base.language()
    }

    fn metadata(&self) -> AdapterMetadata {
        self.base.metadata()
    }

    fn adapt_node(&self, node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        self.base.adapt_node(node, context)
    }

    fn parse_to_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // Use the base adapter's parsing with PHP-specific enhancements
        let ast = self.base.parse_to_ast(source, context)?;
        // Add PHP-specific enhancements if needed
        // For now, the base adapter provides sufficient functionality

        Ok(ast)
    }
}

// PHP-specific functionality can be added here if needed
// The base adapter provides sufficient functionality for basic parsing

/// PHP language parser
pub struct PhpParser {
    adapter: PhpAdapter,
}

impl PhpParser {
    /// Create a new PHP parser
    pub fn new() -> Self {
        Self {
            adapter: PhpAdapter::new(),
        }
    }
}

impl LanguageParser for PhpParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Php
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::Php
    }

    fn extensions(&self) -> &[&str] {
        &["php", "phtml", "php3", "php4", "php5"]
    }
}

impl Default for PhpParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_parser_creation() {
        let parser = PhpParser::new();
        assert_eq!(parser.language(), Language::Php);
        assert!(parser.extensions().contains(&"php"));
    }

    #[test]
    fn test_php_adapter_creation() {
        let adapter = PhpAdapter::new();
        assert_eq!(adapter.language(), Language::Php);
        assert_eq!(adapter.metadata().name, "PHP Adapter");
    }

    #[test]
    fn test_php_basic_parsing() {
        let parser = PhpParser::new();
        let source = r#"<?php
$x = "tainted";
sink($x);
?>"#;
        
        let result = parser.parse(source, Path::new("test.php"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), NodeType::Program);
    }
}
