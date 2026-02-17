//! TaintAnalyzer trait definition
//!
//! Defines the interface for taint analysis functionality.

use crate::executor::types::TaintMatch;
use crate::types::{DataFlowSpec, PropagatorPattern, Rule};
use astgrep_core::{AstNode, Finding, Result};
use astgrep_dataflow::DataFlowAnalysis;
use astgrep_matcher::AdvancedSemgrepMatcher;
use std::path::Path;

/// Trait for taint analysis functionality
///
/// This trait defines the interface for analyzing taint flows from
/// sources to sinks through a program's data flow.
pub trait TaintAnalyzer: Send + Sync {
    /// Execute taint analysis for a rule
    fn execute_taint_analysis(
        &mut self,
        rule: &Rule,
        dataflow_spec: &DataFlowSpec,
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        file_path: Option<&Path>,
    ) -> Result<Vec<Finding>>;

    /// Find taint sources in the AST
    fn find_taint_sources(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str,
    ) -> Result<Vec<TaintMatch>>;

    /// Find taint sinks in the AST
    fn find_taint_sinks(
        &mut self,
        ast: &dyn AstNode,
        dataflow_spec: &DataFlowSpec,
        source_text: &str,
    ) -> Result<Vec<TaintMatch>>;

    /// Detect taint flows between sources and sinks
    fn detect_taint_flows(
        &mut self,
        sources: &[TaintMatch],
        sinks: &[TaintMatch],
        ast: &dyn AstNode,
        dataflow_analysis: Option<&DataFlowAnalysis>,
        assume_safe_booleans: bool,
        assume_safe_numbers: bool,
        only_propagate_through_assignments: bool,
        source_text: &str,
        propagators: &[PropagatorPattern],
    ) -> Result<Vec<(TaintMatch, TaintMatch)>>;

    /// Get reference to pattern matcher
    fn pattern_matcher(&self) -> &AdvancedSemgrepMatcher;

    /// Get mutable reference to pattern matcher
    fn pattern_matcher_mut(&mut self) -> &mut AdvancedSemgrepMatcher;
}
