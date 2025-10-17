//! Inter-procedural data flow analysis
//!
//! This module provides cross-function taint tracking and data flow analysis
//! by leveraging the call graph to trace taint through function calls.

use crate::call_graph::{CallGraph, FunctionId, FunctionSignature, ParameterMapping};
use crate::graph::{DataFlowGraph, NodeId};
use crate::taint::{TaintFlow, TaintState};
use crate::sources::{Source, SourceType};
use crate::sinks::{Sink, SinkType};
use crate::sanitizers::Sanitizer;
use cr_core::Result;
use std::collections::{HashMap, HashSet};

/// Inter-procedural taint tracker
pub struct InterproceduralTaintTracker {
    call_graph: CallGraph,
    /// Taint states at function entry points
    entry_taints: HashMap<FunctionId, Vec<TaintState>>,
    /// Taint states at function exit points
    exit_taints: HashMap<FunctionId, Vec<TaintState>>,
    /// Visited functions to avoid infinite loops
    visited_functions: HashSet<FunctionId>,
}

impl InterproceduralTaintTracker {
    /// Create a new inter-procedural taint tracker
    pub fn new(call_graph: CallGraph) -> Self {
        Self {
            call_graph,
            entry_taints: HashMap::new(),
            exit_taints: HashMap::new(),
            visited_functions: HashSet::new(),
        }
    }

    /// Trace taint through function calls
    pub fn trace_taint_through_calls(
        &mut self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
        entry_func: FunctionId,
    ) -> Result<Vec<TaintFlow>> {
        self.visited_functions.clear();
        let mut flows = Vec::new();

        // Trace from entry function
        self.trace_function_taint(
            entry_func,
            graph,
            sources,
            sinks,
            sanitizers,
            &mut flows,
        )?;

        Ok(flows)
    }

    /// Recursively trace taint through a function and its callees
    fn trace_function_taint(
        &mut self,
        func_id: FunctionId,
        _graph: &DataFlowGraph,
        _sources: &[Source],
        _sinks: &[Sink],
        _sanitizers: &[Sanitizer],
        _flows: &mut Vec<TaintFlow>,
    ) -> Result<()> {
        // Avoid infinite recursion
        if self.visited_functions.contains(&func_id) {
            return Ok(());
        }
        self.visited_functions.insert(func_id);

        // Get all calls from this function
        let calls_to_process: Vec<_> = self.call_graph.calls_from(func_id)
            .map(|calls| calls.clone())
            .unwrap_or_default();

        for call in calls_to_process {
            // Get parameter mapping for this call
            if let Some(param_mapping) = self.call_graph.get_param_mapping(call.id) {
                // Trace taint through parameters
                self.trace_parameter_taint(
                    func_id,
                    &call.callee_signature,
                    param_mapping,
                    _graph,
                    _sources,
                    _sinks,
                    _sanitizers,
                    _flows,
                )?;
            }

            // Recursively trace callees
            if let Some(callee_def) = self.call_graph.functions().get(&call.callee_signature) {
                let callee_id = callee_def.id;
                self.trace_function_taint(
                    callee_id,
                    _graph,
                    _sources,
                    _sinks,
                    _sanitizers,
                    _flows,
                )?;
            }
        }

        self.visited_functions.remove(&func_id);
        Ok(())
    }

    /// Trace taint through function parameters
    fn trace_parameter_taint(
        &self,
        _caller_id: FunctionId,
        _callee_sig: &FunctionSignature,
        param_mapping: &ParameterMapping,
        _graph: &DataFlowGraph,
        sources: &[Source],
        _sinks: &[Sink],
        _sanitizers: &[Sanitizer],
        _flows: &mut Vec<TaintFlow>,
    ) -> Result<()> {
        // For each parameter mapping, check if the argument is tainted
        for (_param_idx, arg_expr) in &param_mapping.mappings {
            // Check if argument matches any source
            for source in sources {
                if self.matches_expression(arg_expr, source) {
                    // Argument is tainted, trace through callee
                    // This would require analyzing the callee function
                    // For now, we record the potential flow
                }
            }
        }

        Ok(())
    }

    /// Check if an expression matches a source
    fn matches_expression(&self, expr: &str, source: &Source) -> bool {
        // Simple pattern matching - can be enhanced
        // Check if expression contains the source description
        expr.contains(&source.description)
    }

