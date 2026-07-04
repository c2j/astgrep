//! AST-based pattern tree for semgrep-compatible structural matching.
//!
//! Parses semgrep patterns using tree-sitter into a structured `PatternTree`
//! that mirrors the target language's AST, enabling proper structural matching
//! instead of text-token matching.

use astgrep_core::{Language, Result};
use astgrep_ast::UniversalNode;
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
        /// Literal attribute constraints (key, expected_value) from ogsql patterns.
        /// Example: ("has_lock", "true"), ("lock_type", "Update").
        constraints: Vec<(String, String)>,
    },

    /// Matches any single AST subtree and binds it to a metavariable.
    /// Semgrep syntax: `$NAME`
    Metavar {
        /// Variable name without the `$` prefix (e.g., "X", "QUERY")
        name: String,
        /// If set, bind to this metadata attribute instead of node.text().
        /// Supported syntax: `$NAME@attr` (e.g., `$VAR@into_vars`)
        bind_attr: Option<String>,
    },

    /// Matches zero or more sibling AST nodes (the `...` ellipsis operator).
    Ellipsis,

    /// Matches zero or more siblings and captures them into a metavariable.
    /// Semgrep syntax: `$...NAME`
    EllipsisMetavar { name: String },

    /// Deep expression match — the inner pattern can match at any depth
    /// inside an expression. Semgrep syntax: `<... PAT ...>`  or  `deep(PAT)`
    DeepExpr(Box<PatternTree>),

    /// Typed metavariable with a type constraint.
    /// Semgrep syntax: `$NAME: TYPE` (e.g., `$X: int`, `$X: str`, `$X: bool`)
    /// Only matches when the target node's type is compatible.
    TypedMetavar { name: String, type_name: String },
}

impl PatternTree {
    /// Returns `true` if this tree contains any metavariables or ellipsis.
    pub fn has_wildcards(&self) -> bool {
        match self {
            PatternTree::Metavar { .. }
            | PatternTree::Ellipsis
            | PatternTree::EllipsisMetavar { .. }
            | PatternTree::TypedMetavar { .. } => true,
            PatternTree::DeepExpr(inner) => inner.has_wildcards(),
            PatternTree::Node { children, .. } => children.iter().any(|c| c.has_wildcards()),
        }
    }

    /// Returns the `kind` field for `Node` variants, for diagnostics.
    pub fn kind_str(&self) -> &str {
        match self {
            PatternTree::Node { kind, .. } => kind,
            PatternTree::Metavar { name, .. } => name,
            PatternTree::Ellipsis => "...",
            PatternTree::EllipsisMetavar { name } => name,
            PatternTree::DeepExpr(_) => "deep_expr",
            PatternTree::TypedMetavar { name, .. } => name,
        }
    }
}

// ---------------------------------------------------------------------------
// Trivial node types (punctuation to skip during matching)
// ---------------------------------------------------------------------------

const TRIVIAL_NODE_TYPES: &[&str] = &[
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ";",
    ",",
    ".",
    ":",
    "::",
    "open_paren",
    "close_paren",
    "open_brace",
    "close_brace",
    "open_bracket",
    "close_bracket",
    "comment",
    "line_comment",
    "block_comment",
];

const BINARY_OPERATORS: &[&str] = &[
    "+", "-", "*", "/", "%", "**", "&", "|", "^", "~", "<<", ">>", "&&", "||", "==", "!=", "<",
    ">", "<=", ">=", "===", "!==",
];

