//! Default implementation of ConditionEvaluator trait

use crate::executor::core_helpers;
use crate::executor::traits::ConditionEvaluator;
use crate::types::{Condition, Pattern};
use astgrep_core::{ComparisonOperator, MetavariableAnalysis, Result, SemgrepMatchResult};
use std::collections::HashMap;

pub struct DefaultConditionEvaluator;

impl DefaultConditionEvaluator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator for DefaultConditionEvaluator {
    fn evaluate_condition(
        &self,
        _condition: &Condition,
        _match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&astgrep_dataflow::DataFlowAnalysis>,
        _full_source: &str,
        _constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        _import_map: &HashMap<String, String>,
    ) -> Result<bool> {
        Ok(true)
    }

    fn check_pattern_conditions(
        &self,
        _pattern: &Pattern,
        _match_result: &SemgrepMatchResult,
        _dataflow_analysis: Option<&astgrep_dataflow::DataFlowAnalysis>,
        _full_source: &str,
        _constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        _import_map: &HashMap<String, String>,
    ) -> Result<bool> {
        Ok(true)
    }

    fn evaluate_comparison(
        &self,
        _metavar_value: &str,
        _operator: &ComparisonOperator,
        _expected_value: &str,
    ) -> Result<bool> {
        Ok(true)
    }

    fn evaluate_analysis_constraint(
        &self,
        _value: &str,
        _analysis: &MetavariableAnalysis,
    ) -> Result<bool> {
        Ok(true)
    }

    fn evaluate_name_constraint(&self, _value: &str, _name_pattern: &str) -> Result<bool> {
        Ok(true)
    }

    fn extract_type_info(
        &self,
        _match_result: &SemgrepMatchResult,
        _var_name: &str,
        _full_source: &str,
        _import_map: &HashMap<String, String>,
    ) -> Option<String> {
        None
    }

    fn infer_type_from_value(&self, value: &str) -> Option<String> {
        core_helpers::infer_type_from_value(value)
    }

    fn check_entropy(
        &self,
        _value: &str,
        _entropy_config: &astgrep_core::EntropyAnalysis,
    ) -> Result<bool> {
        Ok(true)
    }

    fn check_type_analysis(
        &self,
        _value: &str,
        _type_config: &astgrep_core::TypeAnalysis,
    ) -> Result<bool> {
        Ok(true)
    }

    fn check_complexity(
        &self,
        _value: &str,
        _complexity_config: &astgrep_core::ComplexityAnalysis,
    ) -> Result<bool> {
        Ok(true)
    }

    fn calculate_entropy(&self, value: &str) -> f64 {
        core_helpers::calculate_entropy(value)
    }

    fn matches_charset(&self, value: &str, charset: &str) -> bool {
        core_helpers::matches_charset(value, charset)
    }

    fn value_matches_type(&self, value: &str, expected_type: &str) -> bool {
        core_helpers::value_matches_type(value, expected_type)
    }

    fn evaluate_custom_condition(
        &self,
        _custom_condition: &serde_yaml::Value,
        _match_result: &SemgrepMatchResult,
    ) -> Result<bool> {
        Ok(true)
    }

    fn evaluate_python_expression(&self, _metavar_value: &str, _expr: &str) -> Result<bool> {
        Ok(true)
    }

    fn build_import_map(&self, full_source: &str) -> HashMap<String, String> {
        core_helpers::build_import_map(full_source)
    }

    fn resolve_type_with_imports(
        &self,
        simple_type: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String> {
        core_helpers::resolve_type_with_imports(simple_type, import_map)
    }
}
