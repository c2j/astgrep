//! AST-based pattern tree for semgrep-compatible structural matching.
//!
//! Parses semgrep patterns using tree-sitter into a structured `PatternTree`
//! that mirrors the target language's AST, enabling proper structural matching
//! instead of text-token matching.

use astgrep_core::{Language, Result};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Tree};

// ---------------------------------------------------------------------------
// PatternTree
// ---------------------------------------------------------------------------

/// AST-based representation of a semgrep pattern.
///
/// Unlike `ParsedPattern` (which is a flat token sequence), `PatternTree`
/// preserves the hierarchical structure of the pattern as parsed by
/// tree-sitter with the target language's grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternTree {
    /// Matches a specific AST node type with structured children.
    Node {
        /// tree-sitter node type (e.g., "call_expression", "identifier", "string")
        kind: String,
        /// Child pattern trees (non-trivial children only)
        children: Vec<PatternTree>,
        /// Exact text for terminal nodes (identifiers, literals). `None` for
        /// structural nodes whose identity is determined by kind + children.
        text: Option<String>,
    },

    /// Matches any single AST subtree and binds it to a metavariable.
    /// Semgrep syntax: `$NAME`
    Metavar {
        /// Variable name without the `$` prefix (e.g., "X", "QUERY")
        name: String,
    },

    /// Matches zero or more sibling AST nodes (the `...` ellipsis operator).
    Ellipsis,

    /// Matches zero or more siblings and captures them into a metavariable.
    /// Semgrep syntax: `$...NAME`
    EllipsisMetavar {
        name: String,
    },

    /// Deep expression match — the inner pattern can match at any depth
    /// inside an expression. Semgrep syntax: `<... PAT ...>`  or  `deep(PAT)`
    DeepExpr(Box<PatternTree>),
}

impl PatternTree {
    /// Returns `true` if this tree contains any metavariables or ellipsis.
    pub fn has_wildcards(&self) -> bool {
        match self {
            PatternTree::Metavar { .. }
            | PatternTree::Ellipsis
            | PatternTree::EllipsisMetavar { .. } => true,
            PatternTree::DeepExpr(inner) => inner.has_wildcards(),
            PatternTree::Node { children, .. } => children.iter().any(|c| c.has_wildcards()),
        }
    }

    /// Returns the `kind` field for `Node` variants, for diagnostics.
    pub fn kind_str(&self) -> &str {
        match self {
            PatternTree::Node { kind, .. } => kind,
            PatternTree::Metavar { name } => name,
            PatternTree::Ellipsis => "...",
            PatternTree::EllipsisMetavar { name } => name,
            PatternTree::DeepExpr(_) => "deep_expr",
        }
    }
}

// ---------------------------------------------------------------------------
// Trivial node types (punctuation to skip during matching)
// ---------------------------------------------------------------------------

const TRIVIAL_NODE_TYPES: &[&str] = &[
    "(", ")", "{", "}", "[", "]", ";", ",", ".", ":", "::",
    // tree-sitter punctuation node types
    "open_paren", "close_paren", "open_brace", "close_brace",
    "open_bracket", "close_bracket",
    // formatting
    "comment", "line_comment", "block_comment",
];

fn is_trivial_node(node: &Node) -> bool {
    let kind = node.kind();
    if kind.is_empty() {
        return true;
    }
    TRIVIAL_NODE_TYPES.contains(&kind)
        || node.is_extra()
        || (node.child_count() == 0 && kind.len() == 1 && !kind.chars().next().map_or(false, |c| c.is_alphanumeric()))
}

// ---------------------------------------------------------------------------
// Placeholder constants for preprocessing
// ---------------------------------------------------------------------------

const MG_PREFIX: &str = "__mg_";
const MG_SUFFIX: &str = "__";
const MGE_PREFIX: &str = "__mge_";
const ELLIPSIS_PLACEHOLDER: &str = "__ellipsis__";

