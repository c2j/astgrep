//! Traversal module - Location and message utilities
//!
//! This module provides location creation and finding message generation utilities.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::{AstNode, Finding, Location};
use std::path::PathBuf;

impl RuleExecutionEngine {
    /// Try to create a best-effort location for a match using node.location() first,
    /// then fallback to approximating from the pattern's literal anchors in source text.
    pub(crate) fn create_best_location_from_node_or_pattern(
        &self,
        node: &dyn AstNode,
        pattern: &Pattern,
        context: &RuleContext,
    ) -> Location {
        // 1) If the AST node carries precise location, use it.
        if let Some((sl, sc, el, ec)) = node.location() {
            return Location::new(PathBuf::from(&context.file_path), sl, sc, el, ec);
        }
        // 2) Fallback: try to approximate location by searching literal anchors from the pattern
        if let Some(pat_str) = pattern.get_pattern_string() {
            if let Some((start_byte, end_byte)) =
                RuleExecutionEngine::approximate_span_from_pattern(&context.source_code, pat_str)
            {
                let (sl, sc) = Self::byte_index_to_line_col(&context.source_code, start_byte);
                let (el, ec) = Self::byte_index_to_line_col(&context.source_code, end_byte);
                return Location::new(PathBuf::from(&context.file_path), sl, sc, el, ec);
            }
        }
        // 3) Last resort: point at file start
        Location::point(PathBuf::from(&context.file_path), 1, 1)
    }

    /// Generate finding message
    pub(crate) fn generate_finding_message(
        &self,
        rule: &Rule,
        pattern: &Pattern,
        node: &dyn AstNode,
    ) -> String {
        // Use rule.description if available, otherwise generate a default message
        if !rule.description.is_empty() {
            rule.description.clone()
        } else {
            let default_pattern = "<complex pattern>".to_string();
            let pattern_str = pattern.get_pattern_string().unwrap_or(&default_pattern);
            if let Some(text) = node.text() {
                format!(
                    "{}: Found '{}' matching pattern '{}'",
                    rule.name, text, pattern_str
                )
            } else {
                format!(
                    "{}: Found node matching pattern '{}'",
                    rule.name, pattern_str
                )
            }
        }
    }
}
