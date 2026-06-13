//! Data flow and taint analysis for astgrep
//!
//! This crate provides data flow analysis and taint tracking functionality for
//! detecting security vulnerabilities and code quality issues.

#![allow(ambiguous_glob_reexports)]

pub mod advanced_taint;
pub mod call_graph;
pub mod cfg;
pub mod constant_analysis;
pub mod constant_propagation;
pub mod enhanced_taint;
pub mod flows;
pub mod graph;
pub mod interprocedural;
pub mod sanitizers;
pub mod sinks;
pub mod sources;
pub mod symbol_table;
pub mod symbolic_propagation;
pub mod taint;

pub use advanced_taint::*;
pub use call_graph::*;
pub use cfg::*;
pub use constant_analysis::*;
pub use constant_propagation::*;
pub use enhanced_taint::*;
pub use flows::*;
pub use graph::*;
pub use interprocedural::*;
pub use sanitizers::*;
pub use sinks::*;
pub use sources::*;
pub use symbol_table::*;
pub use symbolic_propagation::*;
pub use taint::*;

use astgrep_core::{AstNode, Result};
use std::collections::HashMap;

/// Main data flow analyzer
pub struct DataFlowAnalyzer {
    graph: DataFlowGraph,
    source_detector: SourceDetector,
    sink_detector: SinkDetector,
    sanitizer_detector: SanitizerDetector,
    taint_tracker: TaintTracker,
    constant_propagator: ConstantPropagator,
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
            constant_propagator: ConstantPropagator::new(),
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
        let taint_flows =
            self.taint_tracker
                .track_taint(&self.graph, &sources, &sinks, &sanitizers)?;

        // Perform constant propagation analysis
        let constant_values = self.constant_propagator.analyze_ast(ast)?;

