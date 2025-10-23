//! Advanced taint propagation and analysis
//!
//! This module provides enhanced taint tracking with support for:
//! - Complex data transformations
//! - Conditional taint propagation
//! - Taint merging and splitting
//! - Context-aware analysis

use crate::taint::{TaintFlow, TaintState};
use crate::graph::{DataFlowGraph, NodeId};
use astgrep_core::Result;
use std::collections::{HashMap, HashSet};

/// Represents a data transformation that affects taint
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaintTransformation {
    /// Identity transformation (no change)
    Identity,
    /// String concatenation
    Concatenation,
    /// String encoding (base64, URL, etc.)
    Encoding(String),
    /// String decoding
    Decoding(String),
    /// Hashing
    Hashing(String),
    /// Encryption
    Encryption(String),
    /// Decryption
    Decryption(String),
    /// Type casting
    TypeCast(String),
    /// Method call
    MethodCall(String),
    /// Custom transformation
    Custom(String),
}

impl TaintTransformation {
    /// Check if this transformation sanitizes taint
    pub fn sanitizes(&self) -> bool {
        matches!(
            self,
            TaintTransformation::Hashing(_)
                | TaintTransformation::Encryption(_)
                | TaintTransformation::Encoding(_)
        )
    }

    /// Get the sanitization effectiveness (0.0 to 1.0)
    pub fn effectiveness(&self) -> f32 {
        match self {
            TaintTransformation::Hashing(_) => 1.0,
            TaintTransformation::Encryption(_) => 0.95,
            TaintTransformation::Encoding(_) => 0.7,
            TaintTransformation::Decoding(_) => 0.0,
            TaintTransformation::Decryption(_) => 0.0,
            _ => 0.0,
        }
    }
}

/// Advanced taint state with transformation tracking
#[derive(Debug, Clone)]
pub struct AdvancedTaintState {
    /// Base taint state
    pub base_taint: TaintState,
    /// Transformations applied to this taint
    pub transformations: Vec<TaintTransformation>,
    /// Context information
    pub context: HashMap<String, String>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
}

impl AdvancedTaintState {
    /// Create a new advanced taint state
    pub fn new(base_taint: TaintState) -> Self {
        Self {
            base_taint,
            transformations: Vec::new(),
            context: HashMap::new(),
            confidence: 1.0,
        }
    }

    /// Add a transformation
    pub fn add_transformation(&mut self, transformation: TaintTransformation) {
        self.transformations.push(transformation);
    }

    /// Get the effective confidence after transformations
    pub fn effective_confidence(&self) -> f32 {
        let mut confidence = self.confidence;
        for transformation in &self.transformations {
            confidence *= (1.0 - transformation.effectiveness());
        }
        confidence
    }

    /// Check if taint is effectively sanitized
    pub fn is_sanitized(&self) -> bool {
        self.effective_confidence() < 0.1
    }

    /// Set context information
    pub fn set_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    /// Get context information
    pub fn get_context(&self, key: &str) -> Option<&str> {
        self.context.get(key).map(|s| s.as_str())
    }
}

/// Advanced taint analyzer with transformation tracking
pub struct AdvancedTaintAnalyzer {
    /// Taint states with transformations
    taint_states: HashMap<NodeId, AdvancedTaintState>,
    /// Transformation rules
    transformation_rules: HashMap<String, Vec<TaintTransformation>>,
}

