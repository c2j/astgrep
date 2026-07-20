//! astgrep MCP (Model Context Protocol) server
//!
//! Provides MCP tools for code analysis, rule validation, and rule/language discovery
//! over the Model Context Protocol stdio transport.

pub mod error;
pub mod server;
pub mod tools;

pub use server::serve_stdio;
