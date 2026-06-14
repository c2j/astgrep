//! AST visitor pattern implementation
//!
//! This module provides visitor patterns for traversing and transforming AST nodes.

use crate::nodes::NodeType;
use astgrep_core::{AstNode, Result};

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
        match NodeType::parse_name(node.node_type()) {
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
    use astgrep_core::AstNode;

    fn create_test_ast() -> UniversalNode {
        // Create a simple AST: function add(a, b) { return a + b; }
        let param_a = UniversalNode::new(NodeType::Identifier).with_identifier("a".to_string());
        let param_b = UniversalNode::new(NodeType::Identifier).with_identifier("b".to_string());

        let left_operand =
            UniversalNode::new(NodeType::Identifier).with_identifier("a".to_string());
        let right_operand =
            UniversalNode::new(NodeType::Identifier).with_identifier("b".to_string());

        let binary_expr = UniversalNode::new(NodeType::BinaryExpression)
            .with_binary_operator(BinaryOperator::Add)
            .add_child(left_operand)
            .add_child(right_operand);

        let return_stmt = UniversalNode::new(NodeType::ReturnStatement).add_child(binary_expr);

        let block = UniversalNode::new(NodeType::BlockStatement).add_child(return_stmt);

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

    #[test]
    fn test_ast_visitor_trait_default_impl() {
        struct TestVisitor {
            visited: Vec<String>,
        }
        impl AstVisitor for TestVisitor {
            fn visit_node(&mut self, node: &dyn AstNode) -> Result<()> {
                self.visited.push(node.node_type().to_string());
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.visit(child)?;
                    }
                }
                Ok(())
            }
        }
        let ast = create_test_ast();
        let mut visitor = TestVisitor {
            visited: Vec::new(),
        };
        visitor.visit(&ast).unwrap();
        assert!(visitor
            .visited
            .contains(&"function_declaration".to_string()));
        assert!(visitor.visited.contains(&"identifier".to_string()));
        assert!(visitor.visited.contains(&"binary_expression".to_string()));
    }

    #[test]
    fn test_ast_visitor_specific_methods() {
        struct SpecificVisitor {
            identifiers: usize,
            literals: usize,
            binaries: usize,
            calls: usize,
            functions: usize,
            variables: usize,
            classes: usize,
            ifs: usize,
            whiles: usize,
            fors: usize,
            returns: usize,
            blocks: usize,
        }
        impl AstVisitor for SpecificVisitor {
            fn visit(&mut self, node: &dyn AstNode) -> Result<()> {
                match NodeType::parse_name(node.node_type()) {
                    Some(NodeType::Identifier) => self.visit_identifier(node),
                    Some(NodeType::Literal) => self.visit_literal(node),
                    Some(NodeType::BinaryExpression) => self.visit_binary_expression(node),
                    Some(NodeType::CallExpression) => self.visit_call_expression(node),
                    Some(NodeType::FunctionDeclaration) => self.visit_function_declaration(node),
                    Some(NodeType::VariableDeclaration) => self.visit_variable_declaration(node),
                    Some(NodeType::ClassDeclaration) => self.visit_class_declaration(node),
                    Some(NodeType::IfStatement) => self.visit_if_statement(node),
                    Some(NodeType::WhileStatement) => self.visit_while_statement(node),
                    Some(NodeType::ForStatement) => self.visit_for_statement(node),
                    Some(NodeType::ReturnStatement) => self.visit_return_statement(node),
                    Some(NodeType::BlockStatement) => self.visit_block_statement(node),
                    _ => self.visit_node(node),
                }
            }
            fn visit_identifier(&mut self, node: &dyn AstNode) -> Result<()> {
                self.identifiers += 1;
                self.visit_node(node)
            }
            fn visit_literal(&mut self, node: &dyn AstNode) -> Result<()> {
                self.literals += 1;
                self.visit_node(node)
            }
            fn visit_binary_expression(&mut self, node: &dyn AstNode) -> Result<()> {
                self.binaries += 1;
                self.visit_node(node)
            }
            fn visit_call_expression(&mut self, node: &dyn AstNode) -> Result<()> {
                self.calls += 1;
                self.visit_node(node)
            }
            fn visit_function_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
                self.functions += 1;
                self.visit_node(node)
            }
            fn visit_variable_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
                self.variables += 1;
                self.visit_node(node)
            }
            fn visit_class_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
                self.classes += 1;
                self.visit_node(node)
            }
            fn visit_if_statement(&mut self, node: &dyn AstNode) -> Result<()> {
                self.ifs += 1;
                self.visit_node(node)
            }
            fn visit_while_statement(&mut self, node: &dyn AstNode) -> Result<()> {
                self.whiles += 1;
                self.visit_node(node)
            }
            fn visit_for_statement(&mut self, node: &dyn AstNode) -> Result<()> {
                self.fors += 1;
                self.visit_node(node)
            }
            fn visit_return_statement(&mut self, node: &dyn AstNode) -> Result<()> {
                self.returns += 1;
                self.visit_node(node)
            }
            fn visit_block_statement(&mut self, node: &dyn AstNode) -> Result<()> {
                self.blocks += 1;
                self.visit_node(node)
            }
        }
        let mut visitor = SpecificVisitor {
            identifiers: 0,
            literals: 0,
            binaries: 0,
            calls: 0,
            functions: 0,
            variables: 0,
            classes: 0,
            ifs: 0,
            whiles: 0,
            fors: 0,
            returns: 0,
            blocks: 0,
        };
        let ast = create_test_ast();
        visitor.visit(&ast).unwrap();
        assert!(visitor.identifiers > 0);
        assert_eq!(visitor.functions, 1);
        assert_eq!(visitor.binaries, 1);
        assert_eq!(visitor.returns, 1);
        assert_eq!(visitor.blocks, 1);
    }

    #[test]
    fn test_dispatching_visitor() {
        struct TrackingVisitor {
            identifier_hits: usize,
            literal_hits: usize,
            binary_hits: usize,
            fallback_hits: usize,
        }
        impl AstVisitor for TrackingVisitor {
            fn visit_identifier(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.identifier_hits += 1;
                Ok(())
            }
            fn visit_literal(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.literal_hits += 1;
                Ok(())
            }
            fn visit_binary_expression(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.binary_hits += 1;
                Ok(())
            }
            fn visit_node(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.fallback_hits += 1;
                Ok(())
            }
        }
        let ast = create_test_ast();
        let visitor = TrackingVisitor {
            identifier_hits: 0,
            literal_hits: 0,
            binary_hits: 0,
            fallback_hits: 0,
        };
        let mut dispatch = DispatchingVisitor::new(visitor);

        fn dispatch_tree<V: AstVisitor>(
            node: &dyn AstNode,
            dv: &mut DispatchingVisitor<V>,
        ) -> Result<()> {
            dv.visit(node)?;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    dispatch_tree(child, dv)?;
                }
            }
            Ok(())
        }

        dispatch_tree(&ast, &mut dispatch).unwrap();
        let inner = dispatch.into_inner();
        assert!(inner.identifier_hits > 0);
        assert_eq!(inner.binary_hits, 1);
        assert!(inner.fallback_hits > 0);
    }

    #[test]
    fn test_node_collector_empty() {
        let ast = UniversalNode::new(NodeType::Program);
        let mut collector = NodeCollector::new("identifier".to_string());
        collector.visit(&ast).unwrap();
        assert_eq!(collector.collected_nodes().len(), 0);
    }

    #[test]
    fn test_node_counter_default() {
        let counter = NodeCounter::default();
        assert_eq!(counter.total_count(), 0);
        assert!(counter.counts().is_empty());
    }

    #[test]
    fn test_node_counter_counts() {
        let ast = create_test_ast();
        let mut counter = NodeCounter::new();
        counter.visit(&ast).unwrap();
        let counts = counter.counts();
        assert_eq!(*counts.get("identifier").unwrap(), 4);
        assert_eq!(*counts.get("function_declaration").unwrap(), 1);
        assert_eq!(*counts.get("binary_expression").unwrap(), 1);
        assert_eq!(*counts.get("return_statement").unwrap(), 1);
        assert_eq!(*counts.get("block_statement").unwrap(), 1);
        assert_eq!(counter.total_count(), 8);
    }

    #[test]
    fn test_location_finder_multiline() {
        let ast = UniversalNode::new(NodeType::BlockStatement)
            .with_location(1, 1, 5, 2)
            .add_child(UniversalNode::new(NodeType::Identifier).with_location(2, 5, 2, 10))
            .add_child(UniversalNode::new(NodeType::Literal).with_location(4, 3, 4, 8));
        let mut finder = LocationFinder::new(3, 1);
        finder.visit(&ast).unwrap();
        let found = finder.found_nodes();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_type(), "block_statement");
    }

    #[test]
    fn test_visitor_traversal_correctness() {
        let id = UniversalNode::new(NodeType::Identifier).with_identifier("cond".to_string());
        let ret = UniversalNode::new(NodeType::ReturnStatement);
        let block = UniversalNode::new(NodeType::BlockStatement).add_child(ret);
        let if_stmt = UniversalNode::new(NodeType::IfStatement)
            .add_child(id)
            .add_child(block);
        let program = UniversalNode::new(NodeType::Program).add_child(if_stmt);

        let mut counter = NodeCounter::new();
        counter.visit(&program).unwrap();
        let counts = counter.counts();
        assert_eq!(*counts.get("program").unwrap(), 1);
        assert_eq!(*counts.get("if_statement").unwrap(), 1);
        assert_eq!(*counts.get("identifier").unwrap(), 1);
        assert_eq!(*counts.get("block_statement").unwrap(), 1);
        assert_eq!(*counts.get("return_statement").unwrap(), 1);
        assert_eq!(counter.total_count(), 5);
    }

    #[test]
    fn test_dispatching_visitor_all_dispatch_paths() {
        struct PathTracker {
            dispatched_to: String,
        }
        impl AstVisitor for PathTracker {
            fn visit_identifier(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "identifier".to_string();
                Ok(())
            }
            fn visit_literal(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "literal".to_string();
                Ok(())
            }
            fn visit_binary_expression(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "binary_expression".to_string();
                Ok(())
            }
            fn visit_unary_expression(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "unary_expression".to_string();
                Ok(())
            }
            fn visit_call_expression(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "call_expression".to_string();
                Ok(())
            }
            fn visit_function_declaration(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "function_declaration".to_string();
                Ok(())
            }
            fn visit_variable_declaration(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "variable_declaration".to_string();
                Ok(())
            }
            fn visit_class_declaration(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "class_declaration".to_string();
                Ok(())
            }
            fn visit_if_statement(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "if_statement".to_string();
                Ok(())
            }
            fn visit_while_statement(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "while_statement".to_string();
                Ok(())
            }
            fn visit_for_statement(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "for_statement".to_string();
                Ok(())
            }
            fn visit_return_statement(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "return_statement".to_string();
                Ok(())
            }
            fn visit_block_statement(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "block_statement".to_string();
                Ok(())
            }
            fn visit_node(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.dispatched_to = "fallback".to_string();
                Ok(())
            }
        }

        let cases: Vec<(NodeType, &str)> = vec![
            (NodeType::Identifier, "identifier"),
            (NodeType::Literal, "literal"),
            (NodeType::BinaryExpression, "binary_expression"),
            (NodeType::UnaryExpression, "unary_expression"),
            (NodeType::CallExpression, "call_expression"),
            (NodeType::FunctionDeclaration, "function_declaration"),
            (NodeType::VariableDeclaration, "variable_declaration"),
            (NodeType::ClassDeclaration, "class_declaration"),
            (NodeType::IfStatement, "if_statement"),
            (NodeType::WhileStatement, "while_statement"),
            (NodeType::ForStatement, "for_statement"),
            (NodeType::ReturnStatement, "return_statement"),
            (NodeType::BlockStatement, "block_statement"),
            (NodeType::Program, "fallback"),
            (NodeType::Unknown, "fallback"),
            (NodeType::Comment, "fallback"),
        ];

        for (node_type, expected) in cases {
            let mut dv = DispatchingVisitor::new(PathTracker {
                dispatched_to: String::new(),
            });
            let node = UniversalNode::new(node_type.clone());
            dv.visit(&node).unwrap();
            let inner = dv.into_inner();
            assert_eq!(
                inner.dispatched_to, expected,
                "DispatchingVisitor wrong dispatch for {:?}",
                node_type
            );
        }
    }

    #[test]
    fn test_location_finder_exact_start_boundary() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(5, 10);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);
    }

    #[test]
    fn test_location_finder_exact_end_boundary() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(5, 20);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);
    }

    #[test]
    fn test_location_finder_just_before_start_column() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(5, 9);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_location_finder_just_after_end_column() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(5, 21);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_location_finder_before_start_line() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(4, 15);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_location_finder_after_end_line() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(6, 15);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_location_finder_mid_range() {
        let node = UniversalNode::new(NodeType::Identifier).with_location(5, 10, 5, 20);
        let mut finder = LocationFinder::new(5, 15);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);
    }

    #[test]
    fn test_location_finder_node_without_location() {
        let node = UniversalNode::new(NodeType::Identifier).with_identifier("no_loc".to_string());
        let mut finder = LocationFinder::new(1, 1);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_location_finder_nested_matches() {
        let inner = UniversalNode::new(NodeType::Identifier).with_location(2, 4, 2, 8);
        let outer = UniversalNode::new(NodeType::BlockStatement)
            .with_location(1, 1, 3, 2)
            .add_child(inner);

        let mut finder = LocationFinder::new(2, 6);
        finder.visit(&outer).unwrap();
        let found = finder.found_nodes();
        assert_eq!(found.len(), 2); // both outer and inner contain position
    }

    #[test]
    fn test_location_finder_multiline_boundary() {
        let node = UniversalNode::new(NodeType::FunctionDeclaration).with_location(10, 1, 20, 5);

        let mut finder = LocationFinder::new(10, 1);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);

        let mut finder = LocationFinder::new(15, 3);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);

        let mut finder = LocationFinder::new(20, 5);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 1);

        let mut finder = LocationFinder::new(20, 6);
        finder.visit(&node).unwrap();
        assert_eq!(finder.found_nodes().len(), 0);
    }

    #[test]
    fn test_visitor_leaf_node() {
        let leaf = UniversalNode::new(NodeType::Literal).with_literal(LiteralValue::Integer(42));
        let mut counter = NodeCounter::new();
        counter.visit(&leaf).unwrap();
        let counts = counter.counts();
        assert_eq!(*counts.get("literal").unwrap(), 1);
        assert_eq!(counter.total_count(), 1);
    }

    #[test]
    fn test_node_collector_specific_type_from_nested_tree() {
        let deep = UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::String("deep".to_string()));
        let mid = UniversalNode::new(NodeType::Identifier)
            .with_identifier("mid".to_string())
            .add_child(deep);
        let root = UniversalNode::new(NodeType::Program)
            .add_child(UniversalNode::new(NodeType::Identifier).with_identifier("top".to_string()))
            .add_child(mid);

        let mut collector = NodeCollector::new("identifier".to_string());
        collector.visit(&root).unwrap();
        let collected = collector.collected_nodes();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_dispatching_visitor_into_inner_preserves_state() {
        struct CountingVisitor {
            count: usize,
        }
        impl AstVisitor for CountingVisitor {
            fn visit_node(&mut self, _node: &dyn AstNode) -> Result<()> {
                self.count += 1;
                Ok(())
            }
        }
        let ast = create_test_ast();
        let mut dv = DispatchingVisitor::new(CountingVisitor { count: 0 });
        dv.visit(&ast).unwrap();
        let inner = dv.into_inner();
        assert!(inner.count > 0);
    }
}
