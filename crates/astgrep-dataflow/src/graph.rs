//! Data flow graph representation
//! 
//! This module provides the core data structures for representing data flow graphs.

use astgrep_core::{AstNode, Location};
use std::collections::{HashMap, HashSet};

/// Unique identifier for nodes in the data flow graph
pub type NodeId = usize;

/// Data flow graph
#[derive(Debug, Clone)]
pub struct DataFlowGraph {
    nodes: HashMap<NodeId, DataFlowNode>,
    edges: HashMap<NodeId, Vec<DataFlowEdge>>,
    reverse_edges: HashMap<NodeId, Vec<DataFlowEdge>>,
    next_id: NodeId,
}

impl DataFlowGraph {
    /// Create a new empty data flow graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            next_id: 0,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: DataFlowNode) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, node);
        self.edges.insert(id, Vec::new());
        self.reverse_edges.insert(id, Vec::new());
        id
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, edge_type: EdgeType) {
        let edge = DataFlowEdge {
            from,
            to,
            edge_type,
        };
        
        self.edges.entry(from).or_insert_with(Vec::new).push(edge.clone());
        self.reverse_edges.entry(to).or_insert_with(Vec::new).push(edge);
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&DataFlowNode> {
        self.nodes.get(&id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<NodeId, DataFlowNode> {
        &self.nodes
    }

    /// Get all node IDs
    pub fn get_all_nodes(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    /// Get all node IDs as iterator
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Check if there's a data flow edge between two nodes
    pub fn has_data_flow_edge(&self, from: NodeId, to: NodeId) -> bool {
        if let Some(edges) = self.edges.get(&from) {
            edges.iter().any(|edge| edge.to == to && matches!(edge.edge_type, EdgeType::DataFlow))
        } else {
            false
        }
    }

    /// Get predecessors of a node (alias for compatibility)
    pub fn get_predecessors(&self, id: NodeId) -> Vec<NodeId> {
        self.predecessors(id)
    }

    /// Get outgoing edges from a node
    pub fn outgoing_edges(&self, id: NodeId) -> &[DataFlowEdge] {
        self.edges.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get incoming edges to a node
    pub fn incoming_edges(&self, id: NodeId) -> &[DataFlowEdge] {
        self.reverse_edges.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all successors of a node
    pub fn successors(&self, id: NodeId) -> Vec<NodeId> {
        self.outgoing_edges(id).iter().map(|e| e.to).collect()
    }

    /// Get all predecessors of a node
    pub fn predecessors(&self, id: NodeId) -> Vec<NodeId> {
        self.incoming_edges(id).iter().map(|e| e.from).collect()
    }

    /// Get data flow successors (only data flow edges)
    pub fn data_flow_successors(&self, id: NodeId) -> Vec<NodeId> {
        self.outgoing_edges(id)
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::DataFlow))
            .map(|e| e.to)
            .collect()
    }

    /// Get data flow predecessors (only data flow edges)
    pub fn data_flow_predecessors(&self, id: NodeId) -> Vec<NodeId> {
        self.incoming_edges(id)
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::DataFlow))
            .map(|e| e.from)
            .collect()
    }

    /// Find all paths between two nodes
    pub fn find_paths(&self, from: NodeId, to: NodeId) -> Vec<Vec<NodeId>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();
        
        self.find_paths_recursive(from, to, &mut current_path, &mut visited, &mut paths);
        paths
    }

    /// Recursive helper for path finding
    fn find_paths_recursive(
        &self,
        current: NodeId,
        target: NodeId,
        path: &mut Vec<NodeId>,
        visited: &mut HashSet<NodeId>,
        paths: &mut Vec<Vec<NodeId>>,
    ) {
        if visited.contains(&current) {
            return; // Avoid cycles
        }

        path.push(current);
        visited.insert(current);

        if current == target {
            paths.push(path.clone());
        } else {
            for successor in self.data_flow_successors(current) {
                self.find_paths_recursive(successor, target, path, visited, paths);
            }
        }

        path.pop();
        visited.remove(&current);
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.reverse_edges.clear();
        self.next_id = 0;
    }

    // node_ids method is already defined above

    /// Check if the graph contains a node
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Get nodes by type
    pub fn nodes_by_type(&self, node_type: &str) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.node_type == node_type)
            .map(|(id, _)| *id)
            .collect()
    }
}

impl Default for DataFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Node in the data flow graph
#[derive(Debug, Clone)]
pub struct DataFlowNode {
    pub node_type: String,
    pub text: Option<String>,
    pub location: Option<Location>,
    pub attributes: HashMap<String, String>,
}

