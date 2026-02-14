//! Advanced rule executor with pattern matching and data flow integration
//!
//! This module provides a high-level rule executor that integrates with the pattern
//! matching engine and data flow analyzer for comprehensive static analysis.
//!
//! ## Module Structure
//!
//! - `core`: Main executor implementation with comprehensive analysis
//! - `core_helpers`: Helper functions extracted from core
//! - `dependency`: Variable dependency tracking for dataflow analysis  
//! - `types`: Core types and helper functions
//! - `traits`: Trait definitions for executor components
//! - `impls`: Default implementations of executor traits

pub mod core;
pub mod core_helpers;
pub mod dependency;
pub mod impls;
pub mod traits;
pub mod types;

// Re-export main types for backward compatibility
pub use core::AdvancedRuleExecutor;
pub use core_helpers::*;
pub use dependency::VariableDependencyGraph;
pub use impls::{DefaultConditionEvaluator, DefaultSymbolicExecutor, DefaultTaintAnalyzer};
pub use traits::{ConditionEvaluator, SymbolicExecutor, TaintAnalyzer};
pub use types::{is_operator_node, TaintMatch};
