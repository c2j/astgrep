//! Traversal module - Rule execution
//!
//! This module provides core rule execution logic for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::AstNode;
use std::time::Instant;
use tracing::debug;

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

        debug!("Executing rule: {}", rule.id);
        debug!("Rule has {} patterns", rule.patterns.len());

        let mut findings = Vec::new();

        // For taint mode, use special handling
        if rule.mode == crate::types::RuleMode::Taint {
            debug!("Rule is in taint mode");
            if let Some(ref dataflow) = rule.dataflow {
                match self.execute_taint_mode(dataflow, ast, rule, context) {
                    Ok(mut taint_findings) => {
                        findings.append(&mut taint_findings);
                    }
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
            debug!("Processing pattern {} of {}", i + 1, rule.patterns.len());
            match self.execute_pattern(pattern, ast, rule, context) {
                Ok(mut pattern_findings) => {
                    debug!(
                        "Pattern {} generated {} findings",
                        i + 1,
                        pattern_findings.len()
                    );
                    findings.append(&mut pattern_findings)
                }
                Err(e) => {
                    debug!("Pattern {} failed with error: {}", i + 1, e);
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

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::{AstNode, Language, Severity, Confidence};

    #[derive(Clone)]
    struct MockAstNode {
        node_type: String,
        text: Option<String>,
        loc: Option<(usize, usize, usize, usize)>,
        children: Vec<MockAstNode>,
    }

    impl MockAstNode {
        fn new(node_type: &str) -> Self {
            Self {
                node_type: node_type.to_string(),
                text: None,
                loc: None,
                children: Vec::new(),
            }
        }

        fn with_text(mut self, text: &str) -> Self {
            self.text = Some(text.to_string());
            self
        }
    }

    impl AstNode for MockAstNode {
        fn node_type(&self) -> &str {
            &self.node_type
        }

        fn child_count(&self) -> usize {
            self.children.len()
        }

        fn child(&self, index: usize) -> Option<&dyn AstNode> {
            self.children.get(index).map(|c| c as &dyn AstNode)
        }

        fn location(&self) -> Option<(usize, usize, usize, usize)> {
            self.loc
        }

        fn text(&self) -> Option<&str> {
            self.text.as_deref()
        }

        fn clone_node(&self) -> Box<dyn AstNode> {
            Box::new(self.clone())
        }
    }

    fn create_test_rule(id: &str, pattern: &str) -> Rule {
        Rule::new(
            id.to_string(),
            id.to_string(),
            "Test description".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .add_pattern(Pattern::simple(pattern.to_string()))
    }

    fn create_test_context(source: &str) -> RuleContext {
        RuleContext::new(
            "test.java".to_string(),
            Language::Java,
            source.to_string(),
        )
    }

    #[test]
    fn test_execute_rule_simple_pattern() {
        let mut engine = RuleExecutionEngine::new();
        let rule = create_test_rule("test-rule", "foo");
        let ast = MockAstNode::new("program").with_text("foo bar foo");
        let context = create_test_context("foo bar foo");

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.finding_count(), 2);
        assert_eq!(result.rule_id, "test-rule");
    }

    #[test]
    fn test_execute_rule_no_match() {
        let mut engine = RuleExecutionEngine::new();
        let rule = create_test_rule("no-match", "xyz");
        let ast = MockAstNode::new("program").with_text("foo bar baz");
        let context = create_test_context("foo bar baz");

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.finding_count(), 0);
    }

    #[test]
    fn test_execute_rules_filters_by_language() {
        let mut engine = RuleExecutionEngine::new().set_parallel_execution(false);
        let java_rule = create_test_rule("java-rule", "foo");
        let python_rule = Rule::new(
            "python-rule".to_string(),
            "python-rule".to_string(),
            "Test".to_string(),
            Severity::Warning,
            Confidence::Medium,
            vec![Language::Python],
        )
        .add_pattern(Pattern::simple("bar".to_string()));

        let ast = MockAstNode::new("program").with_text("foo bar");
        let context = create_test_context("foo bar");

        let results = engine.execute_rules(&[java_rule, python_rule], &ast, &context);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rule_id, "java-rule");
    }

    #[test]
    fn test_execute_rules_empty() {
        let mut engine = RuleExecutionEngine::new();
        let ast = MockAstNode::new("program");
        let context = create_test_context("");

        let results = engine.execute_rules(&[], &ast, &context);
        assert!(results.is_empty());
    }

    #[test]
    fn test_execute_rule_with_cache() {
        let mut engine = RuleExecutionEngine::new().set_cache_enabled(true);
        let rule = create_test_rule("cached-rule", "test");
        let ast = MockAstNode::new("program").with_text("test content");
        let context = create_test_context("test content");

        let result1 = engine.execute_rule(&rule, &ast, &context);
        assert!(result1.is_success());
        assert_eq!(result1.finding_count(), 1);

        let result2 = engine.execute_rule(&rule, &ast, &context);
        assert!(result2.is_success());
        assert_eq!(result2.finding_count(), 1);

        let (count, enabled) = engine.cache_stats();
        assert_eq!(count, 1);
        assert!(enabled);
    }

    #[test]
    fn test_execute_rule_disabled_rule() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "disabled-rule".to_string(),
            "disabled-rule".to_string(),
            "Test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .set_enabled(false)
        .add_pattern(Pattern::simple("foo".to_string()));

        let ast = MockAstNode::new("program").with_text("foo");
        let context = create_test_context("foo");

        let results = engine.execute_rules(&[rule], &ast, &context);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_execute_rule_regex_pattern() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "regex-rule".to_string(),
            "regex-rule".to_string(),
            "Test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .add_pattern(Pattern::regex(r"\d+".to_string()));

        let ast = MockAstNode::new("program").with_text("abc 123 def 456");
        let context = create_test_context("abc 123 def 456");

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.finding_count(), 2);
    }

    #[test]
    fn test_execute_rule_with_fix() {
        let mut engine = RuleExecutionEngine::new();
        let rule = Rule::new(
            "fix-rule".to_string(),
            "fix-rule".to_string(),
            "Test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .add_pattern(Pattern::simple("old".to_string()))
        .with_fix("new".to_string());

        let ast = MockAstNode::new("program").with_text("old code");
        let context = create_test_context("old code");

        let result = engine.execute_rule(&rule, &ast, &context);
        assert!(result.is_success());
        assert_eq!(result.finding_count(), 1);
        assert!(result.findings[0].fix_suggestion.is_some());
        assert_eq!(result.findings[0].fix_suggestion.as_ref().unwrap(), "new");
    }
}
