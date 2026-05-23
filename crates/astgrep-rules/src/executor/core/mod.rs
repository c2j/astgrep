//! Advanced rule executor core implementation
//!
//! This module contains the main executor implementation with comprehensive analysis

use crate::executor::dependency::VariableDependencyGraph;
use crate::executor::types::{is_operator_node, TaintMatch};
use crate::types::*;
use astgrep_core::{
    AstNode, ComparisonOperator, Finding, Language, Location, MatchBinding, MetavariableAnalysis,
    Result, SemgrepMatchResult, Severity,
};
use astgrep_dataflow::{DataFlowAnalysis, DataFlowAnalyzer};
use astgrep_matcher::AdvancedSemgrepMatcher;
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::Path;

mod conditions;
mod symbolic;
mod taint;
mod taint_env;
mod utils;

pub struct AdvancedRuleExecutor {
    pattern_matcher: AdvancedSemgrepMatcher,
    dataflow_analyzer: DataFlowAnalyzer,
    execution_stats: ExecutionStatistics,
    /// Constant propagator for variable value tracking
    constant_propagator: Option<astgrep_dataflow::ConstantPropagator>,
    /// Symbolic propagator for alias tracking
    symbolic_propagator: Option<astgrep_dataflow::SymbolicPropagator>,
}

