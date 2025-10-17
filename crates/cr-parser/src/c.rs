//! C language parser for CR-SemService

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use cr_ast::{NodeType, UniversalNode};
use cr_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// C-specific AST adapter
pub struct CAdapter;

impl AstAdapter for CAdapter {
    fn language(&self) -> Language {
        Language::C
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: "C Adapter".to_string(),
            version: "1.0.0".to_string(),
            description: "C language adapter for CR-SemService".to_string(),
            supported_features: vec!["basic_parsing".to_string(), "taint_analysis".to_string()],
        }
    }

    fn adapt_node(&self, _node: &dyn std::any::Any, _context: &AdapterContext) -> Result<UniversalNode> {
        Ok(UniversalNode::new(NodeType::Program))
    }

    fn parse_to_ast(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        // 简单的C解析器实现 - 使用默认实现
        Ok(UniversalNode::new(NodeType::Program).with_text(source.to_string()))
    }
}



/// C language parser
pub struct CParser {
    adapter: CAdapter,
}

impl CParser {
    /// Create a new C parser
    pub fn new() -> Self {
        Self {
            adapter: CAdapter,
        }
    }
}

impl LanguageParser for CParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::C
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::C
    }

    fn extensions(&self) -> &[&str] {
        &["c", "h"]
    }
}

impl Default for CParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_parser_creation() {
        let parser = CParser::new();
        assert_eq!(parser.language(), Language::C);
        assert!(parser.extensions().contains(&"c"));
    }

    #[test]
    fn test_c_function_call_parsing() {
        let adapter = CAdapter;
        let line = "sink(tainted_var);";
        let result = adapter.parse_function_call(line, 1);
        
        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.node_type(), NodeType::FunctionCall);
        assert_eq!(node.value(), Some("sink"));
    }

    #[test]
    fn test_c_assignment_parsing() {
        let adapter = CAdapter;
        let line = "char *x = \"tainted\";";
        let result = adapter.parse_assignment(line, 1);
        
        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.node_type(), NodeType::Assignment);
        assert_eq!(node.children().len(), 2);
    }

    #[test]
    fn test_c_basic_parsing() {
        let parser = CParser::new();
        let source = r#"#include <stdio.h>
char *x = "tainted";
sink(x);"#;
        
        let result = parser.parse(source, Path::new("test.c"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), NodeType::Program);
    }
}
