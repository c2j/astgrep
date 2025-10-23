//! Constant propagation analysis for data flow
//!
//! This module implements constant propagation to track constant values
//! through the program and enable more precise taint analysis.

use crate::graph::{DataFlowGraph, NodeId};
use crate::symbol_table::SymbolTable;
use astgrep_core::Result;
use std::collections::{HashMap, HashSet};

/// Represents a constant value in the program
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstantValue {
    /// String constant
    String(String),
    /// Integer constant
    Integer(i64),
    /// Boolean constant
    Boolean(bool),
    /// Null constant
    Null,
    /// Unknown constant
    Unknown,
}

impl ConstantValue {
    /// Check if this constant matches a pattern
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        match self {
            ConstantValue::String(s) => s.contains(pattern),
            ConstantValue::Integer(i) => i.to_string().contains(pattern),
            ConstantValue::Boolean(b) => b.to_string().contains(pattern),
            _ => false,
        }
    }

    /// Convert to string representation
    pub fn to_string_value(&self) -> Option<String> {
        match self {
            ConstantValue::String(s) => Some(s.clone()),
            ConstantValue::Integer(i) => Some(i.to_string()),
            ConstantValue::Boolean(b) => Some(b.to_string()),
            ConstantValue::Null => Some("null".to_string()),
            ConstantValue::Unknown => None,
        }
    }
}

/// Constant propagation analyzer
pub struct ConstantPropagator {
    /// Map from variable name to constant value
    constants: HashMap<String, ConstantValue>,
    /// Map from node ID to constant value
    node_constants: HashMap<NodeId, ConstantValue>,
    /// Set of variables that are reassigned (not constant)
    reassigned: HashSet<String>,
}

impl ConstantPropagator {
    /// Create a new constant propagator
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            node_constants: HashMap::new(),
            reassigned: HashSet::new(),
        }
    }

    /// Analyze constants in the data flow graph
    pub fn analyze(&mut self, graph: &DataFlowGraph, symbol_table: &SymbolTable) -> Result<()> {
        // First pass: collect all constant assignments
        self.collect_constants(graph, symbol_table)?;

        // Second pass: propagate constants through the graph
        self.propagate_constants(graph)?;

        Ok(())
    }

    /// Collect constant assignments from the graph
    fn collect_constants(&mut self, graph: &DataFlowGraph, symbol_table: &SymbolTable) -> Result<()> {
        for node_id in graph.get_all_nodes() {
            if let Some(node) = graph.get_node(node_id) {
                // Check if this is a constant assignment
                if let Some(constant) = self.extract_constant_from_node(node) {
                    // Get the variable name from the node
                    if let Some(var_name) = self.get_variable_name_from_node(node) {
                        // Check if variable is reassigned
                        if !self.reassigned.contains(&var_name) {
                            self.constants.insert(var_name, constant.clone());
                            self.node_constants.insert(node_id, constant);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Propagate constants through the graph
    fn propagate_constants(&mut self, graph: &DataFlowGraph) -> Result<()> {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for node_id in graph.get_all_nodes() {
                // Get predecessors in the data flow graph
                let predecessors = graph.data_flow_predecessors(node_id);

                for pred_id in predecessors {
                    if let Some(pred_constant) = self.node_constants.get(&pred_id).cloned() {
                        if !self.node_constants.contains_key(&node_id) {
                            self.node_constants.insert(node_id, pred_constant);
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract constant value from a node
    fn extract_constant_from_node(&self, node: &dyn std::any::Any) -> Option<ConstantValue> {
        // This is a placeholder - in real implementation, we would parse the node
        // to extract string literals, integer literals, etc.
        None
    }

    /// Get variable name from a node
    fn get_variable_name_from_node(&self, node: &dyn std::any::Any) -> Option<String> {
        // This is a placeholder - in real implementation, we would extract
        // the variable name from assignment nodes
        None
    }

    /// Get constant value for a variable
    pub fn get_constant(&self, var_name: &str) -> Option<&ConstantValue> {
        self.constants.get(var_name)
    }

    /// Get constant value for a node
    pub fn get_node_constant(&self, node_id: NodeId) -> Option<&ConstantValue> {
        self.node_constants.get(&node_id)
    }

    /// Check if a variable is constant
    pub fn is_constant(&self, var_name: &str) -> bool {
        self.constants.contains_key(var_name) && !self.reassigned.contains(var_name)
    }

    /// Mark a variable as reassigned (not constant)
    pub fn mark_reassigned(&mut self, var_name: String) {
        self.reassigned.insert(var_name.clone());
        self.constants.remove(&var_name);
    }

    /// Get all constants
    pub fn get_all_constants(&self) -> &HashMap<String, ConstantValue> {
        &self.constants
    }

    /// Get all node constants
    pub fn get_all_node_constants(&self) -> &HashMap<NodeId, ConstantValue> {
        &self.node_constants
    }
}

impl Default for ConstantPropagator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_value_string() {
        let cv = ConstantValue::String("password".to_string());
        assert_eq!(cv.to_string_value(), Some("password".to_string()));
        assert!(cv.matches_pattern("pass"));
    }

    #[test]
    fn test_constant_value_integer() {
        let cv = ConstantValue::Integer(42);
        assert_eq!(cv.to_string_value(), Some("42".to_string()));
        assert!(cv.matches_pattern("42"));
    }

    #[test]
    fn test_constant_value_boolean() {
        let cv = ConstantValue::Boolean(true);
        assert_eq!(cv.to_string_value(), Some("true".to_string()));
        assert!(cv.matches_pattern("true"));
    }

    #[test]
    fn test_constant_value_null() {
        let cv = ConstantValue::Null;
        assert_eq!(cv.to_string_value(), Some("null".to_string()));
    }

    #[test]
    fn test_constant_propagator_new() {
        let propagator = ConstantPropagator::new();
        assert!(propagator.constants.is_empty());
        assert!(propagator.node_constants.is_empty());
        assert!(propagator.reassigned.is_empty());
    }

    #[test]
    fn test_constant_propagator_mark_reassigned() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("x".to_string(), ConstantValue::Integer(42));
        
        assert!(propagator.is_constant("x"));
        
        propagator.mark_reassigned("x".to_string());
        
        assert!(!propagator.is_constant("x"));
        assert!(!propagator.constants.contains_key("x"));
    }

    #[test]
    fn test_constant_propagator_get_constant() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("password".to_string(), ConstantValue::String("secret".to_string()));
        
        assert_eq!(
            propagator.get_constant("password"),
            Some(&ConstantValue::String("secret".to_string()))
        );
        assert_eq!(propagator.get_constant("unknown"), None);
    }

    #[test]
    fn test_constant_value_equality() {
        let cv1 = ConstantValue::String("test".to_string());
        let cv2 = ConstantValue::String("test".to_string());
        let cv3 = ConstantValue::String("other".to_string());
        
        assert_eq!(cv1, cv2);
        assert_ne!(cv1, cv3);
    }
}

