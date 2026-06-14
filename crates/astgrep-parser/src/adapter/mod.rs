//! AST adapter modules for converting third-party parser ASTs into
//! astgrep's `UniversalNode`.
//!
//! Each sub-module implements an adapter for a specific parser ecosystem:
//!
//! - `ogsql` — adapter for `ogsql-parser` (openGauss/GaussDB SQL dialect)
//! - `sqlparser` — adapter for `sqlparser-rs` (PolarDB-MySQL dialect)

pub mod ogsql;
pub mod sqlparser;

pub use ogsql::{OgsqlAdapter, OgsqlAdapterError};
pub use sqlparser::{SqlparserAdapter, SqlparserAdapterError};
