//! Default implementation of TaintAnalyzer trait

use crate::executor::traits::TaintAnalyzer;
use crate::executor::types::TaintMatch;
use crate::types::{DataFlowSpec, PropagatorPattern, Rule};
use astgrep_core::{AstNode, Finding, Result};
use astgrep_dataflow::DataFlowAnalysis;
use astgrep_matcher::AdvancedSemgrepMatcher;
use std::path::Path;

pub struct DefaultTaintAnalyzer {
    pattern_matcher: AdvancedSemgrepMatcher,
}

impl DefaultTaintAnalyzer {
    pub fn new() -> Self {
        Self {
            pattern_matcher: AdvancedSemgrepMatcher::new(),
        }
    }
}

impl Default for DefaultTaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TaintAnalyzer for DefaultTaintAnalyzer {
    fn execute_taint_analysis(
        &mut self,
        _rule: &Rule,
        _dataflow_spec: &DataFlowSpec,
        _ast: &dyn AstNode,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
        _file_path: Option<&Path>,
    ) -> Result<Vec<Finding>> {
        Ok(Vec::new())
    }

    fn find_taint_sources(
        &mut self,
        _ast: &dyn AstNode,
        _dataflow_spec: &DataFlowSpec,
        _source_text: &str,
    ) -> Result<Vec<TaintMatch>> {
        Ok(Vec::new())
    }

    fn find_taint_sinks(
        &mut self,
        _ast: &dyn AstNode,
        _dataflow_spec: &DataFlowSpec,
        _source_text: &str,
    ) -> Result<Vec<TaintMatch>> {
        Ok(Vec::new())
    }

    fn detect_taint_flows(
        &mut self,
        _sources: &[TaintMatch],
        _sinks: &[TaintMatch],
        _ast: &dyn AstNode,
        _dataflow_analysis: Option<&DataFlowAnalysis>,
        _assume_safe_booleans: bool,
        _assume_safe_numbers: bool,
        _only_propagate_through_assignments: bool,
        _source_text: &str,
        _propagators: &[PropagatorPattern],
    ) -> Result<Vec<(TaintMatch, TaintMatch)>> {
        Ok(Vec::new())
    }

    fn pattern_matcher(&self) -> &AdvancedSemgrepMatcher {
        &self.pattern_matcher
    }

    fn pattern_matcher_mut(&mut self) -> &mut AdvancedSemgrepMatcher {
        &mut self.pattern_matcher
    }
}