impl AdvancedRuleExecutor {
    /// Create a new advanced rule executor
    pub fn new() -> Self {
        Self {
            pattern_matcher: AdvancedSemgrepMatcher::new(),
            dataflow_analyzer: DataFlowAnalyzer::new(),
            execution_stats: ExecutionStatistics::new(),
            constant_propagator: None,
            symbolic_propagator: None,
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
        let applicable_rules: Vec<&Rule> = rules
            .iter()
            .filter(|rule| rule.applies_to(language))
            .collect();

        if applicable_rules.is_empty() {
            return Ok(ComprehensiveAnalysisResult::empty(start_time.elapsed()));
        }

        // Perform constant propagation analysis if enabled
        self.constant_propagator = if enable_constant_propagation {
            use astgrep_dataflow::ConstantPropagator;
            let mut propagator = ConstantPropagator::new();
            match propagator.analyze_ast(ast) {
                Ok(values) => {
                    if !values.is_empty() {
                        tracing::info!("Constant propagation found {} constants", values.len());
                    }
                    // Set constant values in the pattern matcher
                    eprintln!(
                        "DEBUG: Setting {} constant values in pattern matcher",
                        values.len()
                    );
                    for (k, v) in &values {
                        eprintln!("DEBUG: Constant {} = {:?}", k, v);
                    }
                    self.pattern_matcher.set_constant_values(values);
                    Some(propagator)
                }
                Err(e) => {
                    tracing::warn!("Constant propagation analysis failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        self.pattern_matcher.set_language(language);

        // Perform symbolic propagation analysis if needed
        let enable_symbolic_propagation = applicable_rules
            .iter()
            .any(|r| r.requires_symbolic_propagation());
        if enable_symbolic_propagation {
            use astgrep_dataflow::SymbolicPropagator;
            let mut propagator = SymbolicPropagator::new().with_deep_propagation(true);
            match propagator.analyze(ast) {
                Ok(()) => {
                    eprintln!("DEBUG: Symbolic propagation analysis completed");
                    // Set the symbolic propagator in the pattern matcher
                    self.pattern_matcher
                        .set_symbolic_propagator(propagator.clone());
                    self.symbolic_propagator = Some(propagator);
                }
                Err(e) => {
                    eprintln!("DEBUG: Symbolic propagation analysis failed: {}", e);
                    self.symbolic_propagator = None;
                }
            }
        } else {
            self.symbolic_propagator = None;
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
                    self.execution_stats.record_rule_execution(
                        &rule.id,
                        execution_time,
                        findings.len(),
                    );

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
                    self.execution_stats
                        .record_rule_error(&rule.id, execution_time);

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
                let taint_findings = self.execute_taint_analysis(
                    rule,
                    dataflow_spec,
                    ast,
                    dataflow_analysis,
                    file_path,
                )?;
                findings.extend(taint_findings);
            }
            return Ok(findings);
        }

        // Execute pattern-based analysis
        for pattern in &rule.patterns {
            let pattern_findings =
                self.execute_pattern_analysis(rule, pattern, ast, dataflow_analysis, file_path)?;
            findings.extend(pattern_findings);
        }

        // Execute data flow analysis if specified
        if let Some(ref dataflow_spec) = rule.dataflow {
            if let Some(analysis) = dataflow_analysis {
                let dataflow_findings =
                    self.execute_dataflow_analysis(rule, dataflow_spec, analysis, file_path)?;
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

        // Handle Either patterns by recursively processing each alternative
        // so each inner pattern's conditions are checked
        if let PatternType::Either(inner_patterns) = &pattern.pattern_type {
            for inner_pattern in inner_patterns {
                let inner_findings = self.execute_pattern_analysis(
                    rule,
                    inner_pattern,
                    ast,
                    dataflow_analysis,
                    file_path,
                )?;
                findings.extend(inner_findings);
            }
            return Ok(findings);
        }

        // Preprocess pattern to handle typed metavariable syntax like "($TYPE $VAR).method()"
        let (processed_pattern, type_constraints) = self.preprocess_typed_metavariables(pattern);

        // Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
        let semgrep_pattern = self.convert_pattern_to_semgrep_pattern(&processed_pattern)?;

        // Find pattern matches using the advanced matcher
        let matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;

        // Check if pattern contains ellipsis (indicating potential cross-statement matches)
        let pattern_str = match &processed_pattern.pattern_type {
            PatternType::Simple(s) => s.as_str(),
            _ => "",
        };
        let has_ellipsis = pattern_str.contains("...");

        // If no matches found and symbolic propagation is enabled, try expanding variables
        let matches = if matches.is_empty()
            && self.symbolic_propagator.is_some()
        {
            self.find_matches_via_symbolic_propagation(&semgrep_pattern, ast, &type_constraints)?
        } else {
            matches
        };

        // Heuristic de-dup: keep only smallest, non-overlapping spans to avoid repeated matches
        let mut mm: Vec<(
            (usize, usize),
            usize,
            usize,
            usize,
            usize,
            SemgrepMatchResult,
        )> = matches
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
        mm.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| (a.1, a.2, a.3, a.4).cmp(&(b.1, b.2, b.3, b.4)))
        });

        let overlaps = |a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)| -> bool {
            let (a_sl, a_sc, a_el, a_ec) = a;
            let (b_sl, b_sc, b_el, b_ec) = b;
            // Simple line-based overlap, with basic column checks when on same line
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

        // Get full source code from the root AST node
        let full_source = ast.text().unwrap_or("").to_string();

        eprintln!(
            "DEBUG execute_pattern_analysis: {} matches after dedup",
            filtered.len()
        );

        for match_result in filtered {
            // Check pattern conditions with full source code
            // Also check type constraints from typed metavariable syntax
            let conditions_passed = self.check_pattern_conditions(
                &processed_pattern,
                &match_result,
                dataflow_analysis,
                &full_source,
            )?;

            // Check additional type constraints from typed metavariable preprocessing
            let mut final_conditions_passed = conditions_passed;
            if conditions_passed {
                for (var_name, expected_type) in &type_constraints {
                    // Check if the variable's type matches the expected type
                    if let Some(var_value) = match_result.bindings.get(var_name) {
                        let type_check_passed =
                            self.check_variable_type(var_value, expected_type, &full_source);
                        if !type_check_passed {
                            final_conditions_passed = false;
                            break;
                        }
                    }
                }
            }

            if final_conditions_passed {
                let finding =
                    self.create_finding_from_match(rule, pattern, &match_result, file_path)?;
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

    /// Create a finding from a pattern match
    fn create_finding_from_match(
        &self,
        rule: &Rule,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        file_path: Option<&Path>,
    ) -> Result<Finding> {
        let default_location = || Location {
            file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        };

        let node_location = match_result
            .node
            .location()
            .map(|(start_line, start_col, end_line, end_col)| Location {
                file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            })
            .unwrap_or_else(default_location);

        // If focus-metavariable is set, relocate the finding to the metavar's position
        let location = if let Some(ref focus_vars) = pattern.focus {
            if let Some(first_focus) = focus_vars.first() {
                let var_name = first_focus.strip_prefix('$').unwrap_or(first_focus);
                if let Some(binding) = match_result.bindings.get(var_name) {
                    if let Some((sl, sc, el, ec)) = binding.location {
                        Location {
                            file: file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                            start_line: sl,
                            start_column: sc,
                            end_line: el,
                            end_column: ec,
                        }
                    } else {
                        node_location
                    }
                } else {
                    node_location
                }
            } else {
                node_location
            }
        } else {
            node_location
        };

        let mut message = rule.description.clone();

        // Replace metavariables in message
        for (name, value) in &match_result.bindings {
            let placeholder = format!("${}", name);
            message = message.replace(&placeholder, value.as_ref());
        }

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        let pattern_str = pattern
            .get_pattern_string()
            .unwrap_or(&"<complex pattern>".to_string())
            .clone();
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
            rule.name, &flow.vulnerability_type, flow.source.description, flow.sink.description
        );

        let mut metadata = HashMap::new();
        metadata.insert("rule_name".to_string(), Value::String(rule.name.clone()));
        metadata.insert(
            "analysis_type".to_string(),
            Value::String("dataflow".to_string()),
        );
        metadata.insert(
            "vulnerability_type".to_string(),
            Value::String(flow.vulnerability_type.clone()),
        );
        metadata.insert(
            "confidence".to_string(),
            Value::String(format!("{:.2}", flow.confidence)),
        );

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

    /// Check if a taint flow matches the data flow specification
    fn matches_dataflow_spec(
        &self,
        flow: &astgrep_dataflow::TaintFlow,
        spec: &DataFlowSpec,
    ) -> bool {
        // Simple pattern matching for sources and sinks
        let source_matches = spec
            .sources
            .iter()
            .any(|pattern| match pattern.pattern_type() {
                PatternType::Simple(ref s) => {
                    let text = &flow.source.description;
                    text.contains(s)
                }
                _ => false,
            });

        let sink_matches = spec
            .sinks
            .iter()
            .any(|pattern| match pattern.pattern_type() {
                PatternType::Simple(ref s) => {
                    let text = &flow.sink.description;
                    text.contains(s)
                }
                _ => false,
            });

        source_matches && sink_matches
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

    /// Convert Pattern to SemgrepPattern
    pub(super) fn convert_pattern_to_semgrep_pattern(
        &self,
        pattern: &Pattern,
    ) -> Result<astgrep_core::SemgrepPattern> {
        use astgrep_core::{PatternType as CorePatternType, SemgrepPattern};

        let core_pattern_type = match &pattern.pattern_type {
            crate::PatternType::Simple(pattern_str) => CorePatternType::Simple(pattern_str.clone()),
            crate::PatternType::Either(patterns) => {
                let converted: Result<Vec<_>> = patterns
                    .iter()
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
                let converted: Result<Vec<_>> = patterns
                    .iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::All(converted?)
            }
            crate::PatternType::Any(patterns) => {
                let converted: Result<Vec<_>> = patterns
                    .iter()
                    .map(|p| self.convert_pattern_to_semgrep_pattern(p))
                    .collect();
                CorePatternType::Any(converted?)
            }
        };

        let conditions: Vec<astgrep_core::Condition> = pattern
            .conditions
            .iter()
            .map(|cond| self.convert_condition_to_core(cond))
            .collect::<Result<Vec<_>>>()?;

        Ok(SemgrepPattern {
            pattern_type: core_pattern_type,
            metavariable_pattern: None,
            conditions,
            focus: pattern.focus.clone(),
        })
    }

    /// Convert Condition to core Condition
    fn convert_condition_to_core(&self, condition: &Condition) -> Result<astgrep_core::Condition> {
        use astgrep_core::{
            ComparisonOperator as CoreComparisonOperator, Condition as CoreCondition,
            MetavariableComparison as CoreMetavariableComparison,
        };
        use astgrep_core::{
            MetavariableAnalysisCondition as CoreMetavariableAnalysisCondition,
            MetavariableName as CoreMetavariableName, MetavariableRegex as CoreMetavariableRegex,
            MetavariableType as CoreMetavariableType,
        };

        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                let core_regex = CoreMetavariableRegex {
                    metavariable: metavar_regex.metavariable.clone(),
                    regex: metavar_regex.regex.clone(),
                };
                Ok(CoreCondition::MetavariableRegex(core_regex))
            }
            Condition::MetavariableComparison(metavar_comp) => {
                let core_comp = CoreMetavariableComparison {
                    metavariable: metavar_comp.metavariable.clone(),
                    operator: match &metavar_comp.operator {
                        ComparisonOperator::Equals => CoreComparisonOperator::Equals,
                        ComparisonOperator::NotEquals => CoreComparisonOperator::NotEquals,
                        ComparisonOperator::Contains => CoreComparisonOperator::Contains,
                        ComparisonOperator::StartsWith => CoreComparisonOperator::StartsWith,
                        ComparisonOperator::EndsWith => CoreComparisonOperator::EndsWith,
                        ComparisonOperator::Matches => CoreComparisonOperator::Matches,
                        ComparisonOperator::GreaterThan => CoreComparisonOperator::GreaterThan,
                        ComparisonOperator::LessThan => CoreComparisonOperator::LessThan,
                        ComparisonOperator::PythonExpression(expr) => {
                            CoreComparisonOperator::PythonExpression(expr.clone())
                        }
                    },
                    value: metavar_comp.value.clone(),
                };
                Ok(CoreCondition::MetavariableComparison(core_comp))
            }
            Condition::MetavariableName(metavar_name) => {
                let core_name = CoreMetavariableName {
                    metavariable: metavar_name.metavariable.clone(),
                    name_pattern: metavar_name.name_pattern.clone(),
                };
                Ok(CoreCondition::MetavariableName(core_name))
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                let core_analysis = CoreMetavariableAnalysisCondition {
                    metavariable: metavar_analysis.metavariable.clone(),
                    analysis: metavar_analysis.analysis.clone(),
                };
                Ok(CoreCondition::MetavariableAnalysis(core_analysis))
            }
            Condition::MetavariableType(metavar_type) => {
                let core_type = CoreMetavariableType {
                    metavariable: metavar_type.metavariable.clone(),
                    var_type: metavar_type.var_type.clone(),
                };
                Ok(CoreCondition::MetavariableType(core_type))
            }
            Condition::NodeType(node_type) => Ok(CoreCondition::NodeType(node_type.clone())),
            Condition::NodeAttribute(name, value) => {
                Ok(CoreCondition::NodeAttribute(name.clone(), value.clone()))
            }
            Condition::Custom(value) => Ok(CoreCondition::Custom(value.clone())),
            Condition::MetavariablePattern(_) => {
                // MetavariablePattern is handled directly in the executor conditions,
                // not converted to core Condition. Return a no-op.
                Ok(CoreCondition::Custom(
                    "metavariable_pattern_handled".to_string(),
                ))
            }
        }
    }

    /// Preprocess typed metavariables in pattern
    pub(super) fn preprocess_typed_metavariables(
        &self,
        pattern: &Pattern,
    ) -> (Pattern, Vec<(String, String)>) {
        (pattern.clone(), Vec::new())
    }

    /// Check variable type against expected type
    pub(super) fn check_variable_type(
        &self,
        var_value: &str,
        expected_type: &str,
        full_source: &str,
    ) -> bool {
        let import_map = self.build_import_map(full_source);

        let var_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&var_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str();
                    if let Some(resolved) = self.resolve_type_with_imports(simple_type, &import_map)
                    {
                        if resolved == expected_type || simple_type == expected_type {
                            return true;
                        }
                        if expected_type.ends_with(&format!(".{}", simple_type)) {
                            return true;
                        }
                    }
                }
            }
        }

        if let Some(ref propagator) = self.symbolic_propagator {
            if self.check_type_via_symbolic_propagation(
                var_value,
                expected_type,
                propagator,
                full_source,
            ) {
                return true;
            }
        }

        true
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
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
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

    fn record_rule_execution(
        &mut self,
        rule_id: &str,
        execution_time: std::time::Duration,
        finding_count: usize,
    ) {
        self.rules_executed += 1;
        self.total_findings += finding_count;
        self.total_execution_time += execution_time;
        self.rule_timings
            .insert(rule_id.to_string(), execution_time);
        self.rule_finding_counts
            .insert(rule_id.to_string(), finding_count);
    }

    fn record_rule_error(&mut self, rule_id: &str, execution_time: std::time::Duration) {
        self.rules_executed += 1;
        self.total_execution_time += execution_time;
        self.rule_timings
            .insert(rule_id.to_string(), execution_time);
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

    /// Check if this rule requires symbolic propagation analysis
    pub fn requires_symbolic_propagation(&self) -> bool {
        // Check metadata for symbolic_propagation option
        if let Some(Value::String(val)) = self.metadata.get("symbolic_propagation") {
            return val == "true" || val == "on" || val == "yes" || val == "1";
        }
        if let Some(Value::Bool(val)) = self.metadata.get("symbolic_propagation") {
            return *val;
        }
        // For taint mode rules, enable symbolic propagation by default
        self.mode == crate::types::RuleMode::Taint
    }

    /// Check if this rule has constant propagation enabled
    pub fn has_constant_propagation(&self) -> bool {
        // Check metadata for constant_propagation option
        if let Some(Value::String(val)) = self.metadata.get("constant_propagation") {
            return val == "true" || val == "on" || val == "yes" || val == "1";
        }
        if let Some(Value::Bool(val)) = self.metadata.get("constant_propagation") {
            return *val;
        }
        // Default to true for constant propagation
        true
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

    #[test]
    fn test_advanced_executor_new() {
        let executor = AdvancedRuleExecutor::new();
        let stats = executor.statistics();
        assert_eq!(stats.rules_executed, 0);
        assert_eq!(stats.total_findings, 0);
    }

    #[test]
    fn test_advanced_executor_default() {
        let executor: AdvancedRuleExecutor = Default::default();
        let stats = executor.statistics();
        assert_eq!(stats.rules_executed, 0);
    }

    #[test]
    fn test_advanced_executor_reset() {
        let mut executor = AdvancedRuleExecutor::new();
        executor.execution_stats.record_rule_execution("test", std::time::Duration::from_millis(10), 5);
        assert_eq!(executor.statistics().rules_executed, 1);
        
        executor.reset();
        assert_eq!(executor.statistics().rules_executed, 0);
    }

    #[test]
    fn test_comprehensive_analysis_result_empty() {
        let result = ComprehensiveAnalysisResult::empty(std::time::Duration::from_millis(100));
        assert!(result.findings.is_empty());
        assert!(result.rule_results.is_empty());
        assert!(!result.has_critical_findings());
        let summary = result.summary();
        assert_eq!(summary.total_findings, 0);
    }

    #[test]
    fn test_comprehensive_analysis_result_findings_by_severity() {
        let mut result = ComprehensiveAnalysisResult::empty(std::time::Duration::from_millis(100));
        result.findings.push(Finding {
            rule_id: "rule1".to_string(),
            message: "Error".to_string(),
            location: Location::new(std::path::PathBuf::new(), 1, 1, 1, 1),
            severity: Severity::Error,
            confidence: Confidence::High,
            metadata: HashMap::new(),
            fix_suggestion: None,
        });
        result.findings.push(Finding {
            rule_id: "rule2".to_string(),
            message: "Warning".to_string(),
            location: Location::new(std::path::PathBuf::new(), 1, 1, 1, 1),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            metadata: HashMap::new(),
            fix_suggestion: None,
        });

        assert!(result.has_critical_findings());
        assert_eq!(result.findings_by_severity(Severity::Error).len(), 1);
        assert_eq!(result.findings_by_severity(Severity::Warning).len(), 1);
        
        let summary = result.summary();
        assert_eq!(summary.total_findings, 2);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 1);
    }

    #[test]
    fn test_execution_statistics_record() {
        let mut stats = ExecutionStatistics::new();
        assert_eq!(stats.rules_executed, 0);
        
        stats.record_rule_execution("rule1", std::time::Duration::from_millis(50), 3);
        assert_eq!(stats.rules_executed, 1);
        assert_eq!(stats.total_findings, 3);
        assert_eq!(stats.rule_finding_counts.get("rule1"), Some(&3));
        
        stats.record_rule_error("rule2", std::time::Duration::from_millis(20));
        assert_eq!(stats.rules_executed, 2);
        assert_eq!(stats.total_findings, 3);
    }

    #[test]
    fn test_rule_requires_dataflow() {
        let rule_without = Rule::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        );
        assert!(!rule_without.requires_dataflow());

        let rule_with = Rule::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .with_dataflow(DataFlowSpec::from_strings(
            vec!["source".to_string()],
            vec!["sink".to_string()],
        ));
        assert!(rule_with.requires_dataflow());
    }

    #[test]
    fn test_rule_requires_symbolic_propagation() {
        let rule = Rule::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        );
        assert!(!rule.requires_symbolic_propagation());

        let mut taint_rule = Rule::new(
            "taint".to_string(),
            "taint".to_string(),
            "taint".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .with_dataflow(DataFlowSpec::from_strings(
            vec!["source".to_string()],
            vec!["sink".to_string()],
        ));
        taint_rule.mode = crate::types::RuleMode::Taint;
        assert!(taint_rule.requires_symbolic_propagation());
    }

    #[test]
    fn test_rule_has_constant_propagation() {
        let rule = Rule::new(
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        );
        assert!(rule.has_constant_propagation());

        let disabled = rule.clone().add_metadata(
            "constant_propagation".to_string(),
            "false".to_string(),
        );
        assert!(!disabled.has_constant_propagation());
    }

    #[test]
    fn test_analysis_summary_default() {
        let summary = AnalysisSummary::default();
        assert_eq!(summary.total_findings, 0);
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.rules_executed, 0);
    }

    #[test]
    fn test_comprehensive_analysis_no_applicable_rules() {
        let mut executor = AdvancedRuleExecutor::new();
        let ast = MockAstNode::new("program").with_text("foo");
        let java_rule = Rule::new(
            "java-rule".to_string(),
            "java-rule".to_string(),
            "test".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        );

        let result = executor.execute_comprehensive_analysis(
            &[java_rule],
            &ast,
            Language::Python,
            None,
            false,
        ).unwrap();

        assert!(result.findings.is_empty());
        assert!(result.rule_results.is_empty());
    }
}
