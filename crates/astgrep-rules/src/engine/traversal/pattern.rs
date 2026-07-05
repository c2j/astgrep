//! Traversal module - Pattern matching
//!
//! This module provides pattern matching logic for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::{AstNode, Finding, Language, Location, Result};
use std::path::PathBuf;
use tracing::debug;

// Helper functions from matching module are available through super::matching

impl RuleExecutionEngine {
    /// Execute pattern matching
    pub(crate) fn execute_pattern(
        &self,
        pattern: &Pattern,
        _ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        debug!("Executing pattern for rule: {}", rule.id);
        debug!("Pattern: {:?}", pattern);

        let mut findings = Vec::new();

        // 1) Regex patterns: run real regex over full source
        if let PatternType::Regex(ref regex_str) = &pattern.pattern_type {
            self.execute_regex_pattern(regex_str, rule, context, &mut findings)?;
            return Ok(findings);
        }

        // 2) Simple patterns (with or without metavariables): scan full source
        if let PatternType::Simple(ref pattern_str) = &pattern.pattern_type {
            return self.execute_simple_pattern(pattern_str, pattern, rule, context, _ast);
        }

        // 3) pattern-all: handle patterns with positive and negative constraints
        if let PatternType::All(ref subs) = &pattern.pattern_type {
            if subs.is_empty() {
                return Ok(findings);
            }
            return self.execute_all_pattern(pattern, subs, rule, context, _ast);
        }

        // 4) pattern-either: handle Regex and Simple alternatives
        if let PatternType::Either(ref subs) = &pattern.pattern_type {
            return self.execute_either_pattern(pattern, subs, rule, context, _ast);
        }

        // 5) Inside/NotInside/Not/NotRegex: delegate to advanced executor
        if matches!(
            &pattern.pattern_type,
            PatternType::Inside(_)
                | PatternType::NotInside(_)
                | PatternType::Not(_)
                | PatternType::NotRegex(_)
        ) {
            return self.execute_advanced_pattern(pattern, rule, context, _ast);
        }

        // Fallback: no simple/regex pattern string available, use node-based matching
        self.execute_fallback_matching(pattern, _ast, rule, context)
    }

    /// Execute regex pattern matching
    fn execute_regex_pattern(
        &self,
        regex_str: &str,
        rule: &Rule,
        context: &RuleContext,
        findings: &mut Vec<Finding>,
    ) -> Result<()> {
        use fancy_regex::Regex;

        match Regex::new(regex_str) {
            Ok(re) => {
                for m in re.find_iter(&context.source_code).filter_map(|m| m.ok()) {
                    let (start_line, start_col) =
                        Self::byte_index_to_line_col(&context.source_code, m.start());
                    let (end_line, end_col) =
                        Self::byte_index_to_line_col(&context.source_code, m.end());

                    let location = Location::new(
                        std::path::PathBuf::from(&context.file_path),
                        start_line,
                        start_col,
                        end_line,
                        end_col,
                    );

                    let matched_text =
                        &context.source_code[m.start()..m.end().min(context.source_code.len())];

                    let finding = Finding::new(
                        rule.id.clone(),
                        if !rule.description.is_empty() {
                            rule.description.clone()
                        } else {
                            format!("Match: {}", matched_text)
                        },
                        rule.severity,
                        rule.confidence,
                        location,
                    )
                    .with_metadata("pattern".to_string(), regex_str.to_string());

                    let finding = if let Some(ref fix) = rule.fix {
                        finding.with_fix(fix.clone())
                    } else {
                        finding
                    };
                    findings.push(finding);
                }
                Ok(())
            }
            Err(e) => Err(astgrep_core::AnalysisError::pattern_match_error(format!(
                "Invalid regex: {}",
                e
            ))),
        }
    }

