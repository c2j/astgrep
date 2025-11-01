//! Advanced rule executor with pattern matching and data flow integration
//!
//! This module provides a high-level rule executor that integrates with the pattern
//! matching engine and data flow analyzer for comprehensive static analysis.

use crate::types::*;
use astgrep_core::{AstNode, Finding, Language, Location, Result, Severity, MetavariableAnalysis, ComparisonOperator, SemgrepPattern, SemgrepMatchResult};
use astgrep_matcher::{PatternMatcher, AdvancedSemgrepMatcher};
use astgrep_dataflow::{DataFlowAnalyzer, DataFlowAnalysis};
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
    ) -> Result<ComprehensiveAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        // Filter applicable rules
        let applicable_rules: Vec<&Rule> = rules.iter()
            .filter(|rule| rule.applies_to(language))
            .collect();

        if applicable_rules.is_empty() {
            return Ok(ComprehensiveAnalysisResult::empty(start_time.elapsed()));
        }

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

    /// Simplified Python-like expression evaluation for metavariable-comparison
    /// Supports common forms used in tests: re.match(..., str($X)), int($X) OP N, len(str($X)) OP N, type($X).__name__ == "str"
    fn evaluate_python_expression(&self, value: &str, expr: &str) -> Result<bool> {
        let e = expr.trim();

        // 1) re.match(r"...", str($X))
        if let Ok(re_capt) = regex::Regex::new(r#"(?i)^re\.match\(\s*r?[\"']([^\"']+)[\"']\s*,\s*str\(\s*\$\w+\s*\)\s*\)\s*$"#) {
            if let Some(caps) = re_capt.captures(e) {
                let pat = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if let Ok(rexp) = regex::Regex::new(pat) {
                    return Ok(rexp.is_match(value));
                } else {
                    return Ok(false);
                }
            }
        }

        // Helper: numeric compare
        fn cmp_num(lhs: f64, op: &str, rhs: f64) -> bool {
            match op {
                "==" => lhs == rhs,
                "!=" => lhs != rhs,
                ">=" => lhs >= rhs,
                "<=" => lhs <= rhs,
                ">" => lhs > rhs,
                "<" => lhs < rhs,
                _ => false,
            }
        }

        // Parse numeric value from string representation
        fn parse_num(s: &str) -> Option<f64> {
            let t = s.trim().replace('_', "");
            t.parse::<f64>().ok()
        }

        // 2) int($X) OP N
        if let Ok(int_re) = regex::Regex::new(r#"(?i)^int\(\s*\$\w+\s*\)\s*(==|!=|>=|<=|>|<)\s*(-?\d+)\s*$"#) {
            if let Some(caps) = int_re.captures(e) {
                let op = caps.get(1).unwrap().as_str();
                let rhs: f64 = caps.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                let lhs: f64 = parse_num(value).unwrap_or(0.0);
                return Ok(cmp_num(lhs, op, rhs));
            }
        }

        // 3) len(str($X)) OP N
        if let Ok(len_re) = regex::Regex::new(r#"(?i)^len\(\s*str\(\s*\$\w+\s*\)\s*\)\s*(==|!=|>=|<=|>|<)\s*(\d+)\s*$"#) {
            if let Some(caps) = len_re.captures(e) {
                let op = caps.get(1).unwrap().as_str();
                let rhs: f64 = caps.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                // remove simple quotes around value
                let mut s = value.trim().to_string();
                if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
                    s = s[1..s.len().saturating_sub(1)].to_string();
                }
                let lhs = s.chars().count() as f64;
                return Ok(cmp_num(lhs, op, rhs));
            }
        }

        // 4) type($X).__name__ == "str" | "int" | "float" | "bool"
        if let Ok(type_re) = regex::Regex::new(r#"(?i)^type\(\s*\$\w+\s*\)\.__name__\s*==\s*[\"']([A-Za-z_][A-Za-z0-9_]*)[\"']\s*$"#) {
            if let Some(caps) = type_re.captures(e) {
                let ty = caps.get(1).unwrap().as_str().to_ascii_lowercase();
                let val = value.trim();
                let res = match ty.as_str() {
                    "str" | "string" => (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')),
                    "int" | "integer" => val.parse::<i64>().is_ok(),
                    "float" => val.parse::<f64>().is_ok() && val.contains('.'),
                    "bool" => matches!(val, "True" | "False" | "true" | "false"),
                    _ => false,
                };
                return Ok(res);
            }
        }

        // 5) Fallback simple numeric: $X OP N
        if let Ok(simple) = regex::Regex::new(r#"^\s*\$\w+\s*(==|!=|>=|<=|>|<)\s*(-?\d+(?:\.\d+)?)\s*$"#) {
            if let Some(caps) = simple.captures(e) {
                let op = caps.get(1).unwrap().as_str();
                let rhs = caps.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                let lhs = parse_num(value).unwrap_or(0.0);
                return Ok(cmp_num(lhs, op, rhs));
            }
        }

        // Unsupported expressions default to true to avoid over-filtering in compatibility mode
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
        metadata.insert("rule_name".to_string(), rule.name.clone());
        let pattern_str = pattern.get_pattern_string().unwrap_or(&"<complex pattern>".to_string()).clone();
        metadata.insert("pattern".to_string(), pattern_str);
        
        if let Some(ref category) = rule.get_metadata("category") {
            metadata.insert("category".to_string(), category.to_string());
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
        metadata.insert("rule_name".to_string(), rule.name.clone());
        metadata.insert("analysis_type".to_string(), "dataflow".to_string());
        metadata.insert("vulnerability_type".to_string(), flow.vulnerability_type.clone());
        metadata.insert("confidence".to_string(), format!("{:.2}", flow.confidence));

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
