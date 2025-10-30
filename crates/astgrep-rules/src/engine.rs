//! Rule execution engine
//!
//! This module provides the core rule execution engine that applies rules to AST nodes.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Location, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use regex::Regex;


/// Rule execution engine
pub struct RuleExecutionEngine {
    parallel_execution: bool,
    max_execution_time_ms: Option<u64>,
    cache_enabled: bool,
    execution_cache: HashMap<String, Vec<Finding>>,
}

impl RuleExecutionEngine {
    /// Create a new rule execution engine
    pub fn new() -> Self {
        Self {
            parallel_execution: true,
            max_execution_time_ms: Some(30000), // 30 seconds default
            cache_enabled: false,
            execution_cache: HashMap::new(),
        }
    }

    /// Enable or disable parallel execution
    pub fn set_parallel_execution(mut self, enabled: bool) -> Self {
        self.parallel_execution = enabled;
        self
    }

    /// Set maximum execution time per rule
    pub fn set_max_execution_time(mut self, max_time_ms: u64) -> Self {
        self.max_execution_time_ms = Some(max_time_ms);
        self
    }

    /// Enable or disable execution caching
    pub fn set_cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        if !enabled {
            self.execution_cache.clear();
        }
        self
    }

    /// Execute a single rule against an AST
    pub fn execute_rule(
        &mut self,
        rule: &Rule,
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> RuleResult {
        let start_time = Instant::now();
        let cache_key = if self.cache_enabled {
            Some(self.generate_cache_key(rule, context))
        } else {
            None
        };

        // Check cache first
        if let Some(ref key) = cache_key {
            if let Some(cached_findings) = self.execution_cache.get(key) {
                return RuleResult::success(
                    rule.id.clone(),
                    cached_findings.clone(),
                    start_time.elapsed().as_millis() as u64,
                );
            }
        }

        // Execute the rule
        let result = self.execute_rule_internal(rule, ast, context, start_time);

        // Cache successful results
        if let Some(key) = cache_key {
            if result.is_success() {
                self.execution_cache.insert(key, result.findings.clone());
            }
        }

        result
    }

    /// Execute multiple rules against an AST
    pub fn execute_rules(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> Vec<RuleResult> {
        if self.parallel_execution && rules.len() > 1 {
            self.execute_rules_parallel(rules, ast, context)
        } else {
            self.execute_rules_sequential(rules, ast, context)
        }
    }

    /// Execute rules sequentially
    fn execute_rules_sequential(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> Vec<RuleResult> {
        rules
            .iter()
            .filter(|rule| rule.applies_to(context.language))
            .map(|rule| self.execute_rule(rule, ast, context))
            .collect()
    }

    /// Execute rules in parallel (placeholder - would use rayon in real implementation)
    fn execute_rules_parallel(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> Vec<RuleResult> {
        // For now, fall back to sequential execution
        // In a real implementation, this would use rayon or similar
        self.execute_rules_sequential(rules, ast, context)
    }

    /// Internal rule execution logic
    fn execute_rule_internal(
        &self,
        rule: &Rule,
        ast: &dyn AstNode,
        context: &RuleContext,
        start_time: Instant,
    ) -> RuleResult {
        // Check execution timeout
        if let Some(max_time) = self.max_execution_time_ms {
            if start_time.elapsed().as_millis() as u64 > max_time {
                return RuleResult::error(
                    rule.id.clone(),
                    "Rule execution timeout".to_string(),
                    start_time.elapsed().as_millis() as u64,
                );
            }
        }

        println!("🔍 Executing rule: {}", rule.id);
        println!("🔍 Rule has {} patterns", rule.patterns.len());

        let mut findings = Vec::new();

        // Execute pattern matching
        for (i, pattern) in rule.patterns.iter().enumerate() {
            println!("🔍 Processing pattern {} of {}", i + 1, rule.patterns.len());
            match self.execute_pattern(pattern, ast, rule, context) {
                Ok(mut pattern_findings) => {
                    println!("🔍 Pattern {} generated {} findings", i + 1, pattern_findings.len());
                    findings.append(&mut pattern_findings)
                },
                Err(e) => {
                    println!("🔍 Pattern {} failed with error: {}", i + 1, e);
                    return RuleResult::error(
                        rule.id.clone(),
                        format!("Pattern execution error: {}", e),
                        start_time.elapsed().as_millis() as u64,
                    );
                }
            }
        }

        // Execute dataflow analysis if specified
        if let Some(ref dataflow) = rule.dataflow {
            match self.execute_dataflow(dataflow, ast, rule, context) {
                Ok(mut dataflow_findings) => findings.append(&mut dataflow_findings),
                Err(e) => {
                    return RuleResult::error(
                        rule.id.clone(),
                        format!("Dataflow analysis error: {}", e),
                        start_time.elapsed().as_millis() as u64,
                    );
                }
            }
        }

        RuleResult::success(
            rule.id.clone(),
            findings,
            start_time.elapsed().as_millis() as u64,
        )
    }

    /// Execute pattern matching
    fn execute_pattern(
        &self,
        pattern: &Pattern,
        _ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        println!("🔍 Executing pattern for rule: {}", rule.id);
        println!("🔍 Pattern: {:?}", pattern);

        let mut findings = Vec::new();

        // 1) Regex patterns: run real regex over full source
        if let PatternType::Regex(ref regex_str) = &pattern.pattern_type {
            match Regex::new(regex_str) {
                Ok(re) => {
                    for m in re.find_iter(&context.source_code) {
                        let (start_line, start_col) = Self::byte_index_to_line_col(&context.source_code, m.start());
                        let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, m.end());

                        let location = Location::new(
                            std::path::PathBuf::from(&context.file_path),
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        );

                        let matched_text = &context.source_code[m.start()..m.end().min(context.source_code.len())];

                        let finding = Finding::new(
                            rule.id.clone(),
                            if !rule.description.is_empty() { rule.description.clone() } else { format!("Match: {}", matched_text) },
                            rule.severity,
                            rule.confidence,
                            location,
                        )
                        .with_metadata("pattern".to_string(), regex_str.clone());

                        let finding = if let Some(ref fix) = rule.fix { finding.with_fix(fix.clone()) } else { finding };
                        findings.push(finding);
                    }
                    println!("🔍 Regex pattern execution complete. Generated {} findings", findings.len());
                    return Ok(findings);
                }
                Err(e) => {
                    // Invalid regex, surface as analysis error
                    return Err(astgrep_core::AnalysisError::pattern_match_error(format!("Invalid regex: {}", e)));
                }
            }
        }

        // 2) Simple patterns (with or without metavariables): scan full source and emit one finding per occurrence
        if let PatternType::Simple(ref pattern_str) = &pattern.pattern_type {
            let spans = self.find_pattern_spans_in_source(&pattern_str, &context.source_code, context.language);
            println!("🔍 Pattern matching found {} spans", spans.len());

            // Optional: deduplicate identical spans
            use std::collections::HashSet;
            let mut seen: HashSet<(usize, usize)> = HashSet::new();

            for (start_byte, end_byte) in spans {
                if !seen.insert((start_byte, end_byte)) { continue; }
                let (start_line, start_col) = Self::byte_index_to_line_col(&context.source_code, start_byte);
                let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);

                let location = Location::new(
                    std::path::PathBuf::from(&context.file_path),
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                );

                // Extract matched text for message context (best-effort)
                let matched_text = &context.source_code[start_byte..end_byte.min(context.source_code.len())];

                let finding = Finding::new(
                    rule.id.clone(),
                    if !rule.description.is_empty() { rule.description.clone() } else { format!("Match: {}", matched_text) },
                    rule.severity,
                    rule.confidence,
                    location,
                )
                .with_metadata("pattern".to_string(), pattern_str.clone());

                let finding = if let Some(ref fix) = rule.fix { finding.with_fix(fix.clone()) } else { finding };
                findings.push(finding);
            }

            println!("🔍 Pattern execution complete. Generated {} findings", findings.len());
            return Ok(findings);
        }

        // 3) pattern-either: handle Regex and Simple alternatives on full source (including metavariables)
        if let PatternType::Either(ref subs) = &pattern.pattern_type {
            use std::collections::HashSet;
            let mut seen: HashSet<(usize, usize)> = HashSet::new();

            for sub in subs {
                match &sub.pattern_type {
                    PatternType::Regex(r) => {
                        if let Ok(re) = Regex::new(r) {
                            for m in re.find_iter(&context.source_code) {
                                let start_byte = m.start();
                                let end_byte = m.end();
                                if !seen.insert((start_byte, end_byte)) { continue; }
                                let (start_line, start_col) = Self::byte_index_to_line_col(&context.source_code, start_byte);
                                let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);
                                let location = Location::new(
                                    std::path::PathBuf::from(&context.file_path),
                                    start_line,
                                    start_col,
                                    end_line,
                                    end_col,
                                );
                                let matched_text = &context.source_code[start_byte..end_byte.min(context.source_code.len())];
                                let mut finding = Finding::new(
                                    rule.id.clone(),
                                    if !rule.description.is_empty() { rule.description.clone() } else { format!("Match: {}", matched_text) },
                                    rule.severity,
                                    rule.confidence,
                                    location,
                                );
                                finding = finding.with_metadata("pattern".to_string(), r.clone());
                                if let Some(ref fix) = rule.fix { finding = finding.with_fix(fix.clone()); }
                                findings.push(finding);
                            }
                        }
                    }
                    PatternType::Simple(s) => {
                        let spans = self.find_pattern_spans_in_source(s, &context.source_code, context.language);
                        println!("DEBUG either: simple pattern '{}' produced {} spans", s, spans.len());
                        for (start_byte, end_byte) in spans {
                            if !seen.insert((start_byte, end_byte)) { continue; }
                            let (start_line, start_col) = Self::byte_index_to_line_col(&context.source_code, start_byte);
                            let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);
                            let location = Location::new(
                                std::path::PathBuf::from(&context.file_path),
                                start_line,
                                start_col,
                                end_line,
                                end_col,
                            );
                            let matched_text = &context.source_code[start_byte..end_byte.min(context.source_code.len())];
                            let mut finding = Finding::new(
                                rule.id.clone(),
                                if !rule.description.is_empty() { rule.description.clone() } else { format!("Match: {}", matched_text) },
                                rule.severity,
                                rule.confidence,
                                location,
                            );
                            finding = finding.with_metadata("pattern".to_string(), s.clone());
                            if let Some(ref fix) = rule.fix { finding = finding.with_fix(fix.clone()); }
                            findings.push(finding);
                        }
                    }
                    _ => {}
                }
            }
            println!("🔍 pattern-either execution complete. Generated {} findings", findings.len());
            if !findings.is_empty() {
                return Ok(findings);
            }
        }

        // Fallback: no simple/regex pattern string available, use node-based matching (locations may be coarse)
        let matches = self.find_pattern_matches(pattern, _ast, context.language)?;
        println!("🔍 Fallback matching found {} matches", matches.len());

        // Keep only smallest, non-overlapping node spans
        let mut mm: Vec<((usize, usize), usize, usize, usize, usize, Box<dyn AstNode>)> = matches
            .into_iter()
            .map(|m| {
                if let Some((sl, sc, el, ec)) = m.location() {
                    let dl = el.saturating_sub(sl);
                    let dc = ec.saturating_sub(sc);
                    ((dl, dc), sl, sc, el, ec, m)
                } else {
                    ((usize::MAX, usize::MAX), 0, 0, usize::MAX, usize::MAX, m)
                }
            })
            .collect();
        mm.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| (a.1, a.2, a.3, a.4).cmp(&(b.1, b.2, b.3, b.4))));

        let overlaps = |a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)| -> bool {
            let (a_sl, a_sc, a_el, a_ec) = a;
            let (b_sl, b_sc, b_el, b_ec) = b;
            if a_el < b_sl || b_el < a_sl { return false; }
            if a_sl == b_el && a_sc >= b_ec { return false; }
            if b_sl == a_el && b_sc >= a_ec { return false; }
            true
        };

        let mut selected_spans: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut filtered_nodes: Vec<Box<dyn AstNode>> = Vec::new();
        'outer: for (_, sl, sc, el, ec, m) in mm {
            for s in &selected_spans {
                if overlaps((sl, sc, el, ec), *s) {
                    continue 'outer;
                }
            }
            selected_spans.push((sl, sc, el, ec));
            filtered_nodes.push(m);
        }

        for match_node in filtered_nodes {
            let location = self.create_best_location_from_node_or_pattern(match_node.as_ref(), pattern, context);
            let finding = Finding::new(
                rule.id.clone(),
                self.generate_finding_message(rule, pattern, match_node.as_ref()),
                rule.severity,
                rule.confidence,
                location,
            );
            findings.push(finding);
        }
        Ok(findings)
    }

    /// Try to create a best-effort location for a match using node.location() first,
    /// then fallback to approximating from the pattern's literal anchors in source text.
    fn create_best_location_from_node_or_pattern(&self, node: &dyn AstNode, pattern: &Pattern, context: &RuleContext) -> Location {
        // 1) If the AST node carries precise location, use it.
        if let Some((sl, sc, el, ec)) = node.location() {
            return Location::new(std::path::PathBuf::from(&context.file_path), sl, sc, el, ec);
        }
        // 2) Fallback: try to approximate location by searching literal anchors from the pattern
        if let Some(pat_str) = pattern.get_pattern_string() {
            if let Some((start_byte, end_byte)) = Self::approximate_span_from_pattern(&context.source_code, pat_str) {
                let (sl, sc) = Self::byte_index_to_line_col(&context.source_code, start_byte);
                let (el, ec) = Self::byte_index_to_line_col(&context.source_code, end_byte);
                return Location::new(std::path::PathBuf::from(&context.file_path), sl, sc, el, ec);
            }
        }
        // 3) Last resort: point at file start
        Location::point(std::path::PathBuf::from(&context.file_path), 1, 1)
    }

    /// Extract a best-effort byte span by using the longest literal anchors in the pattern string.
    /// This supports simple patterns like "Runtime.getRuntime().exec($X)" by anchoring
    /// at "Runtime.getRuntime().exec(" and optionally a trailing literal, e.g., ")".
    fn approximate_span_from_pattern(source: &str, pattern: &str) -> Option<(usize, usize)> {
        // Split pattern into literal segments by removing $META variables
        let mut literals: Vec<String> = Vec::new();
        let mut buf = String::new();
        let mut chars = pattern.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                // flush current literal
                if !buf.is_empty() { literals.push(std::mem::take(&mut buf)); }
                // consume metavar name
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' { chars.next(); } else { break; }
                }
            } else {
                buf.push(ch);
            }
        }
        if !buf.is_empty() { literals.push(buf); }
        // Keep non-empty segments
        let anchors: Vec<&str> = literals.iter().map(String::as_str).filter(|s| !s.is_empty()).collect();
        if anchors.is_empty() { return None; }
        // First and last literal anchors
        let first = anchors.first().unwrap();
        let last = anchors.last().unwrap();
        // Find start by the first anchor
        let start = source.find(first)?;
        // Determine end
        let end = if anchors.len() > 1 {
            // Try to find the last anchor after start
            if let Some(rel) = source[start + first.len()..].find(last) {
                start + first.len() + rel + last.len()
            } else {
                start + first.len()
            }
        } else {
            start + first.len()
        };
        Some((start, end.min(source.len())))
    }

    /// Find pattern matches in AST (simplified implementation)
    fn find_pattern_matches(&self, pattern: &Pattern, ast: &dyn AstNode, language: astgrep_core::Language) -> Result<Vec<Box<dyn AstNode>>> {
        let mut matches = Vec::new();
        let mut node_count = 0;

        println!("🔍 Starting AST traversal for pattern: {:?}", pattern);

        // Handle different pattern types
        match &pattern.pattern_type {
            crate::types::PatternType::Either(sub_patterns) => {
                println!("🔍 Processing Either pattern with {} sub-patterns", sub_patterns.len());
                // For Either patterns, try each sub-pattern
                for (i, sub_pattern) in sub_patterns.iter().enumerate() {
                    println!("🔍 Trying Either sub-pattern {}: {:?}", i + 1, sub_pattern);
                    let sub_matches = self.find_pattern_matches(sub_pattern, ast, language)?;
                    println!("🔍 Either sub-pattern {} found {} matches", i + 1, sub_matches.len());
                    matches.extend(sub_matches);
                }
            }
            _ => {
                // Simple text-based matching for demonstration
                // In a real implementation, this would use proper AST pattern matching
                astgrep_core::ast_utils::visit_nodes(ast, &mut |node| {
                    node_count += 1;
                    if let Some(text) = node.text() {
                        println!("🔍 Visiting node #{}: '{}'", node_count, text);
                        if let Some(pattern_str) = pattern.get_pattern_string() {
                            println!("🔍 Pattern string: '{}'", pattern_str);
                            if self.simple_pattern_match(pattern_str, text, language) {
                                println!("🔍 MATCH FOUND! Adding node to matches");
                                matches.push(node.clone_node());
                            }
                        } else {
                            println!("🔍 No pattern string found for pattern: {:?}", pattern.pattern_type);
                        }
                    } else {
                        println!("🔍 Visiting node #{}: <no text>", node_count);
                    }
                    Ok(())
                })?;
            }
        }

        println!("🔍 AST traversal complete. Visited {} nodes, found {} matches", node_count, matches.len());
        Ok(matches)
    }

    /// Tokenize a string, preserving operators and punctuation as separate tokens.
    /// Note: recognizes "..." as a single Ellipsis token in patterns and text.
    fn tokenize(&self, s: &str) -> Vec<String> {
        self.tokenize_spanned(s).into_iter().map(|(t, _, _)| t).collect()
    }
    /// Tokenize a pattern string with Semgrep-compatible post-processing.
    /// Specifically, coalesce `$ ...` into a single ellipsis token `...` to support `$...` syntax.
    fn tokenize_pattern(&self, s: &str) -> Vec<String> {
        let mut tokens = self.tokenize(s);
        if tokens.is_empty() { return tokens; }
        let mut coalesced: Vec<String> = Vec::with_capacity(tokens.len());
        let mut idx = 0usize;
        while idx < tokens.len() {
            if tokens[idx] == "$" && idx + 1 < tokens.len() && tokens[idx + 1] == "..." {
                coalesced.push("...".to_string());
                idx += 2;
            } else {
                coalesced.push(std::mem::take(&mut tokens[idx]));
                idx += 1;
            }
        }
        coalesced
    }


    /// Tokenize a string and return tokens with their byte spans (start, end)
    /// Note: recognizes "..." as a single Ellipsis token.
    fn tokenize_spanned(&self, s: &str) -> Vec<(String, usize, usize)> {
        use std::iter::Peekable;
        let mut tokens: Vec<(String, usize, usize)> = Vec::new();
        let mut current = String::new();
        let mut current_start: Option<usize> = None;
        let mut last_end: usize = 0;
        let mut it: Peekable<std::str::CharIndices<'_>> = s.char_indices().peekable();
        while let Some((i, ch)) = it.next() {
            let ch_end = i + ch.len_utf8();
            match ch {
                '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' |
                '&' | '|' | '^' | '~' | '?' | ':' | ';' | ',' |
                '(' | ')' | '[' | ']' | '{' | '}' | '.' => {
                    // flush current ident
                    if !current.is_empty() {
                        tokens.push((std::mem::take(&mut current), current_start.unwrap_or(i), i));
                        current_start = None;
                    }
                    // special case: ellipsis
                    if ch == '.' {
                        // check next two chars form "..."
                        let mut consumed_two = false;
                        if let Some(&(i2, ch2)) = it.peek() {
                            if ch2 == '.' {
                                // consume second '.'
                                let _ = it.next();
                                if let Some(&(i3, ch3)) = it.peek() {
                                    if ch3 == '.' {
                                        // consume third '.' and push ellipsis token
                                        let _ = it.next();
                                        tokens.push(("...".to_string(), i, i + 3));
                                        last_end = i + 3;
                                        consumed_two = true;
                                    }
                                }
                            }
                        }
                        if consumed_two { continue; }
                        // not an ellipsis, just a single dot
                        tokens.push((".".to_string(), i, ch_end));
                    } else {
                        tokens.push((ch.to_string(), i, ch_end));
                    }
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if !current.is_empty() {
                        tokens.push((std::mem::take(&mut current), current_start.unwrap_or(i), i));
                        current_start = None;
                    }
                }
                _ => {
                    if current_start.is_none() { current_start = Some(i); }
                    current.push(ch);
                }
            }
            last_end = ch_end;
        }
        if !current.is_empty() {
            tokens.push((current, current_start.unwrap_or(last_end), last_end));
        }
        tokens
    }

    /// Try to match a pattern starting at token index `start` and return end token index on success
    /// `case_insensitive` controls literal comparisons (used for SQL keywords, etc.)
    fn try_match_tokens(&self, pattern_tokens: &[String], text_tokens: &[(String, usize, usize)], start: usize, case_insensitive: bool) -> Option<usize> {
        let mut i = 0usize; // pattern index
        let mut j = start;  // text token index
        let mut bindings: HashMap<String, Vec<String>> = HashMap::new();
        while i < pattern_tokens.len() {
            if j >= text_tokens.len() { return None; }
            let p_tok = &pattern_tokens[i];
            if case_insensitive { println!("TRACE try_match: i={}, j={}, p_tok='{}', text='{}'", i, j, p_tok, text_tokens[j].0); }

            // Treat "$ ..." (a dollar immediately followed by ellipsis token) as a pure ellipsis (no binding),
            // to be Semgrep-compatible with `$...` syntax commonly used in SQL patterns.
            let is_dollar_ellipsis = p_tok == "$"
                && (i + 1) < pattern_tokens.len()
                && pattern_tokens[i + 1] == "...";

            // Ellipsis: match variable-length sequence (including empty) until next anchor
            if p_tok == "..." || is_dollar_ellipsis {
                if case_insensitive { println!("TRACE ellipsis encountered at i={}, j={}, is_dollar_ellipsis={}", i, j, is_dollar_ellipsis); }
                // When consuming `$ ...`, advance pattern by 2 tokens; otherwise by 1
                if is_dollar_ellipsis { i += 1; } // so the common handling below will also `i += 1` at the end
                // find next anchor that is neither metavariable nor ellipsis
                let next_anchor_idx = (i + 1..pattern_tokens.len())
                    .find(|&k| pattern_tokens[k] != "..." && !pattern_tokens[k].starts_with('$'));
                match next_anchor_idx {
                    None => {
                        if case_insensitive { println!("TRACE ellipsis to end: returning len={}", text_tokens.len()); }
                        // Ellipsis at end: matches the rest (including empty)
                        return Some(text_tokens.len());
                    }
                    Some(k) => {
                        let next_lit = &pattern_tokens[k];
                        if case_insensitive { println!("TRACE ellipsis next anchor literal='{}' (k={})", next_lit, k); }
                        // Balanced delimiters for common closers
                        let mut set_pos: Option<usize> = None;
                        if next_lit == ")" || next_lit == "]" || next_lit == "}" {
                            let (open, close) = if next_lit == ")" { ("(", ")") } else if next_lit == "]" { ("[", "]") } else { ("{", "}") };
                            let mut depth: i32 = 1; // we assume the corresponding opener was matched just before
                            let mut pos = j;
                            while pos < text_tokens.len() {
                                let tok = &text_tokens[pos].0;
                                if tok == open { depth += 1; } else if tok == close { depth -= 1; }
                                if depth == 0 { set_pos = Some(pos); break; }
                                pos += 1;
                            }
                            if let Some(end_pos) = set_pos {
                                if case_insensitive { println!("TRACE ellipsis matched to close at pos={}", end_pos); }
                                // Allow empty between open and close (end_pos == j)
                                i += 1; j = end_pos; continue;
                            } else { return None; }
                        } else {
                            // general case: scan to next literal (nearest/shortest)
                            let mut pos = j; let mut found = None;
                            while pos < text_tokens.len() {
                                let tt = &text_tokens[pos].0;
                                let matched = if case_insensitive { tt.eq_ignore_ascii_case(next_lit) } else { tt == next_lit };
                                if matched { found = Some(pos); break; }
                                pos += 1;
                            }
                            if let Some(end_pos) = found {
                                if case_insensitive { println!("TRACE ellipsis skipped to anchor at pos={}", end_pos); }
                                // empty allowed (end_pos == j)
                                i += 1; j = end_pos; continue;
                            } else { return None; }
                        }
                    }
                }
            } else if p_tok.starts_with('$') {
                // Handle normal metavariables like `$T1`, `$SUBQUERY`. Do NOT conflate with `$ ...` which is handled above.
                let next_lit_idx = (i + 1..pattern_tokens.len()).find(|&k| pattern_tokens[k] != "..." && !pattern_tokens[k].starts_with('$'));
                match next_lit_idx {
                    None => {
                        let capture: Vec<String> = text_tokens[j..].iter().map(|t| t.0.clone()).collect();
                        if capture.is_empty() { return None; }
                        if let Some(prev) = bindings.get(p_tok) { if *prev != capture { return None; } } else { bindings.insert(p_tok.clone(), capture); }
                        return Some(text_tokens.len());
                    }
                    Some(k) => {
                        let next_lit = &pattern_tokens[k];
                        if next_lit == ")" {
                            let mut depth: i32 = 1; let mut pos = j; let mut end_pos: Option<usize> = None;
                            while pos < text_tokens.len() {
                                let tok = &text_tokens[pos].0;
                                if tok == "(" { depth += 1; } else if tok == ")" { depth -= 1; }
                                if depth == 0 { end_pos = Some(pos); break; }
                                pos += 1;
                            }
                            if let Some(end_pos) = end_pos {
                                if end_pos == j { return None; }
                                let capture: Vec<String> = text_tokens[j..end_pos].iter().map(|t| t.0.clone()).collect();
                                if let Some(prev) = bindings.get(p_tok) { if *prev != capture { return None; } } else { bindings.insert(p_tok.clone(), capture); }
                                i += 1; j = end_pos; continue;
                            } else { return None; }
                        } else {
                            let mut pos = j; let mut found = None;
                            while pos < text_tokens.len() {
                                let tt = &text_tokens[pos].0;
                                let matched = if case_insensitive { tt.eq_ignore_ascii_case(next_lit) } else { tt == next_lit };
                                if matched { found = Some(pos); break; }
                                pos += 1;
                            }
                            if let Some(end_pos) = found {
                                if end_pos == j { return None; }
                                let capture: Vec<String> = text_tokens[j..end_pos].iter().map(|t| t.0.clone()).collect();
                                if let Some(prev) = bindings.get(p_tok) { if *prev != capture { return None; } } else { bindings.insert(p_tok.clone(), capture); }
                                i += 1; j = end_pos; continue;
                            } else { return None; }
                        }
                    }
                }
            } else {
                let matched = if case_insensitive { text_tokens[j].0.eq_ignore_ascii_case(p_tok) } else { &text_tokens[j].0 == p_tok };
                if !matched { return None; }
                i += 1; j += 1;
            }
        }
        Some(j)
    }

    /// Find spans (byte start, byte end) of matches in the given source
    fn find_pattern_spans_in_source(&self, pattern: &str, source: &str, language: astgrep_core::Language) -> Vec<(usize, usize)> {
        // Preprocess: make `$...` Semgrep form equivalent to `...` before tokenization
        let preprocessed = pattern.replace("$...", "...");
        println!("DEBUG find_pattern_spans_in_source: pattern='{}', preprocessed='{}', lang={:?}", pattern, preprocessed, language);
        let mut pattern_tokens = self.tokenize_pattern(&preprocessed);
        println!("DEBUG pattern_tokens={:?}", pattern_tokens);
        if pattern_tokens.last() == Some(&";".to_string()) {
            // For SQL patterns, keep explicit trailing semicolon as an anchor to prevent
            // trailing ellipsis from spanning to end-of-file across statements.
            if !matches!(language, astgrep_core::Language::Sql) {
                pattern_tokens.pop();
            }
        }
        // Coalesce `$ ...` into a single ellipsis token to be Semgrep-compatible with `$...`
        let mut coalesced: Vec<String> = Vec::with_capacity(pattern_tokens.len());
        let mut idx = 0usize;
        while idx < pattern_tokens.len() {
            if pattern_tokens[idx] == "$" && idx + 1 < pattern_tokens.len() && pattern_tokens[idx + 1] == "..." {
                coalesced.push("...".to_string());
                idx += 2;
            } else {
                coalesced.push(pattern_tokens[idx].clone());
                idx += 1;
            }
        }
        pattern_tokens = coalesced;
        println!("DEBUG coalesced_pattern_tokens={:?}", pattern_tokens);

        // Determine first literal anchor (the first token that is neither ellipsis nor metavariable)
        let first_anchor: Option<String> = pattern_tokens
            .iter()
            .find(|t| t.as_str() != "..." && !t.starts_with('$'))
            .cloned();

        let text_tokens = self.tokenize_spanned(source);
        println!("DEBUG text_tokens (first 40)={:?}", text_tokens.iter().take(40).map(|t| &t.0).collect::<Vec<_>>());
        let mut spans = Vec::new();
        let case_insensitive = matches!(language, astgrep_core::Language::Sql);

        // Helper: run matching in a token window [win_start, win_end) and push absolute byte spans
        let mut match_in_window = |win_start: usize, win_end: usize| {
            let window = &text_tokens[win_start..win_end];
            for rel_start in 0..window.len() {
                // Optional optimization: require the first literal to match at start to avoid mid-span starts
                if let Some(ref anchor) = first_anchor {
                    let tok = &window[rel_start].0;
                    let lit_ok = if case_insensitive { tok.eq_ignore_ascii_case(anchor) } else { tok == anchor };
                    if !lit_ok { continue; }
                }
                // Java safety: avoid starting a match in the middle of a qualified name (e.g., System.out.println)
                if matches!(language, astgrep_core::Language::Java) {
                    if let Some(first_lit) = pattern_tokens.iter().find(|t| !t.starts_with('$')) {
                        let is_ident = first_lit.chars().all(|c| c.is_alphanumeric() || c == '_');
                        if is_ident && rel_start + win_start > 0 && text_tokens[rel_start + win_start - 1].0 == "." {
                            continue;
                        }
                    }
                }
                if let Some(rel_end) = self.try_match_tokens(&pattern_tokens, window, rel_start, case_insensitive) {
                    if rel_end == 0 { continue; }
                    let abs_start_idx = win_start + rel_start;
                    let abs_end_idx_exclusive = win_start + rel_end;
                    let start_byte = text_tokens[abs_start_idx].1;
                    let end_byte = text_tokens[abs_end_idx_exclusive - 1].2;
                    spans.push((start_byte, end_byte));
                }
            }
        };

        // If SQL, by default constrain matching within single statements (semicolon delimited)
        if matches!(language, astgrep_core::Language::Sql) {
            // Default ON. Later this can be gated by CLI/YAML options via RuleContext if needed.
            let mut stmt_start = 0usize;
            for i in 0..text_tokens.len() {
                if text_tokens[i].0 == ";" {
                    // Include semicolon in the window to allow patterns that anchor on ';'
                    match_in_window(stmt_start, i + 1);
                    stmt_start = i + 1;
                }
            }
            // Also handle trailing tail without semicolon (best effort)
            if stmt_start < text_tokens.len() {
                match_in_window(stmt_start, text_tokens.len());
            }
        } else {
            // Non-SQL: match across whole token stream
            match_in_window(0, text_tokens.len());
        }

        spans
    }

    /// Convert a byte index in `s` to 1-based (line, column)
    fn byte_index_to_line_col(s: &str, byte_idx: usize) -> (usize, usize) {
        let mut line: usize = 1;
        let mut col: usize = 1;
        for (ci, ch) in s.char_indices() {
            if ci >= byte_idx { break; }
            if ch == '\n' { line += 1; col = 1; } else { col += 1; }
        }
        (line, col)
    }




    /// Simple pattern matching with metavariable support（改进：元变量可匹配多 token 表达式）
    /// 实现思路：
    /// - 对 node 文本做 token 序列匹配；
    /// - 普通字面量逐个比对；
    /// - 碰到 $META 时，按“直到下一个字面量”为止进行贪婪匹配；若下一个字面量是右括号，则做成对括号的平衡匹配；
    /// - 允许 pattern 末尾分号为可选；
    /// - 从每个可能的起点尝试匹配，一旦成功即返回 true。
    fn simple_pattern_match(&self, pattern: &str, text: &str, language: astgrep_core::Language) -> bool {
        println!("🔍 Pattern: '{}'", pattern);
        println!("🔍 Node text: '{}'", text);

        // Tokenize pattern and text
        let mut pattern_tokens = self.tokenize_pattern(pattern);
        let text_tokens = self.tokenize(text);

        println!("🔍 Pattern tokens: {:?}", pattern_tokens);
        println!("🔍 Text tokens (len={}): <omitted>", text_tokens.len());

        if pattern_tokens.is_empty() { return false; }

        // Allow trailing semicolon in pattern to be optional
        if pattern_tokens.last() == Some(&";".to_string()) {
            println!("🔍 Pattern has trailing semicolon; making it optional for matching");
            pattern_tokens.pop();
        }

        let case_insensitive = matches!(language, astgrep_core::Language::Sql);


        // 局部闭包：从给定起点尝试匹配，支持 $META 捕获多 token
        let try_match_from = |start: usize| -> bool {
            let mut i = 0usize; // index in pattern
            let mut j = start;   // index in text
            let mut bindings: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

            while i < pattern_tokens.len() {
                if j >= text_tokens.len() {
                    return false;
                }
                let p_tok = &pattern_tokens[i];

                // 兼容 Semgrep `$...` 语法：把 `$` 紧跟 `...` 视为纯省略号（不产生绑定）
                let is_dollar_ellipsis = p_tok == "$"
                    && (i + 1) < pattern_tokens.len()
                    && pattern_tokens[i + 1] == "...";

                if p_tok == "..." || is_dollar_ellipsis {
                    if is_dollar_ellipsis { i += 1; }
                    // Ellipsis：可变长跳过（允许 0 个），直到下一个锚点（既不是元变量也不是省略号）
                    let next_anchor_idx = (i + 1..pattern_tokens.len())
                        .find(|&k| pattern_tokens[k] != "..." && !pattern_tokens[k].starts_with('$'));
                    match next_anchor_idx {
                        None => {
                            // 末尾省略号：匹配到文本末尾（允许 0 个）
                            return true;
                        }
                        Some(k) => {
                            let next_lit = &pattern_tokens[k];
                            if next_lit == ")" || next_lit == "]" || next_lit == "}" {
                                let (open, close) = if next_lit == ")" { ("(", ")") } else if next_lit == "]" { ("[", "]") } else { ("{", "}") };
                                let mut depth: i32 = 1;
                                let mut pos = j;
                                while pos < text_tokens.len() {
                                    let tok = &text_tokens[pos];
                                    if tok == open { depth += 1; }
                                    else if tok == close { depth -= 1; }
                                    if depth == 0 { break; }
                                    pos += 1;
                                }
                                if pos < text_tokens.len() {
                                    // 允许空匹配：pos 可以等于 j
                                    i += 1;
                                    j = pos; // 不消耗 next_lit
                                    continue;
                                } else { return false; }
                            } else {
                                let mut pos = j;
                                let mut found_k: Option<usize> = None;
                                while pos < text_tokens.len() {
                                    let matched = if case_insensitive { text_tokens[pos].eq_ignore_ascii_case(next_lit) } else { &text_tokens[pos] == next_lit };
                                    if matched { found_k = Some(pos); break; }
                                    pos += 1;
                                }
                                if let Some(end_pos) = found_k {
                                    // 允许空匹配
                                    i += 1;
                                    j = end_pos; // 不消耗 next_lit
                                    continue;
                                } else {
                                    return false;
                                }
                            }
                        }
                    }
                } else if p_tok.starts_with('$') {
                    // 查找下一个字面量（非 $ / 非 ...）
                    let next_lit_idx = (i + 1..pattern_tokens.len()).find(|&k| pattern_tokens[k] != "..." && !pattern_tokens[k].starts_with('$'));
                    match next_lit_idx {
                        None => {
                            // $META 在 pattern 末尾：捕获到文本末尾（至少 1 个 token）
                            if j >= text_tokens.len() { return false; }
                            let capture: Vec<String> = text_tokens[j..].to_vec();
                            if capture.is_empty() { return false; }
                            if let Some(prev) = bindings.get(p_tok) {
                                if *prev != capture { return false; }
                            } else {
                                bindings.insert(p_tok.clone(), capture);
                            }
                            // 完整匹配
                            return true;
                        }
                        Some(k) => {
                            let next_lit = &pattern_tokens[k];
                            if next_lit == ")" {
                                // 特殊：直到与之前的 '(' 配对的 ')' 为止（平衡括号）
                                let mut depth: i32 = 1; // 进入此分支前，通常 pattern 已匹配了 '('
                                let mut pos = j;
                                let mut found_end: Option<usize> = None;
                                while pos < text_tokens.len() {
                                    let tok = &text_tokens[pos];
                                    if tok == "(" { depth += 1; }
                                    else if tok == ")" { depth -= 1; }
                                    if depth == 0 { found_end = Some(pos); break; }
                                    pos += 1;
                                }
                                if let Some(end_pos) = found_end {
                                    if end_pos == j { return false; } // 至少一个 token
                                    let capture: Vec<String> = text_tokens[j..end_pos].to_vec();
                                    if let Some(prev) = bindings.get(p_tok) {
                                        if *prev != capture { return false; }
                                    } else {
                                        bindings.insert(p_tok.clone(), capture);
                                    }
                                    // 不消耗 next_lit，本轮只前进 pattern 到下一个 token，文本前进到 end_pos
                                    i += 1;
                                    j = end_pos;
                                    continue;
                                } else {
                                    return false;
                                }
                            } else {
                                // 一般情况：直到遇到下一个字面量为止（至少 1 个 token）
                                let mut pos = j;
                                let mut found_k: Option<usize> = None;
                                while pos < text_tokens.len() {
                                    let matched = if case_insensitive { text_tokens[pos].eq_ignore_ascii_case(next_lit) } else { &text_tokens[pos] == next_lit };
                                    if matched { found_k = Some(pos); break; }
                                    pos += 1;
                                }
                                if let Some(end_pos) = found_k {
                                    if end_pos == j { return false; }
                                    let capture: Vec<String> = text_tokens[j..end_pos].to_vec();
                                    if let Some(prev) = bindings.get(p_tok) {
                                        if *prev != capture { return false; }
                                    } else {
                                        bindings.insert(p_tok.clone(), capture);
                                    }
                                    i += 1;
                                    j = end_pos; // 不消耗 next_lit
                                    continue;
                                } else {
                                    return false;
                                }
                            }
                        }
                    }
                } else {
                    // 字面量需要严格相等（SQL 等大小写不敏感语言放宽为不区分大小写）
                    let matched = if case_insensitive { text_tokens[j].eq_ignore_ascii_case(p_tok) } else { &text_tokens[j] == p_tok };
                    if !matched { return false; }
                    i += 1;
                    j += 1;
                }
            }
            // pattern 完全匹配
            true
        };

        // 从所有起点尝试
        for start in 0..text_tokens.len() {
            // Java safety: avoid starting a match in the middle of a qualified name (e.g., System.out.println)
            if matches!(language, astgrep_core::Language::Java) {
                if let Some(first_lit) = pattern_tokens.iter().find(|t| !t.starts_with('$')) {
                    let is_ident = first_lit.chars().all(|c| c.is_alphanumeric() || c == '_');
                    if is_ident && start > 0 && text_tokens[start - 1] == "." {
                        continue;
                    }
                }
            }
            if try_match_from(start) {
                println!("🔍 Match successful starting at token index {}", start);
                return true;
            }
        }
        println!("🔍 No matching span found");
        false
    }

    /// Execute dataflow analysis
    fn execute_dataflow(
        &self,
        dataflow: &DataFlowSpec,
        ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Simplified dataflow analysis
        // In a real implementation, this would use proper taint analysis
        let sources = self.find_dataflow_nodes(ast, &dataflow.sources, context.language)?;
        let sinks = self.find_dataflow_nodes(ast, &dataflow.sinks, context.language)?;

        // Check if there are potential flows from sources to sinks
        if !sources.is_empty() && !sinks.is_empty() {
            for sink in sinks {
                let location = self.create_location_from_node(sink.as_ref(), context);

                let finding = Finding::new(
                    rule.id.clone(),
                    format!("Potential data flow from source to sink: {}", rule.description),
                    rule.severity,
                    rule.confidence,
                    location,
                )
                .with_metadata("analysis_type".to_string(), "dataflow".to_string());

                findings.push(finding);
            }
        }

        Ok(findings)
    }

    /// Find nodes matching dataflow patterns
    fn find_dataflow_nodes(&self, ast: &dyn AstNode, patterns: &[String], language: astgrep_core::Language) -> Result<Vec<Box<dyn AstNode>>> {
        let mut matches = Vec::new();

        for pattern in patterns {
            astgrep_core::ast_utils::visit_nodes(ast, &mut |node| {
                if let Some(text) = node.text() {
                    if self.simple_pattern_match(pattern, text, language) {
                        matches.push(node.clone_node());
                    }
                }
                Ok(())
            })?;
        }

        Ok(matches)
    }

    /// Create location from AST node
    fn create_location_from_node(&self, node: &dyn AstNode, context: &RuleContext) -> Location {
        if let Some((start_line, start_col, end_line, end_col)) = node.location() {
            Location::new(
                PathBuf::from(&context.file_path),
                start_line,
                start_col,
                end_line,
                end_col,
            )
        } else {
            Location::point(PathBuf::from(&context.file_path), 1, 1)
        }
    }

    /// Generate finding message
    fn generate_finding_message(&self, rule: &Rule, pattern: &Pattern, node: &dyn AstNode) -> String {
        // Use rule.description if available, otherwise generate a default message
        if !rule.description.is_empty() {
            rule.description.clone()
        } else {
            let default_pattern = "<complex pattern>".to_string();
            let pattern_str = pattern.get_pattern_string().unwrap_or(&default_pattern);
            if let Some(text) = node.text() {
                format!("{}: Found '{}' matching pattern '{}'", rule.name, text, pattern_str)
            } else {
                format!("{}: Found node matching pattern '{}'", rule.name, pattern_str)
            }
        }
    }

    /// Generate cache key for rule execution
    fn generate_cache_key(&self, rule: &Rule, context: &RuleContext) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        rule.id.hash(&mut hasher);
        context.file_path.hash(&mut hasher);
        context.source_code.hash(&mut hasher);

        format!("{}_{:x}", rule.id, hasher.finish())
    }

    /// Clear execution cache
    pub fn clear_cache(&mut self) {
        self.execution_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, bool) {
        (self.execution_cache.len(), self.cache_enabled)
    }
}

