//! C# language parser for astgrep

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// C#-specific AST adapter
pub struct CSharpAdapter;

impl AstAdapter for CSharpAdapter {
    fn language(&self) -> Language {
        Language::CSharp
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata {
            name: "C# Adapter".to_string(),
            version: "1.0.0".to_string(),
            description: "C# language adapter for astgrep".to_string(),
            supported_features: vec!["basic_parsing".to_string(), "taint_analysis".to_string()],
        }
    }

    fn adapt_node(&self, _node: &dyn std::any::Any, _context: &AdapterContext) -> Result<UniversalNode> {
        Ok(UniversalNode::new(NodeType::Program))
    }

    fn parse_to_ast(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        // 简单的C#解析器实现 - 使用默认实现
        Ok(UniversalNode::new(NodeType::Program).with_text(source.to_string()))
    }
}



/// C# language parser
pub struct CSharpParser {
    adapter: CSharpAdapter,
}

impl CSharpParser {
    /// Create a new C# parser
    pub fn new() -> Self {
        Self {
            adapter: CSharpAdapter,
        }
    }
}

impl LanguageParser for CSharpParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            _file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::CSharp
        );
        let ast = self.adapter.parse_to_ast(source, &context)?;
        Ok(Box::new(ast))
    }

    fn language(&self) -> Language {
        Language::CSharp
    }

    fn extensions(&self) -> &[&str] {
        &["cs", "csx"]
    }
}

impl Default for CSharpParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csharp_parser_creation() {
        let parser = CSharpParser::new();
        assert_eq!(parser.language(), Language::CSharp);
        assert!(parser.extensions().contains(&"cs"));
    }

    #[test]
    fn test_csharp_method_call_parsing() {
        let adapter = CSharpAdapter;
        let line = "sink(taintedVar);";
        let result = adapter.parse_method_call(line, 1);
        
        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.node_type(), NodeType::FunctionCall);
        assert_eq!(node.value(), Some("sink"));
    }

    #[test]
    fn test_csharp_assignment_parsing() {
        let adapter = CSharpAdapter;
        let line = "string x = \"tainted\";";
        let result = adapter.parse_assignment(line, 1);
        
        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.node_type(), NodeType::Assignment);
        assert_eq!(node.children().len(), 2);
    }

    #[test]
    fn test_csharp_basic_parsing() {
        let parser = CSharpParser::new();
        let source = r#"using System;
string x = "tainted";
sink(x);"#;
        
        let result = parser.parse(source, Path::new("test.cs"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), NodeType::Program);
    }
}
