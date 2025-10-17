//! Test utilities and mock implementations for CR-SemService
//! 
//! This crate contains all mock implementations and test utilities that were
//! previously scattered throughout the production code. This separation ensures
//! that test code doesn't pollute production code.

pub mod mock_ast;
pub mod mock_parser;
pub mod mock_data;

// Re-export commonly used mock types
pub use mock_ast::{MockAstNode, MockUniversalNode};
pub use mock_parser::MockParser;
pub use mock_data::{MockRules, MockJobs, MockMetrics, MockFindings, MockMetricsData};

/// Test utilities version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use cr_core::{AstNode, LanguageParser};

    #[test]
    fn test_mock_ast_node_creation() {
        let node = MockAstNode::new("test");
        assert_eq!(node.node_type(), "test");
        assert_eq!(node.child_count(), 0);
    }

    #[test]
    fn test_mock_parser_creation() {
        let parser = MockParser::new(cr_core::Language::Java);
        assert_eq!(parser.language(), cr_core::Language::Java);
    }
}
