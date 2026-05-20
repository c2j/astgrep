//! Tree-sitter integration and parser setup
//!
//! This module provides tree-sitter parser initialization and
//! core parsing functionality for various programming languages.

use astgrep_ast::UniversalNode;
use astgrep_core::{Language, Result};
use std::collections::HashMap;
use tree_sitter::{Parser, Tree};

/// Pattern types for AST-based matching
#[derive(Debug, Clone)]
pub enum PatternType {
    StringLiteral(String),
    NumericLiteral(String),
    FunctionCall(String),
    ImportStatement(String),
    MethodCall(String, String), // (object, method)
    Identifier(String),
    // Advanced pattern types
    MetaVariable(String),                              // $VAR, $FUNC
    MetaFunctionCall(String, Vec<String>),             // $FUNC($ARG1, $ARG2)
    PatternEither(Vec<PatternType>),                   // pattern-either
    PatternNot(Box<PatternType>),                      // pattern-not
    PatternInside(Box<PatternType>, Box<PatternType>), // pattern-inside
    PatternWhere(Box<PatternType>, String),            // pattern-where
    Generic(String),
}

/// Metavariable bindings for pattern matching
#[derive(Debug, Clone)]
pub struct MetaVariableBindings {
    bindings: std::collections::HashMap<String, String>,
}

impl Default for MetaVariableBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaVariableBindings {
    pub fn new() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
        }
    }

    pub fn bind(&mut self, var_name: &str, value: &str) -> bool {
        if let Some(existing) = self.bindings.get(var_name) {
            // Check if binding is consistent
            existing == value
        } else {
            self.bindings
                .insert(var_name.to_string(), value.to_string());
            true
        }
    }

    pub fn get(&self, var_name: &str) -> Option<&String> {
        self.bindings.get(var_name)
    }
}

/// Tree-sitter based parser
pub struct TreeSitterParser {
    parsers: HashMap<Language, Parser>,
}

impl TreeSitterParser {
    /// Create a new tree-sitter parser
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();

