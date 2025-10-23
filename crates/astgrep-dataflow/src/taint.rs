//! Taint tracking for data flow analysis
//! 
//! This module implements taint analysis to track the flow of potentially dangerous data.

use crate::graph::{DataFlowGraph, NodeId};
use crate::sources::{Source, SourceType};
use crate::sinks::{Sink, SinkType};
use crate::sanitizers::{Sanitizer, SanitizerType};
use astgrep_core::{Location, Result, constants::defaults::analysis};
use std::collections::{HashMap, HashSet, VecDeque};

/// Taint tracker for analyzing data flow
pub struct TaintTracker {
    taint_states: HashMap<NodeId, TaintState>,
}

impl TaintTracker {
    /// Create a new taint tracker
    pub fn new() -> Self {
        Self {
            taint_states: HashMap::new(),
        }
    }

    /// Track taint flow through the graph
    pub fn track_taint(
        &mut self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
    ) -> Result<Vec<TaintFlow>> {
        // Initialize taint states
        self.initialize_taint_states(graph, sources, sanitizers);
        
        // Propagate taint through the graph
        self.propagate_taint(graph)?;
        
        // Find flows from sources to sinks
        let flows = self.find_taint_flows(graph, sources, sinks, sanitizers)?;
        
        Ok(flows)
    }

    /// Initialize taint states for sources and sanitizers
    fn initialize_taint_states(&mut self, graph: &DataFlowGraph, sources: &[Source], sanitizers: &[Sanitizer]) {
        self.taint_states.clear();
        
        // Initialize all nodes as untainted
        for node_id in graph.node_ids() {
            self.taint_states.insert(node_id, TaintState::new());
        }
        
        // Mark sources as tainted
        for source in sources {
            if let Some(state) = self.taint_states.get_mut(&source.id) {
                state.add_taint(TaintInfo::new(
                    source.id,
                    source.source_type.clone(),
                    source.confidence,
                ));
            }
        }
        
        // Mark sanitizers
        for sanitizer in sanitizers {
            if let Some(state) = self.taint_states.get_mut(&sanitizer.id) {
                state.set_sanitizer(sanitizer.sanitizer_type.clone(), sanitizer.effectiveness);
            }
        }
    }

    /// Propagate taint through the graph using data flow edges
    fn propagate_taint(&mut self, graph: &DataFlowGraph) -> Result<()> {
        let mut changed = true;
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops
        
        while changed && iteration < MAX_ITERATIONS {
            changed = false;
            iteration += 1;
            
            for node_id in graph.node_ids() {
                let predecessors = graph.data_flow_predecessors(node_id);
                
                if !predecessors.is_empty() {
                    let mut new_taints = Vec::new();
                    
                    // Collect taint from all predecessors
                    for pred_id in predecessors {
                        if let Some(pred_state) = self.taint_states.get(&pred_id) {
                            for taint in &pred_state.taints {
                                let mut new_taint = taint.clone();
                                new_taint.path.push(node_id);
                                new_taints.push(new_taint);
                            }
                        }
                    }
                    
                    // Apply sanitization if this node is a sanitizer
                    if let Some(current_state) = self.taint_states.get(&node_id) {
                        if let Some((sanitizer_type, effectiveness)) = &current_state.sanitizer {
                            new_taints = self.apply_sanitization(new_taints, sanitizer_type, *effectiveness);
                        }
                    }
                    
                    // Update taint state
                    if let Some(current_state) = self.taint_states.get_mut(&node_id) {
                        let old_count = current_state.taints.len();
                        for taint in new_taints {
                            current_state.add_taint(taint);
                        }
                        if current_state.taints.len() != old_count {
                            changed = true;
                        }
                    }
                }
            }
        }
        
        if iteration >= MAX_ITERATIONS {
            eprintln!("Warning: Taint propagation reached maximum iterations");
        }
        
        Ok(())
    }

    /// Apply sanitization to taint information
    fn apply_sanitization(
        &self,
        taints: Vec<TaintInfo>,
        sanitizer_type: &SanitizerType,
        effectiveness: f32,
    ) -> Vec<TaintInfo> {
        let protected_types = sanitizer_type.default_protections();
        
        taints
            .into_iter()
            .filter_map(|mut taint| {
                // Check if this sanitizer protects against the source type
                let source_vuln_type = match taint.source_type {
                    SourceType::UserInput => "XSS",
                    SourceType::DatabaseInput => "SQL_INJECTION",
                    SourceType::FileInput => "PATH_TRAVERSAL",
                    _ => "UNKNOWN",
                };
                
                if protected_types.contains(&source_vuln_type.to_string()) {
                    // Reduce confidence based on sanitizer effectiveness
                    taint.confidence *= (1.0 - effectiveness);
                    
                    // If confidence is very low, consider it sanitized
                    if taint.confidence < analysis::LOW_CONFIDENCE_THRESHOLD as f32 {
                        None
                    } else {
                        Some(taint)
                    }
                } else {
                    // Sanitizer doesn't protect against this type
                    Some(taint)
                }
            })
            .collect()
    }

