//! Language parsers and adapters for astgrep
//!
//! This crate provides language-specific parsers and adapters.

pub mod registry;
pub mod adapters;
pub mod base_adapter;
pub mod java;
pub mod javascript;
pub mod javascript_optimizer;
pub mod python;
pub mod sql;
pub mod bash;
pub mod php;
pub mod php_optimizer;
pub mod csharp;
pub mod tree_sitter_parser;
pub mod c;
pub mod c_simple;
pub mod ruby;
pub mod kotlin;
pub mod swift;
pub mod xml;

pub use registry::*;
pub use adapters::*;

// Re-export types for macro usage
pub use astgrep_core::{Language, Result, AstNode, LanguageParser};
pub use astgrep_ast::{UniversalNode, NodeType};
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
                "php" | "phtml" | "php3" | "php4" | "php5" => Ok(Language::Php),
                "cs" | "csx" => Ok(Language::CSharp),
                "c" | "h" => Ok(Language::C),
                "rb" | "rbw" | "rake" | "gemspec" => Ok(Language::Ruby),
                "kt" | "kts" => Ok(Language::Kotlin),
                "swift" => Ok(Language::Swift),
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
        self.register_parser(Language::JavaScript, Box::new(javascript::JavaScriptParser::new()));
        self.register_parser(Language::Python, Box::new(python::PythonParser::new()));
        self.register_parser(Language::Sql, Box::new(sql::SqlParser::new()));
        self.register_parser(Language::Bash, Box::new(bash::BashParser::new()));
        self.register_parser(Language::Php, Box::new(php::PhpParser::new()));
        self.register_parser(Language::CSharp, Box::new(csharp::CSharpParser::new()));
        self.register_parser(Language::C, Box::new(c::CParser::new()));
        self.register_parser(Language::Ruby, Box::new(ruby::RubyParser::new()));
        self.register_parser(Language::Kotlin, Box::new(kotlin::KotlinParser::new()));
        self.register_parser(Language::Swift, Box::new(swift::SwiftParser::new()));
    }
}

impl Default for LanguageParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