fn is_trivial_node(node: &Node) -> bool {
    let kind = node.kind();
    if kind.is_empty() {
        return true;
    }
    TRIVIAL_NODE_TYPES.contains(&kind)
        || node.is_extra()
        || (node.child_count() == 0
            && kind.len() <= 2
            && !kind.chars().next().map_or(false, |c| c.is_alphanumeric())
            && !BINARY_OPERATORS.contains(&kind))
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
        let trimmed = pattern.trim();
        let (preprocessed, meta_map) = preprocess_pattern(trimmed);

        // Route to ogsql-parser for patterns with metadata binding (@) or
        // PL/pgSQL syntax (:= assignment, multi-statement with ;).
        let needs_ogsql = trimmed.contains('@')
            || trimmed.contains(":=")
            || trimmed.matches(';').count() > 1;
        if matches!(language, Language::Sql) && needs_ogsql {
            return self.parse_ogsql(&preprocessed, &meta_map);
        }

        let tree = self.parse_with_tree_sitter(&preprocessed, language)?;

        let root = tree.root_node();
        let source = &preprocessed;

        let meaningful = self.find_meaningful_node(&root, source);
        let node = meaningful.as_ref().unwrap_or(&root);

        Ok(self.convert_node(node, source, &meta_map))
    }

    /// Parse SQL pattern using ogsql-parser (supports PL/pgSQL syntax like
    /// SELECT INTO variable, BULK COLLECT, etc. that tree-sitter-sequel cannot handle).
    fn parse_ogsql(
        &self,
        preprocessed: &str,
        meta_map: &HashMap<String, PlaceholderKind>,
    ) -> Result<PatternTree> {
        // If the preprocessed string is just a single metavar placeholder
        // (no SQL structure), handle it directly via meta_map.
        let trimmed = preprocessed.trim();
        if let Some(kind) = meta_map.get(trimmed) {
            return Ok(match kind {
                PlaceholderKind::Metavar { name, bind_attr } => {
                    PatternTree::Metavar {
                        name: name.clone(),
                        bind_attr: bind_attr.clone(),
                    }
                }
                PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                PlaceholderKind::EllipsisMetavar(name) => {
                    PatternTree::EllipsisMetavar { name: name.clone() }
                }
                PlaceholderKind::TypedMetavar { name, type_name } => {
                    PatternTree::TypedMetavar {
                        name: name.clone(),
                        type_name: type_name.clone(),
                    }
                }
            });
        }

        let nodes = crate::adapter::ogsql::OgsqlAdapter::parse_to_universal(preprocessed)
            .unwrap_or_else(|_| Vec::new());

        // If direct parsing fails (multi-statement PL/pgSQL patterns with ...),
        // wrap in a DO block and retry so ogsql-parser handles PL/pgSQL syntax.
        let nodes = if nodes.is_empty() && (preprocessed.contains(":=") || preprocessed.matches(';').count() > 1) {
            let wrapped = format!("DO $$ BEGIN {} END $$;", preprocessed);
            crate::adapter::ogsql::OgsqlAdapter::parse_to_universal(&wrapped)
                .unwrap_or_else(|_| Vec::new())
        } else {
            nodes
        };

        if nodes.is_empty() {
            return Err(astgrep_core::AnalysisError::parse_error(
                "ogsql-parser produced empty result for pattern",
            ));
        }

        // Convert first statement's UniversalNode to PatternTree.
        // Collect all metadata-bound metavars into a flat wildcard pattern
        // so each metavar can independently match against the right target node.
        let mut metavar_children = Vec::new();
        Self::collect_ogsql_metavars(&nodes[0], meta_map, &mut metavar_children);

        // Also collect literal attribute constraints (non-placeholder) for enforcement.
        let mut constraints: Vec<(String, String)> = Vec::new();
        Self::collect_ogsql_attr_constraints(&nodes[0], meta_map, &mut constraints);

        if metavar_children.is_empty() && constraints.is_empty() {
            return Ok(Self::universal_to_pattern_tree(&nodes[0], meta_map));
        }

        // Use kind "_" (matches any node) with metavar children.
        // find_recursive tries this against every target node, binding each metavar
        // when its bind_attr matches the target's attributes.
        Ok(PatternTree::Node {
            kind: "_".to_string(),
            children: metavar_children,
            text: None,
            constraints,
        })
    }

    /// Walk the UniversalNode tree and collect all metadata-bound metavar
    /// placeholders into a flat list of PatternTree children.
    fn collect_ogsql_metavars(
        node: &UniversalNode,
        meta_map: &HashMap<String, PlaceholderKind>,
        out: &mut Vec<PatternTree>,
    ) {
        // Check node's text
        if let Some(ref text) = node.text {
            let trimmed = text.trim().to_string();
            if let Some(kind) = meta_map.get(&trimmed) {
                if let PlaceholderKind::Metavar { name, bind_attr } = kind {
                    let inferred = if bind_attr.is_none() {
                        let mut found = None;
                        for (k, v) in &node.attributes {
                            if v.as_str() == trimmed.as_str() {
                                found = Some(k.clone());
                                break;
                            }
                        }
                        found
                    } else {
                        None
                    };
                    let final_attr = bind_attr.clone().or(inferred);
                    // Only include metavars with concrete bind_attr.
                    // Bare $VAR binds to node.text() which varies across statements
                    // and cannot be reliably unified.
                    if let Some(attr) = final_attr {
                        out.push(PatternTree::Metavar {
                            name: name.clone(),
                            bind_attr: Some(attr),
                        });
                    }
                }
            }
        }
        // Check node's metadata attributes
        for (key, value) in &node.attributes {
            if let Some(PlaceholderKind::Metavar { name, .. }) = meta_map.get(value) {
                if !out.iter().any(|c| {
                    if let PatternTree::Metavar { name: n, bind_attr: Some(a) } = c {
                        n == name && a == key.as_str()
                    } else {
                        false
                    }
                }) {
                    out.push(PatternTree::Metavar {
                        name: name.clone(),
                        bind_attr: Some(key.clone()),
                    });
                }
            }
        }
        // Recurse into children
        for child in &node.children {
            Self::collect_ogsql_metavars(child, meta_map, out);
        }
    }

    /// Walk the UniversalNode tree and collect literal attribute constraints
    /// (non-placeholder metadata like has_lock="true", lock_type="Update").
    fn collect_ogsql_attr_constraints(
        node: &UniversalNode,
        meta_map: &HashMap<String, PlaceholderKind>,
        out: &mut Vec<(String, String)>,
    ) {
        for (key, value) in &node.attributes {
            if meta_map.contains_key(value) { continue; }
            if key == "has_order_by" || key == "has_limit" || key == "has_returning"
                || key == "set_operation" || key == "distinct" || key == "has_group_by"
                || key == "has_having" || key == "has_cte" || key == "plan_hints"
            { continue; }
            out.push((key.clone(), value.clone()));
        }
        for child in &node.children {
            Self::collect_ogsql_attr_constraints(child, meta_map, out);
        }
    }

    /// Convert a UniversalNode (from ogsql-parser) to PatternTree.
    /// Resolves placeholders back to metavariables/ellipsis using meta_map.
    fn universal_to_pattern_tree(
        node: &UniversalNode,
        meta_map: &HashMap<String, PlaceholderKind>,
    ) -> PatternTree {
        // Check if this node's text matches a placeholder
        if let Some(ref text) = node.text {
            let trimmed = text.trim().to_string();
            if let Some(kind) = meta_map.get(&trimmed) {
                let result = match kind {
                    PlaceholderKind::Metavar { name, bind_attr } => {
                        // If bind_attr is None, try to infer it from the node's attributes.
                        // E.g., $VAR inside select_statement → check attributes for __mg_VAR__
                        // → if into_vars="__mg_VAR__", bind to "into_vars".
                        let inferred = if bind_attr.is_none() {
                            let mut found = None;
                            for (k, v) in &node.attributes {
                                if v.as_str() == trimmed.as_str() {
                                    found = Some(k.clone());
                                    break;
                                }
                            }
                            found
                        } else {
                            None
                        };
                        PatternTree::Metavar {
                            name: name.clone(),
                            bind_attr: bind_attr.clone().or(inferred),
                        }
                    }
                    PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                    PlaceholderKind::EllipsisMetavar(name) => {
                        PatternTree::EllipsisMetavar { name: name.clone() }
                    }
                    PlaceholderKind::TypedMetavar { name, type_name } => {
                        PatternTree::TypedMetavar {
                            name: name.clone(),
                            type_name: type_name.clone(),
                        }
                    }
                };
                // Also check metadata for additional bind_attr overrides
                return Self::try_metadata_bind(result, node, meta_map);
            }
        }

        // Check metadata attributes for placeholder values.
        // Override bind_attr with the actual metadata key on the target node.
        for (key, value) in &node.attributes {
            if let Some(kind) = meta_map.get(value) {
                return match kind {
                    PlaceholderKind::Metavar { name, .. } => {
                        PatternTree::Metavar {
                            name: name.clone(),
                            bind_attr: Some(key.clone()),
                        }
                    }
                    _ => PatternTree::Ellipsis,
                };
            }
        }

        // Build children from the UniversalNode's children
        let mut children: Vec<PatternTree> = Vec::new();
        for child in &node.children {
            children.push(Self::universal_to_pattern_tree(child, meta_map));
        }

        let kind_str = node.node_type.to_string();
        PatternTree::Node {
            kind: kind_str,
            children,
            text: node.text.clone(),
            constraints: Vec::new(),
        }
    }

    /// If result is a bare Metavar (bind_attr: None), try to infer bind_attr
    /// from the node's metadata attributes.
    fn try_metadata_bind(
        result: PatternTree,
        node: &UniversalNode,
        meta_map: &HashMap<String, PlaceholderKind>,
    ) -> PatternTree {
        if let PatternTree::Metavar { ref name, ref bind_attr, .. } = &result {
            if bind_attr.is_none() {
                for (key, value) in &node.attributes {
                    if let Some(PlaceholderKind::Metavar { name: pn, .. }) = meta_map.get(value) {
                        if pn == name {
                            return PatternTree::Metavar {
                                name: name.clone(),
                                bind_attr: Some(key.clone()),
                            };
                        }
                    }
                }
            }
        }
        result
    }

    fn parse_with_tree_sitter(&mut self, source: &str, language: Language) -> Result<Tree> {
        let parser = self.parsers.get_mut(&language).ok_or_else(|| {
            astgrep_core::AnalysisError::parse_error(&format!(
                "No tree-sitter parser for {:?}",
                language
            ))
        })?;

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

        Err(astgrep_core::AnalysisError::parse_error(&format!(
            "Failed to parse pattern with tree-sitter: {:?}",
            source
        )))
    }

    /// Wrap a pattern in minimal valid context for the language.
    fn wrap_in_context(&self, pattern: &str, language: Language) -> String {
        Self::wrap_in_context_static(pattern, language)
    }

    fn wrap_in_context_static(pattern: &str, language: Language) -> String {
        match language {
            Language::Java => {
                let trimmed = pattern.trim_start();
                if trimmed.starts_with('@') {
                    format!("class __Wrap__ {{ {} void __m__() {{}} }}", pattern)
                } else if trimmed.starts_with("interface")
                    || trimmed.starts_with("class")
                    || trimmed.starts_with("enum")
                    || trimmed.starts_with("record")
                    || trimmed.starts_with("@interface")
                {
                    format!("class __Wrap__ {{ {} }}", pattern)
                } else if Self::looks_like_java_method_decl(pattern) {
                    format!("class __Wrap__ {{ {} }}", pattern)
                } else {
                    format!("class __Wrap__ {{ void m() {{ {} }} }}", pattern)
                }
            }
            Language::JavaScript => format!("function __wrap__() {{ {} }}", pattern),
            Language::Python => {
                // Decorators need to be before a function definition
                if pattern.trim_start().starts_with('@') {
                    format!("{}\ndef __wrap__(): pass", pattern)
                } else {
                    format!("def __wrap__():\n    {}", pattern)
                }
            }
            Language::Bash => {
                let trimmed = pattern.trim_start();
                // If the pattern already looks like a complete script (has shebang),
                // pass through as-is.
                if trimmed.starts_with("#!") {
                    pattern.to_string()
                } else {
                    // Wrap in a function body so incomplete fragments parse correctly.
                    format!("__wrap__() {{\n{}\n}}", pattern)
                }
            }
            _ => pattern.to_string(),
        }
    }

    fn looks_like_java_method_decl(pattern: &str) -> bool {
        if !pattern.contains('(') || !pattern.contains('{') {
            return false;
        }
        let trimmed = pattern.trim_start();
        !matches!(trimmed.split_whitespace().next(), Some(first) if matches!(
            first,
            "if" | "for" | "while" | "switch" | "try" | "catch"
                | "finally" | "do" | "return" | "throw" | "new" | "assert"
        ))
    }

    fn is_wrapper_decl(node: &Node, source: &str) -> bool {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let ck = child.kind();
                if matches!(ck, "identifier" | "property_identifier" | "type_identifier") {
                    let text = &source[child.start_byte()..child.end_byte()];
                    let t = text.trim();
                    if t.starts_with("__wrap") || t == "__m__" || t == "__m" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Find the first meaningful (non-wrapper) AST node.
    fn find_meaningful_node<'a>(&self, root: &Node<'a>, source: &str) -> Option<Node<'a>> {
        let mut current = *root;
        loop {
            let kind = current.kind();
            // Skip wrapper nodes
            if matches!(
                kind,
                "program"
                    | "module"
                    | "translation_unit"
                    | "source_file"
                    | "script"
                    | "expression_statement"
            ) {
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
            if matches!(
                kind,
                "class_declaration"
                    | "class_body"
                    | "block"
                    | "statement_block"
                    | "body"
                    | "compound_statement"
            ) || (matches!(kind, "method_declaration" | "function_definition")
                && Self::is_wrapper_decl(&current, source))
            {
                // Check for annotation/decorator/modifier nodes first — don't dive past them
                for i in 0..current.child_count() {
                    if let Some(child) = current.child(i) {
                        let ck = child.kind();
                        if matches!(
                            ck,
                            "annotation"
                                | "marker_annotation"
                                | "modifier"
                                | "decorator"
                                | "decorator_list"
                                | "modifiers"
                        ) {
                            return Some(child);
                        }
                    }
                }
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
        let range = node.byte_range();
        let text = if range.start <= source.len() && range.end <= source.len() {
            std::str::from_utf8(&source.as_bytes()[range]).unwrap_or("")
        } else {
            ""
        };

        // Check if this entire node text is a metavar placeholder
        if let Some(kind) = meta_map.get(text) {
            return match kind {
                PlaceholderKind::Metavar { name, .. } => PatternTree::Metavar { name: name.clone(), bind_attr: None },
                PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                PlaceholderKind::EllipsisMetavar(name) => {
                    PatternTree::EllipsisMetavar { name: name.clone() }
                }
                PlaceholderKind::TypedMetavar { name, type_name } => PatternTree::TypedMetavar {
                    name: name.clone(),
                    type_name: type_name.clone(),
                },
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
            || matches!(
                node.kind(),
                "identifier"
                    | "type_identifier"
                    | "property_identifier"
                    | "string"
                    | "string_literal"
                    | "number"
                    | "number_literal"
                    | "integer"
                    | "float"
                    | "true"
                    | "false"
                    | "null"
            );

        // For identifiers, check if text matches a metavar placeholder
        if node.kind() == "identifier"
            || node.kind() == "type_identifier"
            || node.kind() == "property_identifier"
        {
            if let Some(kind) = meta_map.get(text) {
                return match kind {
                    PlaceholderKind::Metavar { name, .. } => PatternTree::Metavar { name: name.clone(), bind_attr: None },
                    PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                    PlaceholderKind::EllipsisMetavar(name) => {
                        PatternTree::EllipsisMetavar { name: name.clone() }
                    }
                    PlaceholderKind::TypedMetavar { name, type_name } => {
                        PatternTree::TypedMetavar {
                            name: name.clone(),
                            type_name: type_name.clone(),
                        }
                    }
                };
            }
        }

        // For string literals, check if the unquoted content is a metavar placeholder
        // Pattern: foo("$VAR") → tree-sitter gives string node with text '"__mg_VAR__"'
        if node.kind() == "string" || node.kind() == "string_literal" {
            let unquoted = text
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
            if let Some(inner) = unquoted {
                if let Some(kind) = meta_map.get(inner) {
                    return match kind {
                        PlaceholderKind::Metavar { name, bind_attr } => {
                            PatternTree::Metavar { name: name.clone(), bind_attr: bind_attr.clone() }
                        }
                        PlaceholderKind::Ellipsis => PatternTree::Ellipsis,
                        PlaceholderKind::EllipsisMetavar(name) => {
                            PatternTree::EllipsisMetavar { name: name.clone() }
                        }
                        PlaceholderKind::TypedMetavar { name, type_name } => {
                            PatternTree::TypedMetavar {
                                name: name.clone(),
                                type_name: type_name.clone(),
                            }
                        }
                    };
                }
            }
        }

        PatternTree::Node {
            kind: node.kind().to_string(),
            children,
            text: if is_terminal {
                Some(text.to_string())
            } else {
                None
            },
            constraints: Vec::new(),
        }
    }
}

impl Default for PatternTreeParser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            parsers: HashMap::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Preprocessing
// ---------------------------------------------------------------------------

/// What kind of placeholder a preprocessed token represents.
#[derive(Debug, Clone, PartialEq)]
enum PlaceholderKind {
    Metavar { name: String, bind_attr: Option<String> },
    Ellipsis,
    EllipsisMetavar(String),
    TypedMetavar { name: String, type_name: String },
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
    let mut in_string = false;
    let mut string_delim = ' ';
    // Detect declaration patterns (class/record/interface) for context-aware ellipsis
    let is_decl = pattern.trim_start().starts_with("public ")
        || pattern.trim_start().starts_with("private ")
        || pattern.trim_start().starts_with("class ")
        || pattern.trim_start().starts_with("record ")
        || pattern.trim_start().starts_with("interface ")
        || pattern.trim_start().starts_with("@interface ");
    let mut paren_depth: i32 = 0;
    let mut brace_depth: i32 = 0;

    while i < chars.len() {
        if in_string {
            result.push(chars[i]);
            if chars[i] == string_delim {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if chars[i] == '"' || chars[i] == '\'' {
            in_string = true;
            string_delim = chars[i];
            result.push(chars[i]);
            i += 1;
            continue;
        }
        // Check for typed metavar syntax: (TYPE $NAME)
        if chars[i] == '(' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let type_start = j;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '.')
            {
                j += 1;
            }
            if j > type_start {
                let type_name: String = chars[type_start..j].iter().collect();
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '$' {
                    let mut k = j + 1;
                    while k < chars.len() && (chars[k].is_alphanumeric() || chars[k] == '_') {
                        k += 1;
                    }
                    if k > j + 1 {
                        let name: String = chars[j + 1..k].iter().collect();
                        while k < chars.len() && chars[k].is_whitespace() {
                            k += 1;
                        }
                        if k < chars.len() && chars[k] == ')' {
                            // (TYPE $NAME) → TypedMetavar
                            let placeholder_type: String = type_name
                                .chars()
                                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                                .collect();
                            let placeholder = format!(
                                "{}{}{}_t_{}{}",
                                MG_PREFIX, name, MG_SUFFIX, placeholder_type, MG_SUFFIX
                            );
                            meta_map.insert(
                                placeholder.clone(),
                                PlaceholderKind::TypedMetavar {
                                    name: name.clone(),
                                    type_name,
                                },
                            );
                            result.push_str(&placeholder);
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
        }

        if chars[i] == '$' {
            // Check for ellipsis metavar: $...NAME
            if i + 3 < chars.len()
                && chars[i + 1] == '.'
                && chars[i + 2] == '.'
                && chars[i + 3] == '.'
            {
                // Collect the name after $...
                let mut name = String::new();
                let mut j = i + 4;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '@') {
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
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '@') {
                name.push(chars[j]);
                j += 1;
            }
            if !name.is_empty() {
                // Check for typed metavar: $NAME: TYPE
                if j < chars.len() && chars[j] == ':' {
                    let mut k = j + 1;
                    // skip whitespace after colon
                    while k < chars.len() && chars[k].is_whitespace() {
                        k += 1;
                    }
                    let type_start = k;
                    while k < chars.len()
                        && (chars[k].is_alphanumeric() || chars[k] == '_' || chars[k] == '.')
                    {
                        k += 1;
                    }
                    if k > type_start {
                        let type_name: String = chars[type_start..k].iter().collect();
                        let placeholder_type: String = type_name
                            .chars()
                            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                            .collect();
                        let placeholder = format!(
                            "{}{}{}_t_{}{}",
                            MG_PREFIX, name, MG_SUFFIX, placeholder_type, MG_SUFFIX
                        );
                        meta_map.insert(
                            placeholder.clone(),
                            PlaceholderKind::TypedMetavar {
                                name: name.clone(),
                                type_name,
                            },
                        );
                        result.push_str(&placeholder);
                        i = k;
                        continue;
                    }
                }
                // Split $NAME@attr — @ marks metadata binding target
                let (bind_name, bind_attr) = if let Some(at_pos) = name.find('@') {
                    let attr = name[at_pos + 1..].to_string();
                    let base = name[..at_pos].to_string();
                    (base, Some(attr))
                } else {
                    (name, None)
                };
                let placeholder = format!("{}{}{}", MG_PREFIX, bind_name, MG_SUFFIX);
                meta_map.insert(
                    placeholder.clone(),
                    PlaceholderKind::Metavar { name: bind_name.clone(), bind_attr: bind_attr.clone() },
                );
                result.push_str(&placeholder);
                i = j;
                continue;
            }

            // Bare $ — keep as-is
            result.push('$');
            i += 1;
        } else if chars[i] == '.'
            && i + 2 < chars.len()
            && chars[i + 1] == '.'
            && chars[i + 2] == '.'
        {
            let placeholder = if is_decl {
                if brace_depth > 0 {
                    "int __e__ = 0;"
                } else if paren_depth > 0 {
                    "int __e__"
                } else {
                    ELLIPSIS_PLACEHOLDER
                }
            } else {
                ELLIPSIS_PLACEHOLDER
            };
            meta_map.insert(placeholder.to_string(), PlaceholderKind::Ellipsis);
            result.push_str(placeholder);
            i += 3;
        } else {
            match chars[i] {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1
                    }
                }
                '{' => brace_depth += 1,
                '}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1
                    }
                }
                _ => {}
            }
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
        assert!(matches!(map.get("__mg_X__"), Some(PlaceholderKind::Metavar { name, bind_attr: None }) if name == "X"));
        assert!(
            matches!(map.get("__mg_QUERY__"), Some(PlaceholderKind::Metavar { name, bind_attr: None }) if name == "QUERY")
        );
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

        if let PatternTree::Node {
            kind,
            children,
            text,
            ..
        } = &tree
        {
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
        let tree = parser
            .parse("$X.execute($Y)", Language::JavaScript)
            .unwrap();

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
        let tree = parser
            .parse("$STMT.execute($QUERY)", Language::Java)
            .unwrap();

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
                if let PatternTree::Node {
                    kind: inner_kind,
                    text,
                    ..
                } = &children[0]
                {
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
            PatternTree::Metavar { .. } | PatternTree::TypedMetavar { .. } => false,
        }
    }

    #[test]
    fn test_preprocess_typed_metavar() {
        let (result, map) = preprocess_pattern("func(($VAL: boolean))");
        assert!(result.contains("__mg_VAL___t_boolean__"));
        assert_eq!(map.len(), 1);
        assert!(matches!(
            map.get("__mg_VAL___t_boolean__"),
            Some(PlaceholderKind::TypedMetavar { name, type_name }) if name == "VAL" && type_name == "boolean"
        ));
    }

    #[test]
    fn test_preprocess_typed_metavar_number() {
        let (result, map) = preprocess_pattern("func(($VAL: number))");
        assert!(result.contains("__mg_VAL___t_number__"));
        assert!(matches!(
            map.get("__mg_VAL___t_number__"),
            Some(PlaceholderKind::TypedMetavar { name, type_name }) if name == "VAL" && type_name == "number"
        ));
    }

    #[test]
    fn test_parse_typed_metavar_javascript() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser
            .parse("func(($VAL: boolean))", Language::JavaScript)
            .unwrap();
        assert!(tree.has_wildcards());
    }

    #[test]
    fn test_bash_fragment_gets_wrapped() {
        let result = PatternTreeParser::wrap_in_context_static("echo hello", Language::Bash);
        assert!(result.contains("__wrap__()"), "fragment should be wrapped in function");
        assert!(result.contains("echo hello"));
    }

    #[test]
    fn test_bash_shebang_passes_through() {
        let pattern = "#!/bin/bash\necho hello";
        let result = PatternTreeParser::wrap_in_context_static(pattern, Language::Bash);
        // Should NOT double-wrap
        assert_eq!(result, pattern, "shebang pattern should pass through unchanged");
    }

    #[test]
    fn test_bash_parse_simple_command() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("echo hello", Language::Bash).unwrap();
        // Should parse successfully
        assert!(tree.has_wildcards() == false, "simple command should have no wildcards");
    }

    #[test]
    fn test_bash_parse_metavar() {
        let mut parser = PatternTreeParser::new().unwrap();
        let tree = parser.parse("echo $X", Language::Bash).unwrap();
        // Should parse with metavariable
        assert!(tree.has_wildcards(), "pattern with $X should have wildcards");
    }
}
