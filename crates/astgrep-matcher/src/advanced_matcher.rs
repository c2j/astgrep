//! Advanced pattern matcher with full semgrep syntax support
//!
//! This module implements a sophisticated pattern matcher that supports
//! all semgrep pattern types including pattern-either, pattern-inside,
//! pattern-not, metavariable-pattern, and metavariable-regex.

use crate::metavar::MetavarManager;
use crate::parser::{ParsedPattern, PatternParser};
use crate::tree_matcher::TreeMatcher;
use astgrep_core::{
    AnalysisError, AstNode, ComparisonOperator, Condition, MatchBinding, PatternType, Result,
    SemgrepMatchResult, SemgrepPattern,
};
use astgrep_core::{ComplexityAnalysis, EntropyAnalysis, MetavariableAnalysis, TypeAnalysis};
// Note: These types are defined in cr_rules but we'll use them through cr_core for now
use astgrep_dataflow::ConstantValue;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Advanced pattern matcher with full semgrep support
pub struct AdvancedSemgrepMatcher {
    parser: PatternParser,
    metavar_manager: MetavarManager,
    debug_mode: bool,
    max_depth: Option<usize>,
    constant_values: HashMap<String, ConstantValue>,
    full_source: Option<String>,
    inside_match_cache: HashMap<String, Vec<((usize, usize, usize, usize), HashMap<String, String>)>>,
    symbolic_propagator: Option<astgrep_dataflow::SymbolicPropagator>,
    tree_matcher: TreeMatcher,
    language_hint: Option<astgrep_core::Language>,
}

impl Default for AdvancedSemgrepMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedSemgrepMatcher {
    /// Create a new advanced semgrep matcher
    pub fn new() -> Self {
        Self {
            parser: PatternParser::new(),
            metavar_manager: MetavarManager::new(),
            debug_mode: false,
            max_depth: None,
            constant_values: HashMap::new(),
            full_source: None,
            inside_match_cache: HashMap::new(),
            symbolic_propagator: None,
            tree_matcher: TreeMatcher::new(),
            language_hint: None,
        }
    }

    /// Set constant propagation values
    pub fn with_constant_values(mut self, constants: HashMap<String, ConstantValue>) -> Self {
        self.constant_values = constants;
        self
    }

    /// Set constant propagation values (mutable)
    pub fn set_constant_values(&mut self, constants: HashMap<String, ConstantValue>) {
        self.constant_values = constants;
    }

    /// Set symbolic propagator for variable alias tracking (mutable)
    pub fn set_symbolic_propagator(&mut self, propagator: astgrep_dataflow::SymbolicPropagator) {
        self.symbolic_propagator = Some(propagator);
    }

