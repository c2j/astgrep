//! Tree-to-tree structural pattern matching engine.
//!
//! Matches a `PatternTree` (produced by `PatternTreeParser`) against target
//! AST nodes using recursive structural comparison.  This replaces the
//! text-token matching approach with proper AST-level matching inspired by
//! semgrep's `Pattern_vs_code.ml`.

use astgrep_core::{AstNode, Language, SemgrepMatchResult, MatchBinding};
use astgrep_parser::pattern_tree::{PatternTree, PatternTreeParser};
use std::collections::HashMap;
use std::sync::Mutex;

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
                Err(_) => return Vec::new(),
            },
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        let mut ctx = MatchCtx::new();
        ctx.find_recursive(&tree, root, &mut results);

        // Deduplicate: keep the deepest (smallest) matches, drop ancestor
        // matches if a descendant already matched at the same location.
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
}

impl MatchCtx {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    fn snapshot(&self) -> HashMap<String, String> {
        self.bindings.clone()
    }

    fn restore(&mut self, snapshot: HashMap<String, String>) {
        self.bindings = snapshot;
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
    // Recursive search
    // -----------------------------------------------------------------------

    /// Walk the target AST top-down.  At each node, attempt to match the
    /// pattern tree.  Collect every successful match.
    fn find_recursive(
        &mut self,
        pattern: &PatternTree,
        node: &dyn AstNode,
        results: &mut Vec<SemgrepMatchResult>,
    ) {
        let snap = self.snapshot();
        if self.match_tree(pattern, node) {
            let bindings: HashMap<String, MatchBinding> = self
                .bindings
                .iter()
                .map(|(k, v)| (k.clone(), MatchBinding::new(v.clone())))
                .collect();
            results.push(SemgrepMatchResult::new(node.clone_node(), bindings));
        }
        self.restore(snap);

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_recursive(pattern, child, results);
            }
        }
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
                        if self.match_tree(pattern, child) {
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
        // Phase 1: If pattern has concrete text, it must match the target's text.
        if let Some(ref pt) = pattern_text {
            if let Some(tt) = target.text() {
                let tt = tt.trim();
                let pt = pt.trim();

                if pt == tt {
                    return true;
                }

                let stripped_tt = tt
                    .strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                    .or_else(|| tt.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                    .unwrap_or(tt);
                let stripped_pt = pt
                    .strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                    .or_else(|| pt.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                    .unwrap_or(pt);
                if stripped_pt == stripped_tt {
                    return true;
                }

                return false;
            }
            return false;
        }

        // Phase 2: Text-less pattern — must match structurally via children.
        // The pattern's kind must be compatible with the target's node type.
        let target_kind = target.node_type();
        let kind_match = pattern_kind == target_kind
            || pattern_kind == "_"
            || (pattern_kind.contains('_') && target_kind.contains('_')
                && pattern_kind.split('_').any(|p| target_kind.contains(p)));

        if !kind_match && !pattern_children.is_empty() {
            return false;
        }

        let target_children: Vec<&dyn AstNode> = (0..target.child_count())
            .filter_map(|i| target.child(i))
            .filter(|c| {
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
                } else {
                    false
                }
            })
            .collect();

        if pattern_children.is_empty() {
            return kind_match || target_children.is_empty();
        }

        if pattern_children.len() != target_children.len() && !pattern_children.iter().any(|p| matches!(p, PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. })) {
            return false;
        }

        let has_ellipsis = pattern_children
            .iter()
            .any(|p| matches!(p, PatternTree::Ellipsis | PatternTree::EllipsisMetavar { .. }));

        if has_ellipsis {
            self.match_children_with_ellipsis(pattern_children, &target_children)
        } else {
            self.match_children_exact(pattern_children, &target_children)
        }
    }

    /// Exact child-sequence matching (no ellipsis).
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
            if !self.match_tree(pat, *tgt) {
                self.restore(snap);
                return false;
            }
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
                // Try consuming 0..remaining targets
                for skip in 0..=(targets.len()) {
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
                        .filter_map(|t| t.text())
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
                // Non-ellipsis pattern must match the first target
                if targets.is_empty() {
                    return false;
                }
                let snap = self.snapshot();
                if self.match_tree(first, targets[0]) {
                    return self.match_children_with_ellipsis(rest, &targets[1..]);
                }
                self.restore(snap);
                false
            }
        }
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
    if os < is {
        return true;
    }
    if os > is {
        return false;
    }
    if oc <= ic {
        if oe > ie {
            return true;
        }
        if oe < ie {
            return false;
        }
        return oec >= iec;
    }
    false
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
}