    /// Execute simple pattern matching
    fn execute_simple_pattern(
        &self,
        pattern_str: &str,
        pattern: &Pattern,
        rule: &Rule,
        context: &RuleContext,
        _ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        use std::collections::HashSet;
        let pattern_str = pattern_str.trim();

        // If pattern has conditions or requires symbolic propagation, use AdvancedRuleExecutor
        let requires_advanced =
            !pattern.conditions.is_empty() || rule.requires_symbolic_propagation();

        // Also use advanced executor if pattern contains literals that might need constant propagation
        // This handles cases like "return 5;" which should match "return x;" where x = 5
        // Also handle patterns like "foo(42)" where 42 is inside parentheses
        let has_literal = pattern_str.split_whitespace().any(|tok| {
            // Try to parse as i64 after trimming punctuation
            let cleaned = tok.trim().trim_end_matches(';').trim_end_matches(',');
            cleaned.parse::<i64>().is_ok()
                || (tok.starts_with('"') && tok.ends_with('"'))
                || (tok.starts_with('\'') && tok.ends_with('\''))
        });

        // Also check for literals inside parentheses like "foo(42)"
        let has_paren_literal = pattern_str.contains('(') && pattern_str.contains(')');

        let needs_constant_prop =
            (has_literal || has_paren_literal) && rule.has_constant_propagation();

        // Note: Previously, patterns with semgrep metavariables ($VAR, ...) were
        // routed to the advanced (AST) executor, but the text matcher now handles
        // them correctly via semgrep_pattern_to_regex conversion.
        // Only route to advanced executor if there are actual conditions or
        // constant propagation needs.

        // Patterns with metadata binding syntax ($VAR@attr) must route through
        // TreeMatcher, as text matching cannot bind to metadata attributes.
        if pattern_str.contains('@') {
            return self.execute_advanced_pattern(pattern, rule, context, _ast);
        }

        if requires_advanced || needs_constant_prop {
            return self.execute_advanced_pattern(pattern, rule, context, _ast);
        }

        if Self::pattern_needs_ast_matching(pattern_str) {
            return self.execute_advanced_pattern(pattern, rule, context, _ast);
        }

        // For patterns containing binary operators, augment text matching with AST matching
        // to handle associative reordering (e.g. A & B should also match B & A)
        let has_binary_op = [
            " + ", " - ", " * ", " / ", " % ", " ** ", " & ", " | ", " ^ ", " && ", " || ",
            " and ", " or ", " xor ", " == ", " != ", " < ", " > ", " <= ", " >= ", " << ", " >> ",
        ]
        .iter()
        .any(|op| pattern_str.contains(op));

        let mut text_findings = Vec::new();
        let seg_by_stmt = if matches!(context.language, Language::Sql) {
            self.effective_sql_stmt_boundary(rule, context)
        } else {
            false
        };
        let spans = self.find_pattern_spans_in_source(
            pattern_str,
            &context.source_code,
            context.language,
            seg_by_stmt,
        );

        let mut seen: HashSet<(usize, usize)> = HashSet::new();

        for (start_byte, end_byte) in spans {
            if !seen.insert((start_byte, end_byte)) {
                continue;
            }
            let (start_line, start_col) =
                Self::byte_index_to_line_col(&context.source_code, start_byte);
            let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);

            let location = Location::new(
                PathBuf::from(&context.file_path),
                start_line,
                start_col,
                end_line,
                end_col,
            );

            let matched_text =
                &context.source_code[start_byte..end_byte.min(context.source_code.len())];

            let finding = Finding::new(
                rule.id.clone(),
                if !rule.description.is_empty() {
                    rule.description.clone()
                } else {
                    format!("Match: {}", matched_text)
                },
                rule.severity,
                rule.confidence,
                location,
            )
            .with_metadata("pattern".to_string(), pattern_str.to_string());

            let finding = if let Some(ref fix) = rule.fix {
                finding.with_fix(fix.clone())
            } else {
                finding
            };
            text_findings.push(finding);
        }

        if !has_binary_op {
            if !text_findings.is_empty() {
                return Ok(text_findings);
            }
            if pattern_str.contains("...") || pattern_str.contains('$') {
                return self.execute_advanced_pattern(pattern, rule, context, _ast);
            }
            return Ok(text_findings);
        }

        // Merge AST findings with text findings (AST handles reordering)
        let ast_findings = self.execute_advanced_pattern(pattern, rule, context, _ast)?;
        let text_locs: HashSet<(usize, usize, usize, usize)> = text_findings
            .iter()
            .map(|f| {
                let l = &f.location;
                (l.start_line, l.start_column, l.end_line, l.end_column)
            })
            .collect();
        for af in ast_findings {
            let l = &af.location;
            let key = (l.start_line, l.start_column, l.end_line, l.end_column);
            if !text_locs.contains(&key) {
                text_findings.push(af);
            }
        }

