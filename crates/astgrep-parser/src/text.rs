//! Plain text language parser for astgrep
//!
//! Does NOT use tree-sitter. Wraps text content in a minimal UniversalNode
//! so that `pattern-regex` and string-based patterns can match against raw text.

use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::AstNode;
use astgrep_core::Language;
use astgrep_core::LanguageParser;
use astgrep_core::Result;
use std::path::Path;

/// Plain text parser — creates a single-node AST wrapping the file content.
///
/// The root node is a `Program` with the full text as its `text` field.
/// Each line is also added as a child for line-level pattern matching.
pub struct TextParser;

impl TextParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for TextParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        let mut root = UniversalNode::new(NodeType::Program)
            .with_text(source.to_string())
            .with_attribute("file_path".to_string(), file_path.to_string_lossy().to_string());

        // Add each line as a child for line-level pattern matching
        for (line_idx, line) in source.lines().enumerate() {
            let line_node = UniversalNode::new(NodeType::ExpressionStatement)
                .with_text(line.to_string())
                .with_location(line_idx + 1, 1, line_idx + 1, line.len() + 1);
            root.children.push(line_node);
        }

        Ok(Box::new(root))
    }

    fn language(&self) -> Language {
        Language::Text
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        Language::Text
            .extensions()
            .iter()
            .any(|ext| file_path.to_string_lossy().ends_with(ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_parser_root_contains_full_text() {
        let parser = TextParser::new();
        let source = "line one\nline two\nline three";
        let node = parser.parse(source, std::path::Path::new("test.txt")).unwrap();
        assert_eq!(node.text(), Some(source));
        assert_eq!(node.child_count(), 3);
    }

    #[test]
    fn test_text_parser_line_children() {
        let parser = TextParser::new();
        let node = parser.parse("hello\nworld", std::path::Path::new("test.txt")).unwrap();
        assert_eq!(node.child(0).unwrap().text(), Some("hello"));
        assert_eq!(node.child(1).unwrap().text(), Some("world"));
    }

    #[test]
    fn test_text_parser_supports_txt_files() {
        let parser = TextParser::new();
        assert!(parser.supports_file(std::path::Path::new("commit.txt")));
        assert!(parser.supports_file(std::path::Path::new("README.md")));
        assert!(!parser.supports_file(std::path::Path::new("Main.java")));
    }

    #[test]
    fn test_text_parser_language() {
        let parser = TextParser::new();
        assert_eq!(parser.language(), Language::Text);
    }
}
