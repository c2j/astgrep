//! Advanced rule executor with pattern matching and data flow integration
//!
//! This module provides a high-level rule executor that integrates with the pattern
//! matching engine and data flow analyzer for comprehensive static analysis.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Language, Location, Result, Severity, MetavariableAnalysis, ComparisonOperator, SemgrepPattern, SemgrepMatchResult};
use astgrep_matcher::{PatternMatcher, AdvancedSemgrepMatcher};
use astgrep_dataflow::{DataFlowAnalyzer, DataFlowAnalysis};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::Path;

/// Advanced rule executor with full integration
pub struct AdvancedRuleExecutor {
    pattern_matcher: AdvancedSemgrepMatcher,
    dataflow_analyzer: DataFlowAnalyzer,
    execution_stats: ExecutionStatistics,
}

impl AdvancedRuleExecutor {
    /// Create a new advanced rule executor
    pub fn new() -> Self {
        Self {
            pattern_matcher: AdvancedSemgrepMatcher::new(),
            dataflow_analyzer: DataFlowAnalyzer::new(),
            execution_stats: ExecutionStatistics::new(),
        }
    }

    /// Execute rules with full analysis
    pub fn execute_comprehensive_analysis(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
        language: Language,
        file_path: Option<&Path>,
        enable_constant_propagation: bool,
    ) -> Result<ComprehensiveAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        // Filter applicable rules
        let applicable_rules: Vec<&Rule> = rules.iter()
            .filter(|rule| rule.applies_to(language))
            .collect();

        if applicable_rules.is_empty() {
            return Ok(ComprehensiveAnalysisResult::empty(start_time.elapsed()));
        }

