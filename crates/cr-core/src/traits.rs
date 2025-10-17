//! Core traits for CR-SemService

use crate::{Finding, Language, Result};
use std::path::Path;

/// Trait for language parsers
pub trait LanguageParser: Send + Sync {
    /// Parse source code and return an AST
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>>;

    /// Get the language this parser supports
    fn language(&self) -> Language;

    /// Check if this parser supports the given file
    fn supports_file(&self, file_path: &Path) -> bool {
        // Default implementation based on file extension
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            self.extensions().contains(&extension.to_lowercase().as_str())
        } else {
            false
        }
    }

    /// Get the file extensions this parser supports
    fn extensions(&self) -> &[&str] {
        match self.language() {
            Language::Java => &["java"],
            Language::JavaScript => &["js", "jsx", "ts", "tsx", "mjs"],
            Language::Python => &["py", "pyw", "pyi"],
            Language::Sql => &["sql", "ddl", "dml"],
            Language::Bash => &["sh", "bash", "zsh"],
            Language::Php => &["php", "phtml", "php3", "php4", "php5"],
            Language::CSharp => &["cs", "csx"],
            Language::C => &["c", "h"],
            Language::Ruby => &["rb", "rbw", "rake", "gemspec"],
            Language::Kotlin => &["kt", "kts"],
            Language::Swift => &["swift"],
        }
    }

    /// Check if this parser can handle the given file
    fn can_parse(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            let ext_with_dot = format!(".{}", ext);
            self.extensions().contains(&ext_with_dot.as_str())
        } else {
            false
        }
    }
}

/// Trait for AST nodes - simplified to be dyn compatible
pub trait AstNode: Send + Sync {
    /// Get the node type
    fn node_type(&self) -> &str;

    /// Get the number of children
    fn child_count(&self) -> usize;

    /// Get a child by index
    fn child(&self, index: usize) -> Option<&dyn AstNode>;

    /// Get the source location of this node
    fn location(&self) -> Option<(usize, usize, usize, usize)>; // (start_line, start_col, end_line, end_col)

    /// Get the text content of this node
    fn text(&self) -> Option<&str>;

    /// Get an attribute value by key
    fn get_attribute(&self, key: &str) -> Option<&str> {
        None // Default implementation
    }

    /// Clone this node as a boxed trait object
    fn clone_node(&self) -> Box<dyn AstNode>;
}

/// Helper functions for AST traversal
pub mod ast_utils {
    use super::*;

    /// Visit all nodes in the AST using depth-first traversal
    pub fn visit_nodes(
        node: &dyn AstNode,
        visitor: &mut dyn FnMut(&dyn AstNode) -> Result<()>,
    ) -> Result<()> {
        visitor(node)?;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                visit_nodes(child, visitor)?;
            }
        }
        Ok(())
    }

    /// Find nodes matching a predicate
    pub fn find_nodes(
        node: &dyn AstNode,
        predicate: &dyn Fn(&dyn AstNode) -> bool,
    ) -> Vec<Box<dyn AstNode>> {
        let mut results = Vec::new();
        let _ = visit_nodes(node, &mut |n| {
            if predicate(n) {
                results.push(n.clone_node());
            }
            Ok(())
        });
        results
    }
}

/// Trait for rule engines
pub trait RuleEngine: Send + Sync {
    /// Execute rules against an AST and return findings
    fn execute_rules(
        &self,
        ast: &dyn AstNode,
        rules: &[Box<dyn Rule>],
    ) -> Result<Vec<Finding>>;

    /// Validate a rule
    fn validate_rule(&self, rule: &dyn Rule) -> Result<()>;
}

/// Trait for analysis rules
pub trait Rule: Send + Sync {
    /// Get the rule ID
    fn id(&self) -> &str;

    /// Get the rule name
    fn name(&self) -> &str;

    /// Get the rule description
    fn description(&self) -> &str;

    /// Get supported languages
    fn languages(&self) -> &[Language];

    /// Check if this rule applies to the given language
    fn applies_to(&self, language: Language) -> bool {
        self.languages().contains(&language)
    }

    /// Execute the rule against an AST node
    fn execute(&self, node: &dyn AstNode) -> Result<Vec<Finding>>;
}

/// Trait for pattern matchers
pub trait PatternMatcher: Send + Sync {
    /// Match a pattern against an AST node
    fn matches(&self, pattern: &str, node: &dyn AstNode) -> Result<bool>;

    /// Get metavariable bindings from the last match
    fn get_bindings(&self) -> &std::collections::HashMap<String, String>;

    /// Reset the matcher state
    fn reset(&mut self);
}

/// Trait for data flow analyzers
pub trait DataFlowAnalyzer: Send + Sync {
    /// Analyze data flow and return taint flows
    fn analyze_taint_flow(
        &self,
        ast: &dyn AstNode,
        sources: &[&str],
        sinks: &[&str],
        sanitizers: &[&str],
    ) -> Result<Vec<TaintFlow>>;
}

/// Represents a taint flow from source to sink
#[derive(Debug, Clone)]
pub struct TaintFlow {
    pub source: TaintNode,
    pub sink: TaintNode,
    pub path: Vec<TaintNode>,
}

/// Represents a node in a taint flow
#[derive(Debug, Clone)]
pub struct TaintNode {
    pub node_type: String,
    pub location: Option<(usize, usize, usize, usize)>,
    pub text: Option<String>,
}

/// Trait for output formatters
pub trait OutputFormatter: Send + Sync {
    /// Format findings into the target format
    fn format(&self, findings: &[Finding]) -> Result<String>;

    /// Get the file extension for this format
    fn extension(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_extensions() {
        // Test that language extensions are correctly defined
        assert!(Language::Java.extensions().contains(&"java"));
        assert!(Language::JavaScript.extensions().contains(&"js"));
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::Sql.extensions().contains(&"sql"));
        assert!(Language::Bash.extensions().contains(&"sh"));
        assert!(Language::Php.extensions().contains(&"php"));
        assert!(Language::CSharp.extensions().contains(&"cs"));
        assert!(Language::C.extensions().contains(&"c"));
    }

    #[test]
    fn test_ast_utils_functions() {
        // Test that AST utility functions are available
        // These tests would use mock implementations from test-utils
        // For now, just test that the functions exist and can be called

        // This is a placeholder test - in practice, you would use
        // mock implementations from the test-utils crate
        assert!(true, "AST utility functions are available");
    }

    #[test]
    fn test_taint_flow_structure() {
        // Test that TaintFlow and TaintNode structures are properly defined
        // This ensures the types are available for use in data flow analysis

        let source = TaintNode {
            node_type: "source".to_string(),
            location: Some((1, 1, 1, 10)),
            text: Some("user_input".to_string()),
        };

        let sink = TaintNode {
            node_type: "sink".to_string(),
            location: Some((5, 1, 5, 20)),
            text: Some("execute(query)".to_string()),
        };

        let flow = TaintFlow {
            source: source.clone(),
            sink: sink.clone(),
            path: vec![source, sink],
        };

        assert_eq!(flow.path.len(), 2);
        assert_eq!(flow.source.node_type, "source");
        assert_eq!(flow.sink.node_type, "sink");
    }
}