impl DataFlowNode {
    /// Create a new data flow node
    pub fn new(node_type: String) -> Self {
        Self {
            node_type,
            text: None,
            location: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a data flow node from an AST node
    pub fn from_ast_node(ast_node: &dyn AstNode) -> Self {
        Self {
            node_type: ast_node.node_type().to_string(),
            text: ast_node.text().map(|s| s.to_string()),
            location: ast_node.location().map(|(start_line, start_col, end_line, end_col)| {
                Location {
                    file: std::path::PathBuf::new(),
                    start_line,
                    start_column: start_col,
                    end_line,
                    end_column: end_col,
                }
            }),
            attributes: HashMap::new(),
        }
    }

    /// Set text content
    pub fn with_text(mut self, text: String) -> Self {
        self.text = Some(text);
        self
    }

    /// Set location
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Get an attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Check if this node represents a function call
    pub fn is_function_call(&self) -> bool {
        self.node_type == "call_expression"
    }

    /// Check if this node represents a variable
    pub fn is_variable(&self) -> bool {
        self.node_type == "identifier"
    }

    /// Check if this node represents a literal
    pub fn is_literal(&self) -> bool {
        self.node_type == "literal" || self.node_type == "string_literal" || self.node_type == "integer_literal"
    }

    /// Get the function name if this is a function call
    pub fn function_name(&self) -> Option<&str> {
        if self.is_function_call() {
            self.text.as_deref()
        } else {
            None
        }
    }

    /// Get the variable name if this is a variable
    pub fn variable_name(&self) -> Option<&str> {
        if self.is_variable() {
            self.text.as_deref()
        } else {
            None
        }
    }
}

/// Edge in the data flow graph
#[derive(Debug, Clone)]
pub struct DataFlowEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub edge_type: EdgeType,
}

/// Type of edge in the data flow graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    /// Control flow edge (execution order)
    ControlFlow,
    /// Data flow edge (data dependency)
    DataFlow,
    /// Call edge (function call)
    Call,
    /// Return edge (function return)
    Return,
}

impl EdgeType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::ControlFlow => "control_flow",
            EdgeType::DataFlow => "data_flow",
            EdgeType::Call => "call",
            EdgeType::Return => "return",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = DataFlowGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let mut graph = DataFlowGraph::new();
        
        let node1 = DataFlowNode::new("identifier".to_string()).with_text("x".to_string());
        let node2 = DataFlowNode::new("literal".to_string()).with_text("42".to_string());
        
        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_add_edges() {
        let mut graph = DataFlowGraph::new();
        
        let id1 = graph.add_node(DataFlowNode::new("identifier".to_string()));
        let id2 = graph.add_node(DataFlowNode::new("literal".to_string()));
        
        graph.add_edge(id1, id2, EdgeType::DataFlow);
        
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.successors(id1), vec![id2]);
        assert_eq!(graph.predecessors(id2), vec![id1]);
    }

    #[test]
    fn test_data_flow_edges() {
        let mut graph = DataFlowGraph::new();
        
        let id1 = graph.add_node(DataFlowNode::new("identifier".to_string()));
        let id2 = graph.add_node(DataFlowNode::new("literal".to_string()));
        let id3 = graph.add_node(DataFlowNode::new("call".to_string()));
        
        graph.add_edge(id1, id2, EdgeType::DataFlow);
        graph.add_edge(id1, id3, EdgeType::ControlFlow);
        
        assert_eq!(graph.data_flow_successors(id1), vec![id2]);
        assert_eq!(graph.successors(id1).len(), 2);
    }

    #[test]
    fn test_find_paths() {
        let mut graph = DataFlowGraph::new();
        
        let id1 = graph.add_node(DataFlowNode::new("start".to_string()));
        let id2 = graph.add_node(DataFlowNode::new("middle".to_string()));
        let id3 = graph.add_node(DataFlowNode::new("end".to_string()));
        
        graph.add_edge(id1, id2, EdgeType::DataFlow);
        graph.add_edge(id2, id3, EdgeType::DataFlow);
        
        let paths = graph.find_paths(id1, id3);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], vec![id1, id2, id3]);
    }

    #[test]
    fn test_node_properties() {
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("printf".to_string());
        
        assert!(node.is_function_call());
        assert!(!node.is_variable());
        assert_eq!(node.function_name(), Some("printf"));
    }

    #[test]
    fn test_nodes_by_type() {
        let mut graph = DataFlowGraph::new();
        
        let id1 = graph.add_node(DataFlowNode::new("identifier".to_string()));
        let id2 = graph.add_node(DataFlowNode::new("literal".to_string()));
        let id3 = graph.add_node(DataFlowNode::new("identifier".to_string()));
        
        let identifiers = graph.nodes_by_type("identifier");
        assert_eq!(identifiers.len(), 2);
        assert!(identifiers.contains(&id1));
        assert!(identifiers.contains(&id3));
    }
}