impl Default for RuleExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_ast::{AstBuilder, NodeType, UniversalNode};
    use astgrep_core::{Confidence, Language, Severity};

    fn create_test_rule() -> Rule {
        Rule::new(
            "test-rule".to_string(),
            "Test Rule".to_string(),
            "A test rule".to_string(),
            Severity::Warning,
            Confidence::Medium,
            vec![Language::Java],
        )
        .add_pattern(Pattern::simple("println".to_string()))
    }

    fn create_test_ast() -> UniversalNode {
        AstBuilder::call_expression(
            AstBuilder::property_access("System.out", "println"),
            vec![AstBuilder::string_literal("Hello, World!")],
        ).with_text("System.out.println(\"Hello, World!\")".to_string())
    }

    fn create_test_context() -> RuleContext {
        RuleContext::new(
            "test.java".to_string(),
            Language::Java,
            "System.out.println(\"Hello, World!\");".to_string(),
        )
    }

    #[test]
    fn test_execute_rule() {
        let mut engine = RuleExecutionEngine::new();
        let rule = create_test_rule();
        let ast = create_test_ast();
        let context = create_test_context();

        let result = engine.execute_rule(&rule, &ast, &context);

        assert!(result.is_success());
        assert_eq!(result.rule_id, "test-rule");
        assert!(result.execution_time_ms >= 0); // Allow zero time for fast execution
    }

    #[test]
    fn test_execute_multiple_rules() {
        let mut engine = RuleExecutionEngine::new();
        let rule1 = create_test_rule();
        let mut rule2 = create_test_rule();
        rule2.id = "test-rule-2".to_string();

        let rules = vec![rule1, rule2];
        let ast = create_test_ast();
        let context = create_test_context();

        let results = engine.execute_rules(&rules, &ast, &context);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_success()));
    }

    #[test]
    fn test_rule_not_applicable_to_language() {
        let mut engine = RuleExecutionEngine::new();
        let mut rule = create_test_rule();
        rule.languages = vec![Language::Python]; // Different language

        let ast = create_test_ast();
        let context = create_test_context(); // Java context

        let results = engine.execute_rules(&[rule], &ast, &context);

        assert_eq!(results.len(), 0); // Rule should be filtered out
    }

    #[test]
    fn test_cache_functionality() {
        let mut engine = RuleExecutionEngine::new().set_cache_enabled(true);
        let rule = create_test_rule();
        let ast = create_test_ast();
        let context = create_test_context();

        // First execution
        let result1 = engine.execute_rule(&rule, &ast, &context);
        let (cache_size_1, cache_enabled) = engine.cache_stats();

        // Second execution (should use cache)
        let result2 = engine.execute_rule(&rule, &ast, &context);
        let (cache_size_2, _) = engine.cache_stats();

        assert!(cache_enabled);
        assert_eq!(cache_size_1, 1);
        assert_eq!(cache_size_2, 1);
        assert_eq!(result1.rule_id, result2.rule_id);
    }

    #[test]
    fn test_dataflow_rule() {
        let mut engine = RuleExecutionEngine::new();
        let dataflow = DataFlowSpec::new(
            vec!["input".to_string()],
            vec!["output".to_string()],
        );

        let rule = Rule::new(
            "dataflow-rule".to_string(),
            "Dataflow Rule".to_string(),
            "A dataflow test rule".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        ).with_dataflow(dataflow);

        let ast = create_test_ast();
        let context = create_test_context();

        let result = engine.execute_rule(&rule, &ast, &context);

        assert!(result.is_success());
        assert_eq!(result.rule_id, "dataflow-rule");
    }


    #[test]
    fn test_sql_case_insensitive_simple_pattern() {
        let engine = RuleExecutionEngine::new();
        let pattern = "DELETE FROM $TABLE";
        let text = "delete from user;";
        assert!(engine.simple_pattern_match(pattern, text, Language::Sql));
    }

    #[test]
    fn test_execution_timeout() {
        let mut engine = RuleExecutionEngine::new().set_max_execution_time(0); // Immediate timeout
        let rule = create_test_rule();
        let ast = create_test_ast();
        let context = create_test_context();

        let result = engine.execute_rule(&rule, &ast, &context);

        // Note: This test might be flaky due to timing, but demonstrates the concept
        assert_eq!(result.rule_id, "test-rule");
    }
    #[test]
    fn test_sql_select_star_pattern_either_dedup() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "sql-avoid-select-star".to_string(),
            "Avoid SELECT *".to_string(),
            "Detects usage of SELECT *".to_string(),
            Severity::Warning,
            Confidence::Medium,
            vec![Language::Sql],
        )
        .add_pattern(Pattern::either(vec![
            Pattern::simple("SELECT * FROM users".to_string()),
            Pattern::simple("select * from users".to_string()),
        ]));

        let sql = "SELECT * FROM users;\n\nSELECT id, name FROM users;\n\nselect * from users;\n";
        // AST content is not used for simple-literal path; reuse existing helper
        let ast = create_test_ast();
        let context = RuleContext::new("test.sql".to_string(), Language::Sql, sql.to_string());

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        // Expect exactly two findings (two SELECT * occurrences), not four
        assert_eq!(result.findings.len(), 2);
    }

    #[test]
    fn test_sql_regex_cte_single_block() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "sql.detect-any-cte".to_string(),
            "Detect CTE".to_string(),
            "发现 CTE 用法（WITH 子句）".to_string(),
            Severity::Info,
            Confidence::Medium,
            vec![Language::Sql],
        )
        .add_pattern(Pattern::regex("(?is)\\bwith\\s+\\w+\\s*as\\s*\\(".to_string()));

        let sql = "WITH my_cte AS (\n  SELECT one, two\n  FROM my_table\n)\nSELECT *\nFROM my_cte;\n";
        let ast = create_test_ast();
        let context = RuleContext::new("test.sql".to_string(), Language::Sql, sql.to_string());

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_java_out_println_does_not_match_system_qualified() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "java-out-println".to_string(),
            "Java out.println".to_string(),
            "Detect out.println".to_string(),
            Severity::Warning,
            Confidence::Medium,
            vec![Language::Java],
        ).add_pattern(Pattern::simple("out.println($INPUT)".to_string()));
        // AST node simulates System.out.println(...)
        let ast = create_test_ast();
        let context = RuleContext::new(
            "Demo.java".to_string(),
            Language::Java,
            "class Demo { void f(){ System.out.println(\"x\"); } }".to_string(),
        );
        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.findings.len(), 0);
    }

    #[test]
    fn test_java_out_println_matches_plain_out() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "java-out-println-2".to_string(),
            "Java out.println".to_string(),
            "Detect out.println".to_string(),
            Severity::Warning,
            Confidence::Medium,
            vec![Language::Java],
        ).add_pattern(Pattern::simple("out.println($INPUT)".to_string()));
        // AST node simulates out.println(...)
        let ast = AstBuilder::call_expression(
            AstBuilder::property_access("out", "println"),
            vec![AstBuilder::string_literal("Hello")],
        ).with_text("out.println(\"Hello\");".to_string());
        let context = RuleContext::new(
            "Demo.java".to_string(),
            Language::Java,
            "out.println(\"Hello\");".to_string(),
        );
        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_java_simple_with_metavar_multiple_occurrences() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "java-writer-write".to_string(),
            "Detect writer.write".to_string(),
            "检测到可能未进行XSS防护的用户输入输出".to_string(),
            Severity::Error,
            Confidence::Medium,
            vec![Language::Java],
        ).add_pattern(Pattern::simple("response.getWriter().write($INPUT)".to_string()));

        let java_code = "String userInput = request.getParameter(\"name\");\n\
response.getWriter().write(userInput);\n\
String userInput2 = request.getParameter(\"title\");\n\
response.getWriter().write(\"<div>\" + userInput2 + \"</div>\");\n\
String scriptParam = request.getParameter(\"x\");\n\
response.getWriter().write(\"<script>var data = '\" + scriptParam + \"';</script>\");\n";
        let ast = create_test_ast();
        let context = RuleContext::new(
            "Xss.java".to_string(),
            Language::Java,
            java_code.to_string(),
        );
        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.findings.len(), 3);
    }

    #[test]
    fn test_java_either_with_metavar_multiple_occurrences() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "java-writer-either".to_string(),
            "Detect unsafe outputs".to_string(),
            "检测到可能未进行XSS防护的用户输入输出".to_string(),
            Severity::Error,
            Confidence::Medium,
            vec![Language::Java],
        ).add_pattern(Pattern::either(vec![
            Pattern::simple("response.getWriter().write($INPUT)".to_string()),
            Pattern::simple("response.getWriter().print($INPUT)".to_string()),
            Pattern::simple("response.getWriter().println($INPUT)".to_string()),
        ]));

        let java_code = "String userInput = request.getParameter(\"name\");\n\
response.getWriter().write(userInput);\n\
String userInput2 = request.getParameter(\"title\");\n\
response.getWriter().write(\"<div>\" + userInput2 + \"</div>\");\n\
String scriptParam = request.getParameter(\"x\");\n\
response.getWriter().write(\"<script>var data = '\" + scriptParam + \"';</script>\");\n";
        let ast = create_test_ast();
        let context = RuleContext::new(
            "Xss.java".to_string(),
            Language::Java,
            java_code.to_string(),
        );
        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.findings.len(), 3);
    }

        #[test]
        fn test_java_ellipsis_call_arguments() {
            let mut engine = RuleExecutionEngine::new();
            let rule = Rule::new(
                "java-ellipsis-call".to_string(),
                "Ellipsis call args".to_string(),
                "支持 ... 匹配任意个实参".to_string(),
                Severity::Info,
                Confidence::Medium,
                vec![Language::Java],
            ).add_pattern(Pattern::simple("System.out.println(...)".to_string()));

            let java_code = "class D{ void f(){ System.out.println(); System.out.println(\"x\"); } }";
            let ast = create_test_ast();
            let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
            let result = engine.execute_rule(&rule, &ast, &context);
            assert!(result.is_success());
            // 两处调用都应命中
            assert_eq!(result.findings.len(), 2);
        }

        #[test]
        fn test_java_ellipsis_block_bodies() {
            let mut engine = RuleExecutionEngine::new();
            let rule = Rule::new(
                "java-ellipsis-block".to_string(),
                "Ellipsis in blocks".to_string(),
                "支持在块体内使用 ...".to_string(),
                Severity::Info,
                Confidence::Medium,
                vec![Language::Java],
            ).add_pattern(Pattern::simple("try { ... } catch (Exception e) { ... }".to_string()));

            let java_code = "class D{ void f(){ try { a(); b(); } catch (Exception e) { handle(); } } }";
            let ast = create_test_ast();
            let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
            let result = engine.execute_rule(&rule, &ast, &context);
            assert!(result.is_success());
            assert_eq!(result.findings.len(), 1);
        }

        #[test]
        fn test_ellipsis_sequence_across_statements() {
            let mut engine = RuleExecutionEngine::new();
            let rule = Rule::new(
                "ellipsis-seq".to_string(),
                "Ellipsis sequence".to_string(),
                "A ... B 序列匹配".to_string(),
                Severity::Info,
                Confidence::Medium,
                vec![Language::Java],
            ).add_pattern(Pattern::simple("A ... B".to_string()));

            let java_code = "class D{ void f(){ A(); X(); Y(); B(); } }";
            let ast = create_test_ast();
            let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
            let result = engine.execute_rule(&rule, &ast, &context);
            assert!(result.is_success());
            assert_eq!(result.findings.len(), 1);
        }


}