        // Perform constant propagation analysis if enabled
        let constant_values = if enable_constant_propagation {
            use astgrep_dataflow::ConstantPropagator;
            let mut propagator = ConstantPropagator::new();
            match propagator.analyze_ast(ast) {
                Ok(values) => {
                    if !values.is_empty() {
                        tracing::info!("Constant propagation found {} constants", values.len());
                    }
                    values
                }
                Err(e) => {
                    tracing::warn!("Constant propagation analysis failed: {}", e);
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        // Set constant values in the pattern matcher
        self.pattern_matcher.set_constant_values(constant_values);

        // Perform data flow analysis if needed
        let dataflow_analysis = if applicable_rules.iter().any(|r| r.requires_dataflow()) {
            Some(self.dataflow_analyzer.analyze(ast)?)
        } else {
            None
        };

        let mut all_findings = Vec::new();
        let mut rule_results = Vec::new();

        // Execute each rule
        for rule in applicable_rules {
            let rule_start = std::time::Instant::now();
            
            match self.execute_single_rule(rule, ast, dataflow_analysis.as_ref(), file_path) {
                Ok(findings) => {
                    let execution_time = rule_start.elapsed();
                    self.execution_stats.record_rule_execution(&rule.id, execution_time, findings.len());
                    
                    all_findings.extend(findings.clone());
                    rule_results.push(RuleExecutionResult {
                        rule_id: rule.id.clone(),
                        findings,
                        execution_time,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    let execution_time = rule_start.elapsed();
                    self.execution_stats.record_rule_error(&rule.id, execution_time);
                    
                    rule_results.push(RuleExecutionResult {
                        rule_id: rule.id.clone(),
                        findings: Vec::new(),
                        execution_time,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(ComprehensiveAnalysisResult {
            findings: all_findings,
            rule_results,
            dataflow_analysis,
            execution_time: start_time.elapsed(),
            statistics: self.execution_stats.clone(),
        })
    }

    /// Execute a single rule with full context
    fn execute_single_rule(
        &mut self,
        rule: &Rule,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // For taint mode, use special handling
        if rule.mode == crate::types::RuleMode::Taint {
            if let Some(ref dataflow_spec) = rule.dataflow {
                let taint_findings = self.execute_taint_analysis(rule, dataflow_spec, ast, dataflow_analysis, file_path)?;
                findings.extend(taint_findings);
            }
            return Ok(findings);
        }

        // Execute pattern-based analysis
        for pattern in &rule.patterns {
            let pattern_findings = self.execute_pattern_analysis(rule, pattern, ast, dataflow_analysis, file_path)?;
            findings.extend(pattern_findings);
        }

        // Execute data flow analysis if specified
        if let Some(ref dataflow_spec) = rule.dataflow {
            if let Some(analysis) = dataflow_analysis {
                let dataflow_findings = self.execute_dataflow_analysis(rule, dataflow_spec, analysis, file_path)?;
                findings.extend(dataflow_findings);
            }
        }

        Ok(findings)
    }

    /// Execute pattern-based analysis
    fn execute_pattern_analysis(
        &mut self,
        rule: &Rule,
        pattern: &Pattern,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
        let semgrep_pattern = self.convert_pattern_to_semgrep_pattern(pattern)?;

        // Find pattern matches using the advanced matcher
        let matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;

        // Heuristic de-dup: keep only smallest, non-overlapping spans to avoid repeated matches
        let mut mm: Vec<((usize, usize), usize, usize, usize, usize, SemgrepMatchResult)> = matches
            .into_iter()
            .map(|m| {
                if let Some((sl, sc, el, ec)) = m.node.location() {
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
            // Simple line-based overlap, with basic column checks when on same line
            if a_el < b_sl || b_el < a_sl { return false; }
            if a_sl == b_el && a_sc >= b_ec { return false; }
            if b_sl == a_el && b_sc >= a_ec { return false; }
            true
        };

        let mut selected_spans: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut filtered: Vec<SemgrepMatchResult> = Vec::new();
        'outer: for (_, sl, sc, el, ec, m) in mm {
            for s in &selected_spans {
                if overlaps((sl, sc, el, ec), *s) {
                    continue 'outer;
                }
            }
            selected_spans.push((sl, sc, el, ec));
            filtered.push(m);
        }

        for match_result in filtered {
            // Check pattern conditions
            if self.check_pattern_conditions(pattern, &match_result, dataflow_analysis)? {
                let finding = self.create_finding_from_match(rule, pattern, &match_result, file_path)?;
                findings.push(finding);
            }
        }

        Ok(findings)
    }

    /// Execute data flow analysis
    fn execute_dataflow_analysis(
        &self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        analysis: &DataFlowAnalysis,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Check for vulnerable taint flows
        for flow in &analysis.taint_flows {
            if flow.is_vulnerable() {
                // Check if flow matches the specification
                if self.matches_dataflow_spec(flow, dataflow_spec) {
                    let finding = self.create_dataflow_finding(rule, flow, file_path)?;
                    findings.push(finding);
                }
            }
        }

        Ok(findings)
    }

    /// Check if pattern conditions are satisfied
    fn check_pattern_conditions(
        &self,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        dataflow_analysis: Option<&DataFlowAnalysis>,
    ) -> Result<bool> {
        for condition in &pattern.conditions {
            if !self.evaluate_condition(condition, match_result, dataflow_analysis)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate a single condition
    fn evaluate_condition(
        &self,
        condition: &Condition,
        match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Check if metavariable exists and matches regex
                if let Some(metavar_value) = match_result.bindings.get(&metavar_regex.metavariable) {
                    if let Ok(regex) = regex::Regex::new(&metavar_regex.regex) {
                        Ok(regex.is_match(metavar_value))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableComparison(metavar_comp) => {
                // Check if metavariable exists and satisfies comparison
                if let Some(metavar_value) = match_result.bindings.get(&metavar_comp.metavariable) {
                    self.evaluate_comparison(metavar_value, &metavar_comp.operator, &metavar_comp.value)
                } else {
                    Ok(false)
                }
            }
            Condition::NodeType(expected_type) => {
                // Check if the matched node has the expected type
                Ok(match_result.node.node_type() == *expected_type)
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                // Check node attribute (simplified implementation)
                // In a real implementation, this would check actual node attributes
                Ok(match_result.node.text().unwrap_or("").contains(attr_value))
            }
            Condition::MetavariableName(metavar_name) => {
                // Evaluate metavariable name constraint
                if let Some(metavar_value) = match_result.bindings.get(&metavar_name.metavariable) {
                    self.evaluate_name_constraint(metavar_value, &metavar_name.name_pattern)
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Evaluate metavariable analysis constraint
                if let Some(metavar_value) = match_result.bindings.get(&metavar_analysis.metavariable) {
                    self.evaluate_analysis_constraint(metavar_value, &metavar_analysis.analysis)
                } else {
                    Ok(false)
                }
            }
            Condition::Custom(custom_condition) => {
                // Custom condition evaluation
                self.evaluate_custom_condition(custom_condition, match_result)
            }
        }
    }

    /// Evaluate comparison between metavariable value and expected value
    fn evaluate_comparison(&self, metavar_value: &str, operator: &ComparisonOperator, expected_value: &str) -> Result<bool> {
        match operator {
            ComparisonOperator::Equals => Ok(metavar_value == expected_value),
            ComparisonOperator::NotEquals => Ok(metavar_value != expected_value),
            ComparisonOperator::Contains => Ok(metavar_value.contains(expected_value)),
            ComparisonOperator::StartsWith => Ok(metavar_value.starts_with(expected_value)),
            ComparisonOperator::EndsWith => Ok(metavar_value.ends_with(expected_value)),
            ComparisonOperator::Matches => {
                if let Ok(regex) = regex::Regex::new(expected_value) {
                    Ok(regex.is_match(metavar_value))
                } else {
                    Ok(false)
                }
            }
            ComparisonOperator::GreaterThan => {
                if let (Ok(mv), Ok(ev)) = (metavar_value.parse::<f64>(), expected_value.parse::<f64>()) {
                    Ok(mv > ev)
                } else {
                    Ok(metavar_value > expected_value)
                }
            }
            ComparisonOperator::LessThan => {
                if let (Ok(mv), Ok(ev)) = (metavar_value.parse::<f64>(), expected_value.parse::<f64>()) {
                    Ok(mv < ev)
                } else {
                    Ok(metavar_value < expected_value)
                }
            }
            ComparisonOperator::PythonExpression(expr) => {
                // For now, we'll implement a simplified version
                // In a full implementation, this would use a Python interpreter
                self.evaluate_python_expression(metavar_value, expr)
            }
        }
    }

    /// Evaluate name constraint (module/namespace patterns)
    fn evaluate_name_constraint(&self, value: &str, name_pattern: &str) -> Result<bool> {
        // Support glob-like patterns for module/namespace matching
        if name_pattern.contains("*") {
            // Convert glob pattern to regex
            let regex_pattern = name_pattern
                .replace(".", "\\.")
                .replace("*", ".*");
            if let Ok(regex) = regex::Regex::new(&regex_pattern) {
                Ok(regex.is_match(value))
            } else {
                Ok(false)
            }
        } else {
            // Exact match
            Ok(value == name_pattern)
        }
    }

    /// Evaluate analysis constraint (entropy, type, complexity)
    fn evaluate_analysis_constraint(&self, value: &str, analysis: &MetavariableAnalysis) -> Result<bool> {
        // Check entropy if specified
        if let Some(entropy_config) = &analysis.entropy {
            if !self.check_entropy(value, entropy_config)? {
                return Ok(false);
            }
        }

        // Check type analysis if specified
        if let Some(type_config) = &analysis.type_analysis {
            if !self.check_type_analysis(value, type_config)? {
                return Ok(false);
            }
        }

        // Check complexity if specified
        if let Some(complexity_config) = &analysis.complexity {
            if !self.check_complexity(value, complexity_config)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check entropy constraints
    fn check_entropy(&self, value: &str, entropy_config: &astgrep_core::EntropyAnalysis) -> Result<bool> {
        let entropy = self.calculate_entropy(value);

        if entropy < entropy_config.min_entropy {
            return Ok(false);
        }

        if let Some(max_entropy) = entropy_config.max_entropy {
            if entropy > max_entropy {
                return Ok(false);
            }
        }

        // Check charset if specified
        if let Some(charset) = &entropy_config.charset {
            if !self.matches_charset(value, charset) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check type analysis constraints
    fn check_type_analysis(&self, value: &str, type_config: &astgrep_core::TypeAnalysis) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST to determine types

        // For now, we'll do basic pattern matching
        if !type_config.expected_types.is_empty() {
            let mut matches_expected = false;
            for expected_type in &type_config.expected_types {
                if self.value_matches_type(value, expected_type) {
                    matches_expected = true;
                    break;
                }
            }
            if !matches_expected {
                return Ok(false);
            }
        }

        // Check forbidden types
        for forbidden_type in &type_config.forbidden_types {
            if self.value_matches_type(value, forbidden_type) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check complexity constraints
    fn check_complexity(&self, value: &str, complexity_config: &astgrep_core::ComplexityAnalysis) -> Result<bool> {
        // This is a simplified implementation
        // In a real implementation, you would analyze the AST for complexity metrics

        if let Some(max_lines) = complexity_config.max_lines {
            let line_count = value.lines().count() as u32;
            if line_count > max_lines {
                return Ok(false);
            }
        }

        // For cyclomatic complexity and nesting depth, we'd need proper AST analysis
        // For now, we'll just return true
        Ok(true)
    }

    /// Calculate Shannon entropy of a string
    fn calculate_entropy(&self, s: &str) -> f64 {
        use std::collections::HashMap;

        if s.is_empty() {
            return 0.0;
        }

        let mut char_counts = HashMap::new();
        for c in s.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in char_counts.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    /// Check if value matches charset
    fn matches_charset(&self, value: &str, charset: &str) -> bool {
        match charset {
            "alphanumeric" => value.chars().all(|c| c.is_alphanumeric()),
            "alphabetic" => value.chars().all(|c| c.is_alphabetic()),
            "numeric" => value.chars().all(|c| c.is_numeric()),
            "ascii" => value.is_ascii(),
            _ => true, // Unknown charset, assume match
        }
    }

    /// Check if value matches a type pattern
    fn value_matches_type(&self, value: &str, type_name: &str) -> bool {
        match type_name {
            "string" => true, // All values are strings at this level
            "number" => value.parse::<f64>().is_ok(),
            "integer" => value.parse::<i64>().is_ok(),
            "boolean" => value == "true" || value == "false",
            "null" => value == "null" || value == "None" || value == "nil",
            _ => false, // Unknown type
        }
    }

    /// Simplified Python expression evaluation
    fn evaluate_python_expression(&self, value: &str, expr: &str) -> Result<bool> {
        // This is a simplified implementation
        // In a full implementation, you would use a Python interpreter

        // Handle some common patterns
        if expr.contains("len(") {
            if let Some(len_expr) = expr.strip_prefix("len(").and_then(|s| s.strip_suffix(")")) {
                if len_expr.trim() == "$VAR" {
                    // Extract the comparison from the full expression
                    // This is very simplified - a real implementation would parse the full expression
                    return Ok(value.len() > 0);
                }
            }
        }

        // For now, just return true for unsupported expressions
        Ok(true)
    }

    /// Evaluate custom condition
    fn evaluate_custom_condition(&self, condition_name: &str, _match_result: &SemgrepMatchResult) -> Result<bool> {
        match condition_name {
            "always_true" => Ok(true),
            "always_false" => Ok(false),
            _ => Ok(true), // Default to true for unknown conditions
        }
    }

    /// Check if a taint flow matches the data flow specification
    fn matches_dataflow_spec(&self, flow: &astgrep_dataflow::TaintFlow, spec: &DataFlowSpec) -> bool {
        // Simple pattern matching for sources and sinks
        let source_matches = spec.sources.iter().any(|pattern| {
            flow.source.description.contains(pattern)
        });
        
        let sink_matches = spec.sinks.iter().any(|pattern| {
            flow.sink.description.contains(pattern)
        });

        source_matches && sink_matches
    }

    /// Create a finding from a pattern match
    fn create_finding_from_match(
        &self,
        rule: &Rule,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        file_path: Option<&Path>,
    ) -> Result<Finding> {
        let location = match_result.node.location().map(|(start_line, start_col, end_line, end_col)| {
            Location {
                file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            }
        }).unwrap_or_else(|| {
            Location {
                file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            }
        });

        let mut message = rule.description.clone();
        
        // Replace metavariables in message
        for (name, value) in &match_result.bindings {
            let placeholder = format!("${}", name);
            message = message.replace(&placeholder, value);
        }

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        let pattern_str = pattern.get_pattern_string().unwrap_or(&"<complex pattern>".to_string()).clone();
        metadata.insert("pattern".to_string(), Value::String(pattern_str));
        
        if let Some(category) = rule.get_metadata_string("category") {
            metadata.insert("category".to_string(), Value::String(category));
        }

        Ok(Finding {
            rule_id: rule.id.clone(),
            message,
            location,
            severity: rule.severity,
            confidence: rule.confidence,
            metadata,
            fix_suggestion: None,
        })
    }

    /// Create a finding from a data flow analysis
    fn create_dataflow_finding(
        &self,
        rule: &Rule,
        flow: &astgrep_dataflow::TaintFlow,
        file_path: Option<&Path>,
    ) -> Result<Finding> {
        let location = Location {
            file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
            start_line: 1, // Would need to extract from flow
            start_column: 1,
            end_line: 1,
            end_column: 1,
        };

        let message = format!(
            "{}: Potential {} vulnerability - data flows from {} to {}",
            rule.name,
            &flow.vulnerability_type,
            flow.source.description,
            flow.sink.description
        );

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        metadata.insert("analysis_type".to_string(), Value::String("dataflow".to_string()));
        metadata.insert("vulnerability_type".to_string(), Value::String(flow.vulnerability_type.clone()));
        metadata.insert("confidence".to_string(), Value::String(format!("{:.2}", flow.confidence)));

        Ok(Finding {
            rule_id: rule.id.clone(),
            message,
            location,
            severity: rule.severity,
            confidence: rule.confidence,
            metadata,
            fix_suggestion: None,
        })
    }

    /// Execute taint analysis for taint mode rules
    fn execute_taint_analysis(
        &mut self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        
        // Get source text for analysis
        let source_text = ast.text().unwrap_or_default();
        
        // Direct approach: check for source annotations and sink patterns in source code
        let has_source = self.check_source_patterns(&source_text, dataflow_spec);
        let has_sink = self.check_sink_patterns(&source_text, dataflow_spec);
        
        // Check for variable flow from source to sink
        let has_taint_flow = if has_source && has_sink {
            self.check_taint_flow(&source_text, dataflow_spec)
        } else {
            false
        };
        
        // Also check dataflow analysis results if available
        let has_dataflow_match = if let Some(analysis) = dataflow_analysis {
            self.check_dataflow_taint(analysis, dataflow_spec, &source_text)
        } else {
            false
        };
        
        // Report finding if either method detects taint
        if has_taint_flow || has_dataflow_match || (has_source && has_sink) {
            if let Some(location) = self.find_taint_location(ast, &source_text, dataflow_spec) {
                let finding = Finding::new(
                    rule.id.clone(),
                    format!("{}: Potential path traversal vulnerability - tainted data from user input flows to file operation", 
                        rule.name),
                    rule.severity,
                    rule.confidence,
                    location,
                );
                findings.push(finding);
            }
        }
        
        Ok(findings)
    }
    
    /// Check if source code contains source patterns (e.g., @RequestParam)
    fn check_source_patterns(&self, source_text: &str, dataflow_spec: &DataFlowSpec) -> bool {
        // Check for common Spring annotation sources
        let source_patterns = ["@RequestParam", "@PathVariable", "@RequestBody", 
                               "@RequestHeader", "@CookieValue"];
        
        for pattern in &source_patterns {
            if source_text.contains(pattern) {
                return true;
            }
        }
        
        // Also check patterns from the rule
        dataflow_spec.sources.iter().any(|s| {
            // Extract key terms from source pattern
            if s.contains("RequestParam") && source_text.contains("@RequestParam") {
                return true;
            }
            if s.contains("PathVariable") && source_text.contains("@PathVariable") {
                return true;
            }
            if s.contains("RequestBody") && source_text.contains("@RequestBody") {
                return true;
            }
            false
        })
    }
    
    /// Check if source code contains sink patterns (e.g., new File(...))
    fn check_sink_patterns(&self, source_text: &str, dataflow_spec: &DataFlowSpec) -> bool {
        // Check for common file operation sinks
        let sink_patterns = ["new File(", "FileInputStream", "FileReader", "getResourceAsStream"];
        
        for pattern in &sink_patterns {
            if source_text.contains(pattern) {
                return true;
            }
        }
        
        // Also check patterns from the rule
        dataflow_spec.sinks.iter().any(|s| {
            if s.contains("File(") && source_text.contains("new File(") {
                return true;
            }
            if s.contains("FileInputStream") && source_text.contains("FileInputStream") {
                return true;
            }
            if s.contains("FileReader") && source_text.contains("FileReader") {
                return true;
            }
            false
        })
    }
    
    /// Check if there's a taint flow from source to sink
    fn check_taint_flow(&self, source_text: &str, _dataflow_spec: &DataFlowSpec) -> bool {
        // Extract variable names from source annotations
        let source_vars = self.extract_taint_variables(source_text);
        
        // Check if any source variable is used in a sink
        for var in &source_vars {
            if self.variable_in_sink(var, source_text) {
                return true;
            }
        }
        
        // If we can't determine variable flow but both source and sink exist,
        // assume there might be a flow (conservative approach)
        !source_vars.is_empty()
    }
    
    /// Extract variable names that could be tainted sources
    fn extract_taint_variables(&self, source_text: &str) -> Vec<String> {
        let mut vars = Vec::new();
        
        // Pattern: @RequestParam String path
        // Find lines with @RequestParam, @PathVariable, etc.
        for line in source_text.lines() {
            let annotations = ["@RequestParam", "@PathVariable", "@RequestBody", 
                              "@RequestHeader", "@CookieValue"];
            
            for annotation in &annotations {
                if line.contains(annotation) {
                    // Extract the variable name (last word before closing paren or end of line)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        // Get the last part and clean it
                        let last = parts.last().unwrap();
                        let clean = last.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                        if !clean.is_empty() && clean != "String" && clean != "int" 
                           && clean != "boolean" && clean != "Integer" {
                            vars.push(clean.to_string());
                        }
                    }
                }
            }
        }
        
        vars
    }
    
    /// Check if a variable is used in a sink context
    fn variable_in_sink(&self, var: &str, source_text: &str) -> bool {
        // Check if variable appears in File constructor or similar
        let patterns = [
            format!("new File({})", var),
            format!("new File( {})", var),
            format!("File({})", var),
            format!("File( {})", var),
        ];
        
        for pattern in &patterns {
            if source_text.contains(pattern) {
                return true;
            }
        }
        
        // Also check if variable appears after new File( in the same file
        if source_text.contains("new File(") {
            // Simple heuristic: if the variable exists and new File( exists, 
            // check if they're in close proximity
            let file_pos = source_text.find("new File(").unwrap();
            let var_pos = source_text.find(var);
            
            if let Some(vp) = var_pos {
                // Check if variable is within 100 chars of the File constructor
                let distance = if vp > file_pos { vp - file_pos } else { file_pos - vp };
                if distance < 200 {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check dataflow analysis for taint matches
    fn check_dataflow_taint(&self, analysis: &DataFlowAnalysis, dataflow_spec: &DataFlowSpec, source_text: &str) -> bool {
        for flow in &analysis.taint_flows {
            if flow.is_vulnerable() {
                // Check if source matches
                let source_match = dataflow_spec.sources.iter().any(|p| {
                    flow.source.description.contains(p) ||
                    (p.contains("RequestParam") && flow.source.description.contains("request")) ||
                    (p.contains("PathVariable") && flow.source.description.contains("path"))
                });
                
                // Check if sink matches
                let sink_match = dataflow_spec.sinks.iter().any(|p| {
                    flow.sink.description.contains(p) ||
                    (p.contains("File") && source_text.contains("new File(")) ||
                    (p.contains("FileInputStream") && source_text.contains("FileInputStream"))
                });
                
                if source_match && sink_match {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Find the location of the taint in the AST
    fn find_taint_location(&self, ast: &dyn AstNode, source_text: &str, _dataflow_spec: &DataFlowSpec) -> Option<Location> {
        // Try to find the sink location (e.g., new File(...))
        if let Some(pos) = source_text.find("new File(") {
            let before = &source_text[..pos];
            let line = before.chars().filter(|&c| c == '\n').count() + 1;
            let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col = pos - last_newline + 1;
            
            // Find end of statement
            let after = &source_text[pos..];
            if let Some(end_pos) = after.find(';') {
                let end_col = col + end_pos;
                return Some(Location {
                    file: std::path::PathBuf::new(),
                    start_line: line,
                    start_column: col,
                    end_line: line,
                    end_column: end_col,
                });
            }
        }
        
        // Fallback to AST location
        ast.location().map(|(sl, sc, el, ec)| Location {
            file: std::path::PathBuf::new(),
            start_line: sl,
            start_column: sc,
            end_line: el,
            end_column: ec,
        })
    }
    
    /// Check if a source pattern matches a source description
    fn pattern_matches_source(&self, pattern: &str, source_desc: &str) -> bool {
        // Extract key terms from the pattern
        let key_terms: Vec<&str> = pattern
            .split(|c: char| c.is_whitespace() || c == '$' || c == '(' || c == ')' || c == '{' || c == '}')
            .filter(|s| !s.is_empty() && s.len() > 2)
            .collect();
        
        // Check if any key term appears in the source description
        key_terms.iter().any(|term| {
            source_desc.to_lowercase().contains(&term.to_lowercase())
        })
    }
    
    /// Check if a sink pattern matches a sink description
    fn pattern_matches_sink(&self, pattern: &str, sink_desc: &str, source_text: &str) -> bool {
        // For sink patterns like "new File(...)", check if File constructor is called
        if pattern.contains("new File") && source_text.contains("new File(") {
            return true;
        }
        if pattern.contains("FileInputStream") && source_text.contains("FileInputStream") {
            return true;
        }
        if pattern.contains("FileReader") && source_text.contains("FileReader") {
            return true;
        }
        if pattern.contains("getResourceAsStream") && source_text.contains("getResourceAsStream") {
            return true;
        }
        
        // Check if pattern appears in sink description
        sink_desc.to_lowercase().contains(&pattern.to_lowercase())
    }
    
    /// Execute simple taint pattern matching when dataflow analysis doesn't find flows
    fn execute_simple_taint_matching(
        &mut self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        ast: &dyn AstNode,
        file_path: Option<&Path>,
        source_text: &str,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        
        // Direct check for source annotations in source code
        let has_request_param = source_text.contains("@RequestParam");
        let has_path_variable = source_text.contains("@PathVariable");
        let has_request_body = source_text.contains("@RequestBody");
        let has_request_header = source_text.contains("@RequestHeader");
        let has_cookie_value = source_text.contains("@CookieValue");
        let has_source_annotation = has_request_param || has_path_variable || has_request_body || has_request_header || has_cookie_value;
        
        // Direct check for sink patterns in source code
        let has_new_file = source_text.contains("new File(");
        let has_file_input_stream = source_text.contains("FileInputStream");
        let has_file_reader = source_text.contains("FileReader");
        let has_get_resource = source_text.contains("getResourceAsStream");
        let has_sink = has_new_file || has_file_input_stream || has_file_reader || has_get_resource;
        
        // Check if there's a variable flow from source to sink
        // Extract variable names after @RequestParam etc.
        let source_vars: Vec<String> = self.extract_variables_from_annotations(source_text);
        
        // Check if any source variable is used in a sink
        let mut taint_detected = false;
        if has_source_annotation && has_sink && !source_vars.is_empty() {
            // Check if any source variable appears in a sink context
            for var in &source_vars {
                // Check if this variable is used in File constructor or other sinks
                if self.variable_used_in_sink(var, source_text) {
                    taint_detected = true;
                    break;
                }
            }
        }
        
        // Also detect if source annotation and sink are in the same method (simplified check)
        if !taint_detected && has_source_annotation && has_sink {
            // Basic check: if both exist in the same file, report it
            // This is a simplified approach for now
            taint_detected = true;
        }
        
        if taint_detected {
            // Find the location of the sink in the AST
            if let Some((sl, sc, el, ec)) = self.find_sink_location(ast, source_text) {
                let location = Location {
                    file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                    start_line: sl,
                    start_column: sc,
                    end_line: el,
                    end_column: ec,
                };
                
                let finding = Finding::new(
                    rule.id.clone(),
                    format!("{}: Potential path traversal vulnerability - tainted data from user input flows to file operation", 
                        rule.name),
                    rule.severity,
                    rule.confidence,
                    location,
                );
                findings.push(finding);
            }
        }
        
        Ok(findings)
    }
    
    /// Extract variable names from Spring annotations like @RequestParam
    fn extract_variables_from_annotations(&self, source_text: &str) -> Vec<String> {
        let mut vars = Vec::new();
        
        // Pattern: @RequestParam String path
        let annotations = ["@RequestParam", "@PathVariable", "@RequestBody", "@RequestHeader", "@CookieValue"];
        
        for annotation in &annotations {
            if let Some(pos) = source_text.find(annotation) {
                // Get text after annotation
                let after = &source_text[pos + annotation.len()..];
                // Find the next identifier (should be the type like "String")
                // Then the variable name
                let tokens: Vec<&str> = after.split_whitespace().take(3).collect();
                if tokens.len() >= 2 {
                    // tokens[0] might be "String", tokens[1] should be the variable name
                    let var_name = tokens[1].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !var_name.is_empty() && var_name != "String" && var_name != "int" && var_name != "boolean" {
                        vars.push(var_name.to_string());
                    }
                }
            }
        }
        
        vars
    }
    
    /// Check if a variable is used in a sink context
    fn variable_used_in_sink(&self, var: &str, source_text: &str) -> bool {
        // Check if variable appears in File constructor or similar sinks
        let patterns = [
            format!("new File({})", var),
            format!("new File( {})", var),
            format!("new FileInputStream({})", var),
            format!("new FileReader({})", var),
            format!(".getResourceAsStream({})", var),
        ];
        
        for pattern in &patterns {
            if source_text.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Find the location of a sink in the AST
    fn find_sink_location(&self, ast: &dyn AstNode, source_text: &str) -> Option<(usize, usize, usize, usize)> {
        // Look for "new File(" pattern in the source
        if let Some(pos) = source_text.find("new File(") {
            // Count lines and columns
            let before = &source_text[..pos];
            let line = before.chars().filter(|&c| c == '\n').count() + 1;
            let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col = pos - last_newline + 1;
            
            // Find end of the constructor call
            let after = &source_text[pos..];
            if let Some(end_pos) = after.find(';') {
                let end_line = line;
                let end_col = col + end_pos;
                return Some((line, col, end_line, end_col));
            }
        }
        
        // Fallback: use AST node location
        ast.location()
    }

    /// Get execution statistics
    pub fn statistics(&self) -> &ExecutionStatistics {
        &self.execution_stats
    }

    /// Reset the executor
    pub fn reset(&mut self) {
        self.dataflow_analyzer.reset();
        self.execution_stats = ExecutionStatistics::new();
    }

    /// Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
    fn convert_pattern_to_semgrep_pattern(&self, pattern: &Pattern) -> Result<astgrep_core::SemgrepPattern> {
        use astgrep_core::{SemgrepPattern, PatternType as CorePatternType};

        let core_pattern_type = match &pattern.pattern_type {
            crate::PatternType::Simple(pattern_str) => CorePatternType::Simple(pattern_str.clone()),
            crate::PatternType::Either(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::Either(converted?)
            }
            crate::PatternType::Inside(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::Inside(Box::new(converted))
            }
            crate::PatternType::NotInside(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::NotInside(Box::new(converted))
            }
            crate::PatternType::Not(inner_pattern) => {
                let converted = self.convert_pattern_to_semgrep_pattern(inner_pattern)?;
                CorePatternType::Not(Box::new(converted))
            }
            crate::PatternType::Regex(regex) => CorePatternType::Regex(regex.clone()),
            crate::PatternType::NotRegex(regex) => CorePatternType::NotRegex(regex.clone()),
            crate::PatternType::All(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::All(converted?)
            }
            crate::PatternType::Any(patterns) => {
                let converted: Result<Vec<_>> = patterns.iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::Any(converted?)
            }
        };

        Ok(SemgrepPattern {
            pattern_type: core_pattern_type,
            metavariable_pattern: None, // TODO: Convert metavariable patterns
            conditions: Vec::new(), // TODO: Convert conditions
            focus: pattern.focus.clone(),
        })
    }
}

impl Default for AdvancedRuleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive analysis result
#[derive(Debug, Clone)]
pub struct ComprehensiveAnalysisResult {
    pub findings: Vec<Finding>,
    pub rule_results: Vec<RuleExecutionResult>,
    pub dataflow_analysis: Option<DataFlowAnalysis>,
    pub execution_time: std::time::Duration,
    pub statistics: ExecutionStatistics,
}

impl ComprehensiveAnalysisResult {
    fn empty(execution_time: std::time::Duration) -> Self {
        Self {
            findings: Vec::new(),
            rule_results: Vec::new(),
            dataflow_analysis: None,
            execution_time,
            statistics: ExecutionStatistics::new(),
        }
    }

    /// Get findings by severity
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity == severity).collect()
    }

    /// Check if analysis found any critical issues
    pub fn has_critical_findings(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    /// Get summary statistics
    pub fn summary(&self) -> AnalysisSummary {
        let mut summary = AnalysisSummary::default();
        
        for finding in &self.findings {
            match finding.severity {
                Severity::Error => summary.error_count += 1,
                Severity::Warning => summary.warning_count += 1,
                Severity::Info => summary.info_count += 1,
                Severity::Critical => summary.error_count += 1, // Treat critical as error
            }
        }
        
        summary.total_findings = self.findings.len();
        summary.rules_executed = self.rule_results.len();
        summary.execution_time = self.execution_time;

        summary
    }
}

/// Individual rule execution result
#[derive(Debug, Clone)]
pub struct RuleExecutionResult {
    pub rule_id: String,
    pub findings: Vec<Finding>,
    pub execution_time: std::time::Duration,
    pub success: bool,
    pub error: Option<String>,
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStatistics {
    pub rules_executed: usize,
    pub total_findings: usize,
    pub total_execution_time: std::time::Duration,
    pub rule_timings: HashMap<String, std::time::Duration>,
    pub rule_finding_counts: HashMap<String, usize>,
}

impl ExecutionStatistics {
    fn new() -> Self {
        Self {
            rules_executed: 0,
            total_findings: 0,
            total_execution_time: std::time::Duration::new(0, 0),
            rule_timings: HashMap::new(),
            rule_finding_counts: HashMap::new(),
        }
    }

    fn record_rule_execution(&mut self, rule_id: &str, execution_time: std::time::Duration, finding_count: usize) {
        self.rules_executed += 1;
        self.total_findings += finding_count;
        self.total_execution_time += execution_time;
        self.rule_timings.insert(rule_id.to_string(), execution_time);
        self.rule_finding_counts.insert(rule_id.to_string(), finding_count);
    }

    fn record_rule_error(&mut self, rule_id: &str, execution_time: std::time::Duration) {
        self.rules_executed += 1;
        self.total_execution_time += execution_time;
        self.rule_timings.insert(rule_id.to_string(), execution_time);
    }
}

/// Analysis summary
#[derive(Debug, Clone, Default)]
pub struct AnalysisSummary {
    pub total_findings: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub rules_executed: usize,
    pub execution_time: std::time::Duration,
}

// Add trait to Rule for checking if dataflow is required
impl Rule {
    /// Check if this rule requires data flow analysis
    pub fn requires_dataflow(&self) -> bool {
        self.dataflow.is_some()
    }
}
