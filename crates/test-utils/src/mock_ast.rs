//! Mock AST node implementations for testing

use astgrep_core::AstNode;
use astgrep_ast::{UniversalNode, NodeType};

/// Mock AST node for testing purposes
#[derive(Clone, Debug)]
pub struct MockAstNode {
    node_type: String,
    children: Vec<MockAstNode>,
    location: Option<(usize, usize, usize, usize)>,
    text: Option<String>,
}

impl MockAstNode {
    /// Create a new mock AST node
    pub fn new(node_type: &str) -> Self {
        Self {
            node_type: node_type.to_string(),
            children: Vec::new(),
            location: None,
            text: None,
        }
    }

    /// Set location information
    pub fn with_location(mut self, start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        self.location = Some((start_line, start_col, end_line, end_col));
        self
    }

    /// Set text content
    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// Add a child node
    pub fn add_child(mut self, child: MockAstNode) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn with_children(mut self, children: Vec<MockAstNode>) -> Self {
        self.children = children;
        self
    }
}

impl AstNode for MockAstNode {
    fn node_type(&self) -> &str {
        &self.node_type
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn child(&self, index: usize) -> Option<&dyn AstNode> {
        self.children.get(index).map(|c| c as &dyn AstNode)
    }

    fn location(&self) -> Option<(usize, usize, usize, usize)> {
        self.location
    }

    fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    fn clone_node(&self) -> Box<dyn AstNode> {
        Box::new(self.clone())
    }
}

/// Mock UniversalNode for testing
#[derive(Clone, Debug)]
pub struct MockUniversalNode {
    inner: UniversalNode,
}

impl MockUniversalNode {
    /// Create a new mock universal node
    pub fn new(node_type: NodeType) -> Self {
        Self {
            inner: UniversalNode::new(node_type),
        }
    }

    /// Create with text content
    pub fn with_text(node_type: NodeType, text: &str) -> Self {
        Self {
            inner: UniversalNode::new(node_type).with_text(text.to_string()),
        }
    }

    /// Get the inner UniversalNode
    pub fn inner(&self) -> &UniversalNode {
        &self.inner
    }

    /// Get mutable reference to inner UniversalNode
    pub fn inner_mut(&mut self) -> &mut UniversalNode {
        &mut self.inner
    }
}

impl AstNode for MockUniversalNode {
    fn node_type(&self) -> &str {
        self.inner.node_type()
    }

    fn child_count(&self) -> usize {
        self.inner.child_count()
    }

    fn child(&self, index: usize) -> Option<&dyn AstNode> {
        self.inner.child(index)
    }

    fn location(&self) -> Option<(usize, usize, usize, usize)> {
        self.inner.location()
    }

    fn text(&self) -> Option<&str> {
        self.inner.text()
    }

    fn clone_node(&self) -> Box<dyn AstNode> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_ast_node_basic() {
        let node = MockAstNode::new("test")
            .with_text("test content")
            .with_location(1, 1, 1, 10);

        assert_eq!(node.node_type(), "test");
        assert_eq!(node.text(), Some("test content"));
        assert_eq!(node.location(), Some((1, 1, 1, 10)));
        assert_eq!(node.child_count(), 0);
    }

    #[test]
    fn test_mock_ast_node_with_children() {
        let child1 = MockAstNode::new("child1");
        let child2 = MockAstNode::new("child2");
        
        let parent = MockAstNode::new("parent")
            .add_child(child1)
            .add_child(child2);

        assert_eq!(parent.child_count(), 2);
        assert_eq!(parent.child(0).unwrap().node_type(), "child1");
        assert_eq!(parent.child(1).unwrap().node_type(), "child2");
        assert!(parent.child(2).is_none());
    }

    #[test]
    fn test_mock_universal_node() {
        let node = MockUniversalNode::with_text(NodeType::Program, "test program");
        
        assert_eq!(node.node_type(), "program");
        assert_eq!(node.text(), Some("test program"));
    }
}
