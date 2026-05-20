//! Traversal module - Rule execution engine types
//!
//! This module provides type definitions for the rule execution engine.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Result};
use astgrep_matcher::PatternMatcher;
use std::collections::HashMap;

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
        dataflow: &crate::types::DataFlowSpec,
        ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        use crate::executor::AdvancedRuleExecutor;

        let mut executor = AdvancedRuleExecutor::new();

        let findings = executor.execute_taint_analysis(
            rule,
            dataflow,
            ast,
            None,
            Some(std::path::Path::new(&context.file_path)),
        )?;

        Ok(findings)
    }

    /// Find pattern matches using the pattern matcher
    pub(crate) fn find_pattern_matches(
        &self,
        _pattern: &crate::types::Pattern,
        _ast: &dyn AstNode,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new_default() {
        let engine = RuleExecutionEngine::new();
        assert!(engine.parallel_execution);
        assert_eq!(engine.max_execution_time_ms, Some(30000));
        assert!(!engine.cache_enabled);
        assert!(engine.execution_cache.is_empty());
        assert!(engine.constant_values.is_empty());

        let default_engine = RuleExecutionEngine::default();
        assert!(default_engine.parallel_execution);
    }

    #[test]
    fn test_engine_set_parallel_execution() {
        let engine = RuleExecutionEngine::new().set_parallel_execution(false);
        assert!(!engine.parallel_execution);
    }

    #[test]
    fn test_engine_set_max_execution_time() {
        let engine = RuleExecutionEngine::new().set_max_execution_time(5000);
        assert_eq!(engine.max_execution_time_ms, Some(5000));
    }

    #[test]
    fn test_engine_cache_enable_disable() {
        let engine = RuleExecutionEngine::new().set_cache_enabled(true);
        let (count, enabled) = engine.cache_stats();
        assert_eq!(count, 0);
        assert!(enabled);

        let engine = engine.set_cache_enabled(false);
        let (_, enabled) = engine.cache_stats();
        assert!(!enabled);
    }

    #[test]
    fn test_engine_clear_cache() {
        let mut engine = RuleExecutionEngine::new();
        engine.execution_cache.insert("key".to_string(), vec![]);
        assert_eq!(engine.execution_cache.len(), 1);
        engine.clear_cache();
        assert!(engine.execution_cache.is_empty());
    }

    #[test]
    fn test_engine_set_constant_values() {
        use astgrep_dataflow::ConstantValue;
        let mut engine = RuleExecutionEngine::new();
        let mut constants = HashMap::new();
        constants.insert("x".to_string(), ConstantValue::String("hello".to_string()));
        engine.set_constant_values(constants);
        assert_eq!(engine.constant_values.len(), 1);
    }
}
