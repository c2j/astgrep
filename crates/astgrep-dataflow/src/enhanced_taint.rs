//! Enhanced taint analysis with improved precision

use crate::graph::{DataFlowGraph, NodeId};
use crate::sanitizers::Sanitizer;
use crate::sinks::Sink;
use crate::sources::{Source, SourceType};
use astgrep_core::Result;
use std::collections::{HashMap, HashSet};

/// Applied sanitizer information
#[derive(Debug, Clone)]
pub struct AppliedSanitizer {
    /// Sanitizer identifier
    pub sanitizer_id: NodeId,
    /// Types of vulnerabilities this sanitizer protects against
    pub protected_types: HashSet<String>,
    /// Effectiveness of the sanitizer (0.0 to 1.0)
    pub effectiveness: f32,
}

/// Enhanced taint information with more detailed tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancedTaintInfo {
    /// Unique identifier for this taint
    pub id: u64,
    /// Source node that introduced this taint
    pub source_id: NodeId,
    /// Type of the source
    pub source_type: SourceType,
    /// Confidence level (0 to 100)
    pub confidence: u8,
    /// Path through the program
    pub path: Vec<NodeId>,
    /// Context information
    pub context: TaintContext,
    /// Field access path for field-sensitive analysis
    pub field_path: Vec<String>,
    /// Types of vulnerabilities this taint can lead to
    pub vulnerability_types: HashSet<String>,
}

impl std::hash::Hash for EnhancedTaintInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.source_id.hash(state);
        self.confidence.hash(state);
        self.path.hash(state);
        self.context.hash(state);
        self.field_path.hash(state);
        // Note: HashSet doesn't implement Hash, so we skip vulnerability_types
    }
}

/// Context information for taint
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaintContext {
    /// Call stack for context-sensitive analysis
    pub call_stack: Vec<NodeId>,
    /// Loop nesting depth
    pub loop_depth: usize,
    /// Conditional context
    pub conditional_context: Vec<NodeId>,
}

/// Enhanced taint state for a node
#[derive(Debug, Clone)]
pub struct EnhancedTaintState {
    /// Set of taint information
    taints: HashSet<EnhancedTaintInfo>,
    /// Applied sanitizers (kept for future taint policy analysis)
    #[allow(dead_code)]
    sanitizers: Vec<AppliedSanitizer>,
    /// Field-specific taint information (kept for future field-sensitive analysis)
    #[allow(dead_code)]
    field_taints: HashMap<String, HashSet<EnhancedTaintInfo>>,
}

/// Enhanced taint flow information
#[derive(Debug, Clone)]
pub struct EnhancedTaintFlow {
    /// Source of the taint
    pub source: Source,
    /// Sink where taint reaches
    pub sink: Sink,
    /// Path from source to sink
    pub path: Vec<NodeId>,
    /// Confidence in this flow
    pub confidence: u8,
    /// Vulnerability types
    pub vulnerability_types: HashSet<String>,
    /// Sanitizers that were bypassed
    pub sanitizers_bypassed: Vec<AppliedSanitizer>,
    /// Context information
    pub context_info: TaintContext,
}

/// Configuration for taint analysis
#[derive(Debug, Clone)]
pub struct TaintAnalysisConfig {
    /// Maximum path length to prevent infinite loops
    pub max_path_length: usize,
    /// Maximum number of contexts to track
    pub max_contexts: usize,
    /// Enable field-sensitive analysis
    pub field_sensitive: bool,
    /// Enable context-sensitive analysis
    pub context_sensitive: bool,
    /// Enable path-sensitive analysis
    pub path_sensitive: bool,
    /// Minimum confidence threshold
    pub min_confidence: u8,
    /// Minimum confidence threshold for reporting flows
    pub min_confidence_threshold: u8,
}

impl Default for TaintAnalysisConfig {
    fn default() -> Self {
        Self {
            max_path_length: 50,
            max_contexts: 100,
            field_sensitive: true,
            context_sensitive: true,
            path_sensitive: false,
            min_confidence: 10,
            min_confidence_threshold: 30,
        }
    }
}