// ---------------------------------------------------------------------------
// PatternTreeParser
// ---------------------------------------------------------------------------

/// Parses semgrep patterns into `PatternTree` using tree-sitter.
///
/// # Algorithm
///
/// 1. **Preprocess**: Replace metavariables (`$X`) with valid identifiers
///    (`__mg_X__`), ellipsis (`...`) with `__ellipsis__`.
/// 2. **Parse**: Use tree-sitter with the target language grammar to get a CST.
/// 3. **Post-process**: Walk the CST, identifying metavar/ellipsis placeholders
///    and converting to `PatternTree`.
pub struct PatternTreeParser {
    parsers: HashMap<Language, Parser>,
}

impl PatternTreeParser {
    /// Create a new parser with tree-sitter grammars for all supported languages.
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();

        let _ = Self::init_language(&mut parsers, Language::Python, || {
            let mut p = Parser::new();
            p.set_language(&tree_sitter_python::LANGUAGE.into())?;
            Ok(p)
        });

        let _ = Self::init_language(&mut parsers, Language::JavaScript, || {
            let mut p = Parser::new();
            p.set_language(&tree_sitter_javascript::LANGUAGE.into())?;
            Ok(p)
        });

        let _ = Self::init_language(&mut parsers, Language::Java, || {
            let mut p = Parser::new();
            p.set_language(&tree_sitter_java::LANGUAGE.into())?;
            Ok(p)
        });

        let _ = Self::init_language(&mut parsers, Language::Bash, || {
            let mut p = Parser::new();
            p.set_language(&tree_sitter_bash::LANGUAGE.into())?;
            Ok(p)
        });

