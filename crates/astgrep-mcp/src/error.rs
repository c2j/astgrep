//! Error handling utilities for the MCP server.

/// Convert an anyhow error to a human-readable message string.
pub fn analysis_error_to_msg(err: &anyhow::Error) -> String {
    format!("{err:#}")
}
