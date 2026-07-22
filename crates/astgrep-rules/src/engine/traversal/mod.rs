//! Traversal module
//!
//! This module provides the rule execution engine that applies rules to AST nodes.
//! It has been refactored from a single large file into focused sub-modules:
//!
//! - `types`: Type definitions (TaintMatch, RuleExecutionEngine struct)
//! - `utils`: Tokenization and utility functions
//! - `pattern`: Pattern matching logic (execute_pattern and helpers)
//! - `execution`: Core rule execution logic (execute_rule, execute_rules, etc.)
//! - `dataflow`: Dataflow and taint analysis
//! - `matching`: Find pattern matches and span utilities
//! - `location`: Location utilities and message generation

pub mod dataflow;
pub mod execution;
pub mod location;
pub mod matching;
pub mod pattern;
pub mod text_pattern;
pub mod types;
pub mod utils;

// Re-export main types for backward compatibility
pub use types::{RuleExecutionEngine, TaintMatch};
