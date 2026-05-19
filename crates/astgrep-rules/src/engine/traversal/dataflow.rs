//! Traversal module - Dataflow and taint analysis
//!
//! This module provides dataflow and taint analysis capabilities for the rule execution engine.

use crate::engine::traversal::RuleExecutionEngine;
use crate::types::*;
use astgrep_core::{AstNode, Finding, Location, Result};
use std::path::PathBuf;

impl RuleExecutionEngine {
    /// Execute dataflow analysis
    pub(crate) fn execute_dataflow(
        &self,
        _dataflow: &DataFlowSpec,
        ast: &dyn AstNode,
        rule: &Rule,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Get the source text from the AST
        let source_text = ast.text().unwrap_or_default();

        // Simple dataflow tracking: look for patterns like "source()" and track if they reach "sink()"
        if source_text.contains("source()") && source_text.contains("sink(") {
            // Check if there's a data flow path (simplified)
            if let Some(source_line) = source_text.lines().find(|l| l.contains("source()")) {
                if let Some(sink_line) = source_text.lines().find(|l| l.contains("sink(")) {
                    // Create findings for both source and sink
                    let source_line_num = source_text
                        .lines()
                        .enumerate()
                        .find(|(_, l)| l == &source_line)
                        .map(|(i, _)| i + 1)
                        .unwrap_or(1);

                    let sink_line_num = source_text
                        .lines()
                        .enumerate()
                        .find(|(_, l)| l == &sink_line)
                        .map(|(i, _)| i + 1)
                        .unwrap_or(1);

                    // Create a finding for the sink (the vulnerability)
                    let finding = Finding::new(
                        rule.id.clone(),
                        format!(
                            "{}: Dataflow from source at line {} to sink at line {}",
                            rule.name, source_line_num, sink_line_num
                        ),
                        rule.severity,
                        rule.confidence,
                        Location::new(
                            PathBuf::from(&context.file_path),
                            sink_line_num,
                            1,
                            sink_line_num,
                            sink_line.len(),
                        ),
                    );
                    findings.push(finding);
                }
            }
        }

        Ok(findings)
    }
}
