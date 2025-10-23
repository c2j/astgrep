//! Base adapters for converting language-specific ASTs to universal AST
//! 
//! This module provides the base functionality for adapting different language ASTs.

use astgrep_ast::{AstBuilder, UniversalNode, NodeType};
use astgrep_core::{AstNode, Language, LanguageParser, Location, Result};
use std::collections::HashMap;
use std::path::Path;

/// Base adapter trait for converting language-specific ASTs
pub trait AstAdapter: Send + Sync {
    /// Convert a language-specific AST node to universal AST
    fn adapt_node(&self, node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode>;

    /// Parse source code directly to universal AST
    fn parse_to_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // Default implementation - just create a program node with the source
        Ok(UniversalNode::new(NodeType::Program).with_text(source.to_string()))
    }

    /// Get the language this adapter handles
    fn language(&self) -> Language;

    /// Get adapter metadata
    fn metadata(&self) -> AdapterMetadata;
}

/// Context for AST adaptation
#[derive(Debug, Clone)]
pub struct AdapterContext {
    pub file_path: String,
    pub source_code: String,
    pub language: Language,
    pub line_map: Vec<usize>, // Byte offsets for each line
    pub metadata: HashMap<String, String>,
}

impl AdapterContext {
    /// Create a new adapter context
    pub fn new(file_path: String, source_code: String, language: Language) -> Self {
        let line_map = Self::build_line_map(&source_code);
        Self {
            file_path,
            source_code,
            language,
            line_map,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get location from byte offset
    pub fn get_location(&self, start_offset: usize, end_offset: usize) -> Location {
        let (start_line, start_col) = self.offset_to_line_col(start_offset);
        let (end_line, end_col) = self.offset_to_line_col(end_offset);
        
        Location::new(
            Path::new(&self.file_path).to_path_buf(),
            start_line,
            start_col,
            end_line,
            end_col,
        )
    }

    /// Convert byte offset to line and column
    fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        for (line_num, &line_start) in self.line_map.iter().enumerate() {
            if line_num + 1 >= self.line_map.len() || offset < self.line_map[line_num + 1] {
                let col = offset.saturating_sub(line_start);
                return (line_num + 1, col + 1); // 1-based indexing
            }
        }
        
        // If we reach here, offset is at or beyond the end
        let last_line = self.line_map.len();
        let last_line_start = self.line_map.last().copied().unwrap_or(0);
        let col = offset.saturating_sub(last_line_start);
        (last_line, col + 1)
    }

    /// Build line map from source code
    fn build_line_map(source: &str) -> Vec<usize> {
        let mut line_map = vec![0]; // First line starts at offset 0
        
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_map.push(i + 1);
            }
        }
        
        line_map
    }
}

/// Adapter metadata
#[derive(Debug, Clone)]
pub struct AdapterMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub supported_features: Vec<String>,
}

impl AdapterMetadata {
    /// Create new adapter metadata
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            name,
            version,
            description,
            supported_features: Vec::new(),
        }
    }

    /// Add a supported feature
    pub fn with_feature(mut self, feature: String) -> Self {
        self.supported_features.push(feature);
        self
    }
}

/// Base parser implementation with adapter support
pub struct BaseParser {
    language: Language,
    adapter: Box<dyn AstAdapter>,
}

impl BaseParser {
    /// Create a new base parser
    pub fn new(language: Language, adapter: Box<dyn AstAdapter>) -> Self {
        Self { language, adapter }
    }

    /// Parse source code using the adapter
    pub fn parse_with_adapter(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            self.language,
        );

        // In a real implementation, this would:
        // 1. Use a language-specific parser (tree-sitter, etc.)
        // 2. Get the language-specific AST
        // 3. Use the adapter to convert to universal AST
        
        // For now, create a simple placeholder AST
        let universal_node = self.create_placeholder_ast(source, &context)?;
        Ok(Box::new(universal_node))
    }

    /// Create a placeholder AST for demonstration
    fn create_placeholder_ast(&self, source: &str, context: &AdapterContext) -> Result<UniversalNode> {
        // This is a simplified implementation
        // Real parsers would use tree-sitter or similar
        
        let root = AstBuilder::program(vec![
            AstBuilder::expression_statement(
                AstBuilder::string_literal(source)
                    .with_text(source.to_string())
                    .with_location(1, 1, 1, source.len())
            )
        ])
        .with_text(source.to_string())
        .with_location(1, 1, 1, source.len());

        Ok(root)
    }
}

