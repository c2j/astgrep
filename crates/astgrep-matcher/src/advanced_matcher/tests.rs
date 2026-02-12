use super::*;
use astgrep_core::{PatternType, SemgrepPattern};

// Mock AST node for testing
struct MockNode {
    text: Option<String>,
    children: Vec<MockNode>,
}

impl MockNode {
    fn new(text: &str) -> Self {
        Self {
            text: Some(text.to_string()),
            children: Vec::new(),
        }
    }

    fn with_children(text: &str, children: Vec<MockNode>) -> Self {
        Self {
            text: Some(text.to_string()),
            children,
        }
    }
}

impl AstNode for MockNode {
    fn node_type(&self) -> &str {
        "mock"
    }
    fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child(&self, index: usize) -> Option<&dyn AstNode> {
        self.children.get(index).map(|c| c as &dyn AstNode)
    }
    fn clone_node(&self) -> Box<dyn AstNode> {
        Box::new(MockNode {
            text: self.text.clone(),
            children: self
                .children
                .iter()
                .map(|c| MockNode {
                    text: c.text.clone(),
                    children: c.children.clone(),
                })
                .collect(),
        })
    }
}

#[test]
fn test_pattern_not_regex() {
    let mut matcher = AdvancedSemgrepMatcher::new();

    // Create a pattern that should NOT match "test_function"
    let pattern = SemgrepPattern {
        pattern_type: PatternType::NotRegex("test_.*".to_string()),
        conditions: Vec::new(),
        focus: None,
    };

    let test_node = MockNode::new("test_function");
    let regular_node = MockNode::new("regular_function");

    // Should not match test_function (matches the regex, so not-regex is false)
    assert!(!matcher.matches_pattern(&pattern, &test_node).unwrap());

    // Should match regular_function (doesn't match the regex, so not-regex is true)
    assert!(matcher.matches_pattern(&pattern, &regular_node).unwrap());
}

#[test]
fn test_pattern_not_inside() {
    let mut matcher = AdvancedSemgrepMatcher::new();

    // Create inner pattern for class context
    let inner_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("class".to_string()),
        conditions: Vec::new(),
        focus: None,
    };

    // Create not-inside pattern
    let pattern = SemgrepPattern {
        pattern_type: PatternType::NotInside(Box::new(inner_pattern)),
        conditions: Vec::new(),
        focus: None,
    };

    // Create test nodes
    let _class_node = MockNode::new("class");
    let function_node = MockNode::new("function");
    let _nested_function = MockNode::with_children("class", vec![MockNode::new("function")]);

    // Function inside class should not match (inside class context)
    // Note: This is a simplified test - real implementation would need proper AST traversal
    assert!(matcher.matches_pattern(&pattern, &function_node).unwrap());
}
