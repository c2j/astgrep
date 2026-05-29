//! Core types and helper functions for the rule executor

use astgrep_core::AstNode;
use std::collections::HashMap;

/// Check if a node type/text is an operator
pub fn is_operator_node(node_type: &str, node_text: Option<&str>) -> bool {
    if matches!(
        node_type,
        "=" | "+="
            | "-="
            | "*="
            | "/="
            | "%="
            | "++"
            | "--"
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "&&"
            | "||"
            | "!"
            | "&"
            | "|"
            | "^"
            | "~"
            | "<<"
            | ">>"
            | ">>>"
            | "assignment_operator"
            | "operator"
    ) {
        return true;
    }

    if let Some(text) = node_text {
        if text.len() <= 3
            && matches!(
                text,
                "=" | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "+"
                    | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "=="
                    | "!="
                    | "<"
                    | ">"
                    | "<="
                    | ">="
                    | "&&"
                    | "||"
                    | "!"
                    | "&"
                    | "|"
                    | "^"
                    | "~"
                    | "<<"
                    | ">>"
                    | ">>>"
            )
        {
            return true;
        }
    }

    false
}

/// Represents a taint match (source or sink)
pub struct TaintMatch {
    pub node: Box<dyn AstNode>,
    pub bindings: HashMap<String, String>,
    /// Locations of metavariable bindings (var_name_without_$ -> (start_line, start_col, end_line, end_col))
    pub binding_locations: HashMap<String, (usize, usize, usize, usize)>,
    pub var_name: Option<String>,
    /// Method name containing this match (for scope isolation)
    pub method_name: Option<String>,
    /// Focus metavariables from the pattern (used to relocate finding to metavar position)
    pub focus_metavariables: Vec<String>,
}

impl Clone for TaintMatch {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone_node(),
            bindings: self.bindings.clone(),
            binding_locations: self.binding_locations.clone(),
            var_name: self.var_name.clone(),
            method_name: self.method_name.clone(),
            focus_metavariables: self.focus_metavariables.clone(),
        }
    }
}
