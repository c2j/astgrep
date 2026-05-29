//! Tree-to-tree structural pattern matching engine.
//!
//! Matches a `PatternTree` (produced by `PatternTreeParser`) against target
//! AST nodes using recursive structural comparison.  This replaces the
//! text-token matching approach with proper AST-level matching inspired by
//! semgrep's `Pattern_vs_code.ml`.
//!
//! ## Phases
//!
//! - **Phase B**: Deep ellipsis chain matching — flatten nested
//!   member_expression / call_expression into flat lists for ellipsis matching
//!   in method chains.
//! - **Phase C**: Associative binary chain matching — flatten and / or / +
//!   chains so ellipsis and metavars match across nesting levels.
//! - **Phase D**: Equivalence matching — var / let / const keyword
//!   equivalence, unordered object child matching, basic constant propagation.

use astgrep_core::{AstNode, Language, MatchBinding, SemgrepMatchResult};
use astgrep_parser::pattern_tree::{PatternTree, PatternTreeParser};
use std::collections::HashMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Chain-flattening constants (Phase B)
// ---------------------------------------------------------------------------

/// Node kinds that form chains (member access / call / subscript).
const CHAIN_NODE_KINDS: &[&str] = &[
    "member_expression",
    "call_expression",
    "subscript_expression",
    "field_access",
    "method_invocation",
    "method_reference",
    "attribute",
    "scoped_identifier",
    "selector_expression",
];

/// Node kinds to skip when flattening chains.
const CHAIN_SKIP_KINDS: &[&str] = &[
    "arguments",
    "argument_list",
    "parenthesized_expression",
    "parenthesis",
    "type_arguments",
    "type_argument_list",
];

// ---------------------------------------------------------------------------
// Binary-chain constants (Phase C)
// ---------------------------------------------------------------------------

/// Tree-sitter kinds that represent binary / logical expressions.
fn is_binary_expression_kind(kind: &str) -> bool {
    matches!(
        kind,
        "binary_expression"
            | "binary_operator"
            | "boolean_operator"
            | "boolean_operation"
            | "logical_expression"
    )
}

/// Operators that are associative (can be re-grouped arbitrarily).
fn is_associative_operator(op: &str) -> bool {
    matches!(op.trim(), "and" | "&&" | "or" | "||" | "+" | "*" | "&" | "|")
}

// ---------------------------------------------------------------------------
// Chain helpers (Phase B)
// ---------------------------------------------------------------------------

fn is_chain_kind(kind: &str) -> bool {
    CHAIN_NODE_KINDS.contains(&kind)
}

fn should_skip_in_chain(kind: &str, node: &dyn AstNode) -> bool {
    if CHAIN_SKIP_KINDS.contains(&kind) {
        return true;
    }
    if let Some(t) = node.text() {
        let t = t.trim();
        if t.is_empty()
            || t == "."
            || t == "("
            || t == ")"
            || t == "{"
            || t == "}"
            || t == "["
            || t == "]"
            || t == ";"
            || t == ","
            || t == ":"
            || t == "()"
        {
            return true;
        }
    } else {
        return true;
    }
    if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
        return true;
    }
    false
}

fn pattern_contains_ellipsis(tree: &PatternTree) -> bool {
    match tree {
        PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. } => true,
        PatternTree::Node { children, .. } => children.iter().any(pattern_contains_ellipsis),
        PatternTree::DeepExpr(inner) => pattern_contains_ellipsis(inner),
        PatternTree::Metavar { .. } => false,
    }
}

fn is_pattern_chain_node(tree: &PatternTree) -> bool {
    match tree {
        PatternTree::Node { kind, .. } => is_chain_kind(kind),
        _ => false,
    }
}

/// Flatten a target AST node chain into a flat list of elements.
///
/// For `o.foo().m().bar()` returns `[identifier("o"), prop("foo"), prop("m"), prop("bar")]`.
fn flatten_node_chain<'a>(node: &'a dyn AstNode) -> Vec<&'a dyn AstNode> {
    let kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
    if is_chain_kind(kind) {
        let mut result = Vec::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_kind = child.get_attribute("ts_kind").unwrap_or(child.node_type());
                if should_skip_in_chain(child_kind, child) {
                    continue;
                }
                if is_chain_kind(child_kind) {
                    result.extend(flatten_node_chain(child));
                } else {
                    result.push(child);
                }
            }
        }
        result
    } else {
        vec![node]
    }
}