impl AdvancedTaintAnalyzer {
    /// Create a new advanced taint analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            taint_states: HashMap::new(),
            transformation_rules: HashMap::new(),
        };
        analyzer.load_default_rules();
        analyzer
    }

    /// Load default transformation rules
    fn load_default_rules(&mut self) {
        // String methods that don't sanitize
        let string_methods = vec![
            TaintTransformation::MethodCall("substring".to_string()),
            TaintTransformation::MethodCall("toUpperCase".to_string()),
            TaintTransformation::MethodCall("toLowerCase".to_string()),
            TaintTransformation::MethodCall("trim".to_string()),
        ];
        self.transformation_rules.insert("string_methods".to_string(), string_methods);

        // Sanitization methods
        let sanitization_methods = vec![
            TaintTransformation::MethodCall("htmlEscape".to_string()),
            TaintTransformation::MethodCall("sqlEscape".to_string()),
            TaintTransformation::MethodCall("urlEncode".to_string()),
        ];
        self.transformation_rules.insert("sanitization_methods".to_string(), sanitization_methods);
    }

    /// Track a taint through the graph with transformations
    pub fn track_taint_with_transformations(
        &mut self,
        graph: &DataFlowGraph,
        start_node: NodeId,
        transformations: Vec<TaintTransformation>,
    ) -> Result<Vec<(NodeId, AdvancedTaintState)>> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![start_node];

        while let Some(node_id) = queue.pop() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);

            // Create advanced taint state
            let mut taint_state = AdvancedTaintState::new(TaintState::default());
            for transformation in &transformations {
                taint_state.add_transformation(transformation.clone());
            }

            results.push((node_id, taint_state));

            // Add successors to queue
            for successor in graph.successors(node_id) {
                if !visited.contains(&successor) {
                    queue.push(successor);
                }
            }
        }

        Ok(results)
    }

    /// Merge multiple taint states
    pub fn merge_taints(&self, taints: &[AdvancedTaintState]) -> AdvancedTaintState {
        if taints.is_empty() {
            return AdvancedTaintState::new(TaintState::default());
        }

        let mut merged = taints[0].clone();
        for taint in &taints[1..] {
            // Merge transformations (keep most restrictive)
            for transformation in &taint.transformations {
                if !merged.transformations.contains(transformation) {
                    merged.transformations.push(transformation.clone());
                }
            }
            // Average confidence
            merged.confidence = (merged.confidence + taint.confidence) / 2.0;
        }

        merged
    }

    /// Split taint for conditional branches
    pub fn split_taint(&self, taint: &AdvancedTaintState, condition: &str) -> (AdvancedTaintState, AdvancedTaintState) {
        let mut true_branch = taint.clone();
        let mut false_branch = taint.clone();

        true_branch.set_context("branch".to_string(), "true".to_string());
        true_branch.set_context("condition".to_string(), condition.to_string());

        false_branch.set_context("branch".to_string(), "false".to_string());
        false_branch.set_context("condition".to_string(), condition.to_string());

        (true_branch, false_branch)
    }

    /// Get taint state for a node
    pub fn get_taint_state(&self, node_id: NodeId) -> Option<&AdvancedTaintState> {
        self.taint_states.get(&node_id)
    }

    /// Set taint state for a node
    pub fn set_taint_state(&mut self, node_id: NodeId, state: AdvancedTaintState) {
        self.taint_states.insert(node_id, state);
    }

    /// Clear all taint states
    pub fn clear(&mut self) {
        self.taint_states.clear();
    }

    /// Apply context-aware filtering to taint states
    pub fn filter_by_context(
        &self,
        taints: &[AdvancedTaintState],
        context_key: &str,
        context_value: &str,
    ) -> Vec<AdvancedTaintState> {
        taints
            .iter()
            .filter(|taint| taint.get_context(context_key) == Some(context_value))
            .cloned()
            .collect()
    }

    /// Combine multiple taint states with context awareness
    pub fn combine_with_context(
        &self,
        taints: &[AdvancedTaintState],
        merge_strategy: &str,
    ) -> AdvancedTaintState {
        match merge_strategy {
            "union" => {
                // Union: keep all transformations
                let mut combined = taints[0].clone();
                for taint in &taints[1..] {
                    for transformation in &taint.transformations {
                        if !combined.transformations.contains(transformation) {
                            combined.transformations.push(transformation.clone());
                        }
                    }
                }
                combined
            }
            "intersection" => {
                // Intersection: keep only common transformations
                if taints.is_empty() {
                    return AdvancedTaintState::new(TaintState::default());
                }
                let mut combined = taints[0].clone();
                for taint in &taints[1..] {
                    combined.transformations.retain(|t| taint.transformations.contains(t));
                }
                combined
            }
            "most_restrictive" => {
                // Most restrictive: highest sanitization effectiveness
                let mut most_restrictive = taints[0].clone();
                for taint in &taints[1..] {
                    let current_effectiveness: f32 = most_restrictive
                        .transformations
                        .iter()
                        .map(|t| t.effectiveness())
                        .sum();
                    let new_effectiveness: f32 =
                        taint.transformations.iter().map(|t| t.effectiveness()).sum();

                    if new_effectiveness > current_effectiveness {
                        most_restrictive = taint.clone();
                    }
                }
                most_restrictive
            }
            _ => self.merge_taints(taints),
        }
    }
}