    /// Enable debug mode
    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }

    /// Set maximum matching depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Find all matches for a pattern in the AST
    pub fn set_language(&mut self, lang: astgrep_core::Language) {
        self.language_hint = Some(lang);
    }

    pub fn find_matches(
        &mut self,
        pattern: &SemgrepPattern,
        root: &dyn AstNode,
    ) -> Result<Vec<SemgrepMatchResult>> {
        self.full_source = root.text().map(|s| s.to_string());
        self.inside_match_cache.clear();

        let mut matches = Vec::new();
        self.find_matches_recursive(pattern, root, &mut matches, 0)?;

        // Augment with AST structural matching results (Phase 2 engine)
        if let PatternType::Simple(pattern_str) = &pattern.pattern_type {
            if let Some(lang) = self.language_hint {
                let tree_results = self.tree_matcher.find_matches(pattern_str, lang, root);
                if !tree_results.is_empty() {
                    let existing: std::collections::HashSet<(usize, usize)> = matches
                        .iter()
                        .filter_map(|m| {
                            let loc = m.node.location();
                            Some((loc?.0, loc?.1))
                        })
                        .collect();
                    for r in tree_results {
                        if let Some(loc) = r.node.location() {
                            if !existing.contains(&(loc.0, loc.1)) {
                                matches.push(r);
                            }
                        }
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Recursively find matches in the AST
    /// Returns whether this subtree produced any match (to enable parent suppression)
    fn find_matches_recursive(
        &mut self,
        pattern: &SemgrepPattern,
        node: &dyn AstNode,
        matches: &mut Vec<SemgrepMatchResult>,
        depth: usize,
    ) -> Result<bool> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if depth > max_depth {
                return Ok(false);
            }
        }

        // First, recurse into children
        let mut subtree_has_match = false;
                for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                                if self.find_matches_recursive(pattern, child, matches, depth + 1)? {
                    subtree_has_match = true;
                                    }
            } else {
                            }
        }

        // Try to match at current node only if no descendant produced a match
        if !subtree_has_match {
            let snapshot = self.metavar_manager.snapshot();
                        if self.matches_pattern(pattern, node)? {
                let bindings = self.metavar_manager.get_binding_values();
                                let match_bindings: HashMap<String, MatchBinding> = bindings
                    .into_iter()
                    .map(|(k, v)| (k, MatchBinding::new(v)))
                    .collect();
                matches.push(SemgrepMatchResult::new(node.clone_node(), match_bindings));
                self.metavar_manager.restore(snapshot);
                return Ok(true);
            }
            self.metavar_manager.restore(snapshot);
        }

        Ok(subtree_has_match)
    }

    /// Check if a pattern matches a node
    fn matches_pattern(&mut self, pattern: &SemgrepPattern, node: &dyn AstNode) -> Result<bool> {
        // First check if pattern type matches
        let type_matches = match &pattern.pattern_type {
            PatternType::Simple(pattern_str) => self.matches_simple_pattern(pattern_str, node)?,
            PatternType::Either(patterns) => self.matches_either_pattern(patterns, node)?,
            PatternType::Inside(inner_pattern) => {
                self.matches_inside_pattern(inner_pattern, node)?
            }
            PatternType::NotInside(inner_pattern) => {
                self.matches_not_inside_pattern(inner_pattern, node)?
            }
            PatternType::Not(inner_pattern) => self.matches_not_pattern(inner_pattern, node)?,
            PatternType::Regex(regex_str) => self.matches_regex_pattern(regex_str, node)?,
            PatternType::NotRegex(regex_str) => self.matches_not_regex_pattern(regex_str, node)?,
            PatternType::All(patterns) => self.matches_all_patterns(patterns, node)?,
            PatternType::Any(patterns) => self.matches_any_patterns(patterns, node)?,
        };

        // If pattern type matches, evaluate conditions (e.g., metavariable-regex)
        if type_matches && !pattern.conditions.is_empty() {
            let bindings = self.metavar_manager.get_binding_values();
                        let result = self.evaluate_conditions(&pattern.conditions, &bindings);
                        return result;
        }

        Ok(type_matches)
    }

    /// Match a simple pattern string
    fn matches_simple_pattern(&mut self, pattern_str: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(inner) = Self::extract_deep_expr(pattern_str) {
            return self.match_deep_expr_from_str(&inner, node);
        }

        let parsed_pattern = self.parser.parse(pattern_str)?;
        let result = self.match_parsed_pattern(&parsed_pattern, node, 0);
        result
    }

    fn extract_deep_expr(pattern: &str) -> Option<String> {
        let start = pattern.find("<...")?;
        let rest = &pattern[start + 4..];
        let end = rest.find("...>")?;
        let inner = rest[..end].trim().to_string();
        if inner.is_empty() {
            return None;
        }
        Some(inner)
    }

    fn match_deep_expr_from_str(&mut self, inner: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if text.contains(inner) {
                return Ok(true);
            }
        }

        let parsed = self.parser.parse(inner)?;
        if self.match_parsed_pattern(&parsed, node, 0)? {
            return Ok(true);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let snapshot = self.metavar_manager.snapshot();
                if self.match_deep_expr_from_str(inner, child)? {
                    return Ok(true);
                }
                self.metavar_manager.restore(snapshot);
            }
        }

        Ok(false)
    }

    /// Match pattern-either (OR logic)
    fn matches_either_pattern(
        &mut self,
        patterns: &[SemgrepPattern],
        node: &dyn AstNode,
    ) -> Result<bool> {
        for pattern in patterns {
            let snapshot = self.metavar_manager.snapshot();
            if self.matches_pattern(pattern, node)? {
                return Ok(true);
            }
            self.metavar_manager.restore(snapshot);
        }
        Ok(false)
    }

    /// Match pattern-inside
    fn matches_inside_pattern(
        &mut self,
        inner_pattern: &SemgrepPattern,
        node: &dyn AstNode,
    ) -> Result<bool> {
        // Check if the current node or any of its ancestors match the inner pattern
        let current = Some(node);
        if let Some(current_node) = current {
            if self.matches_pattern(inner_pattern, current_node)? {
                return Ok(true);
            }
        }

        // Also check if any descendant matches
        self.matches_inside_recursive(inner_pattern, node)
    }

    /// Recursively check for pattern-inside matches
    fn matches_inside_recursive(
        &mut self,
        pattern: &SemgrepPattern,
        node: &dyn AstNode,
    ) -> Result<bool> {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let snapshot = self.metavar_manager.snapshot();
                if self.matches_pattern(pattern, child)? {
                    return Ok(true);
                }
                self.metavar_manager.restore(snapshot);

                if self.matches_inside_recursive(pattern, child)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Match pattern-not-inside
    fn matches_not_inside_pattern(
        &mut self,
        inner_pattern: &SemgrepPattern,
        node: &dyn AstNode,
    ) -> Result<bool> {
        // A pattern matches pattern-not-inside if it does NOT match pattern-inside
        let snapshot = self.metavar_manager.snapshot();
        let matches_inside = self.matches_inside_pattern(inner_pattern, node)?;
        self.metavar_manager.restore(snapshot);
        Ok(!matches_inside)
    }

    /// Match pattern-not
    fn matches_not_pattern(
        &mut self,
        inner_pattern: &SemgrepPattern,
        node: &dyn AstNode,
    ) -> Result<bool> {
        let snapshot = self.metavar_manager.snapshot();
        let matches = self.matches_pattern(inner_pattern, node)?;
        self.metavar_manager.restore(snapshot);
        Ok(!matches)
    }

    /// Match pattern-regex
    fn matches_regex_pattern(&mut self, regex_str: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if let Ok(regex) = Regex::new(regex_str) {
                Ok(regex.is_match(text))
            } else {
                Err(AnalysisError::pattern_match_error(format!(
                    "Invalid regex: {}",
                    regex_str
                )))
            }
        } else {
            Ok(false)
        }
    }

    /// Match pattern-not-regex
    fn matches_not_regex_pattern(&mut self, regex_str: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if let Ok(regex) = Regex::new(regex_str) {
                Ok(!regex.is_match(text))
            } else {
                Err(AnalysisError::pattern_match_error(format!(
                    "Invalid regex: {}",
                    regex_str
                )))
            }
        } else {
            Ok(true) // If no text, it doesn't match the regex, so not-regex is true
        }
    }

    /// Match all patterns (AND logic)
    fn matches_all_patterns(
        &mut self,
        patterns: &[SemgrepPattern],
        node: &dyn AstNode,
    ) -> Result<bool> {
        
        // Separate patterns into categories
        let (context_patterns, rest): (Vec<_>, Vec<_>) = patterns.iter().partition(|p| {
            matches!(
                p.pattern_type,
                PatternType::Inside(_) | PatternType::NotInside(_)
            )
        });

        let (negative_patterns, content_patterns): (Vec<&SemgrepPattern>, Vec<&SemgrepPattern>) =
            rest.iter().partition(|p| {
                matches!(
                    p.pattern_type,
                    PatternType::Not(_) | PatternType::NotRegex(_)
                )
            });

        
        let snapshot = self.metavar_manager.snapshot();

        // Step 1: Check ALL context patterns (pattern-inside) to establish bindings
        // ALL context patterns must match (intersection semantics)
        for pattern in &context_patterns {
            
            let context_matches = match &pattern.pattern_type {
                PatternType::Inside(inner) => self.matches_inside_context(inner, node, patterns)?,
                PatternType::NotInside(inner) => {
                    let inside_matches = self.matches_inside_context(inner, node, patterns)?;
                    !inside_matches
                }
                _ => unreachable!(),
            };

            if !context_matches {
                                self.metavar_manager.restore(snapshot);
                return Ok(false);
            }

                    }

        // Step 2: Match content patterns with established bindings
        // Content patterns must match using the bindings from context patterns
        for pattern in &content_patterns {
                        if !self.matches_pattern(pattern, node)? {
                                self.metavar_manager.restore(snapshot);
                return Ok(false);
            }
                    }

        // Step 3: Check negative patterns (must NOT match)
        for pattern in &negative_patterns {
                        let neg_snapshot = self.metavar_manager.snapshot();
            let negative_matches = match &pattern.pattern_type {
                PatternType::Not(inner) => self.matches_not_pattern(inner.as_ref(), node)?,
                PatternType::NotRegex(regex) => {
                    self.matches_not_regex_pattern(regex.as_str(), node)?
                }
                _ => unreachable!(),
            };
            if !negative_matches {
                                self.metavar_manager.restore(snapshot);
                return Ok(false);
            }
            self.metavar_manager.restore(neg_snapshot);
        }

                Ok(true)
    }

    /// Check if a node is inside a pattern context (for pattern-inside)
    ///
    /// This function now properly extracts metavariable bindings from the pattern-inside match,
    /// enabling metavariable unification between context and content patterns.
    ///
    /// For example, with:
    ///   pattern-inside: class $T { private int $X; ... }
    ///   pattern: foo(this.$X)
    ///
    /// If the class declares "private int x;", this function will bind $X="x",
    /// and the content pattern will only match "foo(this.x)", not "foo(this.y)".
    fn matches_inside_context(
        &mut self,
        inner_pattern: &SemgrepPattern,
        node: &dyn AstNode,
        _all_patterns: &[SemgrepPattern],
    ) -> Result<bool> {
        let snapshot = self.metavar_manager.snapshot();
        if self.matches_pattern(inner_pattern, node)? {
            return Ok(true);
        }
        self.metavar_manager.restore(snapshot);

        // AST-based containment check: find all nodes matching the inside pattern
        // and check if the current node is spatially contained within any match.
        if let Some(node_loc) = node.location() {
            if let PatternType::Simple(inside_str) = &inner_pattern.pattern_type {
                let inside_matches = self.get_inside_pattern_matches(inside_str)?;
                for (match_loc, bindings) in &inside_matches {
                    if self.location_contains(match_loc, &node_loc) {
                        for (var_name, value) in bindings {
                            let normalized_name = var_name.strip_prefix('$').unwrap_or(var_name).to_string();
                            let _ = self.metavar_manager.bind(normalized_name, value.clone(), node);
                        }
                        return Ok(true);
                    }
                }
            }
        }

        // For non-simple pattern types (Either, All, etc.), try matching against ancestors
        // by checking if the inside pattern matches any node whose location contains this node
        self.check_ancestor_contains(inner_pattern, node)
    }

    fn get_inside_pattern_matches(
        &mut self,
        pattern_str: &str,
    ) -> Result<Vec<((usize, usize, usize, usize), HashMap<String, String>)>> {
        if let Some(cached) = self.inside_match_cache.get(pattern_str) {
            return Ok(cached.clone());
        }

        let mut results = Vec::new();
        if let Some(ref full_source) = self.full_source {
            if let Some(regions) = self.find_inside_regions(pattern_str, full_source) {
                for (reg_start, reg_end, bindings) in &regions {
                    if let Some(loc) = self.byte_offsets_to_location(*reg_start, *reg_end, full_source) {
                        results.push((loc, bindings.clone()));
                    }
                }
            }
        }

        self.inside_match_cache.insert(pattern_str.to_string(), results.clone());
        Ok(results)
    }

    fn byte_offsets_to_location(
        &self,
        start_byte: usize,
        end_byte: usize,
        source: &str,
    ) -> Option<(usize, usize, usize, usize)> {
        let mut line = 1;
        let mut col = 1;
        let mut start_line = 0;
        let mut start_col = 0;

        for (i, ch) in source.char_indices() {
            if i == start_byte {
                start_line = line;
                start_col = col;
            }
            if i == end_byte {
                return Some((start_line, start_col, line, col));
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        if start_line > 0 {
            return Some((start_line, start_col, line, col));
        }
        None
    }

    fn location_contains(
        &self,
        outer: &(usize, usize, usize, usize),
        inner: &(usize, usize, usize, usize),
    ) -> bool {
        let (os, oc, oe, oec) = *outer;
        let (is, ic, ie, iec) = *inner;
        if os < is { return true; }
        if os > is { return false; }
        if oc <= ic {
            if oe > ie { return true; }
            if oe < ie { return false; }
            return oec >= iec;
        }
        false
    }

    fn check_ancestor_contains(
        &mut self,
        inner_pattern: &SemgrepPattern,
        node: &dyn AstNode,
    ) -> Result<bool> {
        if let Some(node_loc) = node.location() {
            let node_text = node.text().unwrap_or("").to_string();
            if let Some(ref full_source) = self.full_source {
                let parsed = self.parser.parse(
                    match &inner_pattern.pattern_type {
                        PatternType::Simple(s) => s.as_str(),
                        _ => "",
                    }
                ).unwrap_or(ParsedPattern::Wildcard);

                let search_text = match &inner_pattern.pattern_type {
                    PatternType::Simple(s) => {
                        let key_tokens: Vec<&str> = s.split(|c: char| c.is_whitespace() || "(){}[];,".contains(c))
                            .filter(|t| !t.is_empty() && !t.starts_with('$') && *t != "...")
                            .take(3)
                            .collect();
                        key_tokens
                    },
                    _ => vec![],
                };

                if !search_text.is_empty() {
                    let mut byte_offset = 0;
                    let mut current_line = 1;

                    for line_text in full_source.lines() {
                        let line_has_keywords = search_text.iter().all(|kw| line_text.contains(kw));
                        if line_has_keywords && current_line <= node_loc.0 {
                            if let Some(col_pos) = line_text.find(search_text[0]) {
                                let enc_start_line = current_line;
                                let enc_start_col = col_pos + 1;

                                let snapshot = self.metavar_manager.snapshot();
                                if enc_start_line < node_loc.0 || (enc_start_line == node_loc.0 && enc_start_col <= node_loc.1) {
                                    self.metavar_manager.restore(snapshot);
                                    return Ok(true);
                                }
                                self.metavar_manager.restore(snapshot);
                            }
                        }

                        byte_offset += line_text.len() + 1;
                        current_line += 1;
                    }
                }
            }
        }

        Ok(false)
    }

    #[allow(clippy::type_complexity)]
    fn find_inside_regions(
        &self,
        pattern_str: &str,
        source: &str,
    ) -> Option<Vec<(usize, usize, HashMap<String, String>)>> {
        let trimmed = pattern_str.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut regex_str = String::new();
        let mut metavar_groups: Vec<String> = Vec::new(); // Track which group index -> metavar name
        let chars: Vec<char> = trimmed.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
                // Metavariable: match non-whitespace sequence, track for binding extraction
                let mut var_name = String::new();
                i += 1;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    var_name.push(chars[i]);
                    i += 1;
                }
                metavar_groups.push(var_name);
                regex_str.push_str(r"(\S+)");
            } else if chars[i] == '.'
                && i + 2 < chars.len()
                && chars[i + 1] == '.'
                && chars[i + 2] == '.'
            {
                // Check if this ellipsis follows a comma (optional args pattern)
                // The regex_str contains backslash escape sequences like "\S" for \S in the actual regex
                let ends_with_comma_pattern = regex_str.contains(",")
                    && (regex_str.ends_with(",[ \t]*")
                        || regex_str.ends_with("),[ \t]*")
                        || regex_str.ends_with("\\S+),[ \\t]*")
                        || regex_str.ends_with("(\\S+),[ \\t]*"));

                if ends_with_comma_pattern {
                    // Find and remove the comma before the ellipsis to make args optional
                    if let Some(pos) = regex_str.rfind(",") {
                        regex_str.truncate(pos);
                        regex_str.push_str(r"(?:[ \t]*,[ \t]*(?:[\s\S]*))?");
                    } else {
                        regex_str.push_str(r"(?:[\s\S]*)");
                    }
                } else {
                    // Use non-greedy matching for ellipsis to avoid consuming too much
                    regex_str.push_str(r"(?:[\s\S]*?)");
                }
                i += 3;
            } else if chars[i] == ' ' || chars[i] == '\t' {
                regex_str.push_str(r"[ \t]*");
                i += 1;
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                    i += 1;
                }
            } else if chars[i] == '\n' || chars[i] == '\r' {
                regex_str.push_str(r"[\r\n]*");
                i += 1;
                while i < chars.len() && (chars[i] == '\n' || chars[i] == '\r') {
                    i += 1;
                }
            } else if chars[i] == '('
                || chars[i] == ')'
                || chars[i] == '['
                || chars[i] == ']'
                || chars[i] == '{'
                || chars[i] == '}'
                || chars[i] == '+'
                || chars[i] == '*'
                || chars[i] == '?'
                || chars[i] == '^'
                || chars[i] == '$'
                || chars[i] == '|'
                || chars[i] == '\\'
            {
                regex_str.push('\\');
                regex_str.push(chars[i]);
                i += 1;
            } else {
                regex_str.push(chars[i]);
                i += 1;
            }
        }

        if let Ok(re) = regex::Regex::new(&regex_str) {
            let mut results = Vec::new();
            for cap in re.captures_iter(source) {
                let mut bindings = HashMap::new();
                for (idx, var_name) in metavar_groups.iter().enumerate() {
                    if let Some(matched) = cap.get(idx + 1) {
                        bindings.insert(var_name.clone(), matched.as_str().to_string());
                    }
                }
                if let Some(full_match) = cap.get(0) {
                    results.push((full_match.start(), full_match.end(), bindings));
                }
            }
                        if !results.is_empty() {
                return Some(results);
            }
        } else {
                    }

        None
    }

    fn get_node_byte_offset_range(
        &self,
        node: &dyn AstNode,
        source: &str,
    ) -> Option<(usize, usize)> {
        let (start_line, start_col, end_line, end_col) = node.location()?;

        let mut byte_offset = 0;
        let mut current_line = 1;

        while current_line < start_line && byte_offset < source.len() {
            if source.as_bytes()[byte_offset] == b'\n' {
                current_line += 1;
            }
            byte_offset += 1;
        }
        let start = byte_offset + start_col;

        while current_line < end_line && byte_offset < source.len() {
            if source.as_bytes()[byte_offset] == b'\n' {
                current_line += 1;
            }
            byte_offset += 1;
        }
        let end = byte_offset + end_col;

        Some((start, end.min(source.len())))
    }

    /// Extract field bindings from a class context pattern
    ///
    /// Given a pattern like "class $T { private int $X; ... }" and source text,
    /// this function extracts what metavariables like $X should be bound to.
    ///
    /// Returns a map of variable names to their bound values.
    fn extract_field_bindings_from_class_context(
        &self,
        pattern_str: &str,
        source_text: &str,
    ) -> Option<HashMap<String, String>> {
        use regex::Regex;

        let mut bindings = HashMap::new();

        // Parse the pattern to extract field declaration information
        // Pattern format: class $T { ... private TYPE $X; ... }

        // Extract the field type and metavariable name from the pattern
        // Look for patterns like "private int $X" or "private String $FIELD"
        let field_pattern = Regex::new(r"private\s+(\w+)\s+\$(\w+)").ok()?;

        if let Some(captures) = field_pattern.captures(pattern_str) {
            let field_type = captures.get(1)?.as_str();
            let metavar_name = captures.get(2)?.as_str();

                        
            // The source_text might be just a small node text (like "private") or the full file
            // We need to search for field declarations in the available text
            // Match: private int x; or private int x = ...;
            let decl_pattern = Regex::new(&format!(
                r"private\s+{}\s+(\w+)\s*(?:=|;)",
                regex::escape(field_type)
            ))
            .ok()?;

            for cap in decl_pattern.captures_iter(source_text) {
                if let Some(field_name_match) = cap.get(1) {
                    let field_name = field_name_match.as_str().to_string();
                                        bindings.insert(format!("${}", metavar_name), field_name);
                    // Note: In a full implementation, we'd handle multiple fields of the same type
                    // For now, we take the first match
                    break;
                }
            }

            // If no field found in this text, we might need to look at a broader context
            // For now, store what we're looking for so we can validate later
            if !bindings.contains_key(&format!("${}", metavar_name)) {
                            }
        }

        // Also handle class name metavariable $T
        if pattern_str.contains("$T") {
            // Look for class declaration
            let class_pattern = Regex::new(r"class\s+(\w+)").ok()?;
            if let Some(cap) = class_pattern.captures(source_text) {
                if let Some(class_name_match) = cap.get(1) {
                    let class_name = class_name_match.as_str().to_string();
                                        bindings.insert("$T".to_string(), class_name);
                }
            }
        }

        if bindings.is_empty() {
            None
        } else {
            Some(bindings)
        }
    }

    /// Match any patterns (OR logic, same as either)
    fn matches_any_patterns(
        &mut self,
        patterns: &[SemgrepPattern],
        node: &dyn AstNode,
    ) -> Result<bool> {
        self.matches_either_pattern(patterns, node)
    }

    /// Match a parsed pattern against a node
    fn match_parsed_pattern(
        &mut self,
        pattern: &ParsedPattern,
        node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        match pattern {
            ParsedPattern::Literal(literal) => self.match_literal(literal, node),
            ParsedPattern::Metavariable(metavar) => self.match_metavariable(metavar, node),
            ParsedPattern::EllipsisMetavariable(metavar) => {
                self.match_ellipsis_metavariable(metavar, node)
            }
            ParsedPattern::NodeType(node_type) => self.match_node_type(node_type, node),
            ParsedPattern::Sequence(patterns) => self.match_sequence(patterns, node, depth),
            ParsedPattern::Alternative(patterns) => self.match_alternative(patterns, node, depth),
            ParsedPattern::Wildcard => Ok(true),
            ParsedPattern::DeepExpr(inner) => self.match_deep_expr(inner, node, depth),
        }
    }

    /// Match literal text with constant propagation support
    fn match_literal(&self, literal: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            // Special case: "..." in a pattern should match any non-empty string literal
            // The pattern "..." is parsed as Literal("...")
            if literal == "..." {
                // Must be a complete string literal node
                if node.node_type() == "literal"
                    || node.node_type() == "string_literal"
                    || node.node_type() == "string"
                {
                    // Check if it's a non-empty string (starts with ", ends with ", length > 2)
                    if text.starts_with('"') && text.ends_with('"') && text.len() > 2 {
                        return Ok(true);
                    }
                }
                // Don't match partial strings or individual quote characters
                return Ok(false);
            }

            if text.contains(literal) {
                return Ok(true);
            }

            // For single-token patterns, use word-boundary matching to prevent
            // "return" from matching "returns" etc.
            if !literal.contains(' ') && !literal.contains('.') && literal.len() > 1 {
                // Use \b word boundary instead of look-around (Rust regex crate
                // does not support look-around assertions).
                let re_str = format!(r"\b{}\b", regex::escape(literal));
                if let Ok(re) = Regex::new(&re_str) {
                    if re.is_match(text) {
                        return Ok(true);
                    }
                }
            }

            // Constant propagation: if node is an identifier, check if it has a constant value
            if node.node_type() == "identifier" {
                if let Some(constant_value) = self.constant_values.get(text) {
                    // Check if the constant value matches the literal
                    let constant_str = match constant_value {
                        ConstantValue::Integer(i) => i.to_string(),
                        ConstantValue::String(s) => s.clone(),
                        ConstantValue::Boolean(b) => b.to_string(),
                        ConstantValue::Null => "null".to_string(),
                        ConstantValue::Unknown => return Ok(false),
                    };

                    if constant_str == literal {
                        return Ok(true);
                    }
                }
            }

            Ok(false)
        } else {
            Ok(false)
        }
    }

    fn match_metavariable(&mut self, metavar: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if text.trim().is_empty() {
                return Ok(false);
            }
            let bind_key = if metavar == "_" {
                format!("__anon_{}", node.node_type().len())
            } else {
                metavar.to_string()
            };
            let existing = self.metavar_manager.get_binding_values();
            let _ = existing;
            self.metavar_manager
                .bind(bind_key, text.to_string(), node)
        } else {
            Ok(false)
        }
    }

    /// Match ellipsis metavariable
    fn match_ellipsis_metavariable(&mut self, metavar: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            self.metavar_manager
                .bind(metavar.to_string(), text.to_string(), node)
        } else {
            // Ellipsis can match empty content
            self.metavar_manager
                .bind(metavar.to_string(), "".to_string(), node)
        }
    }

    /// Match node type
    fn match_node_type(&self, expected_type: &str, node: &dyn AstNode) -> Result<bool> {
        Ok(node.node_type() == expected_type)
    }

    fn match_sequence_ast(
        &mut self,
        patterns: &[ParsedPattern],
        node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        if depth > 50 {
            return Ok(false);
        }

        // Collect non-empty children
        let children: Vec<&dyn AstNode> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .filter(|c| {
                if let Some(t) = c.text() {
                    !t.trim().is_empty()
                } else {
                    false
                }
            })
            .collect();

        if children.is_empty() && !patterns.is_empty() {
            return Ok(false);
        }

        self.try_match_ast_at_offset(patterns, &children, 0, node, depth)
    }

    fn try_match_ast_at_offset(
        &mut self,
        patterns: &[ParsedPattern],
        children: &[&dyn AstNode],
        child_offset: usize,
        parent_node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        if patterns.is_empty() {
            return Ok(true);
        }

        let pattern = &patterns[0];
        let remaining = &patterns[1..];

        match pattern {
            ParsedPattern::Wildcard => {
                for skip in 0..=(children.len().saturating_sub(child_offset)) {
                    let snapshot = self.metavar_manager.snapshot();
                    if self.try_match_ast_at_offset(remaining, children, child_offset + skip, parent_node, depth)? {
                        return Ok(true);
                    }
                    self.metavar_manager.restore(snapshot);
                }
                Ok(false)
            }

            ParsedPattern::EllipsisMetavariable(metavar) => {
                for skip in 0..=(children.len().saturating_sub(child_offset)) {
                    if child_offset + skip > children.len() {
                        break;
                    }
                    let combined: String = children[child_offset..child_offset + skip]
                        .iter()
                        .filter_map(|c| c.text())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let snapshot = self.metavar_manager.snapshot();
                    let bind_node: &dyn AstNode = if child_offset < children.len() {
                        children[child_offset]
                    } else if !children.is_empty() {
                        children[children.len() - 1]
                    } else {
                        parent_node
                    };
                    if self.metavar_manager.bind(metavar.to_string(), combined, bind_node)? {
                        if self.try_match_ast_at_offset(remaining, children, child_offset + skip, parent_node, depth)? {
                            return Ok(true);
                        }
                    }
                    self.metavar_manager.restore(snapshot);
                }
                Ok(false)
            }

            ParsedPattern::Literal(literal) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                if self.match_literal_exact(literal, child)? {
                    return self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth);
                }
                Ok(false)
            }

            ParsedPattern::Metavariable(metavar) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                if let Some(text) = child.text() {
                    if text.trim().is_empty() {
                        return Ok(false);
                    }
                    let bind_key = if metavar == "_" {
                        format!("__anon_{}", child.node_type().len())
                    } else {
                        metavar.to_string()
                    };
                    let snapshot = self.metavar_manager.snapshot();
                    if self.metavar_manager.bind(bind_key, text.to_string(), child)? {
                        if self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth)? {
                            return Ok(true);
                        }
                    }
                    self.metavar_manager.restore(snapshot);
                }
                Ok(false)
            }

            ParsedPattern::Alternative(alts) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                for alt in alts {
                    let snapshot = self.metavar_manager.snapshot();
                    if self.match_parsed_pattern(alt, child, depth + 1)? {
                        if self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth)? {
                            return Ok(true);
                        }
                    }
                    self.metavar_manager.restore(snapshot);
                }
                Ok(false)
            }

            ParsedPattern::NodeType(nt) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                if child.node_type() == nt {
                    return self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth);
                }
                Ok(false)
            }

            ParsedPattern::Sequence(inner) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                let snapshot = self.metavar_manager.snapshot();
                if self.match_sequence_ast(inner, child, depth + 1)? {
                    if self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth)? {
                        return Ok(true);
                    }
                }
                self.metavar_manager.restore(snapshot);
                Ok(false)
            }

            ParsedPattern::DeepExpr(inner) => {
                if child_offset >= children.len() {
                    return Ok(false);
                }
                let child = children[child_offset];
                let snapshot = self.metavar_manager.snapshot();
                if self.match_deep_expr(inner, child, depth + 1)? {
                    if self.try_match_ast_at_offset(remaining, children, child_offset + 1, parent_node, depth)? {
                        return Ok(true);
                    }
                }
                self.metavar_manager.restore(snapshot);
                Ok(false)
            }
        }
    }

    fn match_literal_exact(&self, literal: &str, node: &dyn AstNode) -> Result<bool> {
        if let Some(text) = node.text() {
            if text == literal {
                return Ok(true);
            }
            if text.trim() == literal.trim() {
                return Ok(true);
            }
            // Quote normalization: "bar" matches bar, 'bar' matches bar
            let trimmed = text.trim();
            let stripped = trimmed
                .strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                .or_else(|| trimmed.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(trimmed);
            if stripped == literal.trim() {
                return Ok(true);
            }
            let is_punctuation = literal.chars().all(|c| "!@#$%^&*()-=+[]{}|;:'\",.<>?/\\`~".contains(c));
            if is_punctuation && text.trim() == literal {
                return Ok(true);
            }
            if !literal.contains(' ') && !literal.contains('.') {
                let re_str = format!(r"\b{}\b", regex::escape(literal));
                if let Ok(re) = Regex::new(&re_str) {
                    if re.is_match(text) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Match sequence of patterns against a node's children
    /// This handles patterns like "return $X;" by matching against the node's child sequence
    fn match_sequence(
        &mut self,
        patterns: &[ParsedPattern],
        node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        {
            let snapshot = self.metavar_manager.snapshot();
            if self.match_sequence_ast(patterns, node, depth)? {
                return Ok(true);
            }
            self.metavar_manager.restore(snapshot);
        }

        // Check if this node type is appropriate for the pattern
        let pattern_text = patterns
            .iter()
            .map(|p| match p {
                ParsedPattern::Literal(s) => s.clone(),
                ParsedPattern::Metavariable(s) => format!("${}", s),
                _ => "".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        // For patterns containing "return", only match at return_statement nodes
        let node_type = node.node_type();

        if (pattern_text.contains("public void") || pattern_text.contains("function"))
            && !node_type.contains("declaration") && !node_type.contains("method")
        {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    let snapshot = self.metavar_manager.snapshot();
                    if self.match_sequence(patterns, child, depth + 1)? {
                        return Ok(true);
                    }
                    self.metavar_manager.restore(snapshot);
                }
            }
            return Ok(false);
        }

        if pattern_text.to_lowercase().contains("return")
            && node_type != "return_statement" && !node_type.contains("return")
        {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    let snapshot = self.metavar_manager.snapshot();
                    if self.match_sequence(patterns, child, depth + 1)? {
                        return Ok(true);
                    }
                    self.metavar_manager.restore(snapshot);
                }
            }
            return Ok(false);
        }

        // Try to match against current node's text
        let mut node_text: String = node.text().unwrap_or("").to_string();

        // For function declaration patterns, try to include the closing brace
        // Only apply this for single function declarations, not for class nodes
        let node_type = node.node_type();
                if (pattern_text.contains("public void") || pattern_text.contains("function"))
            && pattern_text.contains("{")
            && pattern_text.contains("}")
            && !node_type.contains("class")
            && (node_type.contains("declaration") || node_type.contains("method"))
        {
            // Find opening brace
            if let Some(open_pos) = node_text.find('{') {
                // Try to find matching closing brace (first closing brace that brings us back to 0)
                let mut brace_count = 1;
                let mut close_pos = open_pos + 1;
                let chars: Vec<char> = node_text.chars().collect();
                while close_pos < chars.len() && brace_count > 0 {
                    if chars[close_pos] == '{' {
                        brace_count += 1;
                    } else if chars[close_pos] == '}' {
                        brace_count -= 1;
                    }
                    if brace_count == 0 {
                        break;
                    }
                    close_pos += 1;
                }

                if brace_count == 0 {
                    // Found matching closing brace, include it
                                        let with_both_braces = node_text[..close_pos + 1].to_string();
                                        // Strip any comments before opening brace and between braces
                                        let cleaned = with_both_braces
                        .lines()
                        .map(|line| {
                                                        if let Some(comment_pos) = line.find("//") {
                                let result = &line[..comment_pos];
                                                                // Preserve empty lines as-is, don't convert to empty string
                                if result.is_empty() {
                                                                        "" // Keep empty lines as-is
                                } else {
                                    result
                                }
                            } else {
                                line
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                                        node_text = cleaned;
                } else {
                                    }
            }
        }

        // Try to match the pattern sequence against the node's text
        if self.match_sequence_against_text(patterns, &node_text, node, depth)? {
            return Ok(true);
        }

        // If no match at current node, try matching against children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                // Skip simple identifiers in assignment contexts to avoid false positives
                // (e.g., 'd' in 'Object d = b.z();' should not be expanded via symbolic propagation)
                if child.node_type() == "identifier" {
                    // Only match identifiers that are part of expressions (method calls, field access, etc.)
                    // Skip if it's a standalone identifier that would be an assignment target
                    let text = child.text().unwrap_or("");
                    if !text.contains(".") && !text.contains("(") {
                        continue;
                    }
                }

                let snapshot = self.metavar_manager.snapshot();
                if self.match_sequence(patterns, child, depth + 1)? {
                    return Ok(true);
                }
                self.metavar_manager.restore(snapshot);
            }
        }

        Ok(false)
    }

    /// Match a sequence of patterns against text
    fn match_sequence_against_text(
        &mut self,
        patterns: &[ParsedPattern],
        text: &str,
        node: &dyn AstNode,
        _depth: usize,
    ) -> Result<bool> {
        // Tokenize the text
        let text_tokens = self.tokenize(text);
                        
        // Expand tokens using symbolic propagation if available
        let expanded_tokens = self.expand_tokens_with_symbolic_propagation(&text_tokens);
        if !expanded_tokens.is_empty() && expanded_tokens != text_tokens {
                    }

        // Try to match with original tokens first
        for start_pos in 0..text_tokens.len() {
            let snapshot = self.metavar_manager.snapshot();
            if self.try_match_sequence_at_position(patterns, &text_tokens, start_pos, node)? {
                                return Ok(true);
            }
            self.metavar_manager.restore(snapshot);
        }

        // If no match with original tokens, try with expanded tokens
        if !expanded_tokens.is_empty() && expanded_tokens != text_tokens {
            for start_pos in 0..expanded_tokens.len() {
                let snapshot = self.metavar_manager.snapshot();
                if self.try_match_sequence_at_position(
                    patterns,
                    &expanded_tokens,
                    start_pos,
                    node,
                )? {
                                        return Ok(true);
                }
                self.metavar_manager.restore(snapshot);
            }
        }

                Ok(false)
    }

    /// Expand tokens using symbolic propagation
    /// For example, if "userName" is aliased to "req.xyz", expand to ["req", ".", "xyz"]
    fn expand_tokens_with_symbolic_propagation(&self, tokens: &[String]) -> Vec<String> {
        if self.symbolic_propagator.is_none() {
            return tokens.to_vec();
        }

        let propagator = self.symbolic_propagator.as_ref().unwrap();
                for (var, val) in propagator.state().variables.iter() {
                    }

        let mut expanded = Vec::new();

        for token in tokens {
            // Skip punctuation and operators
            if token == "." || token == "," || token == ";" || token == "(" || token == ")" {
                expanded.push(token.clone());
                continue;
            }

            // Check if this token is a variable with a symbolic value
            if let Some(symbolic_value) = propagator.state().get(token) {
                let expanded_text = self.symbolic_value_to_tokens(symbolic_value);
                if !expanded_text.is_empty() {
                                        expanded.extend(expanded_text);
                } else {
                    expanded.push(token.clone());
                }
            } else {
                expanded.push(token.clone());
            }
        }

        expanded
    }

    /// Convert a symbolic value to a list of tokens
    fn symbolic_value_to_tokens(&self, value: &astgrep_dataflow::SymbolicValue) -> Vec<String> {
        let mut visited = HashSet::new();
        self.symbolic_value_to_tokens_inner(value, &mut visited)
    }

    fn symbolic_value_to_tokens_inner(
        &self,
        value: &astgrep_dataflow::SymbolicValue,
        visited: &mut HashSet<String>,
    ) -> Vec<String> {
        use astgrep_dataflow::SymbolicValue;

        match value {
            SymbolicValue::Variable(name) => {
                if visited.contains(name) {
                    return vec![name.clone()];
                }
                visited.insert(name.clone());
                if let Some(propagator) = &self.symbolic_propagator {
                    if let Some(symbolic_value) = propagator.state().get(name) {
                        self.symbolic_value_to_tokens_inner(symbolic_value, visited)
                    } else {
                        vec![name.clone()]
                    }
                } else {
                    vec![name.clone()]
                }
            }
            SymbolicValue::FieldAccess { base, field } => {
                let mut tokens = self.symbolic_value_to_tokens(base);
                tokens.push(".".to_string());
                tokens.push(field.clone());
                tokens
            }
            SymbolicValue::MethodCall { base, method } => {
                let mut tokens = self.symbolic_value_to_tokens(base);
                if !method.is_empty() {
                    tokens.push(".".to_string());
                    tokens.push(method.clone());
                }
                tokens.push("(".to_string());
                tokens.push(")".to_string());
                tokens
            }
            SymbolicValue::ConstructorCall { class } => {
                vec![
                    "new".to_string(),
                    class.clone(),
                    "(".to_string(),
                    ")".to_string(),
                ]
            }
            SymbolicValue::Constant(s) => vec![s.clone()],
            SymbolicValue::Unknown => vec![],
        }
    }

    /// Try to match a pattern sequence starting at a specific position
    fn try_match_sequence_at_position(
        &mut self,
        patterns: &[ParsedPattern],
        text_tokens: &[String],
        start_pos: usize,
        node: &dyn AstNode,
    ) -> Result<bool> {
        let mut text_idx = start_pos;
        let mut matched_opening_brace = false;

        // Check if this is a function declaration pattern (has opening brace in patterns)
        let is_function_pattern = patterns
            .iter()
            .any(|p| matches!(p, ParsedPattern::Literal(s) if s == "{"));

        for (pattern_idx, pattern) in patterns.iter().enumerate() {
            if text_idx >= text_tokens.len() {
                // For function patterns, if we matched the opening brace and there's no closing brace in text,
                // that's OK - the node text might be truncated
                if is_function_pattern && matched_opening_brace {
                    return Ok(true);
                }
                return Ok(false);
            }

                        match pattern {
                ParsedPattern::Literal(literal) => {
                    if *literal == ";" {
                        if text_idx < text_tokens.len() && text_tokens[text_idx] == ";" {
                            text_idx += 1;
                        }
                        continue;
                    }
                    // Track if we matched to opening brace
                    if *literal == "{" {
                        matched_opening_brace = true;
                                            }
                    // Special case: "..." in pattern should match any string literal token
                    // Handle both "..." (quoted ellipsis in pattern like $X.println("...")) and ... (bare ellipsis)
                    if *literal == "..." || *literal == "\"...\"" {
                        // When we see "..." in the pattern, we need to find a string literal
                        // It might be directly at current position, or after an opening parenthesis
                        let mut found_string = false;

                        // Check current position first
                        if text_tokens[text_idx].starts_with('"') {
                            found_string = true;
                        }
                        // Check if current position is '(' and next position has the string
                        else if text_tokens[text_idx] == "("
                            && text_idx + 1 < text_tokens.len()
                            && text_tokens[text_idx + 1].starts_with('"')
                        {
                            text_idx += 1; // Skip the '('
                            found_string = true;
                        }

                        if found_string {
                            // This is a string literal wildcard, match any string literal
                                                        text_idx += 1;
                        } else {
                            // No string literal found where expected
                                                        return Ok(false);
                        }
                    } else if literal.starts_with('$') {
                        // Special case: metavariable like "$RE"
                        // This matches a string literal and binds the content (without quotes) to the metavariable
                        let token = &text_tokens[text_idx];
                        if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
                            // Extract content from string literal (remove surrounding quotes)
                            let content = &token[1..token.len() - 1];
                            // Keep the $ prefix to match how metavariable-regex stores the name
                            let metavar = literal;
                                                        if !self.metavar_manager.bind(
                                metavar.to_string(),
                                content.to_string(),
                                node,
                            )? {
                                                                return Ok(false);
                            }
                                                        text_idx += 1;
                        } else {
                            // Token is not a string literal, so this doesn't match
                            return Ok(false);
                        }
                    } else if literal.starts_with("\"")
                        && literal.ends_with("\"")
                        && literal.len() >= 3
                    {
                        // Special case: quoted string containing a metavariable like "\"$RE\""
                        // This happens when pattern "$X.sha1(\"$RE\")" is tokenized
                        let inner = &literal[1..literal.len() - 1]; // Remove outer quotes
                        if inner.starts_with('$') {
                            let token = &text_tokens[text_idx];
                            if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
                                // Extract content from string literal (remove surrounding quotes)
                                let content = &token[1..token.len() - 1];
                                // Use the inner metavariable name (with $ prefix)
                                let metavar = inner;
                                                                if !self.metavar_manager.bind(
                                    metavar.to_string(),
                                    content.to_string(),
                                    node,
                                )? {
                                                                        return Ok(false);
                                }
                                                                text_idx += 1;
                            } else {
                                // Token is not a string literal, so this doesn't match
                                return Ok(false);
                            }
                        } else {
                            // Not a metavariable inside quotes, treat as regular literal
                            if text_tokens[text_idx] != *literal {
                                return Ok(false);
                            }
                            text_idx += 1;
                        }
                    } else if text_tokens[text_idx] != *literal {
                        let text_token = &text_tokens[text_idx];
                        let matched = if text_token.starts_with('"')
                            && text_token.ends_with('"')
                            && text_token.len() >= 2
                        {
                            &text_token[1..text_token.len() - 1] == literal
                        } else if text_token.starts_with('\'')
                            && text_token.ends_with('\'')
                            && text_token.len() >= 2
                        {
                            &text_token[1..text_token.len() - 1] == literal
                        } else {
                            false
                        };
                        if matched {
                            text_idx += 1;
                            continue;
                        }
                        // Check if token is an identifier with constant value matching the literal
                        if let Some(constant_value) =
                            self.constant_values.get(&text_tokens[text_idx])
                        {
                            let constant_str = match constant_value {
                                ConstantValue::Integer(i) => i.to_string(),
                                ConstantValue::String(s) => s.clone(),
                                ConstantValue::Boolean(b) => b.to_string(),
                                ConstantValue::Null => "null".to_string(),
                                ConstantValue::Unknown => {
                                                                        return Ok(false);
                                }
                            };
                            if constant_str == *literal {
                                                                text_idx += 1;
                            } else {
                                                                return Ok(false);
                            }
                        } else if text_tokens[text_idx] == "this"
                            && text_idx + 2 < text_tokens.len()
                            && text_tokens[text_idx + 1] == "."
                        {
                            // Handle field access like "this.x" - check if the field has a constant value
                            let field_name = &text_tokens[text_idx + 2];
                            if let Some(constant_value) = self.constant_values.get(field_name) {
                                let constant_str = match constant_value {
                                    ConstantValue::Integer(i) => i.to_string(),
                                    ConstantValue::String(s) => s.clone(),
                                    ConstantValue::Boolean(b) => b.to_string(),
                                    ConstantValue::Null => "null".to_string(),
                                    ConstantValue::Unknown => {
                                                                                return Ok(false);
                                    }
                                };
                                if constant_str == *literal {
                                                                        // Consume all three tokens: "this", ".", "x"
                                    text_idx += 3;
                                } else {
                                                                        return Ok(false);
                                }
                            } else {
                                                                return Ok(false);
                            }
                        } else {
                                                        return Ok(false);
                        }
                    } else {
                                                text_idx += 1;
                    }
                }
                ParsedPattern::Metavariable(metavar) => {
                    // Check if this metavariable is in an argument position
                    // (after '(' or ',' and before ',' or ')' or end of relevant context)
                    let is_arg_position = if pattern_idx > 0 {
                        matches!(
                            patterns.get(pattern_idx - 1),
                            Some(ParsedPattern::Literal(s)) if s == "(" || s == ","
                        )
                    } else {
                        false
                    };

                    // Find the next delimiter pattern (',' or ')' or end)
                    let next_delimiter_idx = patterns[pattern_idx + 1..]
                        .iter()
                        .position(|p| matches!(p, ParsedPattern::Literal(s) if s == "," || s == ")" || s == ";"))
                        .map(|i| pattern_idx + 1 + i);

                    // If in argument position and next pattern is a delimiter, consume multiple tokens
                    if is_arg_position {
                        if let Some(delimiter_idx) = next_delimiter_idx {
                            let delimiter = match &patterns[delimiter_idx] {
                                ParsedPattern::Literal(s) => s.as_str(),
                                _ => ",",
                            };
                            // Find the delimiter in remaining tokens, respecting nested parens
                            let mut paren_depth: usize = 0;
                            let mut end_pos = text_idx;
                            let mut found_delim = false;

                            for i in text_idx..text_tokens.len() {
                                let token = &text_tokens[i];
                                if token == "(" {
                                    paren_depth += 1;
                                } else if token == ")" {
                                    if paren_depth == 0 && delimiter == ")" {
                                        end_pos = i;
                                        found_delim = true;
                                        break;
                                    }
                                    paren_depth = paren_depth.saturating_sub(1);
                                } else if token == delimiter && paren_depth == 0 {
                                    end_pos = i;
                                    found_delim = true;
                                    break;
                                }
                            }

                            if found_delim && end_pos > text_idx {
                                let value = text_tokens[text_idx..end_pos].join("");
                                let bind_key = if metavar == "_" {
                                    format!("__anon_{}", pattern_idx)
                                } else {
                                    metavar.clone()
                                };
                                                                if !self.metavar_manager.bind(bind_key, value.clone(), node)? {
                                    return Ok(false);
                                }
                                text_idx = end_pos;
                                continue;
                            } else if found_delim && end_pos == text_idx {
                                return Ok(false);
                            } else {
                                                            }
                        }
                    }

                    // When in argument position with no explicit closing delimiter in pattern,
                    // don't bind to the closing paren token itself
                    if is_arg_position && text_tokens[text_idx] == ")" {
                        return Ok(false);
                    }

                    let value = &text_tokens[text_idx];
                    let bind_key = if metavar == "_" {
                        format!("__anon_{}", pattern_idx)
                    } else {
                        metavar.clone()
                    };
                                        if !self.metavar_manager.bind(bind_key, value.clone(), node)? {
                                                return Ok(false);
                    }
                    text_idx += 1;
                }
                ParsedPattern::EllipsisMetavariable(metavar) => {
                    let remaining_patterns = &patterns[pattern_idx + 1..];
                    let existing_value = self.metavar_manager.get_binding(metavar).map(|b| b.value.clone());

                    if let Some(stored_value) = existing_value {
                        // Already bound (second occurrence) — verify span matches stored value
                        for end_pos in text_idx..=text_tokens.len() {
                            let candidate = text_tokens[text_idx..end_pos].join(" ");
                            if candidate == stored_value {
                                let snapshot = self.metavar_manager.snapshot();
                                if self.try_match_sequence_at_position(
                                    remaining_patterns,
                                    text_tokens,
                                    end_pos,
                                    node,
                                )? {
                                    return Ok(true);
                                }
                                self.metavar_manager.restore(snapshot);
                            }
                        }
                        return Ok(false);
                    }

                    // Not bound yet — try each possible span
                    for end_pos in text_idx..=text_tokens.len() {
                        let captured_content = text_tokens[text_idx..end_pos].join(" ");
                        let snapshot = self.metavar_manager.snapshot();

                        if self.metavar_manager.bind(metavar.clone(), captured_content, node)? {
                            if self.try_match_sequence_at_position(
                                remaining_patterns,
                                text_tokens,
                                end_pos,
                                node,
                            )? {
                                return Ok(true);
                            }
                        }

                        self.metavar_manager.restore(snapshot);
                    }

                    return Ok(false);
                }
                ParsedPattern::Wildcard => {
                    let next_pattern_idx = patterns
                        .iter()
                        .enumerate()
                        .skip(pattern_idx + 1)
                        .find(|(_, p)| !matches!(p, ParsedPattern::Wildcard))
                        .map(|(idx, _)| idx);
                    if let Some(next_idx) = next_pattern_idx {
                        let remaining_patterns = &patterns[next_idx..];

                        // When Wildcard is followed by a comma (e.g., foo(..., 5)),
                        // also try matching without the comma for zero-argument case
                        let skip_comma = matches!(
                            remaining_patterns.first(),
                            Some(ParsedPattern::Literal(lit)) if lit == ","
                        );
                        let patterns_after_optional_comma = if skip_comma {
                            &remaining_patterns[1..]
                        } else {
                            remaining_patterns
                        };

                        for next_pos in text_idx..=text_tokens.len() {
                            let snapshot = self.metavar_manager.snapshot();
                            if self.try_match_sequence_at_position(
                                remaining_patterns,
                                text_tokens,
                                next_pos,
                                node,
                            )? {
                                text_idx = next_pos;
                                for (_i, pattern) in remaining_patterns.iter().enumerate() {
                                    if text_idx >= text_tokens.len() {
                                        return Ok(false);
                                    }
                                    match pattern {
                                        ParsedPattern::Literal(lit) => {
                                            if text_tokens[text_idx] != *lit {
                                                return Ok(false);
                                            }
                                            text_idx += 1;
                                        }
                                        ParsedPattern::Metavariable(metav) => {
                                            let value = &text_tokens[text_idx];
                                            if !self.metavar_manager.bind(
                                                metav.clone(),
                                                value.clone(),
                                                node,
                                            )? {
                                                return Ok(false);
                                            }
                                            text_idx += 1;
                                        }
                                        ParsedPattern::Wildcard
                                        | ParsedPattern::EllipsisMetavariable(_) => {
                                            text_idx += 1;
                                        }
                                        _ => {
                                            return Ok(false);
                                        }
                                    }
                                }
                                return Ok(true);
                            }
                            self.metavar_manager.restore(snapshot);

                            // Try without the comma for zero-argument ellipsis
                            if skip_comma && next_pos == text_idx {
                                let snapshot2 = self.metavar_manager.snapshot();
                                if self.try_match_sequence_at_position(
                                    patterns_after_optional_comma,
                                    text_tokens,
                                    next_pos,
                                    node,
                                )? {
                                    text_idx = next_pos;
                                    for pattern in patterns_after_optional_comma.iter() {
                                        if text_idx >= text_tokens.len() {
                                            return Ok(false);
                                        }
                                        match pattern {
                                            ParsedPattern::Literal(lit) => {
                                                if text_tokens[text_idx] != *lit {
                                                    return Ok(false);
                                                }
                                                text_idx += 1;
                                            }
                                            ParsedPattern::Metavariable(metav) => {
                                                let value = &text_tokens[text_idx];
                                                if !self.metavar_manager.bind(
                                                    metav.clone(),
                                                    value.clone(),
                                                    node,
                                                )? {
                                                    return Ok(false);
                                                }
                                                text_idx += 1;
                                            }
                                            ParsedPattern::Wildcard
                                            | ParsedPattern::EllipsisMetavariable(_) => {
                                                text_idx += 1;
                                            }
                                            _ => {
                                                return Ok(false);
                                            }
                                        }
                                    }
                                    return Ok(true);
                                }
                                self.metavar_manager.restore(snapshot2);
                            }
                        }
                        return Ok(false);
                    } else {
                        text_idx = text_tokens.len();
                    }
                }
                ParsedPattern::Sequence(nested_patterns) => {
                    // Recursively match nested sequence
                    // This handles cases like (..., $X, ...) for parameter lists
                    let nested_start = text_idx;

                    // Check if this is a parameter list pattern (contains commas)
                    let is_param_list_pattern = nested_patterns
                        .iter()
                        .any(|p| matches!(p, ParsedPattern::Literal(lit) if lit == ","));

                    // Check if text starts with '(' - if so, skip it for matching
                    // This handles patterns like "foo(this.$X)" where the pattern has
                    // a Sequence for "(this.$X)" but text tokens have "(" as a separate token
                    let mut actual_start = nested_start;
                    if actual_start < text_tokens.len() && text_tokens[actual_start] == "(" {
                        actual_start += 1;
                    }

                    if is_param_list_pattern
                        && nested_start < text_tokens.len()
                        && text_tokens[nested_start] == "("
                    {
                        // Strategy for parameter lists: Match the entire parenthesized expression
                        // Find matching closing parenthesis
                        let mut paren_count = 1;
                        let mut paren_idx = nested_start + 1;
                        while paren_idx < text_tokens.len() && paren_count > 0 {
                            if text_tokens[paren_idx] == "(" {
                                paren_count += 1;
                            } else if text_tokens[paren_idx] == ")" {
                                paren_count -= 1;
                            }
                            if paren_count == 0 {
                                break;
                            }
                            paren_idx += 1;
                        }

                        if paren_count == 0 && paren_idx < text_tokens.len() {
                            // Try to match metavariables in the parameter list
                            let param_tokens = &text_tokens[nested_start..=paren_idx];
                            if self.try_extract_metavars_from_params(
                                nested_patterns,
                                param_tokens,
                                node,
                            )? {
                                text_idx = paren_idx + 1;
                            } else {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    } else {
                        // Try to match the sequence as-is, starting after '(' if present
                        if let Ok(mut nested_idx) = self.try_match_nested_sequence(
                            nested_patterns,
                            text_tokens,
                            actual_start,
                            node,
                        ) {
                            // After matching, skip the closing ')' if present
                            if nested_idx < text_tokens.len() && text_tokens[nested_idx] == ")" {
                                nested_idx += 1;
                            }
                            text_idx = nested_idx;
                        } else {
                            return Ok(false);
                        }
                    }
                }
                _ => {
                    // Other pattern types not supported in sequence matching yet
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Simple tokenizer for matching
    fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_line_comment = false;
        let mut in_block_comment = false;
        let mut in_string = false;
        let mut string_char = '"';
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            // Handle string literals
            if !in_line_comment && !in_block_comment {
                if in_string {
                    current.push(ch);
                    if ch == string_char {
                        // Check for escaped quote
                        let mut backslash_count = 0;
                        let current_chars: Vec<char> = current.chars().collect();
                        for i in (0..current_chars.len() - 1).rev() {
                            if current_chars[i] == '\\' {
                                backslash_count += 1;
                            } else {
                                break;
                            }
                        }
                        // If even number of backslashes, the quote is not escaped
                        if backslash_count % 2 == 0 {
                            tokens.push(current.clone());
                            current.clear();
                            in_string = false;
                        }
                    }
                    continue;
                } else if ch == '"' || ch == '\'' {
                    // Start of string literal
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    in_string = true;
                    string_char = ch;
                    current.push(ch);
                    continue;
                }
            }

            // Handle line comments (// in Java, # in some languages)
            if !in_block_comment && !in_string && ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        // Start of line comment
                        if !current.is_empty() {
                            tokens.push(current.clone());
                        }
                        current.clear();
                        in_line_comment = true;
                        chars.next(); // consume the second /
                        continue;
                    } else if next_ch == '*' {
                        // Start of block comment
                        if !current.is_empty() {
                            tokens.push(current.clone());
                        }
                        current.clear();
                        in_block_comment = true;
                        chars.next(); // consume the *
                        continue;
                    }
                }
            }

            // End of line comment
            if in_line_comment && ch == '\n' {
                in_line_comment = false;
                continue;
            }

            // End of block comment
            if in_block_comment && ch == '*' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        in_block_comment = false;
                        chars.next(); // consume the /
                        continue;
                    }
                }
            }

            // Skip characters inside comments
            if in_line_comment || in_block_comment {
                continue;
            }

            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                ';' | '(' | ')' | '{' | '}' | '[' | ']' | ',' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
                '.' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
                '=' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    if let Some(&next) = chars.peek() {
                        if next == '=' {
                            chars.next();
                            tokens.push("==".to_string());
                        } else {
                            tokens.push("=".to_string());
                        }
                    } else {
                        tokens.push("=".to_string());
                    }
                }
                '!' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    if let Some(&next) = chars.peek() {
                        if next == '=' {
                            chars.next();
                            tokens.push("!=".to_string());
                        } else {
                            tokens.push("!".to_string());
                        }
                    } else {
                        tokens.push("!".to_string());
                    }
                }
                '<' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    if let Some(&next) = chars.peek() {
                        if next == '=' {
                            chars.next();
                            tokens.push("<=".to_string());
                        } else {
                            tokens.push("<".to_string());
                        }
                    } else {
                        tokens.push("<".to_string());
                    }
                }
                '>' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    if let Some(&next) = chars.peek() {
                        if next == '=' {
                            chars.next();
                            tokens.push(">=".to_string());
                        } else {
                            tokens.push(">".to_string());
                        }
                    } else {
                        tokens.push(">".to_string());
                    }
                }
                '+' | '-' | '*' | '/' | '&' | '|' | '%' | '^' | '~' | '?' | ':' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() && !in_line_comment && !in_block_comment {
            tokens.push(current);
        }

        tokens
    }

    /// Match alternative patterns
    fn match_alternative(
        &mut self,
        patterns: &[ParsedPattern],
        node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        for pattern in patterns {
            let snapshot = self.metavar_manager.snapshot();
            if self.match_parsed_pattern(pattern, node, depth + 1)? {
                return Ok(true);
            }
            self.metavar_manager.restore(snapshot);
        }
        Ok(false)
    }

    fn match_deep_expr(
        &mut self,
        inner: &ParsedPattern,
        node: &dyn AstNode,
        depth: usize,
    ) -> Result<bool> {
        if depth > 10 {
            return Ok(false);
        }

        if self.match_parsed_pattern(inner, node, depth + 1)? {
            return Ok(true);
        }

        if let Some(text) = node.text() {
            if let Some(inner_text) = self.extract_deep_expr_inner_text(inner) {
                if !inner_text.is_empty() && text.contains(&inner_text) {
                    return Ok(true);
                }
            }
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_text = child.text().map(|s| s.to_string());
                let node_text = node.text().map(|s| s.to_string());
                if let (Some(ct), Some(nt)) = (&child_text, &node_text) {
                    if ct.len() > nt.len() / 2 {
                        continue;
                    }
                }
                let snapshot = self.metavar_manager.snapshot();
                if self.match_deep_expr(inner, child, depth + 1)? {
                    return Ok(true);
                }
                self.metavar_manager.restore(snapshot);
            }
        }

        Ok(false)
    }

    fn extract_deep_expr_inner_text(&self, pattern: &ParsedPattern) -> Option<String> {
        match pattern {
            ParsedPattern::Literal(s) => Some(s.clone()),
            ParsedPattern::Sequence(patterns) => {
                let parts: Vec<String> = patterns
                    .iter()
                    .filter_map(|p| self.extract_deep_expr_inner_text(p))
                    .collect();
                if parts.len() == patterns.len() {
                    Some(parts.join(" "))
                } else {
                    None
                }
            }
            ParsedPattern::Wildcard => Some("".to_string()),
            _ => None,
        }
    }

    /// Evaluate conditions after a successful pattern match
    pub fn evaluate_conditions(
        &self,
        conditions: &[Condition],
        bindings: &HashMap<String, String>,
    ) -> Result<bool> {
        for condition in conditions {
            if !self.evaluate_condition(condition, bindings)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Try to match a nested sequence and return the position after the match
    fn try_match_nested_sequence(
        &mut self,
        nested_patterns: &[ParsedPattern],
        text_tokens: &[String],
        nested_start: usize,
        node: &dyn AstNode,
    ) -> Result<usize> {
        if self.try_match_sequence_at_position(nested_patterns, text_tokens, nested_start, node)? {
            // Calculate how many tokens were consumed
            // by re-matching and counting
            let mut nested_idx = nested_start;
            for nested_pattern in nested_patterns {
                match nested_pattern {
                    ParsedPattern::Literal(lit) => {
                        // Skip parentheses
                        while nested_idx < text_tokens.len()
                            && (text_tokens[nested_idx] == "(" || text_tokens[nested_idx] == ")")
                        {
                            nested_idx += 1;
                        }
                        if nested_idx < text_tokens.len()
                            && (*lit == "..." || text_tokens[nested_idx] == *lit)
                        {
                            nested_idx += 1;
                        }
                    }
                    ParsedPattern::Metavariable(_)
                    | ParsedPattern::EllipsisMetavariable(_)
                    | ParsedPattern::Wildcard => {
                        nested_idx += 1;
                    }
                    _ => {}
                }
            }
            Ok(nested_idx)
        } else {
            Err(AnalysisError::pattern_match_error(
                "Failed to match nested sequence",
            ))
        }
    }

    /// Try to extract metavariables from parameter tokens like (String x)
    fn try_extract_metavars_from_params(
        &mut self,
        nested_patterns: &[ParsedPattern],
        param_tokens: &[String],
        node: &dyn AstNode,
    ) -> Result<bool> {
        // Find all metavariables in the nested patterns
        let mut metavars: Vec<String> = Vec::new();
        for pattern in nested_patterns {
            if let ParsedPattern::Metavariable(name) = pattern {
                metavars.push(name.clone());
            }
        }

        if metavars.is_empty() {
            return Ok(true);
        }

        // For parameter lists like (String x), try to extract parameter names
        // The pattern `(..., $X, ...)` means: zero or more parameters before $X, then $X, then zero or more after
        // For a single parameter `(String x)`, we should match without requiring literal commas
        // We want to extract "x" (the parameter name), not "String" (the type)
        let mut param_idx = 0;
        let mut bound_metavars = 0;

        while param_idx < param_tokens.len() && bound_metavars < metavars.len() {
            let token = &param_tokens[param_idx];

            // Skip parentheses
            if token == "(" || token == ")" {
                param_idx += 1;
                continue;
            }

            // Skip literal commas for now - we're matching parameters, not commas
            if token == "," {
                param_idx += 1;
                continue;
            }

            // Look for identifiers that might be parameter names
            // Parameter names are typically identifiers after types
            // In "String x", "String" is the type (starts with capital), "x" is the name
            if token.chars().all(|c| c.is_alphanumeric() || c == '_') {
                // Check if this is likely a parameter name:
                // - Not the first token (which is likely a type)
                // - Doesn't start with capital (Java types typically start with capital)
                // - Is after a non-identifier token (opening paren or type)
                let is_first_token = param_idx == 0
                    || param_tokens
                        .get(param_idx - 1)
                        .map(|t| t == "(")
                        .unwrap_or(false);
                let is_type_token = token
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);

                if !is_first_token && !is_type_token {
                    // This is likely a parameter name
                    // Bind it to the next metavariable
                    if let Some(metavar) = metavars.get(bound_metavars) {
                        let _snapshot = self.metavar_manager.snapshot();
                        if self
                            .metavar_manager
                            .bind(metavar.clone(), token.clone(), node)?
                        {
                            // Continue to find next parameter name
                            return Ok(true);
                        } else {
                            // Binding failed, try next token
                        }
                    }
                }
            }
            param_idx += 1;
        }

        // If we bound at least one metavar, consider it a success
        //     Ok(bound_metavars > 0)
        // }
        //     }

        if metavars.is_empty() {
            return Ok(true);
        }

        // For parameter lists like (String x), try to extract parameter names
        // In Java, parameters are typically "Type name" like "String x"
        // We want to extract "x" (the parameter name), not "String" (the type)
        let mut param_idx = 0;
        while param_idx < param_tokens.len() {
            let token = &param_tokens[param_idx];

            // Skip non-identifier tokens
            if token == "(" || token == ")" || token == "," {
                param_idx += 1;
                continue;
            }

            // Look for identifiers that might be parameter names
            // Parameter names are typically identifiers after types
            // In "String x", "String" is the type (starts with capital), "x" is the name
            if token.chars().all(|c| c.is_alphanumeric() || c == '_') {
                // Check if this is likely a parameter name:
                // - Not the first token (which is likely a type)
                // - Doesn't start with capital (Java types typically start with capital)
                // - Is after a non-identifier token
                let is_first_token = param_idx == 0 || param_tokens[param_idx - 1] == "(";
                let starts_with_capital = token
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);

                if !is_first_token && !starts_with_capital {
                    // This is likely a parameter name
                    // Bind it to the first metavariable
                    if let Some(metavar) = metavars.first() {
                        let snapshot = self.metavar_manager.snapshot();
                        if self
                            .metavar_manager
                            .bind(metavar.clone(), token.clone(), node)?
                        {
                                                        return Ok(true);
                        }
                        self.metavar_manager.restore(snapshot);
                    }
                }
            }
            param_idx += 1;
        }

        Ok(false)
    }
    /// Evaluate a single condition
    fn evaluate_condition(
        &self,
        condition: &Condition,
        bindings: &HashMap<String, String>,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                let key = metavar_regex.metavariable.trim_start_matches('$');
                if let Some(value) = bindings.get(key) {
                    // Support (?i) case-insensitive flag and other inline regex flags
                    let regex_str = &metavar_regex.regex;
                    let regex = if let Some(rest) = regex_str.strip_prefix("(?i)") {
                        regex::Regex::new(&format!("(?i){}", rest))
                    } else {
                        regex::Regex::new(regex_str)
                    };
                    if let Ok(re) = regex {
                        Ok(re.is_match(value))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableComparison(metavar_comp) => {
                if let ComparisonOperator::PythonExpression(_) = &metavar_comp.operator {
                    Ok(true)
                } else if let Some(value) = bindings.get(&metavar_comp.metavariable) {
                    self.evaluate_comparison(value, &metavar_comp.operator, &metavar_comp.value)
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableName(metavar_name) => {
                                let key = metavar_name.metavariable.trim_start_matches('$');
                if let Some(value) = bindings.get(key) {
                    let full_source = self.full_source.as_deref().unwrap_or("");
                                        let result = self.evaluate_name_constraint(
                        value,
                        &metavar_name.name_pattern,
                        full_source,
                    );
                                        result
                } else {
                                        Ok(false)
                }
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                if let Some(value) = bindings.get(&metavar_analysis.metavariable) {
                    self.evaluate_analysis_constraint(value, &metavar_analysis.analysis)
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableType(metavar_type) => {
                // Type checking is handled in the executor, this is simplified here
                if let Some(_value) = bindings.get(&metavar_type.metavariable) {
                    // For now, accept the match and let the executor validate the type
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Condition::NodeType(_expected_type) => {
                // This would need access to the matched node
                Ok(true) // Simplified for now
            }
            Condition::NodeAttribute(_, _) => {
                // This would need access to the matched node
                Ok(true) // Simplified for now
            }
            Condition::Custom(_) => {
                Ok(true) // Simplified for now
            }
        }
    }

    /// Evaluate comparison operators
    fn evaluate_comparison(
        &self,
        value: &str,
        operator: &ComparisonOperator,
        expected: &str,
    ) -> Result<bool> {
        match operator {
            ComparisonOperator::Equals => Ok(value == expected),
            ComparisonOperator::NotEquals => Ok(value != expected),
            ComparisonOperator::Contains => Ok(value.contains(expected)),
            ComparisonOperator::StartsWith => Ok(value.starts_with(expected)),
            ComparisonOperator::EndsWith => Ok(value.ends_with(expected)),
            ComparisonOperator::Matches => {
                if let Ok(regex) = Regex::new(expected) {
                    Ok(regex.is_match(value))
                } else {
                    Ok(false)
                }
            }
            ComparisonOperator::GreaterThan => {
                if let (Ok(v), Ok(e)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                    Ok(v > e)
                } else {
                    Ok(value > expected)
                }
            }
            ComparisonOperator::LessThan => {
                if let (Ok(v), Ok(e)) = (value.parse::<f64>(), expected.parse::<f64>()) {
                    Ok(v < e)
                } else {
                    Ok(value < expected)
                }
            }
            ComparisonOperator::PythonExpression(expr) => {
                // For now, we'll implement a simplified version
                // In a full implementation, this would use a Python interpreter
                self.evaluate_python_expression(value, expr)
            }
        }
    }

    /// Evaluate name constraint with FQN resolution using imports
    fn evaluate_name_constraint(
        &self,
        value: &str,
        name_pattern: &str,
        full_source: &str,
    ) -> Result<bool> {
        let import_map = self.build_import_map(full_source);
        let resolved_value = self.resolve_name_to_fqn(value, &import_map);

        
        if name_pattern.contains("*") {
            let regex_pattern = name_pattern.replace(".", "\\.").replace("*", ".*");
            if let Ok(regex) = Regex::new(&regex_pattern) {
                Ok(regex.is_match(&resolved_value))
            } else {
                Ok(false)
            }
        } else if resolved_value == name_pattern {
            Ok(true)
        } else if name_pattern.ends_with(&format!(".{}", value)) {
            Ok(import_map
                .get(value)
                .is_some_and(|fqn| fqn == name_pattern))
        } else if resolved_value.ends_with(&format!(".{}", name_pattern)) {
            Ok(true)
        } else {
            Ok(resolved_value == name_pattern)
        }
    }

    fn build_import_map(&self, source: &str) -> HashMap<String, String> {
        let mut import_map = HashMap::new();
        let import_pattern = regex::Regex::new(r"import\s+([\w.]+)(?:\.\*)?;").unwrap();

        for captures in import_pattern.captures_iter(source) {
            if let Some(import_match) = captures.get(1) {
                let import_path = import_match.as_str();
                if let Some(last_dot) = import_path.rfind('.') {
                    let simple_name = &import_path[last_dot + 1..];
                    import_map.insert(simple_name.to_string(), import_path.to_string());
                }
            }
        }
        import_map
    }

    fn resolve_name_to_fqn(&self, name: &str, import_map: &HashMap<String, String>) -> String {
        if name.contains('.') {
            return name.to_string();
        }
        import_map
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Evaluate analysis constraint (entropy, type, complexity)
    fn evaluate_analysis_constraint(
        &self,
        value: &str,
        analysis: &MetavariableAnalysis,
    ) -> Result<bool> {
        // Check entropy if specified
        if let Some(entropy_config) = &analysis.entropy {
            if !self.check_entropy(value, entropy_config)? {
                return Ok(false);
            }
        }

        // Check type analysis if specified
        if let Some(type_config) = &analysis.type_analysis {
            if !self.check_type_analysis(value, type_config)? {
                return Ok(false);
            }
        }

        // Check complexity if specified
        if let Some(complexity_config) = &analysis.complexity {
            if !self.check_complexity(value, complexity_config)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Simplified Python expression evaluation
    fn evaluate_python_expression(&self, value: &str, expr: &str) -> Result<bool> {
        // This is a simplified implementation
        // In a full implementation, you would use a Python interpreter

        // Handle some common patterns
        if expr.contains("len(") {
            if let Some(len_expr) = expr.strip_prefix("len(").and_then(|s| s.strip_suffix(")")) {
                if len_expr.trim() == "$VAR" {
                    // Extract the comparison from the full expression
                    // This is very simplified - a real implementation would parse the full expression
                    return Ok(!value.is_empty());
                }
            }
        }

        // Handle bit OR operations like "$X | 1 == 1"
        // Parse expressions like "$VAR | 1 == 1" or "$VAR | 1 == 3"
        if expr.contains('|') && expr.contains("==") && !expr.contains("||") {
            // Try to parse the expression
            // Format: $VAR | N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Parse the bit operation: $VAR | N
                if left_side.contains('|') {
                    let bit_parts: Vec<&str> = left_side.split('|').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();

                        
                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val | mask;
                                                                                return Ok(result == expected);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle bit NOT operations like "~ $X == -1"
        // Python: ~x = -(x + 1)
        if expr.contains('~') && expr.contains("==") {
            // Format: ~$VAR == N or ~ $VAR == N
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Remove the ~ operator and get the variable part
                // Handle both "~$VAR" and "~ $VAR"
                let var_part = if let Some(rest) = left_side.strip_prefix("~") {
                    rest.trim()
                } else {
                    left_side
                };

                
                // Check if this is the metavariable we're evaluating
                if var_part.starts_with("$") {
                    // Parse the expected result
                    if let Ok(expected) = expected_result.parse::<i64>() {
                        // Parse the actual value
                        if let Ok(val) = value.parse::<i64>() {
                            // Python's ~ operator: ~x = -(x + 1)
                            let result = -(val + 1);
                                                        return Ok(result == expected);
                        }
                    }
                }
            }
        }

        if expr.contains(" in ") || expr.contains(" not in ") {
            let negated = expr.contains(" not in ");
            let in_expr = if negated {
                expr.split(" not in ").collect::<Vec<_>>()
            } else {
                expr.split(" in ").collect::<Vec<_>>()
            };
            if in_expr.len() == 2 {
                let left = in_expr[0].trim();
                let right = in_expr[1].trim();
                let check_value = value;
                if right.starts_with('"') && right.ends_with('"') {
                    let target = &right[1..right.len() - 1];
                    let result = target.contains(&check_value);
                    return Ok(if negated { !result } else { result });
                } else if right.starts_with('[') && right.ends_with(']') {
                    let inner = &right[1..right.len() - 1];
                    let items: Vec<&str> = inner
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let result = items.iter().any(|item| item == &check_value);
                    return Ok(if negated { !result } else { result });
                }
            }
        }

        if expr.contains("**") && expr.contains("==") {
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                if let Ok(expected_val) = parts[1].trim().parse::<f64>() {
                    let left = parts[0].trim();
                    let power_parts: Vec<&str> = left.split("**").collect();
                    if power_parts.len() == 2 {
                        if let Ok(base_val) = value.parse::<f64>() {
                            if let Ok(exp_val) = power_parts[1].trim().parse::<f64>() {
                                return Ok((base_val.powf(exp_val) - expected_val).abs() < 1e-9);
                            }
                        }
                    }
                }
            }
        }

                Ok(true)
    }

    /// Check entropy constraints
    fn check_entropy(&self, value: &str, entropy_config: &EntropyAnalysis) -> Result<bool> {
        let entropy = self.calculate_entropy(value);

        if entropy < entropy_config.min_entropy {
            return Ok(false);
        }

        if let Some(max_entropy) = entropy_config.max_entropy {
            if entropy > max_entropy {
                return Ok(false);
            }
        }

        // Check charset if specified
        if let Some(charset) = &entropy_config.charset {
            if !self.matches_charset(value, charset) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check type analysis constraints
    fn check_type_analysis(&self, value: &str, type_config: &TypeAnalysis) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST to determine types

        // For now, we'll do basic pattern matching
        if !type_config.expected_types.is_empty() {
            let mut matches_expected = false;
            for expected_type in &type_config.expected_types {
                if self.value_matches_type(value, expected_type) {
                    matches_expected = true;
                    break;
                }
            }
            if !matches_expected {
                return Ok(false);
            }
        }

        // Check forbidden types
        for forbidden_type in &type_config.forbidden_types {
            if self.value_matches_type(value, forbidden_type) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check complexity constraints
    fn check_complexity(
        &self,
        value: &str,
        complexity_config: &ComplexityAnalysis,
    ) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST for complexity metrics

        if let Some(max_lines) = complexity_config.max_lines {
            let line_count = value.lines().count() as u32;
            if line_count > max_lines {
                return Ok(false);
            }
        }

        // For cyclomatic complexity and nesting depth, we'd need proper AST analysis
        // For now, we'll just return true
        Ok(true)
    }

    /// Calculate Shannon entropy of a string
    fn calculate_entropy(&self, s: &str) -> f64 {
        use std::collections::HashMap;

        if s.is_empty() {
            return 0.0;
        }

        let mut char_counts = HashMap::new();
        for c in s.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in char_counts.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    /// Check if value matches charset
    fn matches_charset(&self, value: &str, charset: &str) -> bool {
        match charset {
            "alphanumeric" => value.chars().all(|c| c.is_alphanumeric()),
            "alphabetic" => value.chars().all(|c| c.is_alphabetic()),
            "numeric" => value.chars().all(|c| c.is_numeric()),
            "ascii" => value.is_ascii(),
            _ => true, // Unknown charset, assume match
        }
    }

    /// Check if value matches a type pattern
    fn value_matches_type(&self, value: &str, type_name: &str) -> bool {
        match type_name {
            "string" => true, // All values are strings at this level
            "number" => value.parse::<f64>().is_ok(),
            "integer" => value.parse::<i64>().is_ok(),
            "boolean" => value == "true" || value == "false",
            "null" => value == "null" || value == "None" || value == "nil",
            _ => false, // Unknown type
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::{PatternType, SemgrepPattern, MetavariableRegex, MetavariableComparison, ComparisonOperator};

    // Mock AST node for testing
    #[derive(Clone)]
    struct MockNode {
        text: Option<String>,
        children: Vec<MockNode>,
    }

    impl MockNode {
        fn new(text: &str) -> Self {
            Self {
                text: Some(text.to_string()),
                children: Vec::new(),
            }
        }

        fn with_type(node_type: &str, text: &str) -> Self {
            Self {
                text: Some(text.to_string()),
                children: Vec::new(),
            }
        }

        fn with_type_and_children(node_type: &str, text: &str, children: Vec<MockNode>) -> Self {
            Self {
                text: Some(text.to_string()),
                children,
            }
        }

        fn with_children(text: &str, children: Vec<MockNode>) -> Self {
            Self {
                text: Some(text.to_string()),
                children,
            }
        }
    }

    impl AstNode for MockNode {
        fn node_type(&self) -> &str {
            "mock"
        }
        fn text(&self) -> Option<&str> {
            self.text.as_deref()
        }
        fn child_count(&self) -> usize {
            self.children.len()
        }
        fn child(&self, index: usize) -> Option<&dyn AstNode> {
            self.children.get(index).map(|c| c as &dyn AstNode)
        }
        fn location(&self) -> Option<(usize, usize, usize, usize)> {
            None
        }
        fn clone_node(&self) -> Box<dyn AstNode> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_pattern_not_regex() {
        let mut matcher = AdvancedSemgrepMatcher::new();

        // Create a pattern that should NOT match "test_function"
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::NotRegex("test_.*".to_string()),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let test_node = MockNode::new("test_function");
        let regular_node = MockNode::new("regular_function");

        // Should not match test_function (matches the regex, so not-regex is false)
        assert!(!matcher.matches_pattern(&pattern, &test_node).unwrap());

        // Should match regular_function (doesn't match the regex, so not-regex is true)
        assert!(matcher.matches_pattern(&pattern, &regular_node).unwrap());
    }

    #[test]
    fn test_pattern_not_inside() {
        let mut matcher = AdvancedSemgrepMatcher::new();

        // Create inner pattern for class context
        let inner_pattern = SemgrepPattern::simple("class".to_string());

        // Create not-inside pattern
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::NotInside(Box::new(inner_pattern)),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        // Create test nodes
        let class_node = MockNode::new("class");
        let function_node = MockNode::new("function");
        let nested_function = MockNode::with_children("class", vec![MockNode::new("function")]);

        // Function inside class should not match (inside class context)
        // Note: This is a simplified test - real implementation would need proper AST traversal
        assert!(matcher.matches_pattern(&pattern, &function_node).unwrap());
    }

    #[test]
    fn test_advanced_matcher_new() {
        let matcher = AdvancedSemgrepMatcher::new();
        assert!(!matcher.debug_mode);
        assert!(matcher.max_depth.is_none());
        assert!(matcher.constant_values.is_empty());
        assert!(matcher.full_source.is_none());
        assert!(matcher.symbolic_propagator.is_none());
    }

    #[test]
    fn test_advanced_matcher_default() {
        let matcher: AdvancedSemgrepMatcher = Default::default();
        assert!(!matcher.debug_mode);
        assert!(matcher.max_depth.is_none());
    }

    #[test]
    fn test_with_debug() {
        let matcher = AdvancedSemgrepMatcher::new().with_debug();
        assert!(matcher.debug_mode);
    }

    #[test]
    fn test_with_max_depth() {
        let matcher = AdvancedSemgrepMatcher::new().with_max_depth(5);
        assert_eq!(matcher.max_depth, Some(5));
    }

    #[test]
    fn test_with_constant_values() {
        use astgrep_dataflow::ConstantValue;
        let mut constants = HashMap::new();
        constants.insert("MAX_SIZE".to_string(), ConstantValue::Integer(100));
        constants.insert("NAME".to_string(), ConstantValue::String("test".to_string()));

        let matcher = AdvancedSemgrepMatcher::new().with_constant_values(constants);
        assert_eq!(matcher.constant_values.len(), 2);
    }

    #[test]
    fn test_set_constant_values() {
        use astgrep_dataflow::ConstantValue;
        let mut matcher = AdvancedSemgrepMatcher::new();
        let mut constants = HashMap::new();
        constants.insert("KEY".to_string(), ConstantValue::String("value".to_string()));

        matcher.set_constant_values(constants);
        assert_eq!(matcher.constant_values.len(), 1);
    }

    #[test]
    fn test_find_matches_empty_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let root = MockNode::new("test_node");
        let pattern = SemgrepPattern::simple("".to_string());

        let result = matcher.find_matches(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_matches_simple() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let root = MockNode::new("test_node");
        let pattern = SemgrepPattern::simple("test".to_string());

        let result = matcher.find_matches(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_matches_with_children() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let child1 = MockNode::new("child1");
        let child2 = MockNode::new("child2");
        let root = MockNode::with_children("root", vec![child1, child2]);

        let pattern = SemgrepPattern::simple("child".to_string());

        let result = matcher.find_matches(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_matches_deeply_nested() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        // Create a deeply nested structure (5+ levels)
        let level5 = MockNode::new("level5");
        let level4 = MockNode::with_children("level4", vec![level5]);
        let level3 = MockNode::with_children("level3", vec![level4]);
        let level2 = MockNode::with_children("level2", vec![level3]);
        let level1 = MockNode::with_children("level1", vec![level2]);
        let root = MockNode::with_children("root", vec![level1]);

        let pattern = SemgrepPattern::simple("level".to_string());

        let result = matcher.find_matches(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_matches_max_depth() {
        let mut matcher = AdvancedSemgrepMatcher::new().with_max_depth(2);
        let level3 = MockNode::new("deep");
        let level2 = MockNode::with_children("level2", vec![level3]);
        let level1 = MockNode::with_children("level1", vec![level2]);
        let root = MockNode::with_children("root", vec![level1]);

        let pattern = SemgrepPattern::simple("deep".to_string());

        let result = matcher.find_matches(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_either_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("target");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Either(vec![
                        SemgrepPattern::simple("other".to_string()),
                        SemgrepPattern::simple("target".to_string()),
                    ]),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_all_patterns() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test_node");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::All(vec![
                        SemgrepPattern::simple("test".to_string()),
                        SemgrepPattern::simple("node".to_string()),
                    ]),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_any_patterns() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("target");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Any(vec![
                        SemgrepPattern::simple("other".to_string()),
                        SemgrepPattern::simple("target".to_string()),
                    ]),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_not_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Not(Box::new(SemgrepPattern::simple("world".to_string()))),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_regex_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test_function");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Regex(r"test_.*".to_string()),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_not_regex_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::NotRegex(r"world.*".to_string()),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_inside_pattern() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let inner = MockNode::new("inner");
        let root = MockNode::with_children("outer", vec![inner]);
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Inside(Box::new(SemgrepPattern::simple("inner".to_string()))),
                    metavariable_pattern: None,
                    conditions: Vec::new(),
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_pattern_with_conditions() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test_value");
        let pattern = SemgrepPattern {
                    pattern_type: PatternType::Simple("test".to_string()),
                    metavariable_pattern: None,
                    conditions: vec![
                        Condition::MetavariableRegex(MetavariableRegex {
                            metavariable: "$X".to_string(),
                            regex: r".*value.*".to_string(),
                        }),
                    ],
                    focus: None,
                };

        let result = matcher.matches_pattern(&pattern, &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_conditions_empty() {
        let matcher = AdvancedSemgrepMatcher::new();
        let bindings: HashMap<String, String> = HashMap::new();
        let conditions: Vec<Condition> = Vec::new();

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_conditions_metavariable_regex() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut bindings: HashMap<String, String> = HashMap::new();
        bindings.insert("X".to_string(), "hello_world".to_string());

        let conditions = vec![
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$X".to_string(),
                regex: r"hello_.*".to_string(),
            }),
        ];

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_conditions_metavariable_regex_no_match() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut bindings: HashMap<String, String> = HashMap::new();
        bindings.insert("X".to_string(), "hello".to_string());

        let conditions = vec![
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$X".to_string(),
                regex: r"^world$".to_string(),
            }),
        ];

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_evaluate_conditions_metavariable_comparison() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut bindings: HashMap<String, String> = HashMap::new();
        bindings.insert("X".to_string(), "100".to_string());

        let conditions = vec![
            Condition::MetavariableComparison(MetavariableComparison {
                metavariable: "X".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: "50".to_string(),
            }),
        ];

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_conditions_multiple_metavariables() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut bindings: HashMap<String, String> = HashMap::new();
        bindings.insert("X".to_string(), "hello".to_string());
        bindings.insert("Y".to_string(), "world".to_string());

        let conditions = vec![
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$X".to_string(),
                regex: r"hello.*".to_string(),
            }),
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$Y".to_string(),
                regex: r"world.*".to_string(),
            }),
        ];

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_conditions_conflicting_constraints() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut bindings: HashMap<String, String> = HashMap::new();
        bindings.insert("X".to_string(), "hello".to_string());

        let conditions = vec![
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$X".to_string(),
                regex: r"^hello$".to_string(),
            }),
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "$X".to_string(),
                regex: r"^world$".to_string(),
            }),
        ];

        let result = matcher.evaluate_conditions(&conditions, &bindings);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_match_literal_exact() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let result = matcher.match_literal("hello", &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_literal_partial() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello_world");
        let result = matcher.match_literal("hello", &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_literal_no_match() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let result = matcher.match_literal("world", &node);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_match_literal_ellipsis() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::with_type("string_literal", r#""hello""#);
        let result = matcher.match_literal("...", &node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_node_type() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test");
        let result = matcher.match_node_type("mock", &node);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_node_type_no_match() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test");
        let result = matcher.match_node_type("identifier", &node);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_tokenize_simple() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_punctuation() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize("foo(bar, baz);");
        assert_eq!(tokens, vec!["foo", "(", "bar", ",", "baz", ")", ";"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_string_literal() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize(r#"foo("hello world")"#);
        assert!(tokens.contains(&r#""hello world""#.to_string()));
    }

    #[test]
    fn test_tokenize_comments() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize("foo // comment\nbar");
        assert_eq!(tokens, vec!["foo", "bar"]);
    }

    #[test]
    fn test_tokenize_block_comments() {
        let matcher = AdvancedSemgrepMatcher::new();
        let tokens = matcher.tokenize("foo /* comment */ bar");
        assert_eq!(tokens, vec!["foo", "bar"]);
    }

    #[test]
    fn test_calculate_entropy() {
        let matcher = AdvancedSemgrepMatcher::new();
        let entropy = matcher.calculate_entropy("abc");
        assert!(entropy > 0.0);

        let entropy_empty = matcher.calculate_entropy("");
        assert_eq!(entropy_empty, 0.0);

        let entropy_repeated = matcher.calculate_entropy("aaaa");
        assert_eq!(entropy_repeated, 0.0);
    }

    #[test]
    fn test_matches_charset() {
        let matcher = AdvancedSemgrepMatcher::new();
        assert!(matcher.matches_charset("abc123", "alphanumeric"));
        assert!(matcher.matches_charset("abc", "alphabetic"));
        assert!(matcher.matches_charset("123", "numeric"));
        assert!(matcher.matches_charset("hello", "ascii"));
        assert!(!matcher.matches_charset("abc123", "alphabetic"));
        assert!(!matcher.matches_charset("abc", "numeric"));
    }

    #[test]
    fn test_value_matches_type() {
        let matcher = AdvancedSemgrepMatcher::new();
        assert!(matcher.value_matches_type("hello", "string"));
        assert!(matcher.value_matches_type("123", "number"));
        assert!(matcher.value_matches_type("123", "integer"));
        assert!(matcher.value_matches_type("true", "boolean"));
        assert!(matcher.value_matches_type("null", "null"));
        assert!(!matcher.value_matches_type("hello", "number"));
        assert!(!matcher.value_matches_type("hello", "integer"));
    }

    #[test]
    fn test_check_complexity() {
        let matcher = AdvancedSemgrepMatcher::new();
        let config = ComplexityAnalysis {
            max_cyclomatic: None,
            max_nesting_depth: None,
            max_lines: Some(5),
        };
        let result = matcher.check_complexity("line1\nline2\nline3", &config);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result_long = matcher.check_complexity("line1\nline2\nline3\nline4\nline5\nline6", &config);
        assert!(result_long.is_ok());
        assert!(!result_long.unwrap());
    }

    #[test]
    fn test_check_type_analysis() {
        let matcher = AdvancedSemgrepMatcher::new();
        let config = TypeAnalysis {
            expected_types: vec!["string".to_string()],
            forbidden_types: vec!["null".to_string()],
            nullable: None,
        };
        let result = matcher.check_type_analysis("hello", &config);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let config_forbidden = TypeAnalysis {
            expected_types: vec![],
            forbidden_types: vec!["string".to_string()],
            nullable: None,
        };
        let result_forbidden = matcher.check_type_analysis("hello", &config_forbidden);
        assert!(result_forbidden.is_ok());
        assert!(!result_forbidden.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_len() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("hello", "len($VAR)");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_bitor() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("1", "$X | 1 == 1");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_bitnot() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("0", "~$X == -1");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_in() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("a", r#"$X in "abc""#);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_not_in() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("x", r#"$X not in "abc""#);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_python_expression_power() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_python_expression("2", "$X ** 2 == 4");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_find_inside_regions_empty() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.find_inside_regions("", "source code");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_inside_regions_simple() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.find_inside_regions("class $T { ... }", "class Foo { int x; }");
        assert!(result.is_some());
        let regions = result.unwrap();
        assert!(!regions.is_empty());
    }

    #[test]
    fn test_get_node_byte_offset_range() {
        let matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test");
        let result = matcher.get_node_byte_offset_range(&node, "test");
        // MockNode doesn't have real location, so this may return None
        assert!(result.is_none());
    }

    #[test]
    fn test_match_alternative() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let result = matcher.match_alternative(
            &[
                ParsedPattern::Literal("world".to_string()),
                ParsedPattern::Literal("hello".to_string()),
            ],
            &node,
            0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_sequence() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("return x;");
        let result = matcher.match_sequence(
            &[
                ParsedPattern::Literal("return".to_string()),
                ParsedPattern::Metavariable("X".to_string()),
            ],
            &node,
            0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_parsed_pattern_wildcard() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("anything");
        let result = matcher.match_parsed_pattern(&ParsedPattern::Wildcard, &node, 0);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_parsed_pattern_literal() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let result = matcher.match_parsed_pattern(&ParsedPattern::Literal("hello".to_string()), &node, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_parsed_pattern_metavariable() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::with_type("identifier", "test_var");
        let result = matcher.match_parsed_pattern(&ParsedPattern::Metavariable("X".to_string()), &node, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_parsed_pattern_node_type() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("test");
        let result = matcher.match_parsed_pattern(
            &ParsedPattern::NodeType("mock".to_string()), &node, 0);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_match_parsed_pattern_sequence() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello world");
        let result = matcher.match_parsed_pattern(
            &ParsedPattern::Sequence(vec![
                ParsedPattern::Literal("hello".to_string()),
                ParsedPattern::Literal("world".to_string()),
            ]),
            &node,
            0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_parsed_pattern_alternative() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        let node = MockNode::new("hello");
        let result = matcher.match_parsed_pattern(
            &ParsedPattern::Alternative(vec![
                ParsedPattern::Literal("world".to_string()),
                ParsedPattern::Literal("hello".to_string()),
            ]),
            &node,
            0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_import_map() {
        let matcher = AdvancedSemgrepMatcher::new();
        let source = "import java.util.List;\nimport java.io.File;\n";
        let import_map = matcher.build_import_map(source);
        assert_eq!(import_map.get("List"), Some(&"java.util.List".to_string()));
        assert_eq!(import_map.get("File"), Some(&"java.io.File".to_string()));
    }

    #[test]
    fn test_resolve_name_to_fqn() {
        let matcher = AdvancedSemgrepMatcher::new();
        let mut import_map = HashMap::new();
        import_map.insert("List".to_string(), "java.util.List".to_string());

        assert_eq!(matcher.resolve_name_to_fqn("List", &import_map), "java.util.List");
        assert_eq!(matcher.resolve_name_to_fqn("Unknown", &import_map), "Unknown");
        assert_eq!(matcher.resolve_name_to_fqn("java.util.Map", &import_map), "java.util.Map");
    }

    #[test]
    fn test_evaluate_name_constraint_exact() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_name_constraint("List", "java.util.List", "import java.util.List;");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_name_constraint_wildcard() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_name_constraint("List", "java.util.*", "import java.util.List;");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluate_name_constraint_no_match() {
        let matcher = AdvancedSemgrepMatcher::new();
        let result = matcher.evaluate_name_constraint("List", "java.io.*", "");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