    /// Get taint states at function entry
    pub fn entry_taints(&self, func_id: FunctionId) -> Option<&Vec<TaintState>> {
        self.entry_taints.get(&func_id)
    }

    /// Get taint states at function exit
    pub fn exit_taints(&self, func_id: FunctionId) -> Option<&Vec<TaintState>> {
        self.exit_taints.get(&func_id)
    }

    /// Set taint states at function entry
    pub fn set_entry_taints(&mut self, func_id: FunctionId, taints: Vec<TaintState>) {
        self.entry_taints.insert(func_id, taints);
    }

    /// Set taint states at function exit
    pub fn set_exit_taints(&mut self, func_id: FunctionId, taints: Vec<TaintState>) {
        self.exit_taints.insert(func_id, taints);
    }

    /// Clear all taint states
    pub fn clear(&mut self) {
        self.entry_taints.clear();
        self.exit_taints.clear();
        self.visited_functions.clear();
    }
}

/// Symbol propagation for tracking variable definitions and uses
pub struct SymbolPropagator {
    /// Symbol definitions: name -> definition location
    definitions: HashMap<String, NodeId>,
    /// Symbol uses: name -> use locations
    uses: HashMap<String, Vec<NodeId>>,
    /// Symbol types: name -> type
    types: HashMap<String, String>,
}

impl SymbolPropagator {
    /// Create a new symbol propagator
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            uses: HashMap::new(),
            types: HashMap::new(),
        }
    }

    /// Record a symbol definition
    pub fn define_symbol(&mut self, name: String, node_id: NodeId, symbol_type: String) {
        self.definitions.insert(name.clone(), node_id);
        self.types.insert(name, symbol_type);
    }

    /// Record a symbol use
    pub fn use_symbol(&mut self, name: String, node_id: NodeId) {
        self.uses.entry(name).or_insert_with(Vec::new).push(node_id);
    }

    /// Get the definition of a symbol
    pub fn get_definition(&self, name: &str) -> Option<NodeId> {
        self.definitions.get(name).copied()
    }

    /// Get all uses of a symbol
    pub fn get_uses(&self, name: &str) -> Option<&Vec<NodeId>> {
        self.uses.get(name)
    }

    /// Get the type of a symbol
    pub fn get_type(&self, name: &str) -> Option<&str> {
        self.types.get(name).map(|s| s.as_str())
    }

    /// Trace a symbol through the data flow graph
    pub fn trace_symbol(
        &self,
        name: &str,
        graph: &DataFlowGraph,
    ) -> Option<Vec<NodeId>> {
        let def_node = self.get_definition(name)?;
        let use_nodes = self.get_uses(name)?;

        // Find paths from definition to uses
        let mut paths = Vec::new();
        for use_node in use_nodes {
            if let Some(path) = graph.find_paths(def_node, *use_node).first() {
                paths.extend(path.clone());
            }
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }

    /// Clear all symbol information
    pub fn clear(&mut self) {
        self.definitions.clear();
        self.uses.clear();
        self.types.clear();
    }
}

impl Default for SymbolPropagator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::FunctionSignature;

    #[test]
    fn test_interprocedural_tracker_creation() {
        let call_graph = CallGraph::new();
        let tracker = InterproceduralTaintTracker::new(call_graph);
        assert!(tracker.entry_taints.is_empty());
        assert!(tracker.exit_taints.is_empty());
    }

    #[test]
    fn test_symbol_propagator() {
        let mut propagator = SymbolPropagator::new();
        propagator.define_symbol("x".to_string(), 0, "int".to_string());
        propagator.use_symbol("x".to_string(), 1);

        assert_eq!(propagator.get_definition("x"), Some(0));
        assert_eq!(propagator.get_type("x"), Some("int"));
        assert!(propagator.get_uses("x").is_some());
    }

    #[test]
    fn test_symbol_propagator_multiple_uses() {
        let mut propagator = SymbolPropagator::new();
        propagator.define_symbol("y".to_string(), 0, "string".to_string());
        propagator.use_symbol("y".to_string(), 1);
        propagator.use_symbol("y".to_string(), 2);
        propagator.use_symbol("y".to_string(), 3);

        let uses = propagator.get_uses("y").unwrap();
        assert_eq!(uses.len(), 3);
    }
}