        Ok(Self { parsers })
    }

    fn init_language(
        map: &mut HashMap<Language, Parser>,
        lang: Language,
        init: impl FnOnce() -> std::result::Result<Parser, tree_sitter::LanguageError>,
    ) -> std::result::Result<(), tree_sitter::LanguageError> {
        let parser = init()?;
        map.insert(lang, parser);
        Ok(())
    }

    /// Parse a semgrep pattern into a `PatternTree`.
    pub fn parse(&mut self, pattern: &str, language: Language) -> Result<PatternTree> {
        let (preprocessed, meta_map) = preprocess_pattern(pattern);
        let tree = self.parse_with_tree_sitter(&preprocessed, language)?;

        let root = tree.root_node();
        let source = &preprocessed;

        // Find the first meaningful node (skip root/program wrappers)
        let meaningful = self.find_meaningful_node(&root, source);
        let node = meaningful.as_ref().unwrap_or(&root);

        Ok(self.convert_node(node, source, &meta_map))
    }

    /// Parse preprocessed source with tree-sitter. Tries direct parse first;
    /// if the result is an error node, wraps in a language-specific context and retries.
    fn parse_with_tree_sitter(&mut self, source: &str, language: Language) -> Result<Tree> {
        let parser = self.parsers.get_mut(&language)
            .ok_or_else(|| astgrep_core::AnalysisError::parse_error(
                &format!("No tree-sitter parser for {:?}", language)
            ))?;

        if let Some(tree) = parser.parse(source, None) {
            let root = tree.root_node();
            if !root.has_error() || Self::has_meaningful_content(&root) {
                return Ok(tree);
            }
        }

        let wrapped = Self::wrap_in_context_static(source, language);
        if let Some(tree) = parser.parse(&wrapped, None) {
            return Ok(tree);
        }

        Err(astgrep_core::AnalysisError::parse_error(
            &format!("Failed to parse pattern with tree-sitter: {:?}", source)
        ))
    }

    /// Wrap a pattern in minimal valid context for the language.
    fn wrap_in_context(&self, pattern: &str, language: Language) -> String {
        Self::wrap_in_context_static(pattern, language)
    }

    fn wrap_in_context_static(pattern: &str, language: Language) -> String {
        match language {
            Language::Java => format!("class __Wrap__ {{ void m() {{ {} }} }}", pattern),
            Language::JavaScript => format!("function __wrap__() {{ {} }}", pattern),
            Language::Python => format!("def __wrap__():\n    {}", pattern),
            Language::Bash => pattern.to_string(),
            _ => pattern.to_string(),
        }
    }

    /// Find the first meaningful (non-wrapper) AST node.
    fn find_meaningful_node<'a>(&self, root: &Node<'a>, source: &str) -> Option<Node<'a>> {
        // Skip program/root nodes to get to the actual pattern content
        let mut current = *root;
        loop {
            let kind = current.kind();
            // Skip wrapper nodes
            if matches!(kind, "program" | "module" | "translation_unit" | "source_file" | "script" | "expression_statement") {
                if current.child_count() == 1 {
                    if let Some(child) = current.child(0) {
                        current = child;
                        continue;
                    }
                }
                // Multiple children — return the first non-trivial one
                for i in 0..current.child_count() {
                    if let Some(child) = current.child(i) {
                        if !is_trivial_node(&child) && !child.is_extra() {
                            return Some(child);
                        }
                    }
                }
            }
            // For wrapped contexts, find the deepest statement
            if matches!(kind,
                "class_declaration" | "class_body" | "method_declaration" |
                "block" | "function_declaration" | "function_definition" |
                "statement_block" | "body" | "compound_statement"
            ) {
                // Dive into the body to find the actual statement
                for i in 0..current.child_count() {
                    if let Some(child) = current.child(i) {
                        if !is_trivial_node(&child) {
                            let inner = self.find_meaningful_node(&child, source);
                            if inner.is_some() {
                                return inner;
                            }
                        }
                    }
                }
            }
            break;
        }
        if current.kind() != "program" && current.kind() != "source_file" && !current.is_error() {
            Some(current)
        } else {
            None
        }
    }

    fn has_meaningful_content(root: &Node) -> bool {
        // Check if there's at least one non-error, non-trivial child
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                if !child.is_error() && !is_trivial_node(&child) {
                    return true;
                }
            }
        }
        false
    }

    /// Convert a tree-sitter CST node into a `PatternTree`.
    fn convert_node(
        &self,
        node: &Node,
        source: &str,
        meta_map: &HashMap<String, PlaceholderKind>,
    ) -> PatternTree {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");

        // Check if this entire node text is a metavar placeholder
        if let Some(kind) = meta_map.get(text) {
            return match kind {
                PlaceholderKind::Metavar(name) => PatternTree::Metavar { name: name.clone() },
                PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                PlaceholderKind::EllipsisMetavar(name) => {
                    PatternTree::EllipsisMetavar { name: name.clone() }
                }
            };
        }

        // Collect non-trivial children
        let children: Vec<PatternTree> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .filter(|c| !is_trivial_node(c))
            .map(|child| self.convert_node(&child, source, meta_map))
            .collect();

        // Determine if we should store text
        let is_terminal = node.child_count() == 0
            || children.is_empty()
            || matches!(node.kind(),
                "identifier" | "type_identifier" | "property_identifier" |
                "string" | "string_literal" | "number" | "number_literal" |
                "integer" | "float" | "true" | "false" | "null"
            );

        // For identifiers, check if text matches a metavar placeholder
        if node.kind() == "identifier" || node.kind() == "type_identifier" || node.kind() == "property_identifier" {
            if let Some(kind) = meta_map.get(text) {
                return match kind {
                    PlaceholderKind::Metavar(name) => PatternTree::Metavar { name: name.clone() },
                    PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                    PlaceholderKind::EllipsisMetavar(name) => {
                        PatternTree::EllipsisMetavar { name: name.clone() }
                    }
                };
            }
        }

        // For string literals, check if the unquoted content is a metavar placeholder
        // Pattern: foo("$VAR") → tree-sitter gives string node with text '"__mg_VAR__"'
        if node.kind() == "string" || node.kind() == "string_literal" {
            let unquoted = text
                .strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
            if let Some(inner) = unquoted {
                if let Some(kind) = meta_map.get(inner) {
                    return match kind {
                        PlaceholderKind::Metavar(name) => PatternTree::Metavar { name: name.clone() },
                        PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                        PlaceholderKind::EllipsisMetavar(name) => {
                            PatternTree::EllipsisMetavar { name: name.clone() }
                        }
                    };
                }
            }
        }

        PatternTree::Node {
            kind: node.kind().to_string(),
            children,
            text: if is_terminal { Some(text.to_string()) } else { None },
        }
    }
}

