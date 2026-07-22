//! Traversal module - Rule execution
//!
//! This module provides core rule execution logic for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::engine::traversal::text_pattern::TextPattern;
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

    /// Execute rules sequentially with batched literal pattern matching.
    /// Literal patterns (no metavariables) are matched via Aho-Corasick in a
    /// single pass over the source. Rules with only literal patterns that had
    /// no matches skip individual execution entirely.
    fn execute_rules_sequential(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        context: &RuleContext,
    ) -> Vec<RuleResult> {
        // Pre-classify patterns once for this execution batch
        self.classify_patterns(rules);

        // Run batched literal matching if there are any literal patterns
        let literal_matches: Option<
            std::collections::HashMap<String, Vec<(usize, usize, usize)>>,
        > = if let Some((ref matcher, _)) = self.classified_patterns {
            let seg_by_stmt = if matches!(context.language, astgrep_core::Language::Sql) {
                rules
                    .iter()
                    .any(|r| self.effective_sql_stmt_boundary(r, context))
            } else {
                false
            };
            let raw_matches: Vec<(String, usize, usize, usize)> = if seg_by_stmt {
                self.scan_source_segmented(matcher, &context.source_code)
            } else {
                matcher
                    .scan(&context.source_code)
                    .into_iter()
                    .map(|(rid, pi, s, e)| (rid.to_string(), pi, s, e))
                    .collect()
            };
            let mut map: std::collections::HashMap<String, Vec<(usize, usize, usize)>> =
                std::collections::HashMap::new();
            for (rid, pi, start, end) in raw_matches {
                map.entry(rid)
                    .or_default()
                    .push((pi, start, end));
            }
            Some(map)
        } else {
            None
        };

        let applicable: Vec<&Rule> = rules
            .iter()
            .filter(|r| r.applies_to(context.language))
            .collect();

        let mut results = Vec::with_capacity(applicable.len());
        for rule in applicable {
            let used_batch = if let Some(ref match_map) = literal_matches {
                if let Some(rule_matches) = match_map.get(&rule.id) {
                    let has_non_literal = rule.patterns.iter().any(|p| {
                        !matches!(&p.pattern_type, crate::types::PatternType::Simple(_))
                            || !p.conditions.is_empty()
                    });

                    let mut findings =
                        self.create_findings_from_literal_matches(rule_matches, rule, context);

                    if !has_non_literal {
                        results.push(RuleResult::success(
                            rule.id.clone(),
                            findings,
                            0,
                        ));
                        true
                    } else {
                        // Mixed patterns: fall through to individual execution
                        // to avoid re-matching the literal patterns twice.
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !used_batch {
                results.push(self.execute_rule(rule, ast, context));
            }
        }
        results
    }

    /// Scan source split by SQL statement boundaries, adjusting offsets to
    /// absolute positions in the original source.
    fn scan_source_segmented(
        &self,
        matcher: &crate::engine::traversal::text_pattern::LiteralPatternMatcher,
        source: &str,
    ) -> Vec<(String, usize, usize, usize)> {
        let mut results = Vec::new();
        let mut offset = 0usize;
        for stmt in source.split(';') {
            let stmt_len = stmt.len();
            for (rid, pi, start, end) in matcher.scan(stmt) {
                results.push((rid.to_string(), pi, offset + start, offset + end));
            }
            offset += stmt_len + 1;
        }
        results
    }

    /// Create Finding objects from batched literal match results.
    fn create_findings_from_literal_matches(
        &self,
        matches: &[(usize, usize, usize)],
        rule: &Rule,
        context: &RuleContext,
    ) -> Vec<astgrep_core::Finding> {
        use std::collections::HashSet;
        let mut findings = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        for (_pi, start_byte, end_byte) in matches {
            if !seen.insert((*start_byte, *end_byte)) {
                continue;
            }
            let (start_line, start_col) =
                Self::byte_index_to_line_col(&context.source_code, *start_byte);
            let (end_line, end_col) =
                Self::byte_index_to_line_col(&context.source_code, *end_byte);
            let location = astgrep_core::Location::new(
                std::path::PathBuf::from(&context.file_path),
                start_line,
                start_col,
                end_line,
                end_col,
            );
            let matched_text =
                &context.source_code[*start_byte..(*end_byte).min(context.source_code.len())];
            let finding = astgrep_core::Finding::new(
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
            .with_metadata("pattern".to_string(), matched_text.to_string());
            let finding = if let Some(ref fix) = rule.fix {
                finding.with_fix(fix.clone())
            } else {
                finding
            };
            findings.push(finding);
        }
        findings
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
        &mut self,
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
    use astgrep_core::{AstNode, Confidence, Language, Severity};

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
        RuleContext::new("test.java".to_string(), Language::Java, source.to_string())
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

    #[test]
    fn test_multi_rule_combined_equals_sum_of_individual() {
        let source = r#"
            SELECT * FROM users WHERE id = 1;
            DELETE FROM logs WHERE ts < now();
            UPDATE accounts SET balance = 0 WHERE status = 'inactive';
            SELECT name, email FROM users;
        "#;
        let rules = vec![
            create_test_rule("rule-select", "SELECT"),
            create_test_rule("rule-delete", "DELETE"),
            create_test_rule("rule-update", "UPDATE"),
            create_test_rule("rule-where", "WHERE"),
        ];
        let ast = MockAstNode::new("program").with_text(source);
        let context = create_test_context(source);
        let mut engine_combined = RuleExecutionEngine::new().set_parallel_execution(false);
        let combined_results = engine_combined.execute_rules(&rules, &ast, &context);
        let mut per_rule_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for rule in &rules {
            let mut engine_single = RuleExecutionEngine::new();
            let result = engine_single.execute_rule(rule, &ast, &context);
            per_rule_counts.insert(rule.id.clone(), result.finding_count());
        }
        for combined in &combined_results {
            let expected = per_rule_counts.get(&combined.rule_id).unwrap();
            assert_eq!(combined.finding_count(), *expected);
        }
        assert_eq!(combined_results.len(), rules.len());
    }

    #[test]
    fn test_multi_rule_overlapping_patterns_both_match() {
        let source = "SELECT * FROM accounts FOR UPDATE;";
        let rule_broad = create_test_rule("broad-select", "SELECT");
        let rule_specific = create_test_rule("specific-for-update", "FOR UPDATE");
        let rules = vec![rule_broad, rule_specific];
        let ast = MockAstNode::new("program").with_text(source);
        let context = create_test_context(source);
        let mut engine = RuleExecutionEngine::new().set_parallel_execution(false);
        let combined = engine.execute_rules(&rules, &ast, &context);
        let combined_broad = combined.iter().find(|r| r.rule_id == "broad-select").unwrap();
        let combined_specific = combined.iter().find(|r| r.rule_id == "specific-for-update").unwrap();
        assert_eq!(combined_broad.finding_count(), 1);
        assert_eq!(combined_specific.finding_count(), 1);
    }

    #[test]
    fn test_multi_rule_non_overlapping_no_interference() {
        let source = "CREATE TABLE t1 (a INT); DROP TABLE t2; ALTER TABLE t3 ADD b TEXT;";
        let rules = vec![
            create_test_rule("detect-create", "CREATE TABLE"),
            create_test_rule("detect-drop", "DROP TABLE"),
            create_test_rule("detect-alter", "ALTER TABLE"),
        ];
        let ast = MockAstNode::new("program").with_text(source);
        let context = create_test_context(source);
        let mut engine = RuleExecutionEngine::new().set_parallel_execution(false);
        let combined = engine.execute_rules(&rules, &ast, &context);
        for (rid, exp) in &[("detect-create", 1usize), ("detect-drop", 1), ("detect-alter", 1)] {
            let cr = combined.iter().find(|r| r.rule_id == *rid).unwrap();
            assert_eq!(cr.finding_count(), *exp);
        }
    }

    #[test]
    fn test_multi_rule_same_pattern_different_ids() {
        let source = "SELECT * FROM users; SELECT * FROM orders;";
        let rules = vec![
            create_test_rule("stars-rule-A", "SELECT *"),
            create_test_rule("stars-rule-B", "SELECT *"),
        ];
        let ast = MockAstNode::new("program").with_text(source);
        let context = create_test_context(source);
        let mut engine = RuleExecutionEngine::new().set_parallel_execution(false);
        let combined = engine.execute_rules(&rules, &ast, &context);
        for rid in &["stars-rule-A", "stars-rule-B"] {
            let cr = combined.iter().find(|r| r.rule_id == *rid).unwrap();
            assert_eq!(cr.finding_count(), 2);
        }
    }

    #[test]
    fn test_multi_rule_empty_list_is_noop() {
        let source = "SELECT * FROM t;";
        let ast = MockAstNode::new("program").with_text(source);
        let context = create_test_context(source);
        let mut engine = RuleExecutionEngine::new();
        let results = engine.execute_rules(&[], &ast, &context);
        assert!(results.is_empty());
    }

    /// Regression: mixed Simple + Regex patterns must not produce duplicate
    /// findings. The Simple pattern is handled by the batch matcher; the Regex
    /// pattern falls through to individual execution. Both should match once.
    #[test]
    fn test_multi_rule_mixed_literal_and_regex_no_duplicates() {
        let source = "SELECT * FROM users;";
        let rule = Rule::new(
            "mixed-rule".to_string(),
            "Mixed".to_string(),
            "Test".to_string(),
            Severity::Info,
            Confidence::High,
            vec![Language::Sql],
        )
        .add_pattern(Pattern::simple("SELECT".to_string()))
        .add_pattern(Pattern::regex(r"FROM\s+\w+".to_string()));

        let ast = MockAstNode::new("program").with_text(source);
        let context = RuleContext::new(
            "test.sql".to_string(),
            Language::Sql,
            source.to_string(),
        );

        let mut engine = RuleExecutionEngine::new().set_parallel_execution(false);
        let results = engine.execute_rules(&[rule], &ast, &context);
        assert_eq!(results.len(), 1);
        // 2 patterns = 2 distinct findings, no duplicates
        assert_eq!(results[0].finding_count(), 2);
    }
}