fn should_skip_pattern_in_chain(tree: &PatternTree) -> bool {
    match tree {
        PatternTree::Node { kind, text, .. } => {
            if CHAIN_SKIP_KINDS.contains(&kind.as_str()) {
                return true;
            }
            if let Some(t) = text {
                let t = t.trim();
                if t.is_empty()
                    || t == "."
                    || t == "("
                    || t == ")"
                    || t == "()"
                    || t == "{}"
                    || t == "[]"
                    || t == ";"
                    || t == ","
                    || t == ":"
                {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Flatten a pattern tree's chain into a flat list of pattern elements.
fn flatten_pattern_chain(kind: &str, children: &[PatternTree]) -> Vec<PatternTree> {
    if !is_chain_kind(kind) {
        return vec![PatternTree::Node {
            kind: kind.to_string(),
            children: children.to_vec(),
            text: None,
        }];
    }

    let mut result = Vec::new();
    for child in children {
        match child {
            PatternTree::Node {
                kind: ck,
                children: cc,
                ..
            } => {
                if is_chain_kind(ck) {
                    result.extend(flatten_pattern_chain(ck, cc));
                } else if !should_skip_pattern_in_chain(child) {
                    result.push(child.clone());
                }
            }
            PatternTree::Ellipsis
            | PatternTree::EllipsisMetavar { .. }
            | PatternTree::Metavar { .. }
            | PatternTree::DeepExpr(_) => {
                result.push(child.clone());
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Binary-chain helpers (Phase C)
// ---------------------------------------------------------------------------

/// Filter children the same way `match_node` does (skip punctuation / comments).
fn filter_node_children(node: &dyn AstNode) -> Vec<&dyn AstNode> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|c| {
            let kind = c.get_attribute("ts_kind").unwrap_or(c.node_type());
            if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                return false;
            }
            if let Some(t) = c.text() {
                let t = t.trim();
                !t.is_empty()
                    && t != "("
                    && t != ")"
                    && t != "{"
                    && t != "}"
                    && t != "["
                    && t != "]"
                    && t != ";"
                    && t != ","
                    && t != ":"
                    && t != "."
                    && t != "="
            } else {
                false
            }
        })
        .collect()
}

fn get_single_block_child(block: &dyn AstNode) -> Option<&dyn AstNode> {
    let children: Vec<&dyn AstNode> = (0..block.child_count())
        .filter_map(|i| block.child(i))
        .filter(|c| {
            let kind = c.get_attribute("ts_kind").unwrap_or(c.node_type());
            if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                return false;
            }
            if let Some(t) = c.text() {
                let t = t.trim();
                !t.is_empty()
                    && t != "(" && t != ")" && t != "{" && t != "}"
                    && t != "[" && t != "]" && t != ";" && t != ","
                    && t != ":" && t != "." && t != "="
            } else {
                false
            }
        })
        .collect();
    if children.len() == 1 {
        Some(children[0])
    } else {
        None
    }
}

/// Flatten a binary-expression target into `(operator, flat_operands)`.
///
/// `((A and B) and C)` → `("and", [A, B, C])`.
fn flatten_binary_node(node: &dyn AstNode) -> Option<(String, Vec<&dyn AstNode>)> {
    let kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
    if !is_binary_expression_kind(kind) {
        return None;
    }
    let children = filter_node_children(node);
    if children.len() < 3 {
        return None;
    }
    // Operator is the middle child
    let op = children[1].text().map(|t| t.trim().to_string())?;
    if !is_associative_operator(&op) {
        return None;
    }
    let mut operands = Vec::new();
    flatten_node_operand(children[0], &op, &mut operands);
    flatten_node_operand(children[2], &op, &mut operands);
    Some((op, operands))
}

fn flatten_node_operand<'a>(
    node: &'a dyn AstNode,
    expected_op: &str,
    operands: &mut Vec<&'a dyn AstNode>,
) {
    let kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
    // Unwrap parenthesized expressions
    if kind == "parenthesized_expression" {
        let children = filter_node_children(node);
        if children.len() == 1 {
            flatten_node_operand(children[0], expected_op, operands);
            return;
        }
    }
    if is_binary_expression_kind(kind) {
        let children = filter_node_children(node);
        if children.len() >= 3 {
            if let Some(op) = children[1].text() {
                if op.trim() == expected_op {
                    flatten_node_operand(children[0], expected_op, operands);
                    flatten_node_operand(children[2], expected_op, operands);
                    return;
                }
            }
        }
    }
    operands.push(node);
}

/// Flatten a binary-expression pattern into `(operator, flat_pattern_operands)`.
fn try_flatten_pattern_binary(
    pattern_kind: &str,
    pattern_children: &[PatternTree],
) -> Option<(String, Vec<PatternTree>)> {
    if !is_binary_expression_kind(pattern_kind) {
        return None;
    }
    // Find the operator first, before filtering
    let mut found_op: Option<String> = None;
    for c in pattern_children {
        if let PatternTree::Node { text: Some(t), .. } = c {
            let t = t.trim();
            if is_associative_operator(t) {
                found_op = Some(t.to_string());
                break;
            }
        }
    }
    let op = found_op?;
    // Filter out punctuation and the operator node
    let mut operands = Vec::new();
    for c in pattern_children {
        match c {
            PatternTree::Node { text: Some(t), .. } => {
                let t = t.trim();
                if t.is_empty() || t == "(" || t == ")" || t == "{" || t == "}"
                    || t == "[" || t == "]" || t == ";" || t == "," || t == ":"
                {
                    continue;
                }
                if t == op { continue; }
                flatten_pattern_operand(c, &op, &mut operands);
            }
            PatternTree::Node { text: None, .. } => {
                flatten_pattern_operand(c, &op, &mut operands);
            }
            PatternTree::Metavar { .. } | PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. } | PatternTree::DeepExpr(_) => {
                operands.push(c.clone());
            }
        }
    }
    if operands.len() < 2 {
        return None;
    }
    Some((op, operands))
}

fn flatten_pattern_operand(pattern: &PatternTree, expected_op: &str, operands: &mut Vec<PatternTree>) {
    match pattern {
        PatternTree::Node {
            kind, children, text, ..
        } => {
            if is_binary_expression_kind(kind) {
                // Try to flatten further
                if let Some((op, flat)) = try_flatten_pattern_binary(kind, children) {
                    if op == expected_op {
                        operands.extend(flat);
                        return;
                    }
                }
            }
            operands.push(pattern.clone());
        }
        PatternTree::Metavar { .. }
        | PatternTree::Ellipsis
        | PatternTree::EllipsisMetavar { .. }
        | PatternTree::DeepExpr(_) => {
            operands.push(pattern.clone());
        }
    }
}

/// Join operand texts with operator for metavar binding.
fn join_operand_texts(operands: &[&dyn AstNode], operator: &str) -> String {
    operands
        .iter()
        .filter_map(|o| o.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(&format!(" {} ", operator))
}

// ---------------------------------------------------------------------------
// Declaration keyword equivalence (Phase D)
// ---------------------------------------------------------------------------

/// Keywords that are equivalent for variable declarations.
const DECL_KEYWORDS: &[&str] = &["var", "let", "const"];

fn declaration_keywords_equivalent(pat: &str, tgt: &str) -> bool {
    let pt = pat.trim();
    let tt = tgt.trim();
    if pt == tt {
        return true;
    }
    // Both are declaration keywords — treat as equivalent
    if DECL_KEYWORDS.contains(&pt) && DECL_KEYWORDS.contains(&tt) {
        return true;
    }
    false
}

fn is_object_like_kind(kind: &str) -> bool {
    matches!(
        kind,
        "object" | "object_pattern" | "object_literal" | "dictionary"
    )
}

/// Map a collection node kind to its delimiter family.
/// Returns `None` for non-collection kinds.
///
/// Used to allow `{ ... }` to match both `set` and `dictionary` in Python,
/// or any brace-delimited collection across languages.
fn collection_delimiter(kind: &str) -> Option<&'static str> {
    match kind {
        // Brace-delimited collections: { }
        "set" | "dictionary" | "object" | "object_pattern" | "object_literal"
        | "struct_expression" | "struct_pattern" | "record" => Some("brace"),

        // Bracket-delimited collections: [ ]
        "list" | "array" | "array_pattern" | "list_pattern"
        | "list_comprehension" | "array_comprehension" => Some("bracket"),

        // Paren-delimited collections: ( )
        "tuple" | "tuple_pattern" | "parenthesized_expression"
        | "generator_expression" => Some("paren"),

        _ => None,
    }
}

/// Check if a pattern node's children are ALL ellipsis (no concrete/metavar children).
/// A pure-ellipsis pattern like `{ ... }` is a wildcard collection that should
/// match any collection with the same delimiters.
 fn is_pure_ellipsis_pattern(children: &[PatternTree]) -> bool {
     !children.is_empty()
         && children.iter().all(|c| {
             matches!(
                 c,
                 PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }
             )
         })
 }

const WRAPPER_NODE_KINDS: &[&str] = &[
    "expression_statement",
    "expression_statement_with_semicolon",
    "statement",
    "simple_statement",
    "declaration_statement",
];

fn unwrap_single_child_wrapper(pattern: &PatternTree) -> Option<&PatternTree> {
    if let PatternTree::Node { kind, children, text } = pattern {
        if text.is_none() && WRAPPER_NODE_KINDS.contains(&kind.as_str()) && children.len() == 1 {
            return Some(&children[0]);
        }
    }
    None
}

/// Check if a pattern child is an "optional collection" — a parameter/argument
/// list node containing only ellipsis. These can match zero target children
/// (e.g., `class A(...)` should match `class A:` which has no argument_list).
fn is_optional_collection(tree: &PatternTree) -> bool {
    match tree {
        PatternTree::Node { kind, children, .. } => {
            matches!(
                kind.as_str(),
                "argument_list"
                    | "arguments"
                    | "parameters"
                    | "parameter_list"
                    | "type_arguments"
                    | "type_argument_list"
                    | "type_parameters"
                    | "inheritance_list"
                    | "decorator_list"
                    | "array"
                    | "list"
            ) && is_pure_ellipsis_pattern(children)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// TreeMatcher
// ---------------------------------------------------------------------------

pub struct TreeMatcher {
    parser: Mutex<PatternTreeParser>,
}

impl TreeMatcher {
    pub fn new() -> Self {
        Self {
            parser: Mutex::new(
                PatternTreeParser::new()
                    .unwrap_or_else(|_| PatternTreeParser::default()),
            ),
        }
    }

    /// Try to find all matches of `pattern_str` in `root` for the given language.
    ///
    /// Returns a (possibly empty) list of match results.  On any parse failure
    /// the list is simply empty — callers should fall back to text matching.
    pub fn find_matches(
        &self,
        pattern_str: &str,
        language: Language,
        root: &dyn AstNode,
    ) -> Vec<SemgrepMatchResult> {
        let tree = match self.parser.lock() {
            Ok(mut p) => match p.parse(pattern_str, language) {
                Ok(t) => t,
                Err(e) => {
                    let _ = e;
                    return Vec::new();
                }
            },
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        let mut ctx = MatchCtx::new();
        ctx.find_recursive(&tree, root, &mut results, None);

        deduplicate_matches(&mut results);
        results
    }
}

impl Default for TreeMatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MatchCtx — mutable state during a single matching traversal
// ---------------------------------------------------------------------------

struct MatchCtx {
    /// metavar name → matched text
    bindings: HashMap<String, String>,
    /// Collected constants for constant propagation (Phase D)
    constants: HashMap<String, String>,
}

impl MatchCtx {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    fn snapshot(&self) -> (HashMap<String, String>, HashMap<String, String>) {
        (self.bindings.clone(), self.constants.clone())
    }

    fn restore(&mut self, snap: (HashMap<String, String>, HashMap<String, String>)) {
        self.bindings = snap.0;
        self.constants = snap.1;
    }

    /// Try to bind `name` to `value`.  Returns `false` if the binding is
    /// inconsistent with an existing binding for the same name.
    fn try_bind(&mut self, name: &str, value: &str) -> bool {
        if let Some(existing) = self.bindings.get(name) {
            return existing == value;
        }
        self.bindings.insert(name.to_string(), value.to_string());
        true
    }

    // -----------------------------------------------------------------------
    // Recursive search (extended for Phase D: constant collection)
    // -----------------------------------------------------------------------

    /// Walk the target AST top-down.  At each node, attempt to match the
    /// pattern tree.  Collect every successful match.
    fn find_recursive(
        &mut self,
        pattern: &PatternTree,
        node: &dyn AstNode,
        results: &mut Vec<SemgrepMatchResult>,
        skip_assoc_op: Option<&str>,
    ) {
        // Phase D: collect constants from this node before matching
        self.collect_constants(node);

        let node_kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
        let is_binary = is_binary_expression_kind(node_kind);
        let mut child_skip_op: Option<String> = None;

        if is_binary {
            let children = filter_node_children(node);
            if children.len() >= 3 {
                if let Some(op) = children[1].text() {
                    if is_associative_operator(op.trim()) {
                        child_skip_op = Some(op.trim().to_string());
                    }
                }
            }
        }

        let should_skip = skip_assoc_op.is_some() && is_binary && child_skip_op.as_deref() == skip_assoc_op;

        if !should_skip {
            let snap = self.snapshot();
            if self.match_tree(pattern, node) {
                let bindings: HashMap<String, MatchBinding> = self
                    .bindings
                    .iter()
                    .map(|(k, v)| (k.clone(), MatchBinding::new(v.clone())))
                    .collect();
                results.push(SemgrepMatchResult::new(node.clone_node(), bindings));
                self.restore(snap);
            } else {
                self.restore(snap);
                if let Some(inner) = unwrap_single_child_wrapper(pattern) {
                    let snap2 = self.snapshot();
                    if self.match_tree(inner, node) {
                        let bindings: HashMap<String, MatchBinding> = self
                            .bindings
                            .iter()
                            .map(|(k, v)| (k.clone(), MatchBinding::new(v.clone())))
                            .collect();
                        results.push(SemgrepMatchResult::new(node.clone_node(), bindings));
                    }
                    self.restore(snap2);
                }
            }
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_recursive(pattern, child, results, child_skip_op.as_deref());
            }
        }
    }

    /// Collect constant values from variable declarations for propagation.
    fn collect_constants(&mut self, node: &dyn AstNode) {
        let kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
        // Only collect from lexical_declaration, variable_declaration
        if kind == "lexical_declaration" || kind == "variable_declaration" {
            let children = filter_node_children(node);
            if children.len() >= 2 {
                // First child might be the keyword (var/let/const), second is the declarator
                for child in &children {
                    let ck = child.get_attribute("ts_kind").unwrap_or(child.node_type());
                    if ck == "variable_declarator" {
                        self.extract_const_from_decl(*child);
                    }
                }
            }
        }
    }

    /// Extract `const x = "value"` pairs for constant propagation.
    fn extract_const_from_decl(&mut self, node: &dyn AstNode) {
        let children = filter_node_children(node);
        if children.len() >= 3 {
            // identifier = value
            if let Some(name) = children[0].text() {
                let name = name.trim().to_string();
                if let Some(value) = children[2].text() {
                    let value = value.trim().to_string();
                    // Only propagate string literals
                    if (value.starts_with('"') && value.ends_with('"'))
                        || (value.starts_with('\'') && value.ends_with('\''))
                    {
                        self.constants.entry(name).or_insert(value);
                    }
                }
            }
        }
    }

    /// Check constant propagation: if the target text is a known constant name,
    /// compare the pattern text with the constant's value.
    fn check_constant_propagation(&self, pattern_text: &str, target_text: &str) -> bool {
        if let Some(constant_value) = self.constants.get(target_text.trim()) {
            let pv = pattern_text
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| {
                    pattern_text
                        .strip_prefix('\'')
                        .and_then(|s| s.strip_suffix('\''))
                })
                .unwrap_or(pattern_text);
            let cv = constant_value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| {
                    constant_value
                        .strip_prefix('\'')
                        .and_then(|s| s.strip_suffix('\''))
                })
                .unwrap_or(constant_value);
            return pv == cv;
        }
        false
    }

    // -----------------------------------------------------------------------
    // Core matching
    // -----------------------------------------------------------------------

    /// Try to match `pattern` against `target`.  On success the bindings are
    /// updated in-place; on failure the caller should restore a snapshot.
    fn match_tree(&mut self, pattern: &PatternTree, target: &dyn AstNode) -> bool {
        match pattern {
            PatternTree::Ellipsis => true,

            PatternTree::Metavar { name } => {
                if name == "_" {
                    let kind = target.node_type();
                    if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                        return false;
                    }
                    return target.text().map_or(false, |t| !t.trim().is_empty());
                }
                if let Some(text) = target.text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return self.try_bind(name, trimmed);
                    }
                }
                false
            }

            PatternTree::EllipsisMetavar { name } => {
                if let Some(text) = target.text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        return self.try_bind(name, trimmed);
                    }
                }
                false
            }

            PatternTree::DeepExpr(inner) => {
                // Try direct match first
                let snap = self.snapshot();
                if self.match_tree(inner, target) {
                    return true;
                }
                self.restore(snap);

                // Then try matching against every descendant
                for i in 0..target.child_count() {
                    if let Some(child) = target.child(i) {
                        let snap2 = self.snapshot();
                        if self.match_tree(inner, child) {
                            return true;
                        }
                        self.restore(snap2);
                    }
                }
                false
            }

            PatternTree::Node {
                kind,
                children,
                text,
            } => self.match_node(kind, children, text, target),
        }
    }

    fn match_node(
        &mut self,
        pattern_kind: &str,
        pattern_children: &[PatternTree],
        pattern_text: &Option<String>,
        target: &dyn AstNode,
    ) -> bool {
        if let Some(ref pt) = pattern_text {
            if let Some(tt) = target.text() {
                let tt = tt.trim();
                let pt = pt.trim();

                if pt == tt {
                    return true;
                }

                // String wildcard: "..." matches any non-empty string literal
                if (pt == "\"...\"" || pt == "'...'" || pt == "`...`")
                    && (tt.starts_with('"') || tt.starts_with('\'') || tt.starts_with('`'))
                {
                    return true;
                }

                // Phase D: declaration keyword equivalence
                if declaration_keywords_equivalent(pt, tt) {
                    return true;
                }

                // Phase D: constant propagation
                if self.check_constant_propagation(pt, tt) {
                    return true;
                }

                let stripped_tt = tt
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .or_else(|| tt.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                    .unwrap_or(tt);
                let stripped_pt = pt
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .or_else(|| pt.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                    .unwrap_or(pt);
                if stripped_pt == stripped_tt {
                    return true;
                }

                // If text match failed but pattern has children, fall through
                // to structural matching (e.g., "..." inside strings needs child matching)
                if pattern_children.is_empty() {
                    return false;
                }
            }
            return false;
        }

        // Phase 2: Text-less pattern — must match structurally via children.
        // The pattern's kind must be compatible with the target's node type.
        let target_kind = target
            .get_attribute("ts_kind")
            .unwrap_or(target.node_type());
        let mut kind_match = pattern_kind == target_kind
            || pattern_kind == "_"
            || (pattern_kind.contains('_') && target_kind.contains('_')
                && pattern_kind.split('_').any(|p| target_kind.contains(p)))
            || (is_chain_kind(pattern_kind) && is_chain_kind(target_kind));

        if !kind_match && !pattern_children.is_empty() {
            if pattern_kind == "assignment_expression"
                && matches!(target_kind,
                    "local_variable_declaration" | "variable_declaration" | "field_declaration"
                    | "public_field_definition" | "field_definition"
                    | "property_definition" | "public_field_definition"
                    | "abstract_field_declaration" | "field_signature")
            {
                let vd_children: Vec<&dyn AstNode> = if matches!(target_kind,
                    "public_field_definition" | "field_definition" | "property_definition"
                    | "abstract_field_declaration" | "field_signature")
                {
                    (0..target.child_count())
                        .filter_map(|i| target.child(i))
                        .filter(|c| {
                            if let Some(t) = c.text() {
                                let t = t.trim();
                                !t.is_empty() && t != "=" && t != "(" && t != ")"
                                    && t != ";" && t != "," && t != ":" && t != "."
                                    && t != "{" && t != "}" && t != "[" && t != "]"
                                    && t != "!" && t != "?"
                            } else { false }
                        })
                        .collect()
                } else {
                    (0..target.child_count())
                        .filter_map(|i| target.child(i))
                        .filter(|c| {
                            let k = c.get_attribute("ts_kind").unwrap_or(c.node_type());
                            k == "variable_declarator"
                        })
                        .flat_map(|vd| {
                            (0..vd.child_count())
                                .filter_map(move |i| vd.child(i))
                                .filter(|c| {
                                    if let Some(t) = c.text() {
                                        let t = t.trim();
                                        !t.is_empty() && t != "=" && t != "(" && t != ")"
                                            && t != ";" && t != "," && t != ":" && t != "."
                                            && t != "{" && t != "}" && t != "[" && t != "]"
                                    } else { false }
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect()
                };

                if vd_children.len() == pattern_children.len() {
                    let snap = self.snapshot();
                    if self.match_children_exact(pattern_children, &vd_children) {
                        return true;
                    }
                    self.restore(snap);
                }
            }

            // Collection delimiter equivalence: wildcard collection patterns
            // (e.g. `{ ... }` parsed as `set`) should match any collection with
            // the same delimiters (e.g. `dictionary`, `object`).
            if is_pure_ellipsis_pattern(pattern_children) {
                if let (Some(pd), Some(td)) =
                    (collection_delimiter(pattern_kind), collection_delimiter(&target_kind))
                {
                    if pd == td {
                        kind_match = true;
                    }
                }
            }

            if !kind_match {
                return false;
            }
        }

        let target_children: Vec<&dyn AstNode> = (0..target.child_count())
            .filter_map(|i| target.child(i))
            .filter(|c| {
                let kind = c.get_attribute("ts_kind").unwrap_or(c.node_type());
                if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                    return false;
                }
                if let Some(t) = c.text() {
                    let t = t.trim();
                    !t.is_empty()
                        && t != "("
                        && t != ")"
                        && t != "{"
                        && t != "}"
                        && t != "["
                        && t != "]"
                        && t != ";"
                        && t != ","
                        && t != ":"
                        && t != "."
                        && t != "="
                } else {
                    false
                }
            })
            .collect();

        if pattern_children.is_empty() {
            return kind_match || target_children.is_empty();
        }

        // Phase B: Deep chain matching for member/field chains with nested ellipsis.
        // When a pattern like `foo. ... .bar` has ellipsis inside nested
        // chain nodes, flatten both pattern and target into linear sequences
        // so ellipsis can match across nesting levels.
        let target_kind_str = target
            .get_attribute("ts_kind")
            .unwrap_or(target.node_type());
        if is_chain_kind(pattern_kind) && is_chain_kind(&target_kind_str) {
            let has_chain_child_with_ellipsis = pattern_children.iter().any(|c| {
                is_pattern_chain_node(c) && pattern_contains_ellipsis(c)
            });
            let has_any_ellipsis = pattern_contains_ellipsis(&PatternTree::Node {
                kind: pattern_kind.to_string(),
                children: pattern_children.to_vec(),
                text: None,
            });
            if has_chain_child_with_ellipsis || (has_any_ellipsis && is_chain_kind(pattern_kind)) {
                let flat_pattern = flatten_pattern_chain(pattern_kind, pattern_children);
                let flat_target = flatten_node_chain(target);

                let has_flat_ellipsis = flat_pattern
                    .iter()
                    .any(|p| matches!(p, PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }));

                let snap = self.snapshot();
                let matched = if has_flat_ellipsis {
                    self.match_children_with_ellipsis(&flat_pattern, &flat_target)
                } else {
                    self.match_children_exact(&flat_pattern, &flat_target)
                };
                if matched {
                    return true;
                }
                self.restore(snap);
            }
        }

        // Phase C: Associative binary chain matching.
        if let Some((pat_op, pat_operands)) =
            try_flatten_pattern_binary(pattern_kind, pattern_children)
        {
            if let Some((tgt_op, tgt_operands)) = flatten_binary_node(target) {
                if pat_op == tgt_op {
                    let snap = self.snapshot();
                    let assoc_result = self.match_associative_operands(&pat_operands, &tgt_operands, &pat_op);
                    if assoc_result {
                        return true;
                    }
                    self.restore(snap);
                }
            }
        }

        // Phase D: Unordered object matching.
        // When matching object-like nodes, allow pattern children to match
        // target children in any order (as long as each pattern child matches
        // at least one target child).
        if is_object_like_kind(&target_kind_str) && pattern_children.len() <= target_children.len()
        {
            let has_ellipsis = pattern_children
                .iter()
                .any(|p| matches!(p, PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }));
            if has_ellipsis || pattern_children.len() < target_children.len() {
                let snap = self.snapshot();
                if self.match_unordered_children(pattern_children, &target_children) {
                    return true;
                }
                self.restore(snap);
            }
        }

        // Regular child matching
        if pattern_children.len() > target_children.len()
            && !pattern_children.iter().any(|p| {
                matches!(
                    p,
                    PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }
                ) || is_optional_collection(p)
            })
        {
            return false;
        }

        let has_ellipsis = pattern_children
            .iter()
            .any(|p| matches!(p, PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }));

        if has_ellipsis {
            self.match_children_with_ellipsis(pattern_children, &target_children)
        } else if pattern_children.len() != target_children.len() {
            // Length mismatch with optional collections: filter out optional
            // collection children (e.g., argument_list([...]) that can match
            // zero targets) and try exact matching on the remainder.
            let filtered: Vec<&PatternTree> = pattern_children
                .iter()
                .filter(|p| !is_optional_collection(p))
                .collect();
            if filtered.len() == target_children.len() {
                self.match_children_exact_ref(&filtered, &target_children)
            } else {
                self.match_children_with_ellipsis(pattern_children, &target_children)
            }
        } else {
            self.match_children_exact(pattern_children, &target_children)
        }
    }
}

