//! ConditionEvaluator trait definition
//!
//! Defines the interface for condition evaluation functionality.

use crate::types::{Condition, Pattern};
use astgrep_core::{ComparisonOperator, MetavariableAnalysis, Result, SemgrepMatchResult};
use std::collections::HashMap;

/// Trait for condition evaluation functionality
///
/// This trait defines the interface for evaluating pattern conditions
/// and metavariable constraints.
pub trait ConditionEvaluator: Send + Sync {
    /// Evaluate a single condition
    fn evaluate_condition(
        &self,
        condition: &Condition,
        match_result: &SemgrepMatchResult,
        dataflow_analysis: Option<&astgrep_dataflow::DataFlowAnalysis>,
        full_source: &str,
        constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        import_map: &HashMap<String, String>,
    ) -> Result<bool>;

    /// Check pattern conditions
    fn check_pattern_conditions(
        &self,
        pattern: &Pattern,
        match_result: &SemgrepMatchResult,
        dataflow_analysis: Option<&astgrep_dataflow::DataFlowAnalysis>,
        full_source: &str,
        constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        import_map: &HashMap<String, String>,
    ) -> Result<bool>;

    /// Evaluate metavariable comparison
    fn evaluate_comparison(
        &self,
        metavar_value: &str,
        operator: &ComparisonOperator,
        expected_value: &str,
    ) -> Result<bool>;

    /// Evaluate analysis constraint
    fn evaluate_analysis_constraint(
        &self,
        value: &str,
        analysis: &MetavariableAnalysis,
    ) -> Result<bool>;

    /// Evaluate name constraint (module/namespace patterns)
    fn evaluate_name_constraint(&self, value: &str, name_pattern: &str) -> Result<bool>;

    /// Extract type information for a variable from the match context
    fn extract_type_info(
        &self,
        match_result: &SemgrepMatchResult,
        var_name: &str,
        full_source: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String>;

    /// Infer the type of a value from its literal representation
    fn infer_type_from_value(&self, value: &str) -> Option<String>;

    /// Check entropy constraints
    fn check_entropy(
        &self,
        value: &str,
        entropy_config: &astgrep_core::EntropyAnalysis,
    ) -> Result<bool>;

    /// Check type analysis constraints
    fn check_type_analysis(
        &self,
        value: &str,
        type_config: &astgrep_core::TypeAnalysis,
    ) -> Result<bool>;

    /// Check complexity constraints
    fn check_complexity(
        &self,
        value: &str,
        complexity_config: &astgrep_core::ComplexityAnalysis,
    ) -> Result<bool>;

    /// Calculate entropy of a string
    fn calculate_entropy(&self, value: &str) -> f64;

    /// Check if a value matches a charset
    fn matches_charset(&self, value: &str, charset: &str) -> bool;

    /// Check if a value matches an expected type
    fn value_matches_type(&self, value: &str, expected_type: &str) -> bool;

    /// Evaluate custom condition
    fn evaluate_custom_condition(
        &self,
        custom_condition: &serde_yaml::Value,
        match_result: &SemgrepMatchResult,
    ) -> Result<bool>;

    /// Evaluate Python expression (simplified implementation)
    fn evaluate_python_expression(&self, metavar_value: &str, expr: &str) -> Result<bool>;

    /// Build import map from source
    fn build_import_map(&self, full_source: &str) -> HashMap<String, String>;

    /// Resolve type with imports
    fn resolve_type_with_imports(
        &self,
        simple_type: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String>;
}
