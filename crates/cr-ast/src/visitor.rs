//! AST visitor pattern implementation
//! 
//! This module provides visitor patterns for traversing and transforming AST nodes.

use crate::nodes::{NodeType, UniversalNode};
use cr_core::{AstNode, Result};

/// Trait for AST visitors
pub trait AstVisitor {
    /// Visit any AST node
    fn visit(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a specific node (default implementation)
    fn visit_node(&mut self, node: &dyn AstNode) -> Result<()> {
        // Default implementation visits all children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit(child)?;
            }
        }
        Ok(())
    }

    /// Visit an identifier node
    fn visit_identifier(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a literal node
    fn visit_literal(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a binary expression node
    fn visit_binary_expression(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a unary expression node
    fn visit_unary_expression(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a call expression node
    fn visit_call_expression(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a function declaration node
    fn visit_function_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a variable declaration node
    fn visit_variable_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a class declaration node
    fn visit_class_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit an if statement node
    fn visit_if_statement(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a while statement node
    fn visit_while_statement(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a for statement node
    fn visit_for_statement(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a return statement node
    fn visit_return_statement(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }

    /// Visit a block statement node
    fn visit_block_statement(&mut self, node: &dyn AstNode) -> Result<()> {
        self.visit_node(node)
    }
}

/// Dispatching visitor that calls specific visit methods based on node type
pub struct DispatchingVisitor<V: AstVisitor> {
    visitor: V,
}

impl<V: AstVisitor> DispatchingVisitor<V> {
    pub fn new(visitor: V) -> Self {
        Self { visitor }
    }

    pub fn visit(&mut self, node: &dyn AstNode) -> Result<()> {
        match NodeType::from_str(node.node_type()) {
            Some(NodeType::Identifier) => self.visitor.visit_identifier(node),
            Some(NodeType::Literal) => self.visitor.visit_literal(node),
            Some(NodeType::BinaryExpression) => self.visitor.visit_binary_expression(node),
            Some(NodeType::UnaryExpression) => self.visitor.visit_unary_expression(node),
            Some(NodeType::CallExpression) => self.visitor.visit_call_expression(node),
            Some(NodeType::FunctionDeclaration) => self.visitor.visit_function_declaration(node),
            Some(NodeType::VariableDeclaration) => self.visitor.visit_variable_declaration(node),
            Some(NodeType::ClassDeclaration) => self.visitor.visit_class_declaration(node),
            Some(NodeType::IfStatement) => self.visitor.visit_if_statement(node),
            Some(NodeType::WhileStatement) => self.visitor.visit_while_statement(node),
            Some(NodeType::ForStatement) => self.visitor.visit_for_statement(node),
            Some(NodeType::ReturnStatement) => self.visitor.visit_return_statement(node),
            Some(NodeType::BlockStatement) => self.visitor.visit_block_statement(node),
            _ => self.visitor.visit_node(node),
        }
    }

    pub fn into_inner(self) -> V {
        self.visitor
    }
}

/// Simple visitor that collects all nodes of a specific type
pub struct NodeCollector {
    target_type: String,
    collected_nodes: Vec<Box<dyn AstNode>>,
}

impl NodeCollector {
    pub fn new(target_type: String) -> Self {
        Self {
            target_type,
            collected_nodes: Vec::new(),
        }
    }

    pub fn collected_nodes(self) -> Vec<Box<dyn AstNode>> {
        self.collected_nodes
    }
}

impl AstVisitor for NodeCollector {
    fn visit_node(&mut self, node: &dyn AstNode) -> Result<()> {
        if node.node_type() == self.target_type {
            self.collected_nodes.push(node.clone_node());
        }
        
        // Continue visiting children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit(child)?;
            }
        }
        Ok(())
    }
}

/// Visitor that counts nodes by type
pub struct NodeCounter {
    counts: std::collections::HashMap<String, usize>,
}

impl NodeCounter {
    pub fn new() -> Self {
        Self {
            counts: std::collections::HashMap::new(),
        }
    }

    pub fn counts(&self) -> &std::collections::HashMap<String, usize> {
        &self.counts
    }

    pub fn total_count(&self) -> usize {
        self.counts.values().sum()
    }
}

impl Default for NodeCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstVisitor for NodeCounter {
    fn visit_node(&mut self, node: &dyn AstNode) -> Result<()> {
        let node_type = node.node_type().to_string();
        *self.counts.entry(node_type).or_insert(0) += 1;
        
        // Continue visiting children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit(child)?;
            }
        }
        Ok(())
    }
}

/// Visitor that finds nodes at a specific location
pub struct LocationFinder {
    target_line: usize,
    target_column: usize,
    found_nodes: Vec<Box<dyn AstNode>>,
}

impl LocationFinder {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            target_line: line,
            target_column: column,
            found_nodes: Vec::new(),
        }
    }

    pub fn found_nodes(self) -> Vec<Box<dyn AstNode>> {
        self.found_nodes
    }

    fn contains_position(&self, location: (usize, usize, usize, usize)) -> bool {
        let (start_line, start_col, end_line, end_col) = location;
        
        if self.target_line < start_line || self.target_line > end_line {
            return false;
        }
        
        if self.target_line == start_line && self.target_column < start_col {
            return false;
        }
        
        if self.target_line == end_line && self.target_column > end_col {
            return false;
        }
        
        true
    }
}

impl AstVisitor for LocationFinder {
    fn visit_node(&mut self, node: &dyn AstNode) -> Result<()> {
        if let Some(location) = node.location() {
            if self.contains_position(location) {
                self.found_nodes.push(node.clone_node());
            }
        }
        
        // Continue visiting children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit(child)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{BinaryOperator, LiteralValue, UniversalNode};
    use cr_core::AstNode;

    fn create_test_ast() -> UniversalNode {
        // Create a simple AST: function add(a, b) { return a + b; }
        let param_a = UniversalNode::new(NodeType::Identifier)
            .with_identifier("a".to_string());
        let param_b = UniversalNode::new(NodeType::Identifier)
            .with_identifier("b".to_string());
        
        let left_operand = UniversalNode::new(NodeType::Identifier)
            .with_identifier("a".to_string());
        let right_operand = UniversalNode::new(NodeType::Identifier)
            .with_identifier("b".to_string());
        
        let binary_expr = UniversalNode::new(NodeType::BinaryExpression)
            .with_binary_operator(BinaryOperator::Add)
            .add_child(left_operand)
            .add_child(right_operand);
        
        let return_stmt = UniversalNode::new(NodeType::ReturnStatement)
            .add_child(binary_expr);
        
        let block = UniversalNode::new(NodeType::BlockStatement)
            .add_child(return_stmt);
        
        UniversalNode::new(NodeType::FunctionDeclaration)
            .with_identifier("add".to_string())
            .add_child(param_a)
            .add_child(param_b)
            .add_child(block)
    }

    #[test]
    fn test_node_collector() {
        let ast = create_test_ast();
        let mut collector = NodeCollector::new("identifier".to_string());
        collector.visit(&ast).unwrap();
        
        let collected = collector.collected_nodes();
        assert_eq!(collected.len(), 4); // "add", "a", "b", "a", "b" but some might be duplicates
    }

    #[test]
    fn test_node_counter() {
        let ast = create_test_ast();
        let mut counter = NodeCounter::new();
        counter.visit(&ast).unwrap();
        
        let counts = counter.counts();
        assert!(counts.get("identifier").unwrap_or(&0) > &0);
        assert!(counts.get("function_declaration").unwrap_or(&0) > &0);
        assert!(counts.get("binary_expression").unwrap_or(&0) > &0);
        assert!(counter.total_count() > 0);
    }

    #[test]
    fn test_location_finder() {
        let ast = UniversalNode::new(NodeType::Identifier)
            .with_identifier("test".to_string())
            .with_location(1, 5, 1, 9);
        
        let mut finder = LocationFinder::new(1, 7);
        finder.visit(&ast).unwrap();
        
        let found = finder.found_nodes();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_type(), "identifier");
    }

    #[test]
    fn test_location_finder_outside_range() {
        let ast = UniversalNode::new(NodeType::Identifier)
            .with_identifier("test".to_string())
            .with_location(1, 5, 1, 9);
        
        let mut finder = LocationFinder::new(1, 15); // Outside range
        finder.visit(&ast).unwrap();
        
        let found = finder.found_nodes();
        assert_eq!(found.len(), 0);
    }
}