        Ok(DataFlowAnalysis {
            graph: self.graph.clone(),
            sources,
            sinks,
            sanitizers,
            taint_flows,
            constant_values,
        })
    }

    /// Build the data flow graph from AST using the CFG builder.
    /// CFG edges model control flow (if/else, loops, try/catch, switch).
    /// Data-flow edges (assignment, call, return) are added by the builder.
    fn build_graph(&mut self, ast: &dyn AstNode) -> Result<()> {
        self.graph = crate::cfg::build_control_flow_graph(ast)?;
        Ok(())
    }

    // Preserved for reference; data-flow edge logic may be integrated into
    // the CFG builder in a follow-up.
    #[allow(dead_code)]
    fn visit_node(&mut self, node: &dyn AstNode, parent_id: Option<NodeId>) -> Result<NodeId> {
        let node_id = self.graph.add_node(DataFlowNode::from_ast_node(node));
        if let Some(parent) = parent_id {
            self.graph.add_edge(parent, node_id, EdgeType::ControlFlow);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_id = self.visit_node(child, Some(node_id))?;
                self.add_data_flow_edges(node, node_id, child, child_id)?;
            }
        }
        Ok(node_id)
    }

    #[allow(dead_code)]
    fn add_data_flow_edges(
        &mut self,
        parent: &dyn AstNode,
        parent_id: NodeId,
        child: &dyn AstNode,
        child_id: NodeId,
    ) -> Result<()> {
        match parent.node_type() {
            "assignment_expression" => {
                if child.node_type() == "identifier" {
                    self.graph.add_edge(parent_id, child_id, EdgeType::DataFlow);
                }
            }
            "call_expression" => {
                if child.node_type() != "identifier"
                    || parent.child(0).map(|c| c.node_type()) != Some("identifier")
                {
                    self.graph.add_edge(child_id, parent_id, EdgeType::DataFlow);
                }
            }
            "return_statement" => {
                self.graph.add_edge(child_id, parent_id, EdgeType::DataFlow);
            }
            _ => {}
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
    pub constant_values: HashMap<String, ConstantValue>,
}

impl DataFlowAnalysis {
    /// Check if there are any vulnerable flows
    pub fn has_vulnerable_flows(&self) -> bool {
        self.taint_flows.iter().any(|flow| flow.is_vulnerable())
    }

    /// Get all vulnerable flows
    pub fn vulnerable_flows(&self) -> Vec<&TaintFlow> {
        self.taint_flows
            .iter()
            .filter(|flow| flow.is_vulnerable())
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{DataFlowGraph, DataFlowNode, EdgeType};
    use crate::sinks::{Sink, SinkType};
    use crate::sources::{Source, SourceType};
    use crate::taint::TaintFlow;

    #[test]
    fn test_dataflow_analyzer_new() {
        let analyzer = DataFlowAnalyzer::new();
        assert_eq!(analyzer.graph().node_count(), 0);
        assert_eq!(analyzer.graph().edge_count(), 0);
    }

    #[test]
    fn test_dataflow_analyzer_default() {
        let analyzer: DataFlowAnalyzer = Default::default();
        assert_eq!(analyzer.graph().node_count(), 0);
        assert_eq!(analyzer.graph().edge_count(), 0);
    }

    #[test]
    fn test_dataflow_analyzer_reset() {
        let mut analyzer = DataFlowAnalyzer::new();
        analyzer.reset();
        assert_eq!(analyzer.graph().node_count(), 0);
        assert_eq!(analyzer.graph().edge_count(), 0);
    }

    #[test]
    fn test_dataflow_analysis_statistics() {
        let mut graph = DataFlowGraph::new();
        let id1 = graph.add_node(DataFlowNode::new("identifier".to_string()));
        let id2 = graph.add_node(DataFlowNode::new("literal".to_string()));
        graph.add_edge(id1, id2, EdgeType::DataFlow);

        let source = Source::new(id1, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(
            id2,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "Test sink".to_string(),
        );
        let flow = TaintFlow::new(source.clone(), sink, vec![id1, id2], 0.9, "SQL_INJECTION".to_string());

        let analysis = DataFlowAnalysis {
            graph: graph.clone(),
            sources: vec![source],
            sinks: vec![Sink::new(
                id2,
                SinkType::SqlExecution,
                "SQL_INJECTION".to_string(),
                "Test sink".to_string(),
            )],
            sanitizers: vec![],
            taint_flows: vec![flow],
            constant_values: std::collections::HashMap::new(),
        };

        let stats = analysis.statistics();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.source_count, 1);
        assert_eq!(stats.sink_count, 1);
        assert_eq!(stats.sanitizer_count, 0);
        assert_eq!(stats.flow_count, 1);
        assert_eq!(stats.vulnerable_flow_count, 1);
    }

    #[test]
    fn test_dataflow_analysis_has_vulnerable_flows() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(
            1,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "Test sink".to_string(),
        );
        let vulnerable_flow = TaintFlow::new(
            source.clone(),
            sink.clone(),
            vec![0, 1],
            0.9,
            "SQL_INJECTION".to_string(),
        );

        let analysis_with_vuln = DataFlowAnalysis {
            graph: DataFlowGraph::new(),
            sources: vec![source.clone()],
            sinks: vec![sink.clone()],
            sanitizers: vec![],
            taint_flows: vec![vulnerable_flow],
            constant_values: std::collections::HashMap::new(),
        };

        assert!(analysis_with_vuln.has_vulnerable_flows());

        let sanitized_sink = Sink::new(
            1,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "Test sink".to_string(),
        );
        let sanitized_flow = TaintFlow::new(
            source.clone(),
            sanitized_sink,
            vec![0, 1],
            0.01,
            "SQL_INJECTION".to_string(),
        );

        let analysis_no_vuln = DataFlowAnalysis {
            graph: DataFlowGraph::new(),
            sources: vec![source],
            sinks: vec![sink],
            sanitizers: vec![],
            taint_flows: vec![sanitized_flow],
            constant_values: std::collections::HashMap::new(),
        };

        assert!(!analysis_no_vuln.has_vulnerable_flows());
    }

    #[test]
    fn test_dataflow_analysis_vulnerable_flows() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(
            1,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "Test sink".to_string(),
        );
        let vulnerable_flow = TaintFlow::new(
            source.clone(),
            sink.clone(),
            vec![0, 1],
            0.9,
            "SQL_INJECTION".to_string(),
        );
        let sanitized_flow = TaintFlow::new(
            source.clone(),
            sink.clone(),
            vec![0, 1],
            0.01,
            "SQL_INJECTION".to_string(),
        );

        let analysis = DataFlowAnalysis {
            graph: DataFlowGraph::new(),
            sources: vec![source],
            sinks: vec![sink],
            sanitizers: vec![],
            taint_flows: vec![vulnerable_flow.clone(), sanitized_flow],
            constant_values: std::collections::HashMap::new(),
        };

        let vulnerable = analysis.vulnerable_flows();
        assert_eq!(vulnerable.len(), 1);
        assert_eq!(vulnerable[0].confidence, vulnerable_flow.confidence);
    }

    #[test]
    fn test_dataflow_statistics_fields() {
        let stats = DataFlowStatistics {
            node_count: 10,
            edge_count: 15,
            source_count: 2,
            sink_count: 3,
            sanitizer_count: 1,
            flow_count: 5,
            vulnerable_flow_count: 2,
        };

        assert_eq!(stats.node_count, 10);
        assert_eq!(stats.edge_count, 15);
        assert_eq!(stats.source_count, 2);
        assert_eq!(stats.sink_count, 3);
        assert_eq!(stats.sanitizer_count, 1);
        assert_eq!(stats.flow_count, 5);
        assert_eq!(stats.vulnerable_flow_count, 2);
    }
}