/// Enhanced taint tracker
pub struct EnhancedTaintTracker {
    /// Taint states for each node
    taint_states: HashMap<NodeId, EnhancedTaintState>,
    /// Call graph for inter-procedural analysis (kept for future inter-procedural taint tracking)
    #[allow(dead_code)]
    call_graph: HashMap<NodeId, Vec<NodeId>>,
    /// Field mappings for field-sensitive analysis (kept for future field-sensitive taint tracking)
    #[allow(dead_code)]
    field_mappings: HashMap<NodeId, FieldInfo>,
    /// Context stack for tracking call contexts
    context_stack: Vec<NodeId>,
    /// Configuration
    config: TaintAnalysisConfig,
}

/// Field information for field-sensitive analysis
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Object this field belongs to
    pub object_id: NodeId,
    /// Field name
    pub field_name: String,
    /// Field type if known
    pub field_type: Option<String>,
}

impl TaintContext {
    fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            loop_depth: 0,
            conditional_context: Vec::new(),
        }
    }
}

impl EnhancedTaintState {
    fn new() -> Self {
        Self {
            taints: HashSet::new(),
            sanitizers: Vec::new(),
            field_taints: HashMap::new(),
        }
    }

    fn add_taint(&mut self, taint: EnhancedTaintInfo) {
        self.taints.insert(taint);
    }

    #[allow(dead_code)]
    fn add_sanitizer(&mut self, sanitizer: AppliedSanitizer) {
        self.sanitizers.push(sanitizer);
    }
}

impl Default for EnhancedTaintTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedTaintTracker {
    /// Create a new enhanced taint tracker
    pub fn new() -> Self {
        Self::with_config(TaintAnalysisConfig::default())
    }

    /// Create a new enhanced taint tracker with custom configuration
    pub fn with_config(config: TaintAnalysisConfig) -> Self {
        Self {
            taint_states: HashMap::new(),
            call_graph: HashMap::new(),
            field_mappings: HashMap::new(),
            context_stack: Vec::new(),
            config,
        }
    }

    /// Perform enhanced taint analysis
    pub fn analyze_taint(
        &mut self,
        graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
    ) -> Result<Vec<EnhancedTaintFlow>> {
        // Clear previous state
        self.taint_states.clear();
        self.context_stack.clear();

        // Initialize taint states for source nodes
        self.initialize_source_taints(sources)?;

        // Propagate taint through the graph
        self.propagate_taint_through_graph(graph)?;

        // Find flows from sources to sinks
        let flows = self.find_enhanced_flows(graph, sources, sinks, sanitizers)?;

        Ok(flows)
    }

    /// Initialize taint states for source nodes
    fn initialize_source_taints(&mut self, sources: &[Source]) -> Result<()> {
        for source in sources {
            // Create vulnerability types set from source type
            let mut vulnerability_types = HashSet::new();
            vulnerability_types.insert(format!("{:?}", source.source_type));

            let taint_info = EnhancedTaintInfo {
                id: source.id as u64,
                source_id: source.id,
                source_type: source.source_type.clone(),
                confidence: 100, // Start with full confidence
                path: vec![source.id],
                context: TaintContext::new(),
                field_path: Vec::new(),
                vulnerability_types,
            };

            let mut taint_state = EnhancedTaintState::new();
            taint_state.add_taint(taint_info);

            self.taint_states.insert(source.id, taint_state);
        }
        Ok(())
    }

    /// Propagate taint through the data flow graph
    fn propagate_taint_through_graph(&mut self, graph: &DataFlowGraph) -> Result<()> {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            // Process each node in the graph
            for node_id in graph.get_all_nodes() {
                if self.propagate_taint_to_node(graph, node_id)? {
                    changed = true;
                }
            }
        }

        if iterations >= MAX_ITERATIONS {
            tracing::warn!("Taint propagation reached maximum iterations, may be incomplete");
        }

