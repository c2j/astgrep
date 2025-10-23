//! Universal AST definitions for astgrep
//!
//! This crate provides the universal AST node types and operations
//! that are used across all supported languages.

pub mod nodes;
pub mod visitor;
pub mod builder;

pub use nodes::*;
pub use visitor::*;
pub use builder::*;

use astgrep_core::{AstNode, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Universal AST representation
#[derive(Debug, Clone)]
pub struct UniversalAst {
    pub root: nodes::UniversalNode,
    pub metadata: AstMetadata,
}

/// Metadata associated with an AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstMetadata {
    pub language: String,
    pub file_path: String,
    pub source_hash: String,
    pub parse_time_ms: u64,
    pub node_count: usize,
    pub custom_attributes: HashMap<String, String>,
}

impl UniversalAst {
    pub fn new(root: nodes::UniversalNode, metadata: AstMetadata) -> Self {
        Self { root, metadata }
    }

    /// Get the root node
    pub fn root(&self) -> &nodes::UniversalNode {
        &self.root
    }

    /// Get AST metadata
    pub fn metadata(&self) -> &AstMetadata {
        &self.metadata
    }

    /// Count total nodes in the AST
    pub fn node_count(&self) -> usize {
        let mut count = 0;
        let _ = astgrep_core::ast_utils::visit_nodes(&self.root, &mut |_| {
            count += 1;
            Ok(())
        });
        count
    }

    /// Find nodes by type
    pub fn find_nodes_by_type(&self, node_type: &str) -> Vec<Box<dyn AstNode>> {
        astgrep_core::ast_utils::find_nodes(&self.root, &|node| {
            node.node_type() == node_type
        })
    }

    /// Get all unique node types in the AST
    pub fn get_node_types(&self) -> Vec<String> {
        let mut types = std::collections::HashSet::new();
        let _ = astgrep_core::ast_utils::visit_nodes(&self.root, &mut |node| {
            types.insert(node.node_type().to_string());
            Ok(())
        });
        let mut result: Vec<String> = types.into_iter().collect();
        result.sort();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodes::{NodeType, UniversalNode};

    #[test]
    fn test_universal_ast_creation() {
        let root = UniversalNode::new(NodeType::Program);
        let metadata = AstMetadata {
            language: "javascript".to_string(),
            file_path: "test.js".to_string(),
            source_hash: "abc123".to_string(),
            parse_time_ms: 10,
            node_count: 1,
            custom_attributes: HashMap::new(),
        };

        let ast = UniversalAst::new(root, metadata);
        assert_eq!(ast.root().node_type(), "program");
        assert_eq!(ast.metadata().language, "javascript");
        assert_eq!(ast.metadata().file_path, "test.js");
    }

    #[test]
    fn test_ast_node_count() {
        let child1 = UniversalNode::new(NodeType::Identifier);
        let child2 = UniversalNode::new(NodeType::Literal);
        let root = UniversalNode::new(NodeType::Program)
            .add_child(child1)
            .add_child(child2);

        let metadata = AstMetadata {
            language: "test".to_string(),
            file_path: "test.txt".to_string(),
            source_hash: "hash".to_string(),
            parse_time_ms: 5,
            node_count: 3,
            custom_attributes: HashMap::new(),
        };

        let ast = UniversalAst::new(root, metadata);
        assert_eq!(ast.node_count(), 3); // root + 2 children
    }

    #[test]
    fn test_find_nodes_by_type() {
        let id1 = UniversalNode::new(NodeType::Identifier);
        let id2 = UniversalNode::new(NodeType::Identifier);
        let literal = UniversalNode::new(NodeType::Literal);
        let root = UniversalNode::new(NodeType::Program)
            .add_child(id1)
            .add_child(id2)
            .add_child(literal);

        let metadata = AstMetadata {
            language: "test".to_string(),
            file_path: "test.txt".to_string(),
            source_hash: "hash".to_string(),
            parse_time_ms: 5,
            node_count: 4,
            custom_attributes: HashMap::new(),
        };

        let ast = UniversalAst::new(root, metadata);
        let identifiers = ast.find_nodes_by_type("identifier");
        assert_eq!(identifiers.len(), 2);

        let literals = ast.find_nodes_by_type("literal");
        assert_eq!(literals.len(), 1);
    }

    #[test]
    fn test_get_node_types() {
        let id = UniversalNode::new(NodeType::Identifier);
        let literal = UniversalNode::new(NodeType::Literal);
        let root = UniversalNode::new(NodeType::Program)
            .add_child(id)
            .add_child(literal);

        let metadata = AstMetadata {
            language: "test".to_string(),
            file_path: "test.txt".to_string(),
            source_hash: "hash".to_string(),
            parse_time_ms: 5,
            node_count: 3,
            custom_attributes: HashMap::new(),
        };

        let ast = UniversalAst::new(root, metadata);
        let types = ast.get_node_types();
        assert!(types.contains(&"program".to_string()));
        assert!(types.contains(&"identifier".to_string()));
        assert!(types.contains(&"literal".to_string()));
        assert_eq!(types.len(), 3);
    }
}