const TRANSPARENT_BLOCK_KINDS: &[&str] = &[
    "statement_block",
    "block",
    "compound_statement",
    "block_statement",
    "function_body",
];

impl MatchCtx {
    /// Exact child-sequence matching (no ellipsis).
    /// Handles transparent block unwrapping: statement_block with a single
    /// child is treated as equivalent to the bare statement.
    fn match_children_exact(
        &mut self,
        patterns: &[PatternTree],
        targets: &[&dyn AstNode],
    ) -> bool {
        if patterns.len() != targets.len() {
            return false;
        }
        for (pat, tgt) in patterns.iter().zip(targets.iter()) {
            let snap = self.snapshot();
            if self.match_tree(pat, *tgt) {
                continue;
            }
            self.restore(snap);

            let snap2 = self.snapshot();
            let pat_kind = match pat {
                PatternTree::Node { kind, .. } => Some(kind.as_str()),
                _ => None,
            };
            let tgt_kind = tgt.get_attribute("ts_kind").unwrap_or(tgt.node_type());
            let pat_is_block = pat_kind.map_or(false, |k| TRANSPARENT_BLOCK_KINDS.contains(&k));
            let tgt_is_block = TRANSPARENT_BLOCK_KINDS.contains(&tgt_kind);

            if pat_is_block != tgt_is_block {
                let matched = if tgt_is_block {
                    let inner = get_single_block_child(*tgt);
                    inner.map_or(false, |inner| self.match_tree(pat, inner))
                } else if pat_is_block {
                    if let PatternTree::Node { children, .. } = pat {
                        children.len() == 1 && self.match_tree(&children[0], *tgt)
                    } else {
                        false
                    }
                } else {
                    false
                };
                if matched {
                    continue;
                }
            }

            // Deep expression matching: if the pattern is a simple expression
            // (call, literal, identifier, etc.) but the target is a statement or
            // assignment, try to find the pattern inside the target recursively.
            if self.deep_match_in_node(pat, *tgt) {
                continue;
            }

            self.restore(snap2);
            return false;
        }
        true
    }