        Ok(())
    }

    /// Propagate taint to a specific node
    fn propagate_taint_to_node(&mut self, graph: &DataFlowGraph, node_id: NodeId) -> Result<bool> {
        let predecessors = graph.get_predecessors(node_id);
        if predecessors.is_empty() {
            return Ok(false);
        }

        let mut new_taints = Vec::new();

        // Collect taint from all predecessors
        for pred_id in predecessors {
            if let Some(pred_state) = self.taint_states.get(&pred_id) {
                for taint in &pred_state.taints {
                    // Create new taint with updated path
                    let mut new_taint = taint.clone();
                    new_taint.path.push(node_id);

                    // Reduce confidence based on path length
                    let confidence_reduction = (new_taint.path.len() as f32 * 0.05).min(0.3);
                    new_taint.confidence =
                        ((new_taint.confidence as f32) * (1.0 - confidence_reduction)) as u8;

                    new_taints.push(new_taint);
                }
            }
        }

        if new_taints.is_empty() {
            return Ok(false);
        }

        // Update or create taint state for this node
        let current_state = self
            .taint_states
            .entry(node_id)
            .or_insert_with(EnhancedTaintState::new);
        let initial_count = current_state.taints.len();

        for taint in new_taints {
            current_state.add_taint(taint);
        }

        Ok(current_state.taints.len() > initial_count)
    }

    /// Find enhanced taint flows from sources to sinks
    fn find_enhanced_flows(
        &self,
        _graph: &DataFlowGraph,
        sources: &[Source],
        sinks: &[Sink],
        sanitizers: &[Sanitizer],
    ) -> Result<Vec<EnhancedTaintFlow>> {
        let mut flows = Vec::new();

        for sink in sinks {
            if let Some(sink_state) = self.taint_states.get(&sink.id) {
                for taint in &sink_state.taints {
                    // Find the original source
                    if let Some(source) = sources.iter().find(|s| s.id == taint.source_id) {
                        // Check for sanitizers along the path
                        let sanitizers_on_path =
                            self.find_sanitizers_on_path(&taint.path, sanitizers);

                        // Calculate final confidence considering sanitizers
                        let final_confidence = self.calculate_confidence_with_sanitizers(
                            taint.confidence,
                            &sanitizers_on_path,
                            &taint.vulnerability_types,
                        );

                        if final_confidence > self.config.min_confidence_threshold {
                            let flow = EnhancedTaintFlow {
                                source: source.clone(),
                                sink: sink.clone(),
                                path: taint.path.clone(),
                                confidence: final_confidence,
                                vulnerability_types: taint.vulnerability_types.clone(),
                                sanitizers_bypassed: sanitizers_on_path,
                                context_info: taint.context.clone(),
                            };
                            flows.push(flow);
                        }
                    }
                }
            }
        }

        Ok(flows)
    }

    /// Find sanitizers along a taint path
    fn find_sanitizers_on_path(
        &self,
        path: &[NodeId],
        sanitizers: &[Sanitizer],
    ) -> Vec<AppliedSanitizer> {
        let mut applied_sanitizers = Vec::new();

        for &node_id in path {
            for sanitizer in sanitizers {
                if sanitizer.id == node_id {
                    applied_sanitizers.push(AppliedSanitizer {
                        sanitizer_id: sanitizer.id,
                        protected_types: sanitizer.vulnerability_types.iter().cloned().collect(),
                        effectiveness: sanitizer.effectiveness,
                    });
                }
            }
        }

        applied_sanitizers
    }

    /// Calculate confidence considering sanitizers
    fn calculate_confidence_with_sanitizers(
        &self,
        base_confidence: u8,
        sanitizers: &[AppliedSanitizer],
        vulnerability_types: &HashSet<String>,
    ) -> u8 {
        let mut confidence = base_confidence as f32 / 100.0;

        for sanitizer in sanitizers {
            // Check if sanitizer protects against any of the vulnerability types
            let protects = vulnerability_types
                .iter()
                .any(|vt| sanitizer.protected_types.contains(vt));

            if protects {
                // Reduce confidence based on sanitizer effectiveness
                confidence *= 1.0 - sanitizer.effectiveness;
            }
        }

        (confidence * 100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{DataFlowGraph, DataFlowNode, EdgeType};
    use crate::sanitizers::{Sanitizer, SanitizerType};
    use crate::sinks::{Sink, SinkType};
    use crate::sources::{Source, SourceType};

    #[test]
    fn test_enhanced_taint_tracker_new() {
        let tracker = EnhancedTaintTracker::new();
        assert!(tracker.taint_states.is_empty());
    }

    #[test]
    fn test_enhanced_taint_tracker_with_config() {
        let config = TaintAnalysisConfig {
            max_path_length: 100,
            max_contexts: 50,
            field_sensitive: false,
            context_sensitive: false,
            path_sensitive: true,
            min_confidence: 20,
            min_confidence_threshold: 40,
        };
        let tracker = EnhancedTaintTracker::with_config(config.clone());
        assert_eq!(tracker.config.max_path_length, 100);
        assert_eq!(tracker.config.min_confidence_threshold, 40);
    }

    #[test]
    fn test_taint_analysis_config_default() {
        let config = TaintAnalysisConfig::default();
        assert_eq!(config.max_path_length, 50);
        assert_eq!(config.max_contexts, 100);
        assert!(config.field_sensitive);
        assert!(config.context_sensitive);
        assert!(!config.path_sensitive);
    }

    #[test]
    fn test_enhanced_taint_tracker_analyze_taint() {
        let mut tracker = EnhancedTaintTracker::new();
        let mut graph = DataFlowGraph::new();

        let source_node = graph.add_node(
            DataFlowNode::new("call_expression".to_string())
                .with_text("request.getParameter".to_string()),
        );
        let sink_node = graph.add_node(
            DataFlowNode::new("call_expression".to_string()).with_text("executeQuery".to_string()),
        );
        graph.add_edge(source_node, sink_node, EdgeType::DataFlow);

        let source = Source::new(
            source_node,
            SourceType::UserInput,
            "HTTP request parameter".to_string(),
        );
        let sink = Sink::new(
            sink_node,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "SQL query execution".to_string(),
        );

        let flows = tracker
            .analyze_taint(&graph, &[source], &[sink], &[])
            .unwrap();
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].confidence, 90);
    }

    #[test]
    fn test_enhanced_taint_tracker_analyze_taint_with_sanitizer() {
        let mut tracker = EnhancedTaintTracker::new();
        let mut graph = DataFlowGraph::new();

        let source_node = graph.add_node(
            DataFlowNode::new("call_expression".to_string())
                .with_text("request.getParameter".to_string()),
        );
        let sanitizer_node = graph.add_node(
            DataFlowNode::new("call_expression".to_string()).with_text("htmlEncode".to_string()),
        );
        let sink_node = graph.add_node(
            DataFlowNode::new("call_expression".to_string()).with_text("innerHTML".to_string()),
        );
        graph.add_edge(source_node, sanitizer_node, EdgeType::DataFlow);
        graph.add_edge(sanitizer_node, sink_node, EdgeType::DataFlow);

        let source = Source::new(
            source_node,
            SourceType::UserInput,
            "HTTP request parameter".to_string(),
        );
        let sink = Sink::new(
            sink_node,
            SinkType::HtmlOutput,
            "XSS".to_string(),
            "HTML output".to_string(),
        );
        let sanitizer = Sanitizer::new(
            sanitizer_node,
            SanitizerType::HtmlEncoding,
            "HTML encoder".to_string(),
        )
        .with_effectiveness(0.9)
        .with_vulnerability_types(vec!["XSS".to_string()]);

        let flows = tracker
            .analyze_taint(&graph, &[source], &[sink], &[sanitizer])
            .unwrap();
        assert_eq!(flows.len(), 1);
        assert!(flows[0].confidence < 100);
        assert_eq!(flows[0].sanitizers_bypassed.len(), 1);
    }

    #[test]
    fn test_enhanced_taint_info_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let info1 = EnhancedTaintInfo {
            id: 1,
            source_id: 0,
            source_type: SourceType::UserInput,
            confidence: 100,
            path: vec![0, 1],
            context: TaintContext::new(),
            field_path: vec![],
            vulnerability_types: {
                let mut set = std::collections::HashSet::new();
                set.insert("XSS".to_string());
                set
            },
        };

        let mut hasher = DefaultHasher::new();
        info1.hash(&mut hasher);
        let _hash = hasher.finish();
    }

    #[test]
    fn test_taint_context_new() {
        let context = TaintContext::new();
        assert!(context.call_stack.is_empty());
        assert_eq!(context.loop_depth, 0);
        assert!(context.conditional_context.is_empty());
    }

    #[test]
    fn test_enhanced_taint_state_new() {
        let state = EnhancedTaintState::new();
        assert!(state.taints.is_empty());
    }
}