        // Initialize Python parser
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert(Language::Python, parser);
        }

        // Initialize JavaScript parser
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert(Language::JavaScript, parser);
        }

        // Initialize Java parser
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert(Language::Java, parser);
        }

        // Initialize SQL parser via tree-sitter-sequel when feature enabled
        #[cfg(feature = "sql-tree-sitter")]
        {
            let mut parser = Parser::new();
            if parser
                .set_language(&tree_sitter_sequel::LANGUAGE.into())
                .is_ok()
            {
                parsers.insert(Language::Sql, parser);
            }
        }

        // Initialize Bash parser
        {
            let mut parser = Parser::new();
            if parser
                .set_language(&tree_sitter_bash::LANGUAGE.into())
                .is_ok()
            {
                parsers.insert(Language::Bash, parser);
            }
        }

        Ok(Self { parsers })
    }

    /// Parse source code using tree-sitter
    pub fn parse(&mut self, source: &str, language: Language) -> Result<Option<Tree>> {
        if let Some(parser) = self.parsers.get_mut(&language) {
            Ok(parser.parse(source, None))
        } else {
            Ok(None)
        }
    }

    /// Convert tree-sitter tree to universal AST
    pub fn tree_to_universal_ast(&self, tree: &Tree, source: &str) -> Result<UniversalNode> {
        let root_node = tree.root_node();
        self.convert_node(&root_node, source)
    }

    /// Classify a pattern into specific types for AST-based matching
    pub fn classify_pattern(&self, pattern: &str) -> PatternType {
        let pattern = pattern.trim();

        // Check for metavariables first
        if pattern.starts_with('$') {
            return self.classify_metavariable_pattern(pattern);
        }

        if pattern.starts_with('"') && pattern.ends_with('"') && pattern.len() >= 2 {
            // String literal: "hello world"
            PatternType::StringLiteral(pattern[1..pattern.len() - 1].to_string())
        } else if pattern.chars().all(|c| c.is_ascii_digit() || c == '.') {
            // Numeric literal: 42, 3.14
            PatternType::NumericLiteral(pattern.to_string())
        } else if let Some(func_name) = pattern.strip_suffix("(...)") {
            if func_name.starts_with('$') {
                PatternType::MetaFunctionCall(func_name.to_string(), vec![])
            } else {
                PatternType::FunctionCall(func_name.to_string())
            }
        } else if pattern.contains('(') && pattern.contains(')') && pattern.contains('$') {
            // Function call with metavariables: eval($CODE), $FUNC($ARG)
            self.parse_meta_function_call(pattern)
        } else if pattern.starts_with("import ") {
            // Import statement: import foo.bar
            PatternType::ImportStatement(pattern.to_string())
        } else if pattern.contains('.') && !pattern.contains(' ') && !pattern.starts_with('"') {
            // Method call: System.out.println, obj.method, $OBJ.method
            let parts: Vec<&str> = pattern.split('.').collect();
            if parts.len() >= 2 {
                let object = parts[..parts.len() - 1].join(".");
                let method = parts[parts.len() - 1].to_string();
                PatternType::MethodCall(object, method)
            } else {
                PatternType::Generic(pattern.to_string())
            }
        } else if pattern.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // Simple identifier: variable_name
            PatternType::Identifier(pattern.to_string())
        } else {
            // Generic pattern
            PatternType::Generic(pattern.to_string())
        }
    }

    /// Classify metavariable patterns
    fn classify_metavariable_pattern(&self, pattern: &str) -> PatternType {
        if let Some(func_name) = pattern.strip_suffix("(...)") {
            PatternType::MetaFunctionCall(func_name.to_string(), vec![])
        } else if pattern.contains('(') && pattern.contains(')') {
            // Meta function call with args: $FUNC($ARG1, $ARG2)
            self.parse_meta_function_call(pattern)
        } else {
            // Simple metavariable: $VAR
            PatternType::MetaVariable(pattern.to_string())
        }
    }

    /// Parse meta function call with arguments
    fn parse_meta_function_call(&self, pattern: &str) -> PatternType {
        if let Some(open_paren) = pattern.find('(') {
            if let Some(close_paren) = pattern.rfind(')') {
                let func_name = pattern[..open_paren].to_string();
                let args_str = &pattern[open_paren + 1..close_paren];
                let args: Vec<String> = args_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                // Check if this is a method call pattern like $obj.method()
                if func_name.contains('.') {
                    let parts: Vec<&str> = func_name.split('.').collect();
                    if parts.len() >= 2 {
                        let object = parts[..parts.len() - 1].join(".");
                        let method = parts[parts.len() - 1].to_string();
                        // Return MethodCall pattern type for method calls
                        return PatternType::MethodCall(object, method);
                    }
                }

                return PatternType::MetaFunctionCall(func_name, args);
            }
        }
        PatternType::Generic(pattern.to_string())
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            parsers: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[test]
    fn test_python_parsing() {
        let mut parser = TreeSitterParser::new().unwrap();
        let source = r#"
def hello():
    print("world")
    x = 42
"#;

        if let Ok(Some(tree)) = parser.parse(source, Language::Python) {
            let universal_ast = parser.tree_to_universal_ast(&tree, source).unwrap();
            assert_eq!(universal_ast.node_type(), "module");
        }
    }

    #[test]
    fn test_pattern_matching() {
        let mut parser = TreeSitterParser::new().unwrap();
        let source = r#"
print("hello")
x = 42
eval(code)
"#;

        if let Ok(Some(tree)) = parser.parse(source, Language::Python) {
            // Test string literal matching
            let matches = parser
                .find_pattern_matches(&tree, source, r#""hello""#)
                .unwrap();
            assert!(!matches.is_empty());

            // Test numeric literal matching
            let matches = parser.find_pattern_matches(&tree, source, "42").unwrap();
            assert!(!matches.is_empty());

            // Test function call matching
            let matches = parser
                .find_pattern_matches(&tree, source, "eval(...)")
                .unwrap();
            assert!(!matches.is_empty());
        }
    }
}
