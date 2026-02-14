//! Traversal module - Rule execution engine types
//!
//! This module provides type definitions for the rule execution engine.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Location, Result};
use astgrep_matcher::PatternMatcher;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Taint match information
pub struct TaintMatch {
    pub node: Box<dyn AstNode>,
    pub bindings: HashMap<String, String>,
    pub var_name: Option<String>,
}

/// Rule execution engine
pub struct RuleExecutionEngine {
    pub(crate) parallel_execution: bool,
    pub(crate) max_execution_time_ms: Option<u64>,
    pub(crate) cache_enabled: bool,
    pub(crate) execution_cache: HashMap<String, Vec<Finding>>,
    /// Constant propagation values: variable name -> constant value
    pub(crate) constant_values: HashMap<String, astgrep_dataflow::ConstantValue>,
    pub(crate) pattern_matcher: PatternMatcher,
}

impl RuleExecutionEngine {
    /// Create a new rule execution engine
    pub fn new() -> Self {
        Self {
            parallel_execution: true,
            max_execution_time_ms: Some(30000), // 30 seconds default
            cache_enabled: false,
            execution_cache: HashMap::new(),
            constant_values: HashMap::new(),
            pattern_matcher: PatternMatcher::new(),
        }
    }

    /// Set constant propagation values
    pub fn set_constant_values(
        &mut self,
        constants: HashMap<String, astgrep_dataflow::ConstantValue>,
    ) {
        self.constant_values = constants;
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

    /// Determine effective SQL statement boundary setting
    pub(crate) fn effective_sql_stmt_boundary(&self, rule: &Rule, ctx: &RuleContext) -> bool {
        // Check rule-level setting first
        if let Some(val) = rule.sql_stmt_boundary {
            return val;
        }
        // Fall back to context-level setting
        ctx.sql_stmt_boundary.unwrap_or(false)
    }

    /// Find pattern spans in source code
    pub(crate) fn find_pattern_spans_in_source(
        &self,
        pattern: &str,
        source: &str,
        _language: astgrep_core::Language,
        sql_stmt_boundary: bool,
    ) -> Vec<(usize, usize)> {
        use regex::Regex;

        let mut spans = Vec::new();

        // Escape special regex characters but keep the pattern as literal
        let escaped = regex::escape(pattern);

        // Try to compile as regex
        match Regex::new(&escaped) {
            Ok(re) => {
                if sql_stmt_boundary {
                    // For SQL, split by statements and match within each
                    for stmt in source.split(';') {
                        for mat in re.find_iter(stmt) {
                            spans.push((mat.start(), mat.end()));
                        }
                    }
                } else {
                    // Normal matching
                    for mat in re.find_iter(source) {
                        spans.push((mat.start(), mat.end()));
                    }
                }
            }
            Err(_) => {
                // If regex fails, do simple string search
                let mut start = 0;
                while let Some(pos) = source[start..].find(pattern) {
                    let match_start = start + pos;
                    let match_end = match_start + pattern.len();
                    spans.push((match_start, match_end));
                    start = match_end;
                }
            }
        }

        spans
    }

    /// Approximate span from pattern by searching literal anchors
    pub(crate) fn approximate_span_from_pattern(
        source: &str,
        pattern: &str,
    ) -> Option<(usize, usize)> {
        // Extract literal parts from pattern (simplified approach)
        let literals: Vec<&str> = pattern
            .split(|c| {
                c == '('
                    || c == ')'
                    || c == '|'
                    || c == '*'
                    || c == '+'
                    || c == '?'
                    || c == '['
                    || c == ']'
            })
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        if literals.is_empty() {
            // Fallback: try to find the whole pattern
            return source
                .find(pattern)
                .map(|start| (start, start + pattern.len()));
        }

        // Try to find the first literal in source
        for literal in &literals {
            if let Some(start) = source.find(literal) {
                // Estimate end based on pattern length
                let end = (start + literal.len() + pattern.len() / 2).min(source.len());
                return Some((start, end));
            }
        }

        None
    }

    /// Execute taint mode analysis
    pub(crate) fn execute_taint_mode(
        &self,
        _dataflow: &crate::types::DataFlowSpec,
        _ast: &dyn AstNode,
        rule: &Rule,
        _context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        // TODO: Implement taint analysis using the dataflow crate
        // For now, return an empty result with a note that taint analysis is not yet implemented
        let findings = Vec::new();

        // Log that taint analysis is not implemented
        eprintln!(
            "Taint analysis for rule '{}' is not yet implemented",
            rule.id
        );

        Ok(findings)
    }

    /// Find pattern matches using the pattern matcher
    pub(crate) fn find_pattern_matches(
        &self,
        pattern: &crate::types::Pattern,
        ast: &dyn AstNode,
        _language: astgrep_core::Language,
        _source: &str,
    ) -> Result<Vec<Box<dyn AstNode>>> {
        // TODO: Implement proper pattern matching
        // For now, return an empty vector
        // This is a placeholder until the pattern matcher is properly integrated
        Ok(Vec::new())
    }

    /// Enable or disable execution caching
    pub fn set_cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        if !enabled {
            self.execution_cache.clear();
        }
        self
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
