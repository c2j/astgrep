//! Language parsers and adapters for astgrep
//!
//! This crate provides language-specific parsers and adapters.

pub mod adapter;
pub mod adapters;
pub mod base_adapter;
pub mod bash;
pub mod dialect;
pub mod java;
pub mod javascript;
pub mod javascript_optimizer;
pub mod pattern_tree;
pub mod python;
pub mod registry;
pub mod script_discovery;
pub mod sql;
pub mod text;
pub mod tree_sitter_parser;
pub mod xml;

pub use adapter::*;
pub use adapters::*;
pub use dialect::{dispatch, DialectParseError, SqlDialectParser};
pub use registry::*;

// Re-export types for macro usage
pub use astgrep_ast::{NodeType, UniversalNode};
pub use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::collections::HashMap;
use std::path::Path;

/// Main language parser registry
pub struct LanguageParserRegistry {
    parsers: HashMap<Language, Box<dyn LanguageParser>>,
}

impl LanguageParserRegistry {
    /// Create a new parser registry
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };

        // Register default parsers
        registry.register_default_parsers();
        registry
    }

    /// Register a parser for a language
    pub fn register_parser(&mut self, language: Language, parser: Box<dyn LanguageParser>) {
        self.parsers.insert(language, parser);
    }

    /// Get a parser for a language
    pub fn get_parser(&self, language: Language) -> Option<&dyn LanguageParser> {
        self.parsers.get(&language).map(|p| p.as_ref())
    }

    /// Parse a file using the appropriate language parser
    pub fn parse_file(&self, file_path: &Path, source: &str) -> Result<Box<dyn AstNode>> {
        let language = self.detect_language(file_path)?;

        if let Some(parser) = self.get_parser(language) {
            parser.parse(source, file_path)
        } else {
            Err(astgrep_core::AnalysisError::unsupported_language(format!(
                "No parser available for language: {:?}",
                language
            )))
        }
    }

    /// Detect language from file extension
    pub fn detect_language(&self, file_path: &Path) -> Result<Language> {
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "java" => Ok(Language::Java),
                "js" | "jsx" | "ts" | "tsx" => Ok(Language::JavaScript),
                "py" | "pyw" => Ok(Language::Python),
                "sql" | "ddl" | "dml" => Ok(Language::Sql),
                "sh" | "bash" | "zsh" => Ok(Language::Bash),
                "txt" | "md" | "log" | "rst" => Ok(Language::Text),
                _ => Err(astgrep_core::AnalysisError::unsupported_language(format!(
                    "Unsupported file extension: {}",
                    extension
                ))),
            }
        } else {
            Err(astgrep_core::AnalysisError::unsupported_language(
                "No file extension found".to_string(),
            ))
        }
    }

    /// Get all supported languages
    pub fn supported_languages(&self) -> Vec<Language> {
        self.parsers.keys().cloned().collect()
    }

    /// Check if a language is supported
    pub fn supports_language(&self, language: Language) -> bool {
        self.parsers.contains_key(&language)
    }

    /// Register default parsers for all supported languages
    fn register_default_parsers(&mut self) {
        self.register_parser(Language::Java, Box::new(java::JavaParser::new()));
        self.register_parser(
            Language::JavaScript,
            Box::new(javascript::JavaScriptParser::new()),
        );
        self.register_parser(Language::Python, Box::new(python::PythonParser::new()));
        self.register_parser(Language::Sql, Box::new(sql::SqlParser::new()));
        self.register_parser(Language::Bash, Box::new(bash::BashParser::new()));
    }

    #[cfg(test)]
    fn clear_parsers(&mut self) {
        self.parsers.clear();
    }
}

