//! Traversal module - Rule execution
//!
//! This module provides core rule execution logic for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::{AstNode, Finding, Result};
use std::time::Instant;

impl RuleExecutionEngine {
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

    /// Execute rules in parallel (placeholder)
    fn execute_rules_parallel(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> Vec<RuleResult> {
        // For now, fall back to sequential execution
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

        // For taint mode, use special handling
        if rule.mode == crate::types::RuleMode::Taint {
            println!("🔍 Rule is in taint mode");
            if let Some(ref dataflow) = rule.dataflow {
                match self.execute_taint_mode(dataflow, ast, rule, context) {
                    Ok(mut taint_findings) => findings.append(&mut taint_findings),
                    Err(e) => {
                        return RuleResult::error(
                            rule.id.clone(),
                            format!("Taint analysis error: {}", e),
                            start_time.elapsed().as_millis() as u64,
                        );
                    }
                }
            }
            return RuleResult::success(
                rule.id.clone(),
                findings,
                start_time.elapsed().as_millis() as u64,
            );
        }

        // Execute pattern matching
        for (i, pattern) in rule.patterns.iter().enumerate() {
            println!("🔍 Processing pattern {} of {}", i + 1, rule.patterns.len());
            match self.execute_pattern(pattern, ast, rule, context) {
                Ok(mut pattern_findings) => {
                    println!(
                        "🔍 Pattern {} generated {} findings",
                        i + 1,
                        pattern_findings.len()
                    );
                    findings.append(&mut pattern_findings)
                }
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
}