impl Default for PatternTreeParser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self { parsers: HashMap::new() })
    }
}

// ---------------------------------------------------------------------------
// Preprocessing
// ---------------------------------------------------------------------------

/// What kind of placeholder a preprocessed token represents.
#[derive(Debug, Clone, PartialEq)]
enum PlaceholderKind {
    Metavar(String),
    Ellipsis,
    EllipsisMetavar(String),
}

/// Preprocess a semgrep pattern by replacing metavariables and ellipsis
/// with valid identifiers that tree-sitter can parse.
///
/// Returns the preprocessed string and a map from placeholder → kind.
fn preprocess_pattern(pattern: &str) -> (String, HashMap<String, PlaceholderKind>) {
    let mut result = String::with_capacity(pattern.len() * 2);
    let mut meta_map = HashMap::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' {
            // Check for ellipsis metavar: $...NAME
            if i + 3 < chars.len() && chars[i + 1] == '.' && chars[i + 2] == '.' && chars[i + 3] == '.' {
                // Collect the name after $...
                let mut name = String::new();
                let mut j = i + 4;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    name.push(chars[j]);
                    j += 1;
                }
                if !name.is_empty() {
                    let placeholder = format!("{}{}{}", MGE_PREFIX, name, MG_SUFFIX);
                    meta_map.insert(placeholder.clone(), PlaceholderKind::EllipsisMetavar(name));
                    result.push_str(&placeholder);
                    i = j;
                    continue;
                }
            }

            // Regular metavariable: $NAME
            let mut name = String::new();
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                name.push(chars[j]);
                j += 1;
            }
            if !name.is_empty() {
                let placeholder = format!("{}{}{}", MG_PREFIX, name, MG_SUFFIX);
                meta_map.insert(placeholder.clone(), PlaceholderKind::Metavar(name));
                result.push_str(&placeholder);
                i = j;
                continue;
            }

            // Bare $ — keep as-is
            result.push('$');
            i += 1;
        } else if chars[i] == '.' && i + 2 < chars.len() && chars[i + 1] == '.' && chars[i + 2] == '.' {
            // Standalone ellipsis: ...
            meta_map.insert(
                ELLIPSIS_PLACEHOLDER.to_string(),
                PlaceholderKind::Ellipsis,
            );
            result.push_str(ELLIPSIS_PLACEHOLDER);
            i += 3;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    (result, meta_map)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Preprocessing tests ----

    #[test]
    fn test_preprocess_metavar() {
        let (result, map) = preprocess_pattern("$X.execute($QUERY)");
        assert!(result.contains("__mg_X__"));
        assert!(result.contains("__mg_QUERY__"));
        assert!(result.contains(".execute("));
        assert_eq!(map.len(), 2);
        assert!(matches!(map.get("__mg_X__"), Some(PlaceholderKind::Metavar(n)) if n == "X"));
        assert!(matches!(map.get("__mg_QUERY__"), Some(PlaceholderKind::Metavar(n)) if n == "QUERY"));
    }

    #[test]
    fn test_preprocess_ellipsis() {
        let (result, map) = preprocess_pattern("foo(...)");
        assert!(result.contains("__ellipsis__"));
        assert!(matches!(
            map.get("__ellipsis__"),
            Some(PlaceholderKind::Ellipsis)
        ));
    }

    #[test]
    fn test_preprocess_ellipsis_metavar() {
        let (result, map) = preprocess_pattern("foo($...ARGS)");
        assert!(result.contains("__mge_ARGS__"));
        assert!(matches!(
            map.get("__mge_ARGS__"),
            Some(PlaceholderKind::EllipsisMetavar(n)) if n == "ARGS"
        ));
    }

    #[test]
    fn test_preprocess_mixed() {
        let (result, map) = preprocess_pattern("$STMT.execute($QUERY)");
        assert!(result.contains("__mg_STMT__"));
        assert!(result.contains("__mg_QUERY__"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_preprocess_no_metavars() {
        let (result, map) = preprocess_pattern("foo(1, 2)");
        assert_eq!(result, "foo(1, 2)");
        assert!(map.is_empty());
    }

    #[test]
    fn test_preprocess_dots_not_ellipsis() {
        // Single or double dots should not be treated as ellipsis
        let (result, map) = preprocess_pattern("obj.method");
        assert_eq!(result, "obj.method");
        assert!(map.is_empty());
    }

    // ---- Parsing tests ----

    #[test]
    fn test_parse_simple_call_javascript() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("foo(1)", Language::JavaScript).unwrap();

        if let PatternTree::Node { kind, children, text } = &tree {
            assert_eq!(kind, "call_expression");
            assert!(!children.is_empty());
            assert!(text.is_none());
        } else {
            panic!("Expected Node, got {:?}", tree);
        }
    }

    #[test]
    fn test_parse_metavar_call_javascript() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("$X.execute($Y)", Language::JavaScript).unwrap();

        // Should contain Metavar nodes
        let has_metavars = tree.has_wildcards();
        assert!(has_metavars, "Pattern should have metavar wildcards");
    }

    #[test]
    fn test_parse_ellipsis_javascript() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("foo(...)", Language::JavaScript).unwrap();

        let has_ellipsis = contains_ellipsis(&tree);
        assert!(has_ellipsis, "Pattern should contain ellipsis");
    }

    #[test]
    fn test_parse_simple_call_python() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("foo(1)", Language::Python).unwrap();

        if let PatternTree::Node { kind, .. } = &tree {
            // Python tree-sitter may wrap in different node types
            assert!(!kind.is_empty());
        } else {
            panic!("Expected Node, got {:?}", tree);
        }
    }

    #[test]
    fn test_parse_metavar_java() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("$STMT.execute($QUERY)", Language::Java).unwrap();

        let has_metavars = tree.has_wildcards();
        assert!(has_metavars, "Java pattern should have metavar wildcards");
    }

    #[test]
    fn test_parse_identifier_python() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("x", Language::Python).unwrap();

        // Python wraps standalone identifiers in expression_statement
        if let PatternTree::Node { kind, children, .. } = &tree {
            if kind == "expression_statement" && children.len() == 1 {
                if let PatternTree::Node { kind: inner_kind, text, .. } = &children[0] {
                    assert_eq!(inner_kind.as_str(), "identifier");
                    assert_eq!(text.as_deref(), Some("x"));
                    return;
                }
            }
            // Direct identifier
            assert_eq!(kind, "identifier");
        } else {
            panic!("Expected Node, got {:?}", tree);
        }
    }

    #[test]
    fn test_parse_metavar_standalone() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("$X", Language::Python).unwrap();

        // A standalone metavar should be recognized
        assert!(tree.has_wildcards(), "Standalone $X should have wildcards");
    }

    // Helper: check if tree contains ellipsis
    fn contains_ellipsis(tree: &PatternTree) -> bool {
        match tree {
            PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. } => true,
            PatternTree::Node { children, .. } => children.iter().any(contains_ellipsis),
            PatternTree::DeepExpr(inner) => contains_ellipsis(inner),
            PatternTree::Metavar { .. } => false,
        }
    }
}
