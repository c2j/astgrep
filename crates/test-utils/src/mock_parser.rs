//! Mock parser implementations for testing

use astgrep_core::{LanguageParser, Language, AstNode, Result};
use crate::mock_ast::MockAstNode;
use std::path::Path;

/// Mock parser for testing purposes
#[derive(Clone, Debug)]
pub struct MockParser {
    language: Language,
    should_fail: bool,
    custom_result: Option<MockAstNode>,
}

impl MockParser {
    /// Create a new mock parser
    pub fn new(language: Language) -> Self {
        Self {
            language,
            should_fail: false,
            custom_result: None,
        }
    }

    /// Configure the parser to fail on parse
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    /// Configure the parser to return a custom result
    pub fn with_custom_result(mut self, result: MockAstNode) -> Self {
        self.custom_result = Some(result);
        self
    }

    /// Create a parser that returns a simple program node
    pub fn simple_program_parser(language: Language) -> Self {
        let program_node = MockAstNode::new("program")
            .with_text("mock program content");
        
        Self::new(language).with_custom_result(program_node)
    }

    /// Create a parser that returns a complex AST
    pub fn complex_ast_parser(language: Language) -> Self {
        let function_node = MockAstNode::new("function")
            .with_text("function test() {}")
            .add_child(MockAstNode::new("identifier").with_text("test"))
            .add_child(MockAstNode::new("block"));

        let class_node = MockAstNode::new("class")
            .with_text("class TestClass {}")
            .add_child(MockAstNode::new("identifier").with_text("TestClass"))
            .add_child(function_node);

        let program_node = MockAstNode::new("program")
            .add_child(class_node);

        Self::new(language).with_custom_result(program_node)
    }
}

impl LanguageParser for MockParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        if self.should_fail {
            return Err(astgrep_core::AnalysisError::parse_error("Mock parser configured to fail"));
        }

        if let Some(ref custom_result) = self.custom_result {
            return Ok(Box::new(custom_result.clone()));
        }

        // Default behavior: create a simple root node with the source as text
        let root = MockAstNode::new("root")
            .with_text(source);

        Ok(Box::new(root))
    }

    fn language(&self) -> Language {
        self.language
    }

    fn extensions(&self) -> &[&str] {
        self.language.extensions()
    }
}

/// Mock parser registry for testing
#[derive(Default)]
pub struct MockParserRegistry {
    parsers: std::collections::HashMap<Language, MockParser>,
}

impl MockParserRegistry {
    /// Create a new mock parser registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mock parser for a language
    pub fn register(&mut self, language: Language, parser: MockParser) {
        self.parsers.insert(language, parser);
    }

    /// Get a parser for a language
    pub fn get_parser(&self, language: Language) -> Option<&MockParser> {
        self.parsers.get(&language)
    }

    /// Create a registry with default parsers for all languages
    pub fn with_default_parsers() -> Self {
        let mut registry = Self::new();
        
        for &language in &[
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::C,
            Language::CSharp,
            Language::Php,
            Language::Bash,
            Language::Sql,
        ] {
            registry.register(language, MockParser::simple_program_parser(language));
        }
        
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_parser_basic() {
        let parser = MockParser::new(Language::Java);
        assert_eq!(parser.language(), Language::Java);
        
        let result = parser.parse("test code", Path::new("test.java"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), "root");
        assert_eq!(ast.text(), Some("test code"));
    }

    #[test]
    fn test_mock_parser_failure() {
        let parser = MockParser::new(Language::Java).with_failure();
        
        let result = parser.parse("test code", Path::new("test.java"));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_parser_custom_result() {
        let custom_node = MockAstNode::new("custom")
            .with_text("custom content");
        
        let parser = MockParser::new(Language::Java)
            .with_custom_result(custom_node);
        
        let result = parser.parse("test code", Path::new("test.java"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), "custom");
        assert_eq!(ast.text(), Some("custom content"));
    }

    #[test]
    fn test_complex_ast_parser() {
        let parser = MockParser::complex_ast_parser(Language::Java);
        
        let result = parser.parse("test code", Path::new("test.java"));
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        assert_eq!(ast.node_type(), "program");
        assert_eq!(ast.child_count(), 1);
        
        let class_node = ast.child(0).unwrap();
        assert_eq!(class_node.node_type(), "class");
        assert_eq!(class_node.child_count(), 2);
    }

    #[test]
    fn test_mock_parser_registry() {
        let mut registry = MockParserRegistry::new();
        let parser = MockParser::new(Language::Java);
        
        registry.register(Language::Java, parser);
        
        let retrieved = registry.get_parser(Language::Java);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().language(), Language::Java);
    }

    #[test]
    fn test_default_parsers_registry() {
        let registry = MockParserRegistry::with_default_parsers();
        
        assert!(registry.get_parser(Language::Java).is_some());
        assert!(registry.get_parser(Language::JavaScript).is_some());
        assert!(registry.get_parser(Language::Python).is_some());
    }
}