impl Default for LanguageParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[derive(Debug)]
    struct MockParser {
        lang: Language,
    }

    impl MockParser {
        fn new(lang: Language) -> Self {
            Self { lang }
        }
    }

    impl LanguageParser for MockParser {
        fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
            Ok(Box::new(astgrep_ast::AstBuilder::program(vec![
                astgrep_ast::AstBuilder::expression_statement(astgrep_ast::AstBuilder::identifier(
                    source,
                )),
            ])))
        }

        fn language(&self) -> Language {
            self.lang
        }

        fn supports_file(&self, file_path: &Path) -> bool {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                match self.lang {
                    Language::Java => ext == "java",
                    Language::JavaScript => matches!(ext, "js" | "jsx" | "ts" | "tsx"),
                    Language::Python => matches!(ext, "py" | "pyw"),
                    Language::Sql => matches!(ext, "sql" | "ddl" | "dml"),
                    Language::Bash => matches!(ext, "sh" | "bash" | "zsh"),
                    Language::Xml => ext == "xml",
                    Language::Text => matches!(ext, "txt" | "md" | "log" | "rst"),
                }
            } else {
                false
            }
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = LanguageParserRegistry::new();
        let languages = registry.supported_languages();
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry: LanguageParserRegistry = Default::default();
        let languages = registry.supported_languages();
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_register_parser() {
        let mut registry = LanguageParserRegistry::new();
        let parser = Box::new(MockParser::new(Language::Java));
        registry.register_parser(Language::Java, parser);

        let retrieved = registry.get_parser(Language::Java);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().language(), Language::Java);
    }

    #[test]
    fn test_get_parser_unregistered() {
        let mut registry = LanguageParserRegistry::new();
        registry.clear_parsers();
        registry.register_parser(Language::Java, Box::new(MockParser::new(Language::Java)));

        assert!(registry.get_parser(Language::Python).is_none());
        assert!(registry.get_parser(Language::Sql).is_none());
    }

    #[test]
    fn test_supported_languages() {
        let mut registry = LanguageParserRegistry::new();
        registry.clear_parsers();
        registry.register_parser(Language::Java, Box::new(MockParser::new(Language::Java)));
        registry.register_parser(
            Language::Python,
            Box::new(MockParser::new(Language::Python)),
        );

        let languages = registry.supported_languages();
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&Language::Java));
        assert!(languages.contains(&Language::Python));
    }

    #[test]
    fn test_supports_language() {
        let mut registry = LanguageParserRegistry::new();
        registry.clear_parsers();
        registry.register_parser(Language::Java, Box::new(MockParser::new(Language::Java)));

        assert!(registry.supports_language(Language::Java));
        assert!(!registry.supports_language(Language::Python));
        assert!(!registry.supports_language(Language::Sql));
    }

    #[test]
    fn test_detect_language() {
        let registry = LanguageParserRegistry::new();

        assert_eq!(
            registry.detect_language(Path::new("test.java")).unwrap(),
            Language::Java
        );
        assert_eq!(
            registry.detect_language(Path::new("test.js")).unwrap(),
            Language::JavaScript
        );
        assert_eq!(
            registry.detect_language(Path::new("test.py")).unwrap(),
            Language::Python
        );
        assert_eq!(
            registry.detect_language(Path::new("test.sql")).unwrap(),
            Language::Sql
        );
        assert_eq!(
            registry.detect_language(Path::new("test.sh")).unwrap(),
            Language::Bash
        );
        assert_eq!(
            registry.detect_language(Path::new("test.ddl")).unwrap(),
            Language::Sql
        );
        assert_eq!(
            registry.detect_language(Path::new("test.dml")).unwrap(),
            Language::Sql
        );

        assert!(registry.detect_language(Path::new("test.unknown")).is_err());
        assert!(registry.detect_language(Path::new("no_extension")).is_err());
    }

    #[test]
    fn test_parse_file() {
        let mut registry = LanguageParserRegistry::new();
        registry.register_parser(Language::Java, Box::new(MockParser::new(Language::Java)));

        let result = registry.parse_file(Path::new("test.java"), "hello");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "program");
        assert_eq!(node.child_count(), 1);
    }

    #[test]
    fn test_parse_file_unsupported_extension() {
        let registry = LanguageParserRegistry::new();

        let result = registry.parse_file(Path::new("test.unknown"), "hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_no_parser() {
        let mut registry = LanguageParserRegistry::new();
        registry.clear_parsers();
        registry.register_parser(Language::Java, Box::new(MockParser::new(Language::Java)));

        let result = registry.parse_file(Path::new("test.py"), "hello");
        assert!(result.is_err());
    }
}
