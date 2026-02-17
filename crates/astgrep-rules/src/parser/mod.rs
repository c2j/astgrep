//! YAML rule parser
//!
//! This module provides functionality to parse rules from YAML format.

pub mod parsing;

// Re-export public types for backward compatibility
pub use parsing::RuleParser;
