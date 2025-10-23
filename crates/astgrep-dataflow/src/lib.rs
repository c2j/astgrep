//! Data flow and taint analysis for astgrep
//!
//! This crate provides data flow analysis and taint tracking functionality for
//! detecting security vulnerabilities and code quality issues.

pub mod graph;
pub mod sources;
pub mod sinks;
pub mod sanitizers;
pub mod taint;
pub mod enhanced_taint;
pub mod flows;
pub mod call_graph;
pub mod interprocedural;
pub mod advanced_taint;
pub mod symbol_table;
pub mod constant_propagation;
pub mod constant_analysis;

pub use graph::*;
pub use sources::*;
pub use sinks::*;
pub use sanitizers::*;
pub use taint::*;
pub use enhanced_taint::*;
pub use flows::*;
pub use call_graph::*;
pub use interprocedural::*;
pub use advanced_taint::*;
pub use symbol_table::*;
pub use constant_propagation::*;
pub use constant_analysis::*;

use astgrep_core::{AstNode, Result};
use std::collections::{HashMap, HashSet};

/// Main data flow analyzer
pub struct DataFlowAnalyzer {
    graph: DataFlowGraph,
    source_detector: SourceDetector,
    sink_detector: SinkDetector,
    sanitizer_detector: SanitizerDetector,
    taint_tracker: TaintTracker,
}

impl DataFlowAnalyzer {
    /// Create a new data flow analyzer
    pub fn new() -> Self {
        Self {
            graph: DataFlowGraph::new(),
            source_detector: SourceDetector::new(),
            sink_detector: SinkDetector::new(),
            sanitizer_detector: SanitizerDetector::new(),
            taint_tracker: TaintTracker::new(),
        }
    }

    /// Analyze data flow in an AST
    pub fn analyze(&mut self, ast: &dyn AstNode) -> Result<DataFlowAnalysis> {
        // Build the data flow graph
        self.build_graph(ast)?;

        // Detect sources, sinks, and sanitizers
        let sources = self.source_detector.detect_sources(&self.graph)?;
        let sinks = self.sink_detector.detect_sinks(&self.graph)?;
        let sanitizers = self.sanitizer_detector.detect_sanitizers(&self.graph)?;

        // Perform taint analysis
        let taint_flows = self.taint_tracker.track_taint(&self.graph, &sources, &sinks, &sanitizers)?;

        Ok(DataFlowAnalysis {
            graph: self.graph.clone(),
            sources,
            sinks,
            sanitizers,
            taint_flows,
        })
    }

    /// Build the data flow graph from AST
    fn build_graph(&mut self, ast: &dyn AstNode) -> Result<()> {
        self.graph.clear();
        self.visit_node(ast, None)?;
        Ok(())
    }

    /// Visit a node and add it to the graph
    fn visit_node(&mut self, node: &dyn AstNode, parent_id: Option<NodeId>) -> Result<NodeId> {
        let node_id = self.graph.add_node(DataFlowNode::from_ast_node(node));

        // Connect to parent if exists
        if let Some(parent) = parent_id {
            self.graph.add_edge(parent, node_id, EdgeType::ControlFlow);
        }

        // Visit children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_id = self.visit_node(child, Some(node_id))?;

                // Add data flow edges based on node type
                self.add_data_flow_edges(node, node_id, child, child_id)?;
            }
        }

        Ok(node_id)
    }

    /// Add data flow edges based on node semantics
    fn add_data_flow_edges(
        &mut self,
        parent: &dyn AstNode,
        parent_id: NodeId,
        child: &dyn AstNode,
        child_id: NodeId,
    ) -> Result<()> {
        match parent.node_type() {
            "assignment_expression" => {
                // For assignments, data flows from right to left
                if child.node_type() == "identifier" {
                    // This is likely the target
                    self.graph.add_edge(parent_id, child_id, EdgeType::DataFlow);
                }
            }
            "call_expression" => {
                // For function calls, data flows from arguments to the call
                if child.node_type() != "identifier" || parent.child(0).map(|c| c.node_type()) != Some("identifier") {
                    self.graph.add_edge(child_id, parent_id, EdgeType::DataFlow);
                }
            }
            "return_statement" => {
                // Data flows from expression to return
                self.graph.add_edge(child_id, parent_id, EdgeType::DataFlow);
            }
            _ => {
                // Default: no special data flow
            }
        }
        Ok(())
    }

    /// Get the current graph
    pub fn graph(&self) -> &DataFlowGraph {
        &self.graph
    }

    /// Reset the analyzer
    pub fn reset(&mut self) {
        self.graph.clear();
        self.taint_tracker.reset();
    }
}

impl Default for DataFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of data flow analysis
#[derive(Debug, Clone)]
pub struct DataFlowAnalysis {
    pub graph: DataFlowGraph,
    pub sources: Vec<Source>,
    pub sinks: Vec<Sink>,
    pub sanitizers: Vec<Sanitizer>,
    pub taint_flows: Vec<TaintFlow>,
}

impl DataFlowAnalysis {
    /// Check if there are any vulnerable flows
    pub fn has_vulnerable_flows(&self) -> bool {
        self.taint_flows.iter().any(|flow| flow.is_vulnerable())
    }

    /// Get all vulnerable flows
    pub fn vulnerable_flows(&self) -> Vec<&TaintFlow> {
        self.taint_flows.iter().filter(|flow| flow.is_vulnerable()).collect()
    }

    /// Get flows by vulnerability type
    pub fn flows_by_type(&self, vuln_type: &str) -> Vec<&TaintFlow> {
        self.taint_flows
            .iter()
            .filter(|flow| flow.vulnerability_type() == Some(vuln_type))
            .collect()
    }

    /// Get statistics about the analysis
    pub fn statistics(&self) -> DataFlowStatistics {
        DataFlowStatistics {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            source_count: self.sources.len(),
            sink_count: self.sinks.len(),
            sanitizer_count: self.sanitizers.len(),
            flow_count: self.taint_flows.len(),
            vulnerable_flow_count: self.vulnerable_flows().len(),
        }
    }
}

/// Statistics about data flow analysis
#[derive(Debug, Clone)]
pub struct DataFlowStatistics {
    pub node_count: usize,
    pub edge_count: usize,
    pub source_count: usize,
    pub sink_count: usize,
    pub sanitizer_count: usize,
    pub flow_count: usize,
    pub vulnerable_flow_count: usize,
}