    /// Try to match a pattern deep inside a target node by recursing into children.
    fn deep_match_in_node(&mut self, pattern: &PatternTree, node: &dyn AstNode) -> bool {
        let node_kind = node.get_attribute("ts_kind").unwrap_or(node.node_type());
        // Only recurse into statement-like or expression wrapper nodes
        if !node_kind.contains("statement")
            && !node_kind.contains("expression")
            && !node_kind.contains("declaration")
            && !node_kind.contains("return")
            && node_kind != "assignment"
            && node_kind != "assignment_expression"
        {
            return false;
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_kind = child.get_attribute("ts_kind").unwrap_or(child.node_type());
                if child_kind == "comment" || child_kind == "line_comment" || child_kind == "block_comment" {
                    continue;
                }
                let snap = self.snapshot();
                if self.match_tree(pattern, child) {
                    return true;
                }
                self.restore(snap);

                if self.deep_match_in_node(pattern, child) {
                    return true;
                }
            }
        }
        false
    }

    /// Variant of `match_children_exact` that accepts pattern references
    /// instead of owned values, used for filtered optional-collection matching.
    fn match_children_exact_ref(
        &mut self,
        patterns: &[&PatternTree],
        targets: &[&dyn AstNode],
    ) -> bool {
        if patterns.len() != targets.len() {
            return false;
        }
        for (pat, tgt) in patterns.iter().zip(targets.iter()) {
            let snap = self.snapshot();
            if self.match_tree(pat, *tgt) {
                continue;
            }
            self.restore(snap);

            let snap2 = self.snapshot();
            let pat_kind = match pat {
                PatternTree::Node { kind, .. } => Some(kind.as_str()),
                _ => None,
            };
            let tgt_kind = tgt.get_attribute("ts_kind").unwrap_or(tgt.node_type());
            let pat_is_block = pat_kind.map_or(false, |k| TRANSPARENT_BLOCK_KINDS.contains(&k));
            let tgt_is_block = TRANSPARENT_BLOCK_KINDS.contains(&tgt_kind);

            if pat_is_block != tgt_is_block {
                let matched = if tgt_is_block {
                    let inner = get_single_block_child(*tgt);
                    inner.map_or(false, |inner| self.match_tree(pat, inner))
                } else if pat_is_block {
                    if let PatternTree::Node { children, .. } = pat {
                        children.len() == 1 && self.match_tree(&children[0], *tgt)
                    } else {
                        false
                    }
                } else {
                    false
                };
                if matched {
                    continue;
                }
                self.restore(snap2);
                return false;
            }
            return false;
        }
        true
    }

    /// Child matching with ellipsis support.
    ///
    /// Implements a backtracking algorithm similar to NFA matching:
    /// ellipsis (`...`) consumes 0..N target children, then the remaining
    /// patterns must match the remaining targets.
    fn match_children_with_ellipsis(
        &mut self,
        patterns: &[PatternTree],
        targets: &[&dyn AstNode],
    ) -> bool {
        if patterns.is_empty() {
            return true;
        }

        let first = &patterns[0];
        let rest = &patterns[1..];

        match first {
            PatternTree::Ellipsis => {
                for skip in 0..=targets.len() {
                    let snap = self.snapshot();
                    if self.match_children_with_ellipsis(rest, &targets[skip..]) {
                        return true;
                    }
                    self.restore(snap);
                }
                false
            }

            PatternTree::EllipsisMetavar { name } => {
                for skip in 0..=targets.len() {
                    let combined: String = targets[..skip]
                        .iter()
                        .filter_map(|n| n.text())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let snap = self.snapshot();
                    if combined.is_empty() || self.try_bind(name, &combined) {
                        if self.match_children_with_ellipsis(rest, &targets[skip..]) {
                            return true;
                        }
                    }
                    self.restore(snap);
                }
                false
            }

            _ => {
                if targets.is_empty() {
                    return false;
                }
                let snap = self.snapshot();
                if self.match_tree(first, targets[0]) {
                    return self.match_children_with_ellipsis(rest, &targets[1..]);
                }
                self.restore(snap);
                // Deep expression matching: try to find pattern inside target
                let snap2 = self.snapshot();
                if self.deep_match_in_node(first, targets[0]) {
                    return self.match_children_with_ellipsis(rest, &targets[1..]);
                }
                self.restore(snap2);
                // Skip non-matching target children
                self.match_children_with_ellipsis(patterns, &targets[1..])
            }
        }
    }

    /// Phase C: Associative operand matching with backtracking.
    ///
    /// Like `match_children_with_ellipsis` but with stricter termination:
    /// metavars consume 1..N operands, ellipsis consumes 0..N,
    /// and ALL pattern operands must be consumed.
    fn match_associative_operands(
        &mut self,
        patterns: &[PatternTree],
        targets: &[&dyn AstNode],
        operator: &str,
    ) -> bool {
        if patterns.is_empty() {
            // Bitwise operators (|, &, +, *) use AC matching: pattern is a
            // submultiset of the target, so extra target operands are fine.
            // Logical operators (and, or, &&, ||) require exact operand count.
            let ac_operators = ["|", "&", "+", "*"];
            if ac_operators.contains(&operator) {
                return true;
            }
            return targets.is_empty();
        }

        let first = &patterns[0];
        let rest = &patterns[1..];

        match first {
            PatternTree::Ellipsis => {
                // Consume 0..N targets
                for skip in 0..=targets.len() {
                    let snap = self.snapshot();
                    if self.match_associative_operands(rest, &targets[skip..], operator) {
                        return true;
                    }
                    self.restore(snap);
                }
                false
            }

            PatternTree::EllipsisMetavar { name } => {
                for skip in 0..=targets.len() {
                    let combined = join_operand_texts(&targets[..skip], operator);
                    let snap = self.snapshot();
                    if combined.is_empty() || self.try_bind(name, &combined) {
                        if self.match_associative_operands(rest, &targets[skip..], operator) {
                            return true;
                        }
                    }
                    self.restore(snap);
                }
                false
            }

            PatternTree::Metavar { name } => {
                // Metavar consumes 1..N operands
                for consume in 1..=targets.len() {
                    let combined = join_operand_texts(&targets[..consume], operator);
                    let snap = self.snapshot();
                    if self.try_bind(name, &combined) {
                        if self.match_associative_operands(rest, &targets[consume..], operator) {
                            return true;
                        }
                    }
                    self.restore(snap);
                }
                false
            }

            _ => {
                // Concrete pattern: try matching any target operand (associative = unordered)
                if targets.is_empty() {
                    return false;
                }
                for i in 0..targets.len() {
                    let snap = self.snapshot();
                    if self.match_tree(first, targets[i]) {
                        let remaining: Vec<&dyn AstNode> = targets.iter().enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, t)| *t)
                            .collect();
                        if self.match_associative_operands(rest, &remaining, operator) {
                            return true;
                        }
                    }
                    self.restore(snap);
                }
                false
            }
        }
    }

    /// Phase D: Unordered object child matching.
    ///
    /// Each pattern child must match at least one target child, but order
    /// doesn't matter. Ellipsis in the pattern means "any number of extra
    /// target children are allowed".
    fn match_unordered_children(
        &mut self,
        patterns: &[PatternTree],
        targets: &[&dyn AstNode],
    ) -> bool {
        // Separate ellipsis from concrete patterns
        let mut has_ellipsis = false;
        let mut concrete_patterns: Vec<&PatternTree> = Vec::new();
        for p in patterns {
            match p {
                PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. } => {
                    has_ellipsis = true;
                }
                _ => {
                    concrete_patterns.push(p);
                }
            }
        }

        // Without ellipsis, need at least as many targets as patterns
        if !has_ellipsis && concrete_patterns.len() > targets.len() {
            return false;
        }

        // Track which targets have been matched
        let mut used: Vec<bool> = vec![false; targets.len()];

        // Try to match each concrete pattern against an unused target
        self.match_unordered_subset(&concrete_patterns, targets, &mut used)
    }

    fn match_unordered_subset(
        &mut self,
        patterns: &[&PatternTree],
        targets: &[&dyn AstNode],
        used: &mut Vec<bool>,
    ) -> bool {
        if patterns.is_empty() {
            return true;
        }
        let first = patterns[0];
        let rest = &patterns[1..];

        for (i, target) in targets.iter().enumerate() {
            if used[i] {
                continue;
            }
            let snap = self.snapshot();
            if self.match_tree(first, *target) {
                used[i] = true;
                if self.match_unordered_subset(rest, targets, used) {
                    return true;
                }
                self.restore(snap);
                used[i] = false;
            } else {
                self.restore(snap);
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Deduplication
// ---------------------------------------------------------------------------

/// Remove matches that are ancestors of other matches (keep deepest).
fn deduplicate_matches(matches: &mut Vec<SemgrepMatchResult>) {
    if matches.len() <= 1 {
        return;
    }

    // Sort by location (start position ascending)
    matches.sort_by(|a, b| {
        let loc_a = a.node.location().unwrap_or((0, 0, 0, 0));
        let loc_b = b.node.location().unwrap_or((0, 0, 0, 0));
        loc_a.0.cmp(&loc_b.0).then(loc_a.1.cmp(&loc_b.1))
    });

    // Remove ancestor matches
    let mut keep = vec![true; matches.len()];
    for i in 0..matches.len() {
        if !keep[i] {
            continue;
        }
        let loc_i = matches[i].node.location().unwrap_or((0, 0, 0, 0));
        for j in (i + 1)..matches.len() {
            if !keep[j] {
                continue;
            }
            let loc_j = matches[j].node.location().unwrap_or((0, 0, 0, 0));
            // If match j is contained within match i, mark i for removal
            if location_contains(&loc_i, &loc_j) {
                keep[i] = false;
                break;
            }
        }
    }

    let mut write = 0;
    for read in 0..matches.len() {
        if keep[read] {
            if write != read {
                let placeholder = SemgrepMatchResult::new(
                    matches[read].node.clone_node(),
                    HashMap::new(),
                );
                let m = std::mem::replace(&mut matches[read], placeholder);
                matches[write] = m;
            }
            write += 1;
        }
    }
    matches.truncate(write);
}

fn location_contains(
    outer: &(usize, usize, usize, usize),
    inner: &(usize, usize, usize, usize),
) -> bool {
    let (os, oc, oe, oec) = *outer;
    let (is, ic, ie, iec) = *inner;
    // Outer starts before or at inner start
    let starts_before = os < is || (os == is && oc <= ic);
    // Outer ends after or at inner end
    let ends_after = oe > ie || (oe == ie && oec >= iec);
    starts_before && ends_after
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_ast::{AstBuilder, UniversalNode};
    use astgrep_core::AstNode;

    fn u(text: &str) -> UniversalNode {
        let id = AstBuilder::identifier(text).with_text(text.to_string());
        AstBuilder::expression_statement(id).with_text(text.to_string())
    }

    #[test]
    fn test_match_literal_exact() {
        let pattern = PatternTree::Node {
            kind: "identifier".to_string(),
            children: vec![],
            text: Some("foo".to_string()),
        };

        let target = AstBuilder::identifier("foo").with_text("foo".to_string());
        let mut ctx = MatchCtx::new();
        assert!(ctx.match_tree(&pattern, &target));
    }

    #[test]
    fn test_match_literal_mismatch() {
        let pattern = PatternTree::Node {
            kind: "identifier".to_string(),
            children: vec![],
            text: Some("foo".to_string()),
        };

        let target = AstBuilder::identifier("bar").with_text("bar".to_string());
        let mut ctx = MatchCtx::new();
        assert!(!ctx.match_tree(&pattern, &target));
    }

    #[test]
    fn test_match_metavar_binds() {
        let pattern = PatternTree::Metavar {
            name: "X".to_string(),
        };

        let target = AstBuilder::identifier("hello").with_text("hello".to_string());
        let mut ctx = MatchCtx::new();
        assert!(ctx.match_tree(&pattern, &target));
        assert_eq!(ctx.bindings.get("X").unwrap(), "hello");
    }

    #[test]
    fn test_match_metavar_consistency() {
        let pattern = PatternTree::Metavar {
            name: "X".to_string(),
        };

        let target1 = AstBuilder::identifier("a").with_text("a".to_string());
        let target2 = AstBuilder::identifier("b").with_text("b".to_string());

        let mut ctx = MatchCtx::new();
        assert!(ctx.match_tree(&pattern, &target1));
        // Same metavar must match same text
        assert!(!ctx.match_tree(&pattern, &target2));
    }

    #[test]
    fn test_match_ellipsis_exact_children() {
        // Pattern: foo(..., bar)
        let pattern = PatternTree::Node {
            kind: "call_expression".to_string(),
            children: vec![
                PatternTree::Node {
                    kind: "identifier".to_string(),
                    children: vec![],
                    text: Some("foo".to_string()),
                },
                PatternTree::Ellipsis,
                PatternTree::Node {
                    kind: "identifier".to_string(),
                    children: vec![],
                    text: Some("bar".to_string()),
                },
            ],
            text: None,
        };

        // Target: foo(x, y, z, bar) — simulated as a node with children
        let target = AstBuilder::simple_call(
            "foo",
            vec![
                AstBuilder::identifier("x").with_text("x".to_string()),
                AstBuilder::identifier("y").with_text("y".to_string()),
                AstBuilder::identifier("bar").with_text("bar".to_string()),
            ],
        )
        .with_text("foo(x, y, bar)".to_string());

        let mut ctx = MatchCtx::new();
        let result = ctx.match_tree(&pattern, &target);
        assert!(result, "foo(x,y,bar) should match pattern foo(..., bar)");
    }

    #[test]
    fn test_location_contains() {
        let outer = (1, 1, 5, 10);
        let inner = (2, 1, 4, 5);
        assert!(location_contains(&outer, &inner));
        assert!(!location_contains(&inner, &outer));
    }

    #[test]
    fn test_declaration_keywords_equivalent() {
        assert!(declaration_keywords_equivalent("var", "let"));
        assert!(declaration_keywords_equivalent("const", "var"));
        assert!(declaration_keywords_equivalent("let", "const"));
        assert!(!declaration_keywords_equivalent("var", "function"));
        assert!(declaration_keywords_equivalent("let", "let"));
    }

    #[test]
    fn test_is_associative_operator() {
        assert!(is_associative_operator("and"));
        assert!(is_associative_operator("&&"));
        assert!(is_associative_operator("or"));
        assert!(is_associative_operator("||"));
        assert!(is_associative_operator("+"));
        assert!(!is_associative_operator("-"));
        assert!(!is_associative_operator("=="));
    }

    #[test]
    fn test_is_binary_expression_kind() {
        assert!(is_binary_expression_kind("binary_expression"));
        assert!(is_binary_expression_kind("boolean_operation"));
        assert!(is_binary_expression_kind("logical_expression"));
        assert!(!is_binary_expression_kind("identifier"));
    }

    #[test]
    fn test_is_object_like_kind() {
        assert!(is_object_like_kind("object"));
        assert!(is_object_like_kind("object_pattern"));
        assert!(is_object_like_kind("object_literal"));
        assert!(!is_object_like_kind("array"));
        assert!(!is_object_like_kind("function"));
    }
}
