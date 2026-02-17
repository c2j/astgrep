//! SymbolicExecutor trait definition
//!
//! Defines the interface for symbolic execution functionality.

use astgrep_core::{AstNode, Result, SemgrepMatchResult, SemgrepPattern};
use std::collections::HashMap;

/// Trait for symbolic execution functionality
///
/// This trait defines the interface for symbolic propagation and
/// variable tracking through the program.
pub trait SymbolicExecutor: Send + Sync {
    /// Check variable type via symbolic propagation
    fn check_type_via_symbolic_propagation(
        &self,
        var_value: &str,
        expected_type: &str,
        propagator: &astgrep_dataflow::SymbolicPropagator,
        full_source: &str,
        import_map: &HashMap<String, String>,
    ) -> bool;

    /// Find matches via symbolic propagation
    fn find_matches_via_symbolic_propagation(
        &mut self,
        pattern: &SemgrepPattern,
        ast: &dyn AstNode,
        type_constraints: &[(String, String)],
        symbolic_propagator: Option<&astgrep_dataflow::SymbolicPropagator>,
        constant_propagator: Option<&astgrep_dataflow::ConstantPropagator>,
        import_map: &HashMap<String, String>,
        source_text: &str,
    ) -> Result<Vec<SemgrepMatchResult>>;

    /// Collect variable declarations from source
    fn collect_variable_declarations(
        &self,
        node: &dyn AstNode,
        declarations: &mut Vec<(String, usize, usize)>,
    ) -> Result<()>;

    /// Collect method calls from source
    fn collect_method_calls(
        &self,
        node: &dyn AstNode,
        method_calls: &mut Vec<(String, String, usize, usize, Box<dyn AstNode>)>,
    ) -> Result<()>;

    /// Parse an ellipsis pattern like "x(). ... .z()"
    fn parse_ellipsis_pattern(&self, pattern_str: &str) -> Option<(String, String)>;

    /// Build a map of imported simple names to their fully qualified names
    fn build_import_map(&self, full_source: &str) -> HashMap<String, String>;

    /// Resolve a simple type name to its fully qualified name using import map
    fn resolve_type_with_imports(
        &self,
        simple_type: &str,
        import_map: &HashMap<String, String>,
    ) -> Option<String>;
}
