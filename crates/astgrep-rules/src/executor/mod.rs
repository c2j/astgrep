//! Advanced rule executor with pattern matching and data flow integration
//!
//! This module provides a high-level rule executor that integrates with the pattern
//! matching engine and data flow analyzer for comprehensive static analysis.
//!
//! ## Module Structure
//!
//! - `core`: Main executor implementation with comprehensive analysis
//! - `dependency`: Variable dependency tracking for dataflow analysis  
//! - `types`: Core types and helper functions

pub mod core;
pub mod dependency;
pub mod types;

// Re-export main types for backward compatibility
pub use core::AdvancedRuleExecutor;
pub use dependency::VariableDependencyGraph;
pub use types::{is_operator_node, TaintMatch};
