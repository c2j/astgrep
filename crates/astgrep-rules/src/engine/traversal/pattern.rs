//! Traversal module - Pattern matching
//!
//! This module provides pattern matching logic for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::{AstNode, Finding, Language, Location, Result};
use std::collections::HashSet;
use std::path::PathBuf;

impl RuleExecutionEngine {
    /// Execute pattern matching
    pub(crate) fn execute_pattern(
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
        use regex::Regex;

        match Regex::new(regex_str) {
            Ok(re) => {
                for m in re.find_iter(&context.source_code) {
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
                    .with_metadata("pattern".to_string(), regex_str.clone());

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
        let mut findings = Vec::new();

        // If pattern has conditions or requires symbolic propagation, use AdvancedRuleExecutor
        let requires_advanced =
            !pattern.conditions.is_empty() || rule.requires_symbolic_propagation();
        if requires_advanced {
            return self.execute_advanced_pattern(pattern, rule, context, _ast);
        }

        let seg_by_stmt = if matches!(context.language, Language::Sql) {
            Self::effective_sql_stmt_boundary(rule, context)
        } else {
            false
        };
        let spans = self.find_pattern_spans_in_source(
            pattern_str,
            &context.source_code,
            context.language,
            seg_by_stmt,
        );

        // Optional: deduplicate identical spans
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
            .with_metadata("pattern".to_string(), pattern_str.clone());

            let finding = if let Some(ref fix) = rule.fix {
                finding.with_fix(fix.clone())
            } else {
                finding
            };
            findings.push(finding);
        }

        Ok(findings)
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
        let result = advanced_executor.execute_comprehensive_analysis(
            &[single_pattern_rule],
            _ast,
            context.language,
            Some(file_path),
            true, // enable constant propagation
        )?;

        Ok(result.findings)
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
        use crate::executor::AdvancedRuleExecutor;

        // Use AdvancedRuleExecutor for proper AST-based matching
        println!("🔍 Pattern has {} sub-patterns and {} conditions, using AdvancedRuleExecutor for All pattern", subs.len(), pattern.conditions.len());
        let mut advanced_executor = AdvancedRuleExecutor::new();

        // Create a rule with the combined pattern
        let mut single_pattern_rule = rule.clone();
        single_pattern_rule.patterns = vec![pattern.clone()];

        // Execute using advanced executor
        let file_path = std::path::Path::new(&context.file_path);
        let result = advanced_executor.execute_comprehensive_analysis(
            &[single_pattern_rule],
            _ast,
            context.language,
            Some(file_path),
            true,
        )?;

        Ok(result.findings)
    }

    /// Execute pattern-either matching
    fn execute_either_pattern(
        &self,
        pattern: &Pattern,
        subs: &[Pattern],
        rule: &Rule,
        context: &RuleContext,
        _ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        use std::collections::HashSet;
        let mut findings = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();

        for sub in subs {
            match &sub.pattern_type {
                PatternType::Regex(r) => {
                    self.execute_regex_subpattern(r, rule, context, &mut findings, &mut seen)?;
                }
                PatternType::Simple(s) => {
                    self.execute_simple_subpattern(s, rule, context, &mut findings, &mut seen)?;
                }
                PatternType::All(_) => {
                    // For complex patterns, use AdvancedRuleExecutor
                    let sub_findings = self.execute_advanced_pattern(sub, rule, context, _ast)?;
                    findings.extend(sub_findings);
                }
                _ => {}
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
        use regex::Regex;

        if let Ok(re) = Regex::new(regex_str) {
            for m in re.find_iter(&context.source_code) {
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
                finding = finding.with_metadata("pattern".to_string(), regex_str.clone());
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
            Self::effective_sql_stmt_boundary(rule, context)
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
            finding = finding.with_metadata("pattern".to_string(), pattern_str.clone());
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
        use astgrep_core::ast_utils;

        let mut findings = Vec::new();
        let matches =
            self.find_pattern_matches(pattern, ast, context.language, &context.source_code)?;

        // Keep only smallest, non-overlapping node spans
        let mm: Vec<((usize, usize), usize, usize, usize, usize, Box<dyn AstNode>)> = matches
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
        use std::cmp::Ordering;

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
