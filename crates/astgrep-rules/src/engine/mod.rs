//! Rule execution engine
//!
//! This module provides a core rule execution engine that applies rules to AST nodes.

pub mod traversal;

// Re-export public types for backward compatibility
pub use traversal::{RuleExecutionEngine, TaintMatch};
