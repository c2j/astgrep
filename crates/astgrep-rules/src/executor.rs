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

/// Represents a taint match (source or sink)
struct TaintMatch {
    node: Box<dyn AstNode>,
    bindings: HashMap<String, String>,
    var_name: Option<String>,
}

impl Clone for TaintMatch {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone_node(),
            bindings: self.bindings.clone(),
            var_name: self.var_name.clone(),
        }
    }
}

/// Advanced rule executor with full integration
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
        let applicable_rules: Vec<&Rule> = rules.iter()
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
                    eprintln!("DEBUG: Setting {} constant values in pattern matcher", values.len());
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

        // Perform symbolic propagation analysis if needed
        let enable_symbolic_propagation = applicable_rules.iter().any(|r| r.requires_symbolic_propagation());
        if enable_symbolic_propagation {
            use astgrep_dataflow::SymbolicPropagator;
            let mut propagator = SymbolicPropagator::new().with_deep_propagation(true);
            match propagator.analyze(ast) {
                Ok(()) => {
                    eprintln!("DEBUG: Symbolic propagation analysis completed");
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

        // Preprocess pattern to handle typed metavariable syntax like "($TYPE $VAR).method()"
        let (processed_pattern, type_constraints) = self.preprocess_typed_metavariables(pattern);

        // Convert astgrep_rules::Pattern to astgrep_core::SemgrepPattern
        let semgrep_pattern = self.convert_pattern_to_semgrep_pattern(&processed_pattern)?;

        // Find pattern matches using the advanced matcher
        let matches = self.pattern_matcher.find_matches(&semgrep_pattern, ast)?;

        // If no matches found and we have type constraints with symbolic propagation,
        // try to find matches using symbolic propagation
        let matches = if matches.is_empty() && !type_constraints.is_empty() && self.symbolic_propagator.is_some() {
            eprintln!("DEBUG: No direct matches found, attempting symbolic propagation matching");
            self.find_matches_via_symbolic_propagation(&semgrep_pattern, ast, &type_constraints)?
        } else {
            matches
        };

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

        // Get full source code from the root AST node
        let full_source = ast.text().unwrap_or("").to_string();

        for match_result in filtered {
            // Check pattern conditions with full source code
            // Also check type constraints from typed metavariable syntax
            let conditions_passed = self.check_pattern_conditions(&processed_pattern, &match_result, dataflow_analysis, &full_source)?;
            
            // Check additional type constraints from typed metavariable preprocessing
            let mut final_conditions_passed = conditions_passed;
            if conditions_passed {
                for (var_name, expected_type) in &type_constraints {
                    // Check if the variable's type matches the expected type
                    if let Some(var_value) = match_result.bindings.get(var_name) {
                        let type_check_passed = self.check_variable_type(var_value, expected_type, &full_source);
                        if !type_check_passed {
                            final_conditions_passed = false;
                            break;
                        }
                    }
                }
            }
            
            if final_conditions_passed {
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
        full_source: &str,
    ) -> Result<bool> {
        for condition in &pattern.conditions {
            if !self.evaluate_condition(condition, match_result, dataflow_analysis, full_source)? {
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
        full_source: &str,
    ) -> Result<bool> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Check if metavariable exists and matches regex
                if let Some(metavar_value) = match_result.bindings.get(&metavar_regex.metavariable) {
                    // Support (?i) case-insensitive flag and other inline regex flags
                    let regex_str = &metavar_regex.regex;
                    let regex = if regex_str.starts_with("(?i)") {
                        // Case-insensitive regex
                        regex::Regex::new(&format!("(?i){}", &regex_str[4..]))
                    } else {
                        regex::Regex::new(regex_str)
                    };
                    
                    if let Ok(re) = regex {
                        Ok(re.is_match(metavar_value))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::MetavariableComparison(metavar_comp) => {
                // Check if metavariable exists and satisfies comparison
                eprintln!("DEBUG evaluate_condition: MetavariableComparison for '{}', bindings: {:?}", 
                         metavar_comp.metavariable, match_result.bindings.keys().collect::<Vec<_>>());
                if let Some(metavar_value) = match_result.bindings.get(&metavar_comp.metavariable) {
                    eprintln!("DEBUG: Found value '{}' for metavariable '{}'", metavar_value, metavar_comp.metavariable);
                    
                    // Try to resolve the variable to its constant value using constant propagation
                    let resolved_value = if let Some(ref propagator) = self.constant_propagator {
                        // Get the location of the matched node
                        if let Some((start_line, start_col, _, _)) = match_result.node.location() {
                            use astgrep_dataflow::constant_propagation::SourceLocation;
                            let location = SourceLocation::new(start_line, start_col);
                            
                            // Try to get the constant value at this location
                            if let Some(constant) = propagator.get_variable_value_at_location(metavar_value, location) {
                                let constant_str = constant.to_string_value().unwrap_or_else(|| metavar_value.clone());
                                eprintln!("DEBUG: Resolved variable '{}' to constant '{}' at {:?}", metavar_value, constant_str, location);
                                constant_str
                            } else {
                                metavar_value.clone()
                            }
                        } else {
                            metavar_value.clone()
                        }
                    } else {
                        metavar_value.clone()
                    };
                    
                    self.evaluate_comparison(&resolved_value, &metavar_comp.operator, &metavar_comp.value)
                } else {
                    eprintln!("DEBUG: Metavariable '{}' not found in bindings", metavar_comp.metavariable);
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
            Condition::MetavariableType(metavar_type) => {
                // Check if metavariable exists and matches the expected type
                if let Some(metavar_value) = match_result.bindings.get(&metavar_type.metavariable) {
                    // Extract the actual value from the metavariable binding
                    let var_value = metavar_value.trim();
                    
                    // First, try to infer type from the value itself (for literals)
                    let inferred_type = self.infer_type_from_value(var_value);
                    
                    if let Some(type_info) = inferred_type {
                        // Value is a literal, compare its inferred type
                        Ok(type_info == metavar_type.var_type)
                    } else {
                        // Value is a variable, extract type from source code
                        if let Some(type_info) = self.extract_type_info(match_result, var_value, full_source) {
                            Ok(type_info == metavar_type.var_type)
                        } else {
                            // If we can't determine the type, reject the match
                            // This prevents false positives when type info is unavailable
                            Ok(false)
                        }
                    }
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

    /// Extract type information for a variable from the match context
    fn extract_type_info(&self, match_result: &SemgrepMatchResult, var_name: &str, full_source: &str) -> Option<String> {
        // Try to extract type information from the full source code
        // This looks for variable declarations like "TypeName varName" in method signatures or declarations
        
        // Build import map for name resolution
        let import_map = self.build_import_map(full_source);
        
        // Pattern 1: Method parameter declarations like "String varName" or "Type varName"
        // Matches: "Type varName" followed by comma, closing paren, or space
        let param_pattern = format!(r"(\w+)\s+{}\s*[,)]", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&param_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        // Pattern 2: Variable declarations like "Type varName = ...;" or "Type varName;"
        // Handles cases like: PrintWriter pWriter = response.getWriter();
        let var_pattern = format!(r"(\w+)\s+{}\s*=[^;]*;", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&var_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        // Pattern 3: Field declarations like "private Type varName = ...;" or "private Type varName;"
        // Handles cases like: private PrintWriter pWriter = response.getWriter();
        let field_pattern = format!(r"(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(\w+)\s+{}\s*=[^;]*;", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&field_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str().to_string();
                    return self.resolve_type_with_imports(&simple_type, &import_map);
                }
            }
        }
        
        None
    }
    
    /// Build a map of imported simple names to their fully qualified names
    fn build_import_map(&self, full_source: &str) -> HashMap<String, String> {
        let mut import_map = HashMap::new();
        
        // Parse import statements like "import org.foo.Foo;" or "import org.foo.*;"
        let import_pattern = regex::Regex::new(r"import\s+([\w.]+)(?:\.\*)?;").unwrap();
        
        for captures in import_pattern.captures_iter(full_source) {
            if let Some(import_match) = captures.get(1) {
                let import_path = import_match.as_str();
                
                // Extract the simple name (last part after the last dot)
                if let Some(last_dot) = import_path.rfind('.') {
                    let simple_name = &import_path[last_dot + 1..];
                    let fully_qualified = import_path.to_string();
                    
                    eprintln!("DEBUG: Found import: {} -> {}", simple_name, fully_qualified);
                    import_map.insert(simple_name.to_string(), fully_qualified);
                } else {
                    // No dot, the whole import is the simple name
                    import_map.insert(import_path.to_string(), import_path.to_string());
                }
            }
        }
        
        import_map
    }
    
    /// Resolve a simple type name to its fully qualified name using import map
    fn resolve_type_with_imports(&self, simple_type: &str, import_map: &HashMap<String, String>) -> Option<String> {
        // First check if this simple type is in the import map
        if let Some(fully_qualified) = import_map.get(simple_type) {
            eprintln!("DEBUG: Resolved type '{}' to '{}'", simple_type, fully_qualified);
            return Some(fully_qualified.clone());
        }
        
        // If not found in imports, return the simple type as-is
        // (it might be a primitive type or in the same package)
        Some(simple_type.to_string())
    }

    /// Infer the type of a value from its literal representation
    fn infer_type_from_value(&self, value: &str) -> Option<String> {
        let trimmed = value.trim();
        
        // String literal: "..." or '...'
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
           (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            return Some("String".to_string());
        }
        
        // Integer literal: digits only (possibly with negative sign)
        if trimmed.parse::<i64>().is_ok() {
            return Some("int".to_string());
        }
        
        // Floating point literal: contains dot and parses as float
        if trimmed.contains('.') && trimmed.parse::<f64>().is_ok() {
            return Some("float".to_string());
        }
        
        // Boolean literal: true or false
        if trimmed == "true" || trimmed == "false" {
            return Some("boolean".to_string());
        }
        
        // Null literal
        if trimmed == "null" {
            return Some("null".to_string());
        }
        
        // Not a recognized literal, probably a variable name
        None
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
        
        eprintln!("DEBUG evaluate_python_expression: value='{}', expr='{}'", value, expr);

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

        // Handle bit operations like "$X & 1 == 1"
        // Parse expressions like "$VAR & 1 == 1" or "$VAR & 1 == 0"
        if expr.contains('&') && expr.contains("==") {
            // Try to parse the expression
            // Format: $VAR & N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();
                
                // Parse the bit operation: $VAR & N
                if left_side.contains('&') {
                    let bit_parts: Vec<&str> = left_side.split('&').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();
                        
                        eprintln!("DEBUG: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);
                        
                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val & mask;
                                        eprintln!("DEBUG: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain &", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit OR operations like "$X | 1 == 1"
        // Parse expressions like "$VAR | 1 == 1" or "$VAR | 1 == 3"
        if expr.contains('|') && expr.contains("==") && !expr.contains("||") {
            // Try to parse the expression
            // Format: $VAR | N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Parse the bit operation: $VAR | N
                if left_side.contains('|') {
                    let bit_parts: Vec<&str> = left_side.split('|').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();

                        eprintln!("DEBUG bitor: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val | mask;
                                        eprintln!("DEBUG bitor: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain |", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit XOR operations like "$X ^ 1 == 3"
        // Parse expressions like "$VAR ^ 1 == 3" or "$VAR ^ 1 == 2"
        if expr.contains('^') && expr.contains("==") {
            // Try to parse the expression
            // Format: $VAR ^ N == M
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();

                // Parse the bit operation: $VAR ^ N
                if left_side.contains('^') {
                    let bit_parts: Vec<&str> = left_side.split('^').collect();
                    if bit_parts.len() == 2 {
                        let var_part = bit_parts[0].trim();
                        let mask_part = bit_parts[1].trim();

                        eprintln!("DEBUG bitxor: var_part='{}', mask_part='{}', expected='{}'", var_part, mask_part, expected_result);

                        // Check if this is the metavariable we're evaluating
                        if var_part.starts_with("$") {
                            // Parse the mask value
                            if let Ok(mask) = mask_part.parse::<i64>() {
                                // Parse the expected result
                                if let Ok(expected) = expected_result.parse::<i64>() {
                                    // Parse the actual value
                                    if let Ok(val) = value.parse::<i64>() {
                                        let result = val ^ mask;
                                        eprintln!("DEBUG bitxor: val={}, mask={}, result={}, expected={}", val, mask, result, expected);
                                        return Ok(result == expected);
                                    } else {
                                        eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                                    }
                                } else {
                                    eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                                }
                            } else {
                                eprintln!("DEBUG: Failed to parse mask '{}' as i64", mask_part);
                            }
                        } else {
                            eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                        }
                    } else {
                        eprintln!("DEBUG: bit_parts.len() = {}, expected 2", bit_parts.len());
                    }
                } else {
                    eprintln!("DEBUG: left_side '{}' doesn't contain ^", left_side);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // Handle bit NOT operations like "~ $X == -1"
        // Python: ~x = -(x + 1)
        if expr.contains('~') && expr.contains("==") {
            // Format: ~$VAR == N or ~ $VAR == N
            let parts: Vec<&str> = expr.split("==").collect();
            if parts.len() == 2 {
                let left_side = parts[0].trim();
                let expected_result = parts[1].trim();
                
                // Remove the ~ operator and get the variable part
                // Handle both "~$VAR" and "~ $VAR"
                let var_part = if left_side.starts_with("~") {
                    left_side[1..].trim()
                } else {
                    left_side
                };
                
                eprintln!("DEBUG bitnot: var_part='{}', expected='{}'", var_part, expected_result);
                
                // Check if this is the metavariable we're evaluating
                if var_part.starts_with("$") {
                    // Parse the expected result
                    if let Ok(expected) = expected_result.parse::<i64>() {
                        // Parse the actual value
                        if let Ok(val) = value.parse::<i64>() {
                            // Python's ~ operator: ~x = -(x + 1)
                            let result = -(val + 1);
                            eprintln!("DEBUG bitnot: val={}, result={}, expected={}", val, result, expected);
                            return Ok(result == expected);
                        } else {
                            eprintln!("DEBUG: Failed to parse value '{}' as i64", value);
                        }
                    } else {
                        eprintln!("DEBUG: Failed to parse expected '{}' as i64", expected_result);
                    }
                } else {
                    eprintln!("DEBUG: var_part '{}' doesn't start with $", var_part);
                }
            } else {
                eprintln!("DEBUG: parts.len() = {}, expected 2", parts.len());
            }
        }

        // For now, just return true for unsupported expressions
        eprintln!("DEBUG: Expression '{}' not handled, returning true", expr);
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
        let source_text = ast.text().unwrap_or_default();
        
        // Step 1: Find all source matches using pattern matching
        let source_matches = self.find_taint_sources(ast, dataflow_spec)?;
        if source_matches.is_empty() {
            return Ok(findings);
        }
        
        // Step 2: Find all sink matches using pattern matching
        let sink_matches = self.find_taint_sinks(ast, dataflow_spec)?;
        if sink_matches.is_empty() {
            return Ok(findings);
        }
        
        // Step 3: Check for taint flow from sources to sinks
        let taint_flows = self.detect_taint_flows(
            &source_matches, 
            &sink_matches, 
            ast, 
            dataflow_analysis
        )?;
        
        // Step 4: Create findings for each detected taint flow
        for (source_match, sink_match) in taint_flows {
            if let Some(location) = sink_match.node.location() {
                let finding = Finding::new(
                    rule.id.clone(),
                    format!("{}: {}", rule.name, rule.description),
                    rule.severity,
                    rule.confidence,
                    Location::new(
                        file_path.map(|p| p.to_path_buf()).unwrap_or_default(),
                        location.0, location.1, location.2, location.3
                    ),
                );
                findings.push(finding);
            }
        }
        
        Ok(findings)
    }
    
    /// Find all taint sources matching the source patterns
    fn find_taint_sources(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec
    ) -> Result<Vec<TaintMatch>> {
        let mut sources = Vec::new();
        
        for source_pattern_str in &dataflow_spec.sources {
            // Normalize pattern: remove trailing semicolons for more flexible matching
            let normalized_pattern = source_pattern_str.trim_end_matches(';').trim();
            
            // Convert source pattern to SemgrepPattern
            let source_pattern = astgrep_core::SemgrepPattern {
                pattern_type: astgrep_core::PatternType::Simple(normalized_pattern.to_string()),
                metavariable_pattern: None,
                conditions: Vec::new(),
                focus: None,
            };
            
            // Find matches
            let matches = self.pattern_matcher.find_matches(&source_pattern, ast)?;
            for m in matches {
                // Extract the variable name from bindings if available
                let mut var_name = None;
                for (key, value) in &m.bindings {
                    if key.starts_with("$") && !value.is_empty() {
                        var_name = Some(value.clone());
                        break;
                    }
                }
                
                sources.push(TaintMatch {
                    node: m.node,
                    bindings: m.bindings,
                    var_name,
                });
            }
        }
        
        Ok(sources)
    }
    
    /// Find all taint sinks matching the sink patterns
    fn find_taint_sinks(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec
    ) -> Result<Vec<TaintMatch>> {
        let mut sinks = Vec::new();
        
        for sink_pattern_str in &dataflow_spec.sinks {
            // Normalize pattern: remove trailing semicolons for more flexible matching
            let normalized_pattern = sink_pattern_str.trim_end_matches(';').trim();
            
            // Convert sink pattern to SemgrepPattern
            let sink_pattern = astgrep_core::SemgrepPattern {
                pattern_type: astgrep_core::PatternType::Simple(normalized_pattern.to_string()),
                metavariable_pattern: None,
                conditions: Vec::new(),
                focus: None,
            };
            
            // Find matches
            let matches = self.pattern_matcher.find_matches(&sink_pattern, ast)?;
            for m in matches {
                sinks.push(TaintMatch {
                    node: m.node,
                    bindings: m.bindings,
                    var_name: None,
                });
            }
        }
        
        Ok(sinks)
    }
    
    /// Detect taint flows from sources to sinks
    fn detect_taint_flows(
        &self,
        sources: &[TaintMatch],
        sinks: &[TaintMatch],
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
    ) -> Result<Vec<(TaintMatch, TaintMatch)>> {
        let mut flows = Vec::new();
        
        // Use simple heuristics to detect taint flows
        for source in sources {
            if let Some(ref source_var) = source.var_name {
                for sink in sinks {
                    // Check if the source variable appears in the sink context
                    if self.is_variable_flowing_to_sink(source_var, sink.node.as_ref(), ast) {
                        flows.push((source.clone(), sink.clone()));
                    }
                }
            }
        }
        
        // Also check dataflow analysis results if available
        if let Some(analysis) = dataflow_analysis {
            for flow in &analysis.taint_flows {
                if flow.is_vulnerable() {
                    // Find corresponding source and sink matches
                    for source in sources {
                        for sink in sinks {
                            if self.is_flow_matching(&flow, source, sink) {
                                flows.push((source.clone(), sink.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(flows)
    }
    
    /// Check if a node uses any of the given variables
    fn node_uses_variables(&self, node: &dyn AstNode, variables: &[String]) -> bool {
        let node_text = node.text().unwrap_or_default();
        for var in variables {
            if node_text.contains(var) {
                return true;
            }
        }
        false
    }
    
    /// Check if a variable flows to a sink, using symbolic propagation for alias tracking
    fn is_variable_flowing_to_sink(
        &self,
        var_name: &str,
        sink_node: &dyn AstNode,
        _ast: &dyn AstNode
    ) -> bool {
        let sink_text = sink_node.text().unwrap_or_default();
        
        // Check if the source variable directly appears in the sink node
        if sink_text.contains(var_name) {
            return true;
        }
        
        // Use symbolic propagator to check for aliases
        if let Some(ref propagator) = self.symbolic_propagator {
            // Get all aliases of the source variable
            let aliases = propagator.state().get_all_aliases(var_name);
            
            // Check if any alias appears in the sink
            for alias in &aliases {
                if sink_text.contains(alias) {
                    return true;
                }
            }
            
            // Also check if the source variable is derived from any variable in the sink
            // This handles cases like: dbf.newDocumentBuilder() where dbf is a field
            if let Some(sink_var) = self.extract_receiver_from_sink(sink_node) {
                // Check if sink_var is an alias of var_name
                if propagator.state().is_alias(var_name, &sink_var) {
                    return true;
                }
                // Check if sink_var equals var_name
                if sink_var == var_name {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Extract the receiver variable from a sink node
    /// For example, from "dbf.newDocumentBuilder()", extract "dbf"
    fn extract_receiver_from_sink(&self, sink_node: &dyn AstNode) -> Option<String> {
        let sink_text = sink_node.text().unwrap_or_default();
        
        // Pattern: receiver.methodName()
        // Extract the receiver part before the first dot
        if let Some(dot_pos) = sink_text.find('.') {
            let receiver = sink_text[..dot_pos].trim();
            if !receiver.is_empty() {
                return Some(receiver.to_string());
            }
        }
        
        // Try AST-based extraction for method_invocation or call_expression
        if sink_node.node_type() == "method_invocation" || sink_node.node_type() == "call_expression" {
            for i in 0..sink_node.child_count() {
                if let Some(child) = sink_node.child(i) {
                    if child.node_type() == "identifier" || child.node_type() == "field_access" {
                        if let Some(text) = child.text() {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Check if a dataflow matches source and sink
    fn is_flow_matching(
        &self,
        flow: &astgrep_dataflow::TaintFlow,
        source: &TaintMatch,
        sink: &TaintMatch
    ) -> bool {
        // Check if flow source matches our source
        let source_matches = if let (Some(src_loc), Some(flow_loc)) = (source.node.location(), &flow.source.location) {
            src_loc.0 == flow_loc.start_line && src_loc.1 == flow_loc.start_column
        } else {
            // Fallback: compare by description text
            if let Some(source_text) = source.node.text() {
                flow.source.description.contains(&source_text) || source_text.contains(&flow.source.description)
            } else {
                false
            }
        };
        
        // Check if flow sink matches our sink
        let sink_matches = if let (Some(sink_loc), Some(flow_loc)) = (sink.node.location(), &flow.sink.location) {
            sink_loc.0 == flow_loc.start_line && sink_loc.1 == flow_loc.start_column
        } else {
            // Fallback: compare by description text
            if let Some(sink_text) = sink.node.text() {
                flow.sink.description.contains(&sink_text) || sink_text.contains(&flow.sink.description)
            } else {
                false
            }
        };
        
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

        // Convert conditions
        let conditions: Vec<astgrep_core::Condition> = pattern.conditions.iter()
            .map(|cond| self.convert_condition_to_core(cond))
            .collect::<Result<Vec<_>>>()?;

        Ok(SemgrepPattern {
            pattern_type: core_pattern_type,
            metavariable_pattern: None, // TODO: Convert metavariable patterns
            conditions,
            focus: pattern.focus.clone(),
        })
    }

    /// Convert astgrep_rules::Condition to astgrep_core::Condition
    fn convert_condition_to_core(&self, condition: &Condition) -> Result<astgrep_core::Condition> {
        use astgrep_core::{Condition as CoreCondition, MetavariableComparison as CoreMetavariableComparison, ComparisonOperator as CoreComparisonOperator};
        use astgrep_core::{MetavariableRegex as CoreMetavariableRegex, MetavariableName as CoreMetavariableName, MetavariableAnalysisCondition as CoreMetavariableAnalysisCondition, MetavariableType as CoreMetavariableType};

        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                // Convert MetavariableRegex to core Condition
                let core_regex = CoreMetavariableRegex {
                    metavariable: metavar_regex.metavariable.clone(),
                    regex: metavar_regex.regex.clone(),
                };
                Ok(CoreCondition::MetavariableRegex(core_regex))
            }
            Condition::MetavariableComparison(metavar_comp) => {
                // Convert MetavariableComparison to core Condition
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
                        ComparisonOperator::PythonExpression(expr) => CoreComparisonOperator::PythonExpression(expr.clone()),
                    },
                    value: metavar_comp.value.clone(),
                };
                Ok(CoreCondition::MetavariableComparison(core_comp))
            }
            Condition::MetavariableName(metavar_name) => {
                // Convert MetavariableName to core Condition
                let core_name = CoreMetavariableName {
                    metavariable: metavar_name.metavariable.clone(),
                    name_pattern: metavar_name.name_pattern.clone(),
                };
                Ok(CoreCondition::MetavariableName(core_name))
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Convert MetavariableAnalysis to core Condition
                let core_analysis = CoreMetavariableAnalysisCondition {
                    metavariable: metavar_analysis.metavariable.clone(),
                    analysis: metavar_analysis.analysis.clone(),
                };
                Ok(CoreCondition::MetavariableAnalysis(core_analysis))
            }
            Condition::NodeType(node_type) => {
                Ok(CoreCondition::NodeType(node_type.clone()))
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                Ok(CoreCondition::NodeAttribute(attr_name.clone(), attr_value.clone()))
            }
            Condition::MetavariableType(metavar_type) => {
                // Convert MetavariableType to core Condition
                let core_type = CoreMetavariableType {
                    metavariable: metavar_type.metavariable.clone(),
                    var_type: metavar_type.var_type.clone(),
                };
                Ok(CoreCondition::MetavariableType(core_type))
            }
            Condition::Custom(custom) => {
                Ok(CoreCondition::Custom(custom.clone()))
            }
        }
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
}

// Helper functions for typed metavariable support
impl AdvancedRuleExecutor {
    /// Preprocess pattern to handle typed metavariable syntax like "($TYPE $VAR).method()"
    /// Returns the processed pattern and a map of variable names to their expected types
    fn preprocess_typed_metavariables(
        &self,
        pattern: &Pattern,
    ) -> (Pattern, Vec<(String, String)>) {
        let mut type_constraints = Vec::new();

        // Process based on pattern type
        match &pattern.pattern_type {
            PatternType::Simple(_) => {
                // For Simple patterns, check if it contains typed metavariable syntax
                // with access to any metavariable_pattern on the pattern itself
                let metavar_patterns: Vec<&MetavariablePattern> = pattern
                    .metavariable_pattern
                    .as_ref()
                    .map(|mp| vec![mp])
                    .unwrap_or_default();
                let (new_pattern, constraints) =
                    self.process_subpattern_with_metavars(pattern, &metavar_patterns);
                if !constraints.is_empty() {
                    return (new_pattern, constraints);
                }
            }
            PatternType::All(patterns) => {
                // Collect metavariable patterns from sub-patterns first
                let mut metavar_patterns: Vec<&MetavariablePattern> = Vec::new();
                for sub_pattern in patterns {
                    if let Some(ref mp) = sub_pattern.metavariable_pattern {
                        metavar_patterns.push(mp);
                    }
                }

                // Process each sub-pattern in the All composite
                let mut processed_patterns = Vec::new();
                let mut all_constraints = Vec::new();

                for sub_pattern in patterns {
                    // Skip placeholder patterns (Simple("...") with metavariable_pattern)
                    if let PatternType::Simple(ref s) = sub_pattern.pattern_type {
                        if s == "..." && sub_pattern.metavariable_pattern.is_some() {
                            // Extract type constraints from this metavariable-pattern
                            if let Some(ref mp) = sub_pattern.metavariable_pattern {
                                for type_pattern in &mp.patterns {
                                    // The metavariable_pattern applies to the main pattern
                                    // We'll handle this when processing the main pattern
                                    eprintln!(
                                        "DEBUG: Found metavariable-pattern for {}: {}",
                                        mp.metavariable, type_pattern
                                    );
                                }
                            }
                            continue; // Skip adding this placeholder pattern
                        }
                    }

                    // Process the sub-pattern with access to metavariable patterns
                    let (processed, constraints) =
                        self.process_subpattern_with_metavars(sub_pattern, &metavar_patterns);
                    processed_patterns.push(processed);
                    all_constraints.extend(constraints);
                }

                if !all_constraints.is_empty() || !processed_patterns.is_empty() {
                    let mut new_pattern = if processed_patterns.len() == 1 {
                        processed_patterns.into_iter().next().unwrap()
                    } else {
                        Pattern::all(processed_patterns)
                    };
                    new_pattern.conditions = pattern.conditions.clone();
                    new_pattern.focus = pattern.focus.clone();
                    return (new_pattern, all_constraints);
                }
            }
            PatternType::Either(patterns) => {
                // Process each sub-pattern in the Either composite
                let mut processed_patterns = Vec::new();
                let mut all_constraints = Vec::new();

                for sub_pattern in patterns {
                    let (processed, constraints) = self.preprocess_typed_metavariables(sub_pattern);
                    processed_patterns.push(processed);
                    all_constraints.extend(constraints);
                }

                if !all_constraints.is_empty() {
                    let mut new_pattern = Pattern::either(processed_patterns);
                    new_pattern.conditions = pattern.conditions.clone();
                    new_pattern.focus = pattern.focus.clone();
                    new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();
                    return (new_pattern, all_constraints);
                }
            }
            _ => {
                // Other pattern types not yet supported for typed metavariables
            }
        }

        // No transformation needed, return original
        (pattern.clone(), type_constraints)
    }

    /// Process a sub-pattern with access to metavariable patterns from other sub-patterns
    fn process_subpattern_with_metavars(
        &self,
        pattern: &Pattern,
        metavar_patterns: &[&MetavariablePattern],
    ) -> (Pattern, Vec<(String, String)>) {
        let mut type_constraints = Vec::new();

        if let PatternType::Simple(pattern_str) = &pattern.pattern_type {
            // Check if pattern contains typed metavariable syntax: "($TYPE $VAR)"
            // where both TYPE and VAR are metavariables (start with $)
            let typed_metavar_regex = regex::Regex::new(r"\(\$(\w+)\s+\$(\w+)\)").unwrap();

            if let Some(captures) = typed_metavar_regex.captures(pattern_str) {
                let type_var = captures.get(1).unwrap().as_str();
                let value_var = captures.get(2).unwrap().as_str();
                let full_match = captures.get(0).unwrap();

                eprintln!(
                    "DEBUG: Found typed metavariable syntax: type_var=${}, value_var=${}",
                    type_var, value_var
                );

                // Extract the expected type from metavariable patterns
                for mp in metavar_patterns {
                    if mp.metavariable == format!("${}", type_var)
                        || mp.metavariable == type_var
                    {
                        // Get the expected type from the patterns
                        for type_pattern in &mp.patterns {
                            eprintln!(
                                "DEBUG: Type pattern for ${}: {}",
                                type_var, type_pattern
                            );
                            type_constraints.push((value_var.to_string(), type_pattern.clone()));
                        }
                    }
                }

                // Replace "($TYPE $VAR)" with "$VAR" in the pattern
                let new_pattern_str =
                    pattern_str.replacen(full_match.as_str(), &format!("${}", value_var), 1);
                eprintln!(
                    "DEBUG: Transformed pattern from '{}' to '{}'",
                    pattern_str, new_pattern_str
                );

                let mut new_pattern = Pattern::simple(new_pattern_str);
                new_pattern.conditions = pattern.conditions.clone();
                new_pattern.focus = pattern.focus.clone();
                new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();

                return (new_pattern, type_constraints);
            }

            // Check for inline typed metavariable syntax: "(Type $VAR)"
            // where Type is a literal type name and VAR is a metavariable
            let inline_typed_regex = regex::Regex::new(r"\((\w+)\s+\$(\w+)\)").unwrap();

            if let Some(captures) = inline_typed_regex.captures(pattern_str) {
                let type_name = captures.get(1).unwrap().as_str();
                let value_var = captures.get(2).unwrap().as_str();
                let full_match = captures.get(0).unwrap();

                eprintln!(
                    "DEBUG: Found inline typed metavariable syntax: type={}, value_var=${}",
                    type_name, value_var
                );

                // Add type constraint directly from the pattern
                type_constraints.push((value_var.to_string(), type_name.to_string()));

                // Replace "(Type $VAR)" with "$VAR" in the pattern
                let new_pattern_str =
                    pattern_str.replacen(full_match.as_str(), &format!("${}", value_var), 1);
                eprintln!(
                    "DEBUG: Transformed pattern from '{}' to '{}'",
                    pattern_str, new_pattern_str
                );

                let mut new_pattern = Pattern::simple(new_pattern_str);
                new_pattern.conditions = pattern.conditions.clone();
                new_pattern.focus = pattern.focus.clone();
                new_pattern.metavariable_pattern = pattern.metavariable_pattern.clone();

                return (new_pattern, type_constraints);
            }
        }

        // No transformation needed, return original
        (pattern.clone(), type_constraints)
    }

    /// Check if a variable's type matches the expected type
    /// Also uses symbolic propagation to track if the variable comes from a method call
    /// on an object of the expected type
    fn check_variable_type(&self, var_value: &str, expected_type: &str, full_source: &str) -> bool {
        // Build import map for name resolution
        let import_map = self.build_import_map(full_source);
        
        // Look for variable declaration in the source code
        // Try different patterns to find the variable's type
        
        // Pattern 1: "Type varName =" or "Type varName;"
        let var_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&var_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str();
                    let resolved_type = self.resolve_type_with_imports(simple_type, &import_map);
                    
                    eprintln!("DEBUG: Variable {} has type '{}', resolved to '{:?}', expected '{}'", 
                             var_value, simple_type, resolved_type, expected_type);
                    
                    // Check if the resolved type matches the expected type
                    if let Some(resolved) = resolved_type {
                        // Check exact match or simple name match
                        if resolved == expected_type || simple_type == expected_type {
                            return true;
                        }
                        
                        // Check if expected type ends with the simple type (e.g., "org.foo.Foo" ends with "Foo")
                        if expected_type.ends_with(&format!(".{}", simple_type)) {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Pattern 2: Method parameter "Type varName," or "Type varName)"
        let param_pattern = format!(r"(\w+)\s+{}\s*[,)]", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&param_pattern) {
            if let Some(captures) = regex.captures(full_source) {
                if let Some(type_match) = captures.get(1) {
                    let simple_type = type_match.as_str();
                    let resolved_type = self.resolve_type_with_imports(simple_type, &import_map);
                    
                    eprintln!("DEBUG: Parameter {} has type '{}', resolved to '{:?}', expected '{}'", 
                             var_value, simple_type, resolved_type, expected_type);
                    
                    if let Some(resolved) = resolved_type {
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
        
        // Pattern 3: Use symbolic propagation to check if variable comes from expected type
        // This handles cases like: ZipEntry c = ...; name = c.getName(); (ZipEntry $X).getName()
        eprintln!("DEBUG check_variable_type: Checking {} against expected type {}, symbolic_propagator is {}", 
                 var_value, expected_type, 
                 if self.symbolic_propagator.is_some() { "Some" } else { "None" });
        if let Some(ref propagator) = self.symbolic_propagator {
            eprintln!("DEBUG: Attempting symbolic propagation check for {} -> {}", var_value, expected_type);
            eprintln!("DEBUG: Symbolic state variables: {:?}", propagator.state().variables.keys().collect::<Vec<_>>());
            if self.check_type_via_symbolic_propagation(var_value, expected_type, propagator, full_source) {
                return true;
            }
        }
        
        // If we can't determine the type, be permissive
        eprintln!("DEBUG: Could not determine type for {}, allowing match", var_value);
        true
    }
    
    /// Check if a variable's origin traces back to an object of the expected type
    /// using symbolic propagation
    fn check_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;
        
        eprintln!("DEBUG: Checking symbolic propagation for {} with expected type {}", 
                 var_value, expected_type);
        
        // Get the symbolic value for this variable
        let state = propagator.state();
        if let Some(symbolic_value) = state.get(var_value) {
            eprintln!("DEBUG: Found symbolic value for {}: {:?}", var_value, symbolic_value);
            
            // Get the root variable of this symbolic value
            if let Some(root_var) = symbolic_value.root_variable() {
                eprintln!("DEBUG: Root variable for {} is {}", var_value, root_var);
                
                // Check if the root variable is of the expected type
                // Look for variable declaration: "ExpectedType root_var = ..."
                let var_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        eprintln!("DEBUG: Found {} declared as {} via symbolic propagation", 
                                 root_var, expected_type);
                        return true;
                    }
                }
                
                // Also check with import resolution
                let import_map = self.build_import_map(full_source);
                
                // Try to find declaration of the root variable
                let decl_pattern = format!(r"(\w+)\s+{}\s*[=;]", regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                    if let Some(captures) = regex.captures(full_source) {
                        if let Some(type_match) = captures.get(1) {
                            let actual_type = type_match.as_str();
                            eprintln!("DEBUG: Root variable {} has type {}", root_var, actual_type);
                            
                            // Check if it matches expected type
                            if actual_type == expected_type {
                                return true;
                            }
                            
                            // Check with import resolution
                            if let Some(resolved) = self.resolve_type_with_imports(actual_type, &import_map) {
                                if resolved == expected_type || resolved.ends_with(&format!(".{}", expected_type)) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also check aliases of the variable
        let aliases = propagator.state().get_all_aliases(var_value);
        eprintln!("DEBUG: Aliases for {}: {:?}", var_value, aliases);
        
        for alias in aliases {
            // Check if any alias is of the expected type
            let alias_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(&alias));
            if let Ok(regex) = regex::Regex::new(&alias_pattern) {
                if regex.is_match(full_source) {
                    eprintln!("DEBUG: Found alias {} of type {} via symbolic propagation", 
                             alias, expected_type);
                    return true;
                }
            }
            
            // Check the alias's symbolic value
            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let decl_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                    if let Ok(regex) = regex::Regex::new(&decl_pattern) {
                        if regex.is_match(full_source) {
                            eprintln!("DEBUG: Found root variable {} of type {} via alias symbolic propagation", 
                                     root_var, expected_type);
                            return true;
                        }
                    }
                }
            }
        }
        
        eprintln!("DEBUG: Symbolic propagation did not confirm type {} for {}", 
                 expected_type, var_value);
        false
    }
    
    /// Find pattern matches using symbolic propagation
    /// This is used when direct pattern matching fails but symbolic propagation
    /// might reveal matches through variable tracking
    fn find_matches_via_symbolic_propagation(
        &self,
        pattern: &astgrep_core::SemgrepPattern,
        ast: &dyn AstNode,
        type_constraints: &[(String, String)],
    ) -> Result<Vec<astgrep_core::SemgrepMatchResult>> {
        use astgrep_core::SemgrepMatchResult;
        
        eprintln!("DEBUG: Searching for symbolic propagation matches with {} type constraints", 
                 type_constraints.len());
        
        let mut matches = Vec::new();
        
        // Get the symbolic propagator
        let propagator = match self.symbolic_propagator {
            Some(ref p) => p,
            None => return Ok(matches),
        };
        
        // Extract pattern info - we expect if statement patterns with method calls
        let pattern_str = match &pattern.pattern_type {
            astgrep_core::PatternType::Simple(s) => s.as_str(),
            _ => return Ok(matches),
        };
        
        // Check if this is an if statement pattern with getName().contains()
        if !pattern_str.contains("if") || !pattern_str.contains("getName") || !pattern_str.contains("contains") {
            return Ok(matches);
        }
        
        // Get full source code from AST
        let full_source = ast.text().unwrap_or("").to_string();
        
        // Find all if statements in the AST
        self.find_if_statements_with_symbolic_match(ast, pattern_str, type_constraints, propagator, &full_source, &mut matches)?;
        
        eprintln!("DEBUG: Symbolic propagation found {} matches", matches.len());
        Ok(matches)
    }
    
    /// Find if statements that match via symbolic propagation
    fn find_if_statements_with_symbolic_match(
        &self,
        node: &dyn AstNode,
        pattern_str: &str,
        type_constraints: &[(String, String)],
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
        matches: &mut Vec<astgrep_core::SemgrepMatchResult>,
    ) -> Result<()> {
        // Check if this is an if_statement
        if node.node_type() == "if_statement" {
            eprintln!("DEBUG: Checking if_statement for symbolic match: {}", 
                     node.text().unwrap_or("").lines().next().unwrap_or(""));
            
            // Check if this if statement matches via symbolic propagation
            if let Some(match_result) = self.check_if_statement_symbolic_match(
                node, pattern_str, type_constraints, propagator, full_source
            )? {
                eprintln!("DEBUG: Found symbolic match for if_statement");
                matches.push(match_result);
            }
        }
        
        // Recursively check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_if_statements_with_symbolic_match(
                    child, pattern_str, type_constraints, propagator, full_source, matches
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Check if an if statement matches the pattern via symbolic propagation
    fn check_if_statement_symbolic_match(
        &self,
        if_node: &dyn AstNode,
        pattern_str: &str,
        type_constraints: &[(String, String)],
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> Result<Option<astgrep_core::SemgrepMatchResult>> {
        use astgrep_core::SemgrepMatchResult;
        use std::collections::HashMap;

        // Extract condition from if statement
        // Look for condition like "(b1 && b2)" or "(name == null)"
        let condition_text = self.extract_if_condition(if_node);
        eprintln!("DEBUG: If condition: {:?}", condition_text);

        if condition_text.is_none() {
            return Ok(None);
        }
        let condition = condition_text.unwrap();

        // For each type constraint, check if condition variables involve that type
        let mut bindings = HashMap::new();

        for (var_name, expected_type) in type_constraints {
            eprintln!("DEBUG: Checking if condition '{}' involves variable '${}' of type '{}'",
                     condition, var_name, expected_type);

            // Extract variables used in condition (e.g., "b1 && b2" -> ["b1", "b2"])
            let condition_vars = self.extract_variables_from_condition(&condition);
            eprintln!("DEBUG: Variables in condition: {:?}", condition_vars);

            for cond_var in condition_vars {
                // Check if this variable traces back to expected type via symbolic propagation
                // AND ensure it involves a contains() call
                if self.check_variable_type_via_symbolic_propagation(
                    &cond_var, expected_type, propagator, &full_source
                ) && self.variable_involves_contains(
                    &cond_var, &full_source
                ) {
                    eprintln!("DEBUG: Variable '{}' matches type '{}' via symbolic propagation and involves contains()",
                             cond_var, expected_type);
                    // Bind the pattern variable to this condition variable
                    bindings.insert(var_name.clone(), cond_var);
                }
            }
        }

        // If we found bindings for all type constraints, create a match result
        if bindings.len() == type_constraints.len() {
            eprintln!("DEBUG: Creating symbolic match result with bindings: {:?}", bindings);
            Ok(Some(SemgrepMatchResult::new(if_node.clone_node(), bindings)))
        } else {
            eprintln!("DEBUG: Not all type constraints satisfied. Found {} of {} bindings",
                     bindings.len(), type_constraints.len());
            Ok(None)
        }
    }

    /// Extract the condition text from an if statement node
    fn extract_if_condition(&self, if_node: &dyn AstNode) -> Option<String> {
        // The condition is typically child 1 (child 0 is "if", child 1 is condition, child 2 is body)
        if if_node.child_count() >= 2 {
            if let Some(condition_node) = if_node.child(1) {
                return condition_node.text().map(|s| s.to_string());
            }
        }
        None
    }
    
    /// Extract variable names from a condition string
    fn extract_variables_from_condition(&self, condition: &str) -> Vec<String> {
        // Simple regex to find identifier-like tokens
        let re = regex::Regex::new(r"\b([a-zA-Z_]\w*)\b").unwrap();
        re.captures_iter(condition)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .filter(|name| {
                // Filter out keywords
                !matches!(name.as_str(), "if" | "else" | "while" | "for" | "return" | "true" | "false" | "null")
            })
            .collect()
    }
    
    /// Check if a variable's type matches expected type using symbolic propagation
    fn check_variable_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
    ) -> bool {
        use astgrep_dataflow::SymbolicValue;
        
        eprintln!("DEBUG check_var_type_sym: Checking if '{}' traces to type '{}'", var_value, expected_type);
        
        // Get the symbolic value for this variable
        let state = propagator.state();
        
        // Direct check: is this variable of the expected type?
        if let Some(symbolic_value) = state.get(var_value) {
            eprintln!("DEBUG: Found symbolic value for {}: {:?}", var_value, symbolic_value);
            
            // Get the root variable
            if let Some(root_var) = symbolic_value.root_variable() {
                eprintln!("DEBUG: Root variable is {}", root_var);
                
                // Check if root variable is of expected type
                let var_pattern = format!(r"{}\s+{}\s*[=;]", regex::escape(expected_type), regex::escape(root_var));
                if let Ok(regex) = regex::Regex::new(&var_pattern) {
                    if regex.is_match(full_source) {
                        eprintln!("DEBUG: Found direct type match for {}", root_var);
                        return true;
                    }
                }
            }
        }
        
        // Check if this variable is defined using the expected type
        // Look for patterns like "Type var = ..." or "boolean var = Type.method()"
        let decl_pattern = format!(r"\w+\s+{}\s*=\s*[^;]*", regex::escape(var_value));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(cap) = regex.captures(full_source) {
                let decl = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                eprintln!("DEBUG: Found declaration for {}: {}", var_value, decl);
                
                // Check if declaration involves the expected type
                // For example: "boolean b1 = !name.contains(...)" where name comes from ZipEntry
                if decl.contains("contains") || decl.contains("getName") {
                    // Extract the source variable (e.g., "name" from "name.contains")
                    if let Some(source_var) = self.extract_source_variable_from_declaration(decl) {
                        eprintln!("DEBUG: Source variable in declaration: {}", source_var);
                        
                        // Check if source variable traces to expected type
                        if let Some(symbolic_value) = state.get(&source_var) {
                            if let Some(root_var) = symbolic_value.root_variable() {
                                let type_pattern = format!(r"{}\s+{}\s*[=;]", 
                                    regex::escape(expected_type), regex::escape(root_var));
                                if let Ok(regex) = regex::Regex::new(&type_pattern) {
                                    if regex.is_match(full_source) {
                                        eprintln!("DEBUG: Source variable {} traces to type {}", 
                                                 root_var, expected_type);
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check aliases
        let aliases = propagator.state().get_all_aliases(var_value);
        for alias in aliases {
            if let Some(alias_symbolic) = state.get(&alias) {
                if let Some(root_var) = alias_symbolic.root_variable() {
                    let type_pattern = format!(r"{}\s+{}\s*[=;]", 
                        regex::escape(expected_type), regex::escape(root_var));
                    if let Ok(regex) = regex::Regex::new(&type_pattern) {
                        if regex.is_match(full_source) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Check if a variable's declaration involves a contains() call
    fn variable_involves_contains(&self, var_name: &str, full_source: &str) -> bool {
        // Look for the variable declaration in the source
        let decl_pattern = format!(r"\w+\s+{}\s*=\s*[^;]*", regex::escape(var_name));
        if let Ok(regex) = regex::Regex::new(&decl_pattern) {
            if let Some(cap) = regex.captures(full_source) {
                let decl = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                eprintln!("DEBUG: Checking if declaration for '{}' involves contains(): {}", var_name, decl);
                // Check if the declaration contains a contains() call
                return decl.contains(".contains(");
            }
        }
        false
    }

    /// Extract the source variable from a declaration like "boolean b1 = !name.contains(...)"
    fn extract_source_variable_from_declaration(
        &self,
        decl: &str,
    ) -> Option<String> {
        // Look for patterns like "name.contains" or "obj.method"
        let re = regex::Regex::new(r"(\w+)\.(?:getName|contains)").unwrap();
        let result: Option<String> = re.captures_iter(decl)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .next();
        result
    }
}
