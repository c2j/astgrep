//! Dependencies module for ASTGreP CLI
//!
//! This module provides dependency analysis and resolution functionality
//! for test scripts and other components.

pub mod script_deps;

// Re-export commonly used types
pub use script_deps::*;
