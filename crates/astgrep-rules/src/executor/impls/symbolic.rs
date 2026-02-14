//! Default implementation of SymbolicExecutor trait

use crate::executor::core_helpers;
use crate::executor::traits::SymbolicExecutor;
use astgrep_core::{AstNode, Result, SemgrepMatchResult, SemgrepPattern};
use std::collections::HashMap;

pub struct DefaultSymbolicExecutor;

impl DefaultSymbolicExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultSymbolicExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolicExecutor for DefaultSymbolicExecutor {
    fn check_type_via_symbolic_propagation(
        &self,
        _var_value: &str,
        _expected_type: &str,
        _propagator: &astgrep_dataflow::SymbolicPropagator,
        _full_source: &str,
        _import_map: &HashMap<String, String>,
    ) -> bool {
        false
    }

    fn find_matches_via_symbolic_propagation(
        &mut self,
        _pattern: &SemgrepPattern,
        _ast: &dyn AstNode,
        _type_constraints: &[(String, String)],
        _symbolic_propagator: Option<&astgrep_dataflow::SymbolicPropagator>,
        _constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        _import_map: &HashMap<String, String>,
        _source_text: &str,
    ) -> Result<Vec<SemgrepMatchResult>> {
        Ok(Vec::new())
    }

    fn collect_variable_declarations(
        &self,
        _node: &dyn AstNode,
        _declarations: &mut Vec<(String, usize, usize)>,
    ) -> Result<()> {
        Ok(())
    }

    fn collect_method_calls(
        &self,
        _node: &dyn AstNode,
        _method_calls: &mut Vec<(String, String, usize, usize, Box<dyn AstNode>)>,
    ) -> Result<()> {
        Ok(())
    }

    fn parse_ellipsis_pattern(&self, pattern_str: &str) -> Option<(String, String)> {
        core_helpers::parse_ellipsis_pattern(pattern_str)
    }

    fn build_import_map(&self, full_source: &str) -> HashMap<String, String> {
        core_helpers::build_import_map(full_source)
    }

    fn resolve_type_with_imports(
        &self,
        simple_type: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String> {
        core_helpers::resolve_type_with_imports(simple_type, import_map)
    }
}

// Helper function for parsing ellipsis pattern (used internally)
fn parse_ellipsis_pattern(pattern_str: &str) -> Option<(String, String)> {
    let pattern = pattern_str.replace(" ", "");

    let start_paren = pattern.find("()")?;
    let start_method = if start_paren > 0 {
        pattern[..start_paren].to_string()
    } else {
        return None;
    };

    let ellipsis_idx = pattern.find("...")?;
    let after_ellipsis = &pattern[ellipsis_idx + 3..];

    if !after_ellipsis.starts_with('.') {
        return None;
    }

    let remaining = &after_ellipsis[1..];
    let end_paren = remaining.find("()")?;
    let end_method = remaining[..end_paren].trim_start_matches('.').to_string();

    Some((start_method, end_method))
}