        Ok(text_findings)
    }

    fn pattern_needs_ast_matching(pattern_str: &str) -> bool {
        pattern_str.contains('{')
            || pattern_str.contains("class ")
            || pattern_str.contains("function ")
            || pattern_str.contains("def ")
            || pattern_str.contains("if ")
            || pattern_str.contains("for ")
            || pattern_str.contains("while ")
            || pattern_str.contains("try ")
            || pattern_str.contains("catch ")
            || pattern_str.contains("import ")
            || pattern_str.contains("from ")
            || pattern_str.contains("implements ")
            || pattern_str.contains("extends ")
            || pattern_str.contains("interface ")
            || pattern_str.contains("record ")
            || pattern_str.contains("@interface ")
            || pattern_str.contains("public ")
            || pattern_str.contains("private ")
            || pattern_str.contains("protected ")
            || pattern_str.contains("return ")
            || pattern_str.contains("throw ")
            || (pattern_str.contains(';') && pattern_str.contains('\n'))
            || pattern_str.starts_with("var ")
            || pattern_str.starts_with("let ")
            || pattern_str.starts_with("const ")
            || pattern_str.contains("new ")
            || pattern_str.contains('@')
            || (pattern_str.contains('(') && pattern_str.contains(')') && pattern_str.contains('$'))
    }

    /// Execute pattern using AdvancedRuleExecutor (for complex patterns)
    fn execute_advanced_pattern(
        &self,
        pattern: &Pattern,
        rule: &Rule,
        context: &RuleContext,
        _ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        use crate::executor::AdvancedRuleExecutor;
        let mut advanced_executor = AdvancedRuleExecutor::new();

        // Create a rule with just this pattern
        let mut single_pattern_rule = rule.clone();
        single_pattern_rule.patterns = vec![pattern.clone()];

        // Execute using advanced executor
        let file_path = std::path::Path::new(&context.file_path);
        let enable_cp = !matches!(
            context.sql_dialect,
            Some(astgrep_core::SqlDialect::GaussDB) | Some(astgrep_core::SqlDialect::OpenGauss)
        );
        let result = advanced_executor.execute_comprehensive_analysis(
            &[single_pattern_rule],
            _ast,
            context.language,
            Some(file_path),
            enable_cp,
            context.sql_dialect,
        )?;

        Ok(result.findings)
    }

    /// Collect text match spans for a single pattern, recursing into
    /// Either/Any sub-patterns. Returns None if the pattern type is too complex
    /// for text-level matching.
    fn collect_spans_for_pattern(
        &self,
        pat: &Pattern,
        source: &str,
        language: Language,
    ) -> Option<Vec<(usize, usize)>> {
        if !pat.conditions.is_empty() {
            return None;
        }
        match &pat.pattern_type {
            PatternType::Simple(s) => {
                Some(self.find_pattern_spans_in_source(s, source, language, false))
            }
            PatternType::Either(alternatives) => {
                let mut union: std::collections::HashSet<(usize, usize)> =
                    std::collections::HashSet::new();
                for alt in alternatives {
                    if let Some(spans) = self.collect_spans_for_pattern(alt, source, language) {
                        for span in spans {
                            union.insert(span);
                        }
                    } else {
                        return None;
                    }
                }
                Some(union.into_iter().collect())
            }
            PatternType::Any(alternatives) => {
                let mut union: std::collections::HashSet<(usize, usize)> =
                    std::collections::HashSet::new();
                for alt in alternatives {
                    if let Some(spans) = self.collect_spans_for_pattern(alt, source, language) {
                        for span in spans {
                            union.insert(span);
                        }
                    } else {
                        return None;
                    }
                }
                Some(union.into_iter().collect())
            }
            PatternType::Not(inner) => self.collect_spans_for_pattern(inner, source, language),
            PatternType::Regex(re) => {
                let mut spans = Vec::new();
                if let Ok(regex) = fancy_regex::Regex::new(re) {
                    for m in regex.find_iter(source).filter_map(|m| m.ok()) {
                        spans.push((m.start(), m.end()));
                    }
                }
                Some(spans)
            }
            PatternType::All(subs) => {
                let mut union: std::collections::HashSet<(usize, usize)> =
                    std::collections::HashSet::new();
                for sub in subs {
                    if let Some(spans) = self.collect_spans_for_pattern(sub, source, language) {
                        union.extend(spans);
                    } else {
                        return None;
                    }
                }
                Some(union.into_iter().collect())
            }
            PatternType::NotRegex(_) | PatternType::Inside(_) | PatternType::NotInside(_) => None,
        }
    }

    /// Execute pattern-all matching
    fn execute_all_pattern(
        &self,
        pattern: &Pattern,
        subs: &[Pattern],
        rule: &Rule,
        context: &RuleContext,
        _ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        // Try text-based matching for decomposable sub-patterns.
        // This handles SQL-in-string patterns and other text-level matches
        // that the AST-based AdvancedRuleExecutor cannot match.
        // Fall back to the advanced executor if the All pattern or any
        // sub-pattern has conditions, or if sub-patterns use Inside/NotInside.

        // Check if the parent All pattern itself has conditions (e.g., metavariable-condition).
        // We try to evaluate conditions textually; only fall back to AdvancedRuleExecutor
        // if conditions can't be evaluated on text matches.
        let has_conditions = !pattern.conditions.is_empty();

        // Collect positive (must match) and negative (must NOT match) spans
        let mut positive_sets: Vec<std::collections::HashSet<(usize, usize)>> = Vec::new();
        let mut negative_set: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();
        let mut can_use_text = true;

        // Structural pattern detection: if any positive sub-pattern requires AST-level
        // structural matching (braces, call syntax with metavars, language keywords),
        // skip text matching entirely and delegate to AdvancedRuleExecutor.
        // The text matcher converts $VAR to flat regex which produces false positives
        // on structural patterns like "$TYPE $METHOD(...) { $...BODY }".
        for sub in subs {
            if let PatternType::Simple(ref s) = sub.pattern_type {
                if Self::pattern_needs_ast_matching(s) {
                    can_use_text = false;
                    debug!(
                        "All pattern sub-pattern '{}' needs AST matching, delegating to AdvancedRuleExecutor",
                        s
                    );
                    break;
                }
            }
        }

        for sub in subs {
            match &sub.pattern_type {
                PatternType::Not(inner) => {
                    if let Some(spans) = self.collect_spans_for_pattern(
                        inner,
                        &context.source_code,
                        context.language,
                    ) {
                        for span in spans {
                            negative_set.insert(span);
                        }
                    } else {
                        can_use_text = false;
                        break;
                    }
                }
                PatternType::NotRegex(_) | PatternType::NotInside(_) => {
                    can_use_text = false;
                    break;
                }
                _ => {
                    if let Some(spans) =
                        self.collect_spans_for_pattern(sub, &context.source_code, context.language)
                    {
                        let set: std::collections::HashSet<(usize, usize)> =
                            spans.into_iter().collect();
                        positive_sets.push(set);
                    } else {
                        can_use_text = false;
                        break;
                    }
                }
            }
        }

        if !can_use_text {
            debug!("Pattern has complex sub-patterns, using AdvancedRuleExecutor for All pattern");
            use crate::executor::AdvancedRuleExecutor;
            let mut advanced_executor = AdvancedRuleExecutor::new();
            let mut single_pattern_rule = rule.clone();
            single_pattern_rule.patterns = vec![pattern.clone()];
            let file_path = std::path::Path::new(&context.file_path);
            let result = advanced_executor.execute_comprehensive_analysis(
                &[single_pattern_rule],
                _ast,
                context.language,
                Some(file_path),
                true,
                context.sql_dialect,
            )?;
            return Ok(result.findings);
        }

        // AND logic: ALL positive patterns must match for the rule to fire.
        // If any positive pattern yields zero matches, the All constraint fails.
        if positive_sets.iter().any(|s| s.is_empty()) {
            return Ok(Vec::new());
        }

        // Union all positive sets for reporting: report findings from all
        // matching patterns (since AND requires each pattern to match somewhere).
        let mut all_spans: Vec<(usize, usize)> = {
            let mut union: std::collections::HashSet<(usize, usize)> =
                std::collections::HashSet::new();
            for set in &positive_sets {
                union.extend(set.iter().copied());
            }
            union.into_iter().collect()
        };

        // Remove spans overlapping with negative patterns
        if !negative_set.is_empty() {
            all_spans.retain(|span| {
                let (s1, e1) = *span;
                // Two byte ranges overlap if s1 < e2 && s2 < e1
                !negative_set.iter().any(|(s2, e2)| s1 < *e2 && *s2 < e1)
            });
        }

        // Apply metavariable conditions from the parent All pattern.
        // A condition on metavariable $X only applies to spans from patterns
        // that actually reference $X. Spans from other patterns pass through
        // unconditionally.
        if has_conditions {
            // Collect the set of metavariables referenced in all conditions
            let mut condition_metavars: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for condition in &pattern.conditions {
                match condition {
                    Condition::MetavariableComparison(c) => {
                        condition_metavars.insert(c.metavariable.clone());
                    }
                    Condition::MetavariableRegex(c) => {
                        condition_metavars.insert(c.metavariable.clone());
                    }
                    _ => {}
                }
            }
            // For each condition metavariable, find which sub-patterns define it
            let mut meta_to_pattern_indices: std::collections::HashMap<String, Vec<usize>> =
                std::collections::HashMap::new();
            for mv in &condition_metavars {
                for (i, sub) in subs.iter().enumerate() {
                    if Self::pattern_references_metavar(sub, mv) {
                        meta_to_pattern_indices
                            .entry(mv.clone())
                            .or_default()
                            .push(i);
                    }
                }
            }
            // Union spans from ALL patterns first
            let mut combined_spans: Vec<(usize, usize)> = Vec::new();
            for set in &positive_sets {
                combined_spans.extend(set.iter().copied());
            }
            // Remove negatives
            combined_spans.retain(|span| !negative_set.contains(span));
            // Now apply conditions: a span from a pattern that defines a condition
            // metavariable must satisfy that condition
            let mut filtered = Vec::new();
            for span in &combined_spans {
                let matched_text =
                    &context.source_code[span.0..span.1.min(context.source_code.len())];
                let mut passes = true;
                for condition in &pattern.conditions {
                    let meta_var = match condition {
                        Condition::MetavariableComparison(c) => Some(&c.metavariable),
                        Condition::MetavariableRegex(c) => Some(&c.metavariable),
                        _ => None,
                    };
                    let Some(mv) = meta_var else {
                        continue;
                    };
                    // Check if this span came from a pattern that defines mv
                    if let Some(indices) = meta_to_pattern_indices.get(mv) {
                        // We don't know exactly which sub-pattern produced this span.
                        // Conservative: if ANY pattern that defines mv COULD have
                        // produced it, check the condition.
                        let affects_this_span = indices.iter().any(|&idx| {
                            if idx < positive_sets.len() {
                                positive_sets[idx].contains(span)
                            } else {
                                false
                            }
                        });
                        if affects_this_span
                            && !self.evaluate_condition_textually(condition, matched_text, subs)
                        {
                            passes = false;
                            break;
                        }
                    }
                }
                if passes {
                    filtered.push(*span);
                }
            }
            all_spans = filtered;
        }

        // Convert spans to findings
        let mut findings = Vec::new();
        let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for (start_byte, end_byte) in all_spans {
            if !seen.insert((start_byte, end_byte)) {
                continue;
            }
            let (start_line, start_col) =
                Self::byte_index_to_line_col(&context.source_code, start_byte);
            let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);
            let location = astgrep_core::Location::new(
                std::path::PathBuf::from(&context.file_path),
                start_line,
                start_col,
                end_line,
                end_col,
            );
            let matched_text =
                &context.source_code[start_byte..end_byte.min(context.source_code.len())];
            let mut finding = astgrep_core::Finding::new(
                rule.id.clone(),
                if !rule.description.is_empty() {
                    rule.description.clone()
                } else {
                    format!("Match: {}", matched_text)
                },
                rule.severity,
                rule.confidence,
                location,
            );
            finding = finding.with_metadata("pattern".to_string(), context.source_code.clone());
            if let Some(ref fix) = rule.fix {
                finding = finding.with_fix(fix.clone());
            }
            findings.push(finding);
        }

        Ok(findings)
    }

    /// Try to evaluate a metavariable condition on a text-matched span.
    /// Returns true if the condition passes or if it can't be evaluated
    /// (caller falls back to AdvancedRuleExecutor).
    /// Returns true if the condition passes or if it can't be evaluated
    /// (caller falls back to AdvancedRuleExecutor).
    fn evaluate_condition_textually(
        &self,
        condition: &Condition,
        matched_text: &str,
        sub_patterns: &[Pattern],
    ) -> bool {
        match condition {
            Condition::MetavariableComparison(comp) => {
                let meta_var = &comp.metavariable;

                // Find the pattern that defines this metavariable (as a Simple pattern)
                let defining_pattern = sub_patterns.iter().find(|p| {
                    if let PatternType::Simple(s) = &p.pattern_type {
                        s.contains(meta_var.as_str())
                    } else if let PatternType::Either(alternatives) = &p.pattern_type {
                        alternatives.iter().any(|alt| {
                            if let PatternType::Simple(s) = &alt.pattern_type {
                                s.contains(meta_var.as_str())
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                });

                let Some(dp) = defining_pattern else {
                    // Metavariable not found in any sub-pattern - skip condition
                    return true;
                };

                let pattern_str = match &dp.pattern_type {
                    PatternType::Simple(s) => Some(s.as_str()),
                    _ => None,
                };
                let Some(pat_str) = pattern_str else {
                    return true;
                };

                // Extract the metavariable value from the matched text.
                // The heuristic: if the pattern contains `"$METAVAR"`, then
                // the metavariable matches content between quotes.
                // Extract by finding the first/last quote in the match.
                let meta_value = if pat_str.contains(&format!("\"{}", meta_var)) {
                    // Find content between innermost quotes in the matched text
                    if let Some(first_quote) = matched_text.find('"') {
                        if let Some(last_quote) = matched_text.rfind('"') {
                            if last_quote > first_quote {
                                Some(&matched_text[first_quote + 1..last_quote])
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    // Fallback: the metavariable matches the entire text
                    Some(matched_text)
                };

                let Some(content) = meta_value else {
                    // Can't extract metavariable value - skip this span
                    return false;
                };

                // Try to evaluate the comparison expression
                // Supported: lines($VAR) > N, $VAR.length() > N, etc.
                let comparison_expr = match &comp.operator {
                    astgrep_core::ComparisonOperator::PythonExpression(ref expr) => expr.as_str(),
                    _ => comp.value.as_str(),
                };
                if comparison_expr.contains("lines(") {
                    let line_count = content.lines().count() as i64;
                    for op_str in &["!=", "==", ">=", "<=", ">", "<"] {
                        if let Some(pos) = comparison_expr.find(op_str) {
                            let right = comparison_expr[pos + op_str.len()..].trim();
                            if let Ok(threshold) = right.parse::<i64>() {
                                let passes = match *op_str {
                                    "==" => line_count == threshold,
                                    "!=" => line_count != threshold,
                                    ">" => line_count > threshold,
                                    "<" => line_count < threshold,
                                    ">=" => line_count >= threshold,
                                    "<=" => line_count <= threshold,
                                    _ => return true,
                                };
                                return passes;
                            }
                        }
                    }
                }
                if comparison_expr.contains("length()") {
                    let content_len = content.len() as i64;
                    let threshold = comparison_expr
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i64>()
                        .unwrap_or(0);
                    for op_str in &[">=", "<=", "==", "!=", ">", "<"] {
                        if comparison_expr.contains(op_str) {
                            let passes = match *op_str {
                                ">" => content_len > threshold,
                                "<" => content_len < threshold,
                                "==" => content_len == threshold,
                                "!=" => content_len != threshold,
                                ">=" => content_len >= threshold,
                                "<=" => content_len <= threshold,
                                _ => return true,
                            };
                            return passes;
                        }
                    }
                }

                // For other comparison types, we can't evaluate textually
                true
            }
            Condition::MetavariableRegex(mr) => {
                // Check if the metavariable content matches a regex
                let meta_var = &mr.metavariable;
                let defining_pattern = sub_patterns.iter().find(|p| {
                    if let PatternType::Simple(s) = &p.pattern_type {
                        s.contains(meta_var.as_str())
                    } else {
                        false
                    }
                });
                let Some(dp) = defining_pattern else {
                    return true;
                };
                let pattern_str = match &dp.pattern_type {
                    PatternType::Simple(s) => Some(s.as_str()),
                    _ => None,
                };
                let Some(pat_str) = pattern_str else {
                    return true;
                };
                let meta_value = if pat_str.contains(&format!("\"{}", meta_var)) {
                    if let Some(first_quote) = matched_text.find('"') {
                        if let Some(last_quote) = matched_text.rfind('"') {
                            if last_quote > first_quote {
                                Some(&matched_text[first_quote + 1..last_quote])
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    Some(matched_text)
                };
                if let Some(content) = meta_value {
                    if let Ok(re) = fancy_regex::Regex::new(&mr.regex) {
                        return re.is_match(content).unwrap_or(false);
                    }
                }
                true // Can't evaluate - pass through
            }
            _ => {
                // Other condition types (MetavariablePattern, etc.) can't be
                // evaluated textually
                true
            }
        }
    }

    /// Check if a pattern (or its sub-patterns for Either/All) references a given metavariable.
    fn pattern_references_metavar(pat: &Pattern, metavar: &str) -> bool {
        match &pat.pattern_type {
            PatternType::Simple(s) => s.contains(metavar),
            PatternType::Regex(r) => r.contains(metavar),
            PatternType::Either(alternatives) => alternatives
                .iter()
                .any(|alt| Self::pattern_references_metavar(alt, metavar)),
            PatternType::Any(alternatives) => alternatives
                .iter()
                .any(|alt| Self::pattern_references_metavar(alt, metavar)),
            PatternType::All(subs) => subs
                .iter()
                .any(|sub| Self::pattern_references_metavar(sub, metavar)),
            PatternType::Not(inner) => Self::pattern_references_metavar(inner, metavar),
            PatternType::Inside(inner) => Self::pattern_references_metavar(inner, metavar),
            PatternType::NotInside(inner) => Self::pattern_references_metavar(inner, metavar),
            PatternType::NotRegex(_) => false,
        }
    }

    /// Execute pattern-either matching
    fn execute_either_pattern(
        &self,
        _pattern: &Pattern,
        subs: &[Pattern],
        rule: &Rule,
        context: &RuleContext,
        _ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        use std::collections::HashSet;
        let mut findings = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();

        for sub in subs {
            // If sub-pattern has conditions, use advanced executor
            if !sub.conditions.is_empty() {
                let sub_findings = self.execute_advanced_pattern(sub, rule, context, _ast)?;
                findings.extend(sub_findings);
                continue;
            }
            match &sub.pattern_type {
                PatternType::Regex(r) => {
                    self.execute_regex_subpattern(r, rule, context, &mut findings, &mut seen)?;
                }
                PatternType::Simple(s) => {
                    self.execute_simple_subpattern(s, rule, context, &mut findings, &mut seen)?;
                }
                PatternType::All(_)
                | PatternType::Either(_)
                | PatternType::Inside(_)
                | PatternType::NotInside(_)
                | PatternType::Not(_)
                | PatternType::NotRegex(_)
                | PatternType::Any(_) => {
                    let sub_findings = self.execute_advanced_pattern(sub, rule, context, _ast)?;
                    for f in sub_findings {
                        let key = (
                            f.location.start_line * 10000 + f.location.start_column,
                            f.location.end_line * 10000 + f.location.end_column,
                        );
                        if seen.insert(key) {
                            findings.push(f);
                        }
                    }
                }
            }
        }

        Ok(findings)
    }

    /// Execute regex sub-pattern for Either pattern
    fn execute_regex_subpattern(
        &self,
        regex_str: &str,
        rule: &Rule,
        context: &RuleContext,
        findings: &mut Vec<astgrep_core::Finding>,
        seen: &mut std::collections::HashSet<(usize, usize)>,
    ) -> Result<()> {
        if let Ok(re) = fancy_regex::Regex::new(regex_str) {
            for m in re.find_iter(&context.source_code).filter_map(|m| m.ok()) {
                let start_byte = m.start();
                let end_byte = m.end();
                if !seen.insert((start_byte, end_byte)) {
                    continue;
                }
                let (start_line, start_col) =
                    Self::byte_index_to_line_col(&context.source_code, start_byte);
                let (end_line, end_col) =
                    Self::byte_index_to_line_col(&context.source_code, end_byte);
                let location = astgrep_core::Location::new(
                    std::path::PathBuf::from(&context.file_path),
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                );
                let matched_text =
                    &context.source_code[start_byte..end_byte.min(context.source_code.len())];
                let mut finding = astgrep_core::Finding::new(
                    rule.id.clone(),
                    if !rule.description.is_empty() {
                        rule.description.clone()
                    } else {
                        format!("Match: {}", matched_text)
                    },
                    rule.severity,
                    rule.confidence,
                    location,
                );
                finding = finding.with_metadata("pattern".to_string(), regex_str.to_string());
                if let Some(ref fix) = rule.fix {
                    finding = finding.with_fix(fix.clone());
                }
                findings.push(finding);
            }
        }
        Ok(())
    }

    /// Execute simple sub-pattern for Either pattern
    fn execute_simple_subpattern(
        &self,
        pattern_str: &str,
        rule: &Rule,
        context: &RuleContext,
        findings: &mut Vec<astgrep_core::Finding>,
        seen: &mut std::collections::HashSet<(usize, usize)>,
    ) -> Result<()> {
        let seg_by_stmt = if matches!(context.language, Language::Sql) {
            self.effective_sql_stmt_boundary(rule, context)
        } else {
            false
        };
        let spans = self.find_pattern_spans_in_source(
            pattern_str,
            &context.source_code,
            context.language,
            seg_by_stmt,
        );

        for (start_byte, end_byte) in spans {
            if !seen.insert((start_byte, end_byte)) {
                continue;
            }
            let (start_line, start_col) =
                Self::byte_index_to_line_col(&context.source_code, start_byte);
            let (end_line, end_col) = Self::byte_index_to_line_col(&context.source_code, end_byte);
            let location = astgrep_core::Location::new(
                std::path::PathBuf::from(&context.file_path),
                start_line,
                start_col,
                end_line,
                end_col,
            );
            let matched_text =
                &context.source_code[start_byte..end_byte.min(context.source_code.len())];
            let mut finding = astgrep_core::Finding::new(
                rule.id.clone(),
                if !rule.description.is_empty() {
                    rule.description.clone()
                } else {
                    format!("Match: {}", matched_text)
                },
                rule.severity,
                rule.confidence,
                location,
            );
            finding = finding.with_metadata("pattern".to_string(), pattern_str.to_string());
            if let Some(ref fix) = rule.fix {
                finding = finding.with_fix(fix.clone());
            }
            findings.push(finding);
        }
        Ok(())
    }

    /// Execute fallback matching when no simple/regex pattern available
    fn execute_fallback_matching(
        &self,
        pattern: &Pattern,
        ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let matches =
            self.find_pattern_matches(pattern, ast, context.language, &context.source_code)?;

        // Keep only smallest, non-overlapping node spans
        let mm: Vec<((usize, usize), usize, usize, usize, usize, Box<dyn AstNode>)> = matches
            .into_iter()
            .map(|m: Box<dyn AstNode>| {
                if let Some((sl, sc, el, ec)) = m.location() {
                    let dl = el.saturating_sub(sl);
                    let dc = ec.saturating_sub(sc);
                    ((dl, dc), sl, sc, el, ec, m)
                } else {
                    ((usize::MAX, usize::MAX), 0, 0, usize::MAX, usize::MAX, m)
                }
            })
            .collect();

        let selected = Self::select_non_overlapping(mm);

        for match_node in selected {
            let location = self.create_best_location_from_node_or_pattern(
                match_node.as_ref(),
                pattern,
                context,
            );
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

    /// Select non-overlapping matches (keep smallest)
    fn select_non_overlapping(
        mm: Vec<((usize, usize), usize, usize, usize, usize, Box<dyn AstNode>)>,
    ) -> Vec<Box<dyn AstNode>> {
        let mut sorted = mm;
        sorted.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| (a.1, a.2, a.3, a.4).cmp(&(b.1, b.2, b.3, b.4)))
        });

        let overlaps = |a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)| -> bool {
            let (a_sl, a_sc, a_el, a_ec) = a;
            let (b_sl, b_sc, b_el, b_ec) = b;
            if a_el < b_sl || b_el < a_sl {
                return false;
            }
            if a_sl == b_el && a_sc >= b_ec {
                return false;
            }
            if b_sl == a_el && b_sc >= a_ec {
                return false;
            }
            true
        };

        let mut selected_spans: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut filtered_nodes: Vec<Box<dyn AstNode>> = Vec::new();

        'outer: for (_, sl, sc, el, ec, m) in sorted {
            for s in &selected_spans {
                if overlaps((sl, sc, el, ec), *s) {
                    continue 'outer;
                }
            }
            selected_spans.push((sl, sc, el, ec));
            filtered_nodes.push(m);
        }

        filtered_nodes
    }
}
