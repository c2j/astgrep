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