    /// Find taint flows from sources to sinks using advanced path analysis
    fn find_taint_flows(
        &self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
    ) -> Result<Vec<TaintFlow>> {
        let mut flows = Vec::new();

        for sink in sinks {
            if let Some(sink_state) = self.taint_states.get(&sink.id) {
                for taint in &sink_state.taints {
                    // Only report flows with sufficient confidence
                    if !taint.is_significant() {
                        continue;
                    }

                    // Find the original source
                    if let Some(source) = sources.iter().find(|s| s.id == taint.source_id) {
                        // Validate the path exists in the graph
                        if !self.validate_taint_path(graph, &taint.path) {
                            continue;
                        }

                        // Find sanitizers along the path
                        let path_sanitizers: Vec<&Sanitizer> = taint.path
                            .iter()
                            .filter_map(|&node_id| {
                                sanitizers.iter().find(|s| s.id == node_id)
                            })
                            .collect();

                        // Calculate effective confidence considering sanitizers
                        let effective_confidence = self.calculate_effective_confidence(
                            taint.confidence,
                            &path_sanitizers,
                            &source.source_type,
                            &sink.vulnerability_type,
                        );

                        // Only report if confidence is still significant after sanitization
                        if effective_confidence > analysis::CONFIDENCE_THRESHOLD as f32 {
                            let mut flow = TaintFlow::new(
                                source.clone(),
                                sink.clone(),
                                taint.path.clone(),
                                effective_confidence,
                                sink.vulnerability_type.clone(),
                            );

                            // Add sanitizers
                            for sanitizer in path_sanitizers {
                                flow.add_sanitizer(sanitizer.clone());
                            }

                            flows.push(flow);
                        }
                    }
                }
            }
        }

        // Sort flows by confidence (highest first)
        flows.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        Ok(flows)
    }

    /// Validate that a taint path exists in the data flow graph
    fn validate_taint_path(&self, graph: &DataFlowGraph, path: &[NodeId]) -> bool {
        if path.len() < 2 {
            return false;
        }

        for window in path.windows(2) {
            let from = window[0];
            let to = window[1];

            // Check if there's a data flow edge from 'from' to 'to'
            if !graph.has_data_flow_edge(from, to) {
                return false;
            }
        }

        true
    }

    /// Calculate effective confidence considering sanitizers
    fn calculate_effective_confidence(
        &self,
        base_confidence: f32,
        sanitizers: &[&Sanitizer],
        source_type: &SourceType,
        vulnerability_type: &str,
    ) -> f32 {
        let mut confidence = base_confidence;

        for sanitizer in sanitizers {
            // Check if sanitizer is effective against this vulnerability type
            let effectiveness = sanitizer.effectiveness_against(source_type, vulnerability_type);

            // Reduce confidence based on sanitizer effectiveness
            confidence *= (1.0 - effectiveness);
        }

        confidence
    }

    /// Reset the taint tracker
    pub fn reset(&mut self) {
        self.taint_states.clear();
    }

    /// Get taint state for a node
    pub fn get_taint_state(&self, node_id: NodeId) -> Option<&TaintState> {
        self.taint_states.get(&node_id)
    }

    /// Perform inter-procedural taint analysis
    pub fn track_interprocedural_taint(
        &mut self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
        call_graph: &HashMap<NodeId, Vec<NodeId>>, // function calls
    ) -> Result<Vec<TaintFlow>> {
        // First perform standard intra-procedural analysis
        let mut flows = self.track_taint(graph, sources, sinks, sanitizers)?;

        // Then extend with inter-procedural analysis
        self.propagate_taint_across_calls(graph, call_graph)?;

        // Find additional flows that cross function boundaries
        let inter_flows = self.find_taint_flows(graph, sources, sinks, sanitizers)?;

        // Merge and deduplicate flows
        flows.extend(inter_flows);
        flows.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        flows.dedup_by(|a, b| a.source.id == b.source.id && a.sink.id == b.sink.id);

        Ok(flows)
    }

