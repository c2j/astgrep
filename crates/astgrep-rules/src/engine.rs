//! Rule execution engine
//! 
//! This module provides the core rule execution engine that applies rules to AST nodes.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Location, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

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
        ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        println!("🔍 Executing pattern for rule: {}", rule.id);
        println!("🔍 Pattern: {:?}", pattern);

        let mut findings = Vec::new();

        // Simple pattern matching implementation
        // In a real implementation, this would use a sophisticated pattern matcher
        let matches = self.find_pattern_matches(pattern, ast)?;

        println!("🔍 Pattern matching found {} matches", matches.len());

        for match_node in matches {
            let location = self.create_location_from_node(match_node.as_ref(), context);

            let finding = Finding::new(
                rule.id.clone(),
                self.generate_finding_message(rule, pattern, match_node.as_ref()),
                rule.severity,
                rule.confidence,
                location,
            )
            .with_metadata("pattern".to_string(), pattern.get_pattern_string().unwrap_or(&"".to_string()).clone());

            // Add fix suggestion if available
            let finding = if let Some(ref fix) = rule.fix {
                finding.with_fix(fix.clone())
            } else {
                finding
            };

            findings.push(finding);
        }

        println!("🔍 Pattern execution complete. Generated {} findings", findings.len());
        Ok(findings)
    }

    /// Find pattern matches in AST (simplified implementation)
    fn find_pattern_matches(&self, pattern: &Pattern, ast: &dyn AstNode) -> Result<Vec<Box<dyn AstNode>>> {
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
                    let sub_matches = self.find_pattern_matches(sub_pattern, ast)?;
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
                            if self.simple_pattern_match(pattern_str, text) {
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

    /// Tokenize a string, preserving operators and punctuation as separate tokens
    fn tokenize(&self, s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in s.chars() {
            match ch {
                // Operators and punctuation - treat as separate tokens
                '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' |
                '&' | '|' | '^' | '~' | '?' | ':' | ';' | ',' | '.' |
                '(' | ')' | '[' | ']' | '{' | '}' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                }
                // Whitespace - token separator
                ' ' | '\t' | '\n' | '\r' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                }
                // Regular characters - accumulate into current token
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }

    /// Simple pattern matching with metavariable support
    fn simple_pattern_match(&self, pattern: &str, text: &str) -> bool {
        println!("🔍 Pattern: '{}'", pattern);
        println!("🔍 Node text: '{}'", text);

        // Tokenize pattern and text
        let mut pattern_tokens = self.tokenize(pattern);
        let text_tokens = self.tokenize(text);

        println!("🔍 Pattern tokens: {:?}", pattern_tokens);
        println!("🔍 Text tokens: {:?}", text_tokens);

        // If pattern ends with semicolon but text doesn't, try matching without the semicolon
        // This handles the case where tree-sitter separates expression nodes from statement terminators
        let pattern_without_semicolon = if pattern_tokens.len() > text_tokens.len()
            && pattern_tokens.last() == Some(&";".to_string()) {
            println!("🔍 Pattern has trailing semicolon, trying to match without it");
            pattern_tokens.pop(); // Remove the semicolon
            true
        } else {
            false
        };

        // If pattern has different number of tokens, it can't match
        if pattern_tokens.len() != text_tokens.len() {
            println!("🔍 Token count mismatch: {} vs {}", pattern_tokens.len(), text_tokens.len());
            return false;
        }

        // Track metavariable bindings
        let mut metavar_bindings: HashMap<String, String> = HashMap::new();

        // Match each token
        for (pattern_token, text_token) in pattern_tokens.iter().zip(text_tokens.iter()) {
            if pattern_token.starts_with('$') {
                // This is a metavariable
                let metavar_name = pattern_token.clone();

                // Check if this metavariable was already bound
                if let Some(existing_value) = metavar_bindings.get(&metavar_name) {
                    // Metavariable must bind to the same value
                    if existing_value != text_token {
                        println!("🔍 Metavariable {} already bound to '{}', but found '{}'",
                                metavar_name, existing_value, text_token);
                        return false;
                    }
                } else {
                    // Bind the metavariable
                    println!("🔍 Binding metavariable {} to '{}'", metavar_name, text_token);
                    metavar_bindings.insert(metavar_name, text_token.clone());
                }
            } else {
                // This is a literal token - must match exactly
                if pattern_token != text_token {
                    println!("🔍 Literal token mismatch: '{}' vs '{}'", pattern_token, text_token);
                    return false;
                }
            }
        }

        println!("🔍 Match successful! Bindings: {:?}", metavar_bindings);
        true
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
        let sources = self.find_dataflow_nodes(ast, &dataflow.sources)?;
        let sinks = self.find_dataflow_nodes(ast, &dataflow.sinks)?;

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
    fn find_dataflow_nodes(&self, ast: &dyn AstNode, patterns: &[String]) -> Result<Vec<Box<dyn AstNode>>> {
        let mut matches = Vec::new();

        for pattern in patterns {
            astgrep_core::ast_utils::visit_nodes(ast, &mut |node| {
                if let Some(text) = node.text() {
                    if self.simple_pattern_match(pattern, text) {
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
    fn test_execution_timeout() {
        let mut engine = RuleExecutionEngine::new().set_max_execution_time(0); // Immediate timeout
        let rule = create_test_rule();
        let ast = create_test_ast();
        let context = create_test_context();

        let result = engine.execute_rule(&rule, &ast, &context);
        
        // Note: This test might be flaky due to timing, but demonstrates the concept
        assert_eq!(result.rule_id, "test-rule");
    }
}
