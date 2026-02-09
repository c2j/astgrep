//! Tree-sitter based parser implementation
//!
//! This module provides tree-sitter based parsing for various languages.
//!
//! ## Module Structure
//! - `integration`: Tree-sitter integration and parser setup
//! - `conversion`: Node conversion and type mapping
//! - `pattern_matching`: Pattern matching traversal
//! - `ast_ops`: AST operations and transformations

pub mod integration;
pub mod conversion;
pub mod pattern_matching;
pub mod ast_ops;

// Re-export public types for backward compatibility
pub use integration::{
    TreeSitterParser,
    PatternType,
    MetaVariableBindings,
};