impl LanguageParser for BaseParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        self.parse_with_adapter(source, file_path)
    }

    fn language(&self) -> Language {
        self.language
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            match (self.language, extension.to_lowercase().as_str()) {
                (Language::Java, "java") => true,
                (Language::JavaScript, "js" | "jsx" | "ts" | "tsx") => true,
                (Language::Python, "py" | "pyw") => true,
                (Language::Sql, "sql" | "ddl" | "dml") => true,
                (Language::Bash, "sh" | "bash" | "zsh") => true,
                (Language::Php, "php" | "phtml" | "php3" | "php4" | "php5") => true,
                (Language::CSharp, "cs" | "csx") => true,
                (Language::C, "c" | "h") => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

/// Utility functions for adapters
pub mod utils {
    use super::*;

    /// Extract text from a byte range in source code
    pub fn extract_text(source: &str, start: usize, end: usize) -> String {
        if start <= end && end <= source.len() {
            source[start..end].to_string()
        } else {
            String::new()
        }
    }

    /// Normalize whitespace in text
    pub fn normalize_whitespace(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Check if text represents a keyword in the given language
    pub fn is_keyword(text: &str, language: Language) -> bool {
        match language {
            Language::Java => JAVA_KEYWORDS.contains(&text),
            Language::JavaScript => JAVASCRIPT_KEYWORDS.contains(&text),
            Language::Python => PYTHON_KEYWORDS.contains(&text),
            Language::Sql => SQL_KEYWORDS.contains(&text.to_uppercase().as_str()),
            Language::Bash => BASH_KEYWORDS.contains(&text),
            Language::Php => false, // TODO: Add PHP keywords
            Language::CSharp => false, // TODO: Add C# keywords
            Language::C => false, // TODO: Add C keywords
            Language::Ruby => false, // TODO: Add Ruby keywords
            Language::Kotlin => false, // TODO: Add Kotlin keywords
            Language::Swift => false, // TODO: Add Swift keywords
            Language::Xml => false, // XML doesn't have keywords in the traditional sense
        }
    }

    /// Create a simple identifier node
    pub fn create_identifier(name: &str, location: Option<(usize, usize, usize, usize)>) -> UniversalNode {
        let mut node = AstBuilder::identifier(name).with_text(name.to_string());
        if let Some((start_line, start_col, end_line, end_col)) = location {
            node = node.with_location(start_line, start_col, end_line, end_col);
        }
        node
    }

    /// Create a simple literal node
    pub fn create_literal(value: &str, location: Option<(usize, usize, usize, usize)>) -> UniversalNode {
        let mut node = AstBuilder::string_literal(value).with_text(value.to_string());
        if let Some((start_line, start_col, end_line, end_col)) = location {
            node = node.with_location(start_line, start_col, end_line, end_col);
        }
        node
    }

    // Language keywords
    const JAVA_KEYWORDS: &[&str] = &[
        "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class",
        "const", "continue", "default", "do", "double", "else", "enum", "extends", "final",
        "finally", "float", "for", "goto", "if", "implements", "import", "instanceof", "int",
        "interface", "long", "native", "new", "package", "private", "protected", "public",
        "return", "short", "static", "strictfp", "super", "switch", "synchronized", "this",
        "throw", "throws", "transient", "try", "void", "volatile", "while",
    ];

    const JAVASCRIPT_KEYWORDS: &[&str] = &[
        "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete",
        "do", "else", "export", "extends", "finally", "for", "function", "if", "import", "in",
        "instanceof", "let", "new", "return", "super", "switch", "this", "throw", "try", "typeof",
        "var", "void", "while", "with", "yield",
    ];

    const PYTHON_KEYWORDS: &[&str] = &[
        "False", "None", "True", "and", "as", "assert", "break", "class", "continue", "def",
        "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
        "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try",
        "while", "with", "yield",
    ];

    const SQL_KEYWORDS: &[&str] = &[
        "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
        "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA", "GRANT", "REVOKE", "COMMIT", "ROLLBACK",
        "TRANSACTION", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "ON", "GROUP", "BY",
        "ORDER", "HAVING", "UNION", "ALL", "DISTINCT", "AS", "AND", "OR", "NOT", "NULL",
        "IS", "IN", "BETWEEN", "LIKE", "EXISTS",
    ];

    const BASH_KEYWORDS: &[&str] = &[
        "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do",
        "done", "function", "select", "time", "in", "break", "continue", "return", "exit",
        "export", "readonly", "local", "declare", "typeset", "unset",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_context_creation() {
        let source = "line 1\nline 2\nline 3";
        let context = AdapterContext::new(
            "test.java".to_string(),
            source.to_string(),
            Language::Java,
        );

        assert_eq!(context.file_path, "test.java");
        assert_eq!(context.language, Language::Java);
        assert_eq!(context.line_map.len(), 3); // 2 newlines + initial 0
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "hello\nworld\ntest";
        let context = AdapterContext::new(
            "test.txt".to_string(),
            source.to_string(),
            Language::Java,
        );

        // Test various positions
        assert_eq!(context.offset_to_line_col(0), (1, 1)); // 'h'
        assert_eq!(context.offset_to_line_col(5), (1, 6)); // '\n'
        assert_eq!(context.offset_to_line_col(6), (2, 1)); // 'w'
        assert_eq!(context.offset_to_line_col(12), (3, 1)); // 't'
    }

    #[test]
    fn test_get_location() {
        let source = "hello\nworld";
        let context = AdapterContext::new(
            "test.txt".to_string(),
            source.to_string(),
            Language::Java,
        );

        let location = context.get_location(0, 5);
        assert_eq!(location.start_line, 1);
        assert_eq!(location.start_column, 1);
        assert_eq!(location.end_line, 1);
        assert_eq!(location.end_column, 6);
    }

    #[test]
    fn test_adapter_metadata() {
        let metadata = AdapterMetadata::new(
            "JavaAdapter".to_string(),
            "1.0.0".to_string(),
            "Java AST adapter".to_string(),
        )
        .with_feature("classes".to_string())
        .with_feature("methods".to_string());

        assert_eq!(metadata.name, "JavaAdapter");
        assert_eq!(metadata.supported_features.len(), 2);
        assert!(metadata.supported_features.contains(&"classes".to_string()));
    }

    #[test]
    fn test_utils_extract_text() {
        let source = "hello world";
        assert_eq!(utils::extract_text(source, 0, 5), "hello");
        assert_eq!(utils::extract_text(source, 6, 11), "world");
        assert_eq!(utils::extract_text(source, 20, 25), ""); // Out of bounds
    }

    #[test]
    fn test_utils_normalize_whitespace() {
        assert_eq!(utils::normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(utils::normalize_whitespace("a\n\tb\r\nc"), "a b c");
    }

    #[test]
    fn test_utils_is_keyword() {
        assert!(utils::is_keyword("class", Language::Java));
        assert!(utils::is_keyword("function", Language::JavaScript));
        assert!(utils::is_keyword("def", Language::Python));
        assert!(utils::is_keyword("SELECT", Language::Sql));
        assert!(utils::is_keyword("if", Language::Bash));
        
        assert!(!utils::is_keyword("myVariable", Language::Java));
    }

    #[test]
    fn test_utils_create_nodes() {
        let id_node = utils::create_identifier("myVar", Some((1, 1, 1, 5)));
        assert_eq!(id_node.node_type(), "identifier");
        assert_eq!(id_node.text().as_deref(), Some("myVar"));

        let lit_node = utils::create_literal("\"hello\"", Some((1, 1, 1, 7)));
        assert_eq!(lit_node.node_type(), "literal");
        assert_eq!(lit_node.text().as_deref(), Some("\"hello\""));
    }
}

/// Macro to generate basic adapter implementations
///
/// This macro reduces boilerplate code for simple language adapters that follow
/// the same pattern but differ only in language-specific details.
#[macro_export]
macro_rules! impl_basic_adapter {
    (
        $adapter:ident,
        $parser:ident,
        $language:expr,
        $name:expr,
        $description:expr,
        $extensions:expr
    ) => {
        /// Language-specific AST adapter
        pub struct $adapter;

        impl $crate::adapters::AstAdapter for $adapter {
            fn language(&self) -> $crate::Language {
                $language
            }

            fn metadata(&self) -> $crate::adapters::AdapterMetadata {
                $crate::adapters::AdapterMetadata {
                    name: $name.to_string(),
                    version: "1.0.0".to_string(),
                    description: $description.to_string(),
                    supported_features: vec!["basic_parsing".to_string(), "taint_analysis".to_string()],
                }
            }

            fn adapt_node(&self, _node: &dyn std::any::Any, _context: &$crate::adapters::AdapterContext) -> $crate::Result<$crate::UniversalNode> {
                Ok($crate::UniversalNode::new($crate::NodeType::Program))
            }

            fn parse_to_ast(&self, source: &str, _context: &$crate::adapters::AdapterContext) -> $crate::Result<$crate::UniversalNode> {
                Ok($crate::UniversalNode::new($crate::NodeType::Program).with_text(source.to_string()))
            }
        }

        /// Language parser
        pub struct $parser {
            adapter: $adapter,
        }

        impl $parser {
            /// Create a new parser
            pub fn new() -> Self {
                Self {
                    adapter: $adapter,
                }
            }
        }

        impl $crate::LanguageParser for $parser {
            fn parse(&self, source: &str, file_path: &std::path::Path) -> $crate::Result<Box<dyn $crate::AstNode>> {
                let context = $crate::adapters::AdapterContext::new(
                    file_path.to_string_lossy().to_string(),
                    source.to_string(),
                    $language,
                );
                let ast = self.adapter.parse_to_ast(source, &context)?;
                Ok(Box::new(ast))
            }

            fn language(&self) -> $crate::Language {
                $language
            }

            fn extensions(&self) -> &[&str] {
                $extensions
            }
        }

        impl Default for $parser {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}