    /// Propagate taint across function calls
    fn propagate_taint_across_calls(
        &mut self,
        graph: &DataFlowGraph,
        call_graph: &HashMap<NodeId, Vec<NodeId>>,
    ) -> Result<()> {
        let mut changed = true;
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iteration < MAX_ITERATIONS {
            changed = false;
            iteration += 1;

            for (&caller_id, callees) in call_graph {
                if let Some(caller_state) = self.taint_states.get(&caller_id).cloned() {
                    for &callee_id in callees {
                        // Propagate taint from caller to callee
                        if let Some(callee_state) = self.taint_states.get_mut(&callee_id) {
                            let old_count = callee_state.taints.len();

                            for taint in &caller_state.taints {
                                let mut new_taint = taint.clone();
                                new_taint.path.push(callee_id);
                                // Reduce confidence for inter-procedural flows
                                new_taint.confidence *= 0.9;
                                callee_state.add_taint(new_taint);
                            }

                            if callee_state.taints.len() != old_count {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze context-sensitive taint flows
    pub fn analyze_context_sensitive(
        &mut self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
        contexts: &HashMap<NodeId, String>, // context information
    ) -> Result<Vec<TaintFlow>> {
        // Group sources and sinks by context
        let mut context_groups: HashMap<String, (Vec<Source>, Vec<Sink>)> = HashMap::new();

        for source in sources {
            let context = contexts.get(&source.id).cloned().unwrap_or_default();
            context_groups.entry(context).or_default().0.push(source.clone());
        }

        for sink in sinks {
            let context = contexts.get(&sink.id).cloned().unwrap_or_default();
            context_groups.entry(context).or_default().1.push(sink.clone());
        }

        let mut all_flows = Vec::new();

        // Analyze each context separately
        for (context, (ctx_sources, ctx_sinks)) in context_groups {
            if !ctx_sources.is_empty() && !ctx_sinks.is_empty() {
                // Reset and analyze for this context
                self.reset();
                let flows = self.track_taint(graph, &ctx_sources, &ctx_sinks, sanitizers)?;

                // Add context information to flows
                let mut context_flows: Vec<TaintFlow> = flows.into_iter().map(|mut flow| {
                    flow.context = Some(context.clone());
                    flow
                }).collect();

                all_flows.append(&mut context_flows);
            }
        }

        Ok(all_flows)
    }
}

impl Default for TaintTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Taint state for a node
#[derive(Debug, Clone)]
pub struct TaintState {
    pub taints: Vec<TaintInfo>,
    pub sanitizer: Option<(SanitizerType, f32)>, // (type, effectiveness)
}

impl TaintState {
    /// Create a new taint state
    pub fn new() -> Self {
        Self {
            taints: Vec::new(),
            sanitizer: None,
        }
    }

    /// Add taint information
    pub fn add_taint(&mut self, taint: TaintInfo) {
        // Avoid duplicate taints from the same source
        if !self.taints.iter().any(|t| t.source_id == taint.source_id) {
            self.taints.push(taint);
        }
    }

    /// Set sanitizer information
    pub fn set_sanitizer(&mut self, sanitizer_type: SanitizerType, effectiveness: f32) {
        self.sanitizer = Some((sanitizer_type, effectiveness));
    }

    /// Check if this node is tainted
    pub fn is_tainted(&self) -> bool {
        !self.taints.is_empty()
    }

    /// Check if this node is a sanitizer
    pub fn is_sanitizer(&self) -> bool {
        self.sanitizer.is_some()
    }

    /// Get the highest confidence taint
    pub fn max_confidence(&self) -> f32 {
        self.taints.iter().map(|t| t.confidence).fold(0.0, f32::max)
    }
}

impl Default for TaintState {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about taint
#[derive(Debug, Clone)]
pub struct TaintInfo {
    pub source_id: NodeId,
    pub source_type: SourceType,
    pub confidence: f32,
    pub path: Vec<NodeId>,
    pub transformations: Vec<TaintTransformation>,
    pub context: Option<String>,
    pub timestamp: std::time::SystemTime,
}

impl TaintInfo {
    /// Create new taint information
    pub fn new(source_id: NodeId, source_type: SourceType, confidence: f32) -> Self {
        Self {
            source_id,
            source_type,
            confidence,
            path: vec![source_id],
            transformations: Vec::new(),
            context: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Check if this taint is significant enough to report
    pub fn is_significant(&self) -> bool {
        self.confidence > analysis::CONFIDENCE_THRESHOLD as f32 && !self.is_effectively_sanitized()
    }

    /// Check if this taint has been effectively sanitized
    pub fn is_effectively_sanitized(&self) -> bool {
        self.transformations.iter().any(|t| matches!(t, TaintTransformation::Sanitized { effectiveness, .. } if *effectiveness > analysis::HIGH_EFFECTIVENESS_THRESHOLD as f32))
    }

    /// Add a transformation to this taint
    pub fn add_transformation(&mut self, transformation: TaintTransformation) {
        match &transformation {
            TaintTransformation::Sanitized { effectiveness, .. } => {
                self.confidence *= (1.0 - effectiveness);
            }
            TaintTransformation::Encoded { .. } => {
                self.confidence *= analysis::taint_multipliers::ENCODING_MULTIPLIER;
            }
            TaintTransformation::Validated { .. } => {
                self.confidence *= analysis::taint_multipliers::VALIDATION_MULTIPLIER;
            }
            TaintTransformation::Filtered { .. } => {
                self.confidence *= analysis::taint_multipliers::FILTERING_MULTIPLIER;
            }
        }
        self.transformations.push(transformation);
    }

    /// Get the age of this taint information
    pub fn age(&self) -> std::time::Duration {
        std::time::SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or_default()
    }
}

/// Represents transformations applied to tainted data
#[derive(Debug, Clone)]
pub enum TaintTransformation {
    Sanitized {
        sanitizer_type: SanitizerType,
        effectiveness: f32,
        location: NodeId,
    },
    Encoded {
        encoding_type: String,
        location: NodeId,
    },
    Validated {
        validation_type: String,
        location: NodeId,
    },
    Filtered {
        filter_type: String,
        location: NodeId,
    },
}

/// Represents a taint flow from source to sink
#[derive(Debug, Clone)]
pub struct TaintFlow {
    pub source: Source,
    pub sink: Sink,
    pub path: Vec<NodeId>,
    pub sanitizers: Vec<Sanitizer>,
    pub confidence: f32,
    pub vulnerability_type: String,
    pub context: Option<String>,
    pub transformations: Vec<TaintTransformation>,
    pub flow_type: FlowType,
}

impl TaintFlow {
    /// Create a new taint flow
    pub fn new(
        source: Source,
        sink: Sink,
        path: Vec<NodeId>,
        confidence: f32,
        vulnerability_type: String,
    ) -> Self {
        Self {
            source,
            sink,
            path,
            sanitizers: Vec::new(),
            confidence,
            vulnerability_type,
            context: None,
            transformations: Vec::new(),
            flow_type: FlowType::Direct,
        }
    }

    /// Add a sanitizer to this flow
    pub fn add_sanitizer(&mut self, sanitizer: Sanitizer) {
        self.sanitizers.push(sanitizer);
    }

    /// Set the context for this flow
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the flow type
    pub fn with_flow_type(mut self, flow_type: FlowType) -> Self {
        self.flow_type = flow_type;
        self
    }
}

/// Types of data flows
#[derive(Debug, Clone, PartialEq)]
pub enum FlowType {
    /// Direct flow within the same function
    Direct,
    /// Flow through function calls
    Interprocedural,
    /// Flow through object fields
    FieldSensitive,
    /// Flow through array/collection elements
    IndexSensitive,
    /// Flow through implicit channels (timing, exceptions, etc.)
    Implicit,
}

/// Severity levels for taint flows
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl FlowSeverity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowSeverity::Info => "info",
            FlowSeverity::Low => "low",
            FlowSeverity::Medium => "medium",
            FlowSeverity::High => "high",
            FlowSeverity::Critical => "critical",
        }
    }

    /// Get numeric score for comparison
    pub fn score(&self) -> u8 {
        match self {
            FlowSeverity::Info => 0,
            FlowSeverity::Low => 1,
            FlowSeverity::Medium => 2,
            FlowSeverity::High => 3,
            FlowSeverity::Critical => 4,
        }
    }
}

impl TaintFlow {
    /// Check if this flow represents a vulnerability
    pub fn is_vulnerable(&self) -> bool {
        // A flow is vulnerable if:
        // 1. It has high confidence (> threshold)
        // 2. No effective sanitizers protect against the vulnerability type
        self.confidence > analysis::CONFIDENCE_THRESHOLD as f32 && !self.is_effectively_sanitized()
    }

    /// Check if this flow is effectively sanitized
    pub fn is_effectively_sanitized(&self) -> bool {
        self.sanitizers.iter().any(|s| {
            s.protects_against(&self.vulnerability_type) && s.is_highly_effective()
        })
    }

    /// Get the vulnerability type
    pub fn vulnerability_type(&self) -> Option<&str> {
        Some(&self.vulnerability_type)
    }

    /// Get the severity of this flow
    pub fn severity(&self) -> FlowSeverity {
        if !self.is_vulnerable() {
            return FlowSeverity::Info;
        }

        match (self.source.severity(), self.sink.severity()) {
            (crate::sources::SourceSeverity::High, crate::sinks::SinkSeverity::Critical) => FlowSeverity::Critical,
            (crate::sources::SourceSeverity::High, crate::sinks::SinkSeverity::High) => FlowSeverity::High,
            (crate::sources::SourceSeverity::Medium, crate::sinks::SinkSeverity::Critical) => FlowSeverity::High,
            (crate::sources::SourceSeverity::Medium, crate::sinks::SinkSeverity::High) => FlowSeverity::Medium,
            _ => FlowSeverity::Low,
        }
    }

    /// Get the path length
    pub fn path_length(&self) -> usize {
        self.path.len()
    }

    /// Get locations along the path
    pub fn path_locations(&self) -> Vec<Option<Location>> {
        // This would need access to the graph to get node locations
        // For now, return empty locations
        vec![None; self.path.len()]
    }
}

// FlowSeverity is already defined above

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::DataFlowGraph;

    #[test]
    fn test_taint_tracker_creation() {
        let tracker = TaintTracker::new();
        assert!(tracker.taint_states.is_empty());
    }

    #[test]
    fn test_taint_state() {
        let mut state = TaintState::new();
        assert!(!state.is_tainted());
        assert!(!state.is_sanitizer());

        let taint = TaintInfo::new(0, SourceType::UserInput, 0.8);

        state.add_taint(taint);
        assert!(state.is_tainted());
        assert_eq!(state.max_confidence(), 0.8);
    }

    #[test]
    fn test_taint_flow_vulnerability() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(1, SinkType::HtmlOutput, "XSS".to_string(), "Test sink".to_string());
        
        let flow = TaintFlow::new(
            source,
            sink,
            vec![0, 1],
            0.8,
            "XSS".to_string(),
        );

        assert!(flow.is_vulnerable());
        assert!(!flow.is_effectively_sanitized());
        assert_eq!(flow.vulnerability_type(), Some("XSS"));
    }

    #[test]
    fn test_taint_flow_with_sanitizer() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(2, SinkType::HtmlOutput, "XSS".to_string(), "Test sink".to_string());
        let sanitizer = Sanitizer::new(1, SanitizerType::HtmlEncoding, "HTML encoder".to_string())
            .with_effectiveness(0.9)
            .with_vulnerability_types(vec!["XSS".to_string()]);
        
        let mut flow = TaintFlow::new(
            source,
            sink,
            vec![0, 1, 2],
            0.8,
            "XSS".to_string(),
        );
        flow.add_sanitizer(sanitizer);

        assert!(flow.is_effectively_sanitized());
        assert!(!flow.is_vulnerable()); // Should not be vulnerable due to effective sanitization
    }

    #[test]
    fn test_flow_severity() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(1, SinkType::SqlExecution, "SQL_INJECTION".to_string(), "Test sink".to_string());
        
        let flow = TaintFlow::new(
            source,
            sink,
            vec![0, 1],
            0.8,
            "SQL_INJECTION".to_string(),
        );

        assert_eq!(flow.severity(), FlowSeverity::Critical);
    }

    #[test]
    fn test_simple_taint_tracking() {
        let mut tracker = TaintTracker::new();
        let mut graph = DataFlowGraph::new();
        
        // Create a simple flow: source -> sink
        let source_id = graph.add_node(crate::graph::DataFlowNode::new("call_expression".to_string()));
        let sink_id = graph.add_node(crate::graph::DataFlowNode::new("call_expression".to_string()));
        graph.add_edge(source_id, sink_id, crate::graph::EdgeType::DataFlow);
        
        let sources = vec![Source::new(source_id, SourceType::UserInput, "Test source".to_string())];
        let sinks = vec![Sink::new(sink_id, SinkType::HtmlOutput, "XSS".to_string(), "Test sink".to_string())];
        let sanitizers = Vec::new();
        
        let flows = tracker.track_taint(&graph, &sources, &sinks, &sanitizers).unwrap();
        
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].source.id, source_id);
        assert_eq!(flows[0].sink.id, sink_id);
        assert!(flows[0].is_vulnerable());
    }
}