impl Default for AdvancedTaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taint_transformation_sanitizes() {
        let hash = TaintTransformation::Hashing("SHA256".to_string());
        assert!(hash.sanitizes());

        let concat = TaintTransformation::Concatenation;
        assert!(!concat.sanitizes());
    }

    #[test]
    fn test_taint_transformation_effectiveness() {
        let hash = TaintTransformation::Hashing("SHA256".to_string());
        assert_eq!(hash.effectiveness(), 1.0);

        let encoding = TaintTransformation::Encoding("base64".to_string());
        assert_eq!(encoding.effectiveness(), 0.7);

        let concat = TaintTransformation::Concatenation;
        assert_eq!(concat.effectiveness(), 0.0);
    }

    #[test]
    fn test_advanced_taint_state_creation() {
        let base_taint = TaintState::default();
        let advanced = AdvancedTaintState::new(base_taint);

        assert_eq!(advanced.transformations.len(), 0);
        assert_eq!(advanced.confidence, 1.0);
    }

    #[test]
    fn test_advanced_taint_state_add_transformation() {
        let base_taint = TaintState::default();
        let mut advanced = AdvancedTaintState::new(base_taint);

        advanced.add_transformation(TaintTransformation::Concatenation);
        advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));

        assert_eq!(advanced.transformations.len(), 2);
    }

    #[test]
    fn test_advanced_taint_state_effective_confidence() {
        let base_taint = TaintState::default();
        let mut advanced = AdvancedTaintState::new(base_taint);

        advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));
        let effective = advanced.effective_confidence();
        assert!(effective < 1.0);
        assert!(effective > 0.0);
    }

    #[test]
    fn test_advanced_taint_state_is_sanitized() {
        let base_taint = TaintState::default();
        let mut advanced = AdvancedTaintState::new(base_taint);

        advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));
        assert!(advanced.is_sanitized());
    }

    #[test]
    fn test_advanced_taint_analyzer_creation() {
        let analyzer = AdvancedTaintAnalyzer::new();
        assert!(!analyzer.transformation_rules.is_empty());
    }

    #[test]
    fn test_merge_taints() {
        let analyzer = AdvancedTaintAnalyzer::new();
        let base_taint = TaintState::default();

        let mut taint1 = AdvancedTaintState::new(base_taint.clone());
        taint1.add_transformation(TaintTransformation::Concatenation);
        taint1.confidence = 0.9;

        let mut taint2 = AdvancedTaintState::new(base_taint);
        taint2.add_transformation(TaintTransformation::Encoding("base64".to_string()));
        taint2.confidence = 0.8;

        let merged = analyzer.merge_taints(&[taint1, taint2]);
        assert_eq!(merged.transformations.len(), 2);
        assert_eq!(merged.confidence, 0.85);
    }

    #[test]
    fn test_split_taint() {
        let analyzer = AdvancedTaintAnalyzer::new();
        let base_taint = TaintState::default();
        let taint = AdvancedTaintState::new(base_taint);

        let (true_branch, false_branch) = analyzer.split_taint(&taint, "x > 0");

        assert_eq!(true_branch.get_context("branch"), Some("true"));
        assert_eq!(false_branch.get_context("branch"), Some("false"));
    }
}

