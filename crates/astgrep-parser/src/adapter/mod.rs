//! AST adapter modules for converting third-party parser ASTs into
//! astgrep's `UniversalNode`.
//!
//! Each sub-module implements an adapter for a specific parser ecosystem:
//!
//! - `ogsql` — adapter for `ogsql-parser` (openGauss/GaussDB SQL dialect)

pub mod ogsql;

// Re-export the ogsql adapter types so callers can use
// `astgrep_parser::OgsqlAdapter` directly.
pub use ogsql::{OgsqlAdapter, OgsqlAdapterError};
