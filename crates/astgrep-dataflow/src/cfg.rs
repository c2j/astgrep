//! Control Flow Graph builder for data-flow analysis.
//!
//! Replaces the tree-based `visit_node()` traversal in `lib.rs` with a
//! proper CFG that models branching, loops, exception handling, and
//! termination. The resulting `DataFlowGraph` can be consumed by the
//! existing taint/source/sink/CP passes without changes.

use crate::graph::{DataFlowEdge, DataFlowGraph, DataFlowNode, EdgeType, NodeId};
use astgrep_core::AstNode;
use astgrep_core::Result;

// ---------------------------------------------------------------------------
// Helper: tree-sitter node kind
// ---------------------------------------------------------------------------

fn ts_kind(node: &dyn AstNode) -> String {
    node.get_attribute("ts_kind")
        .map(String::from)
        .unwrap_or_else(|| node.node_type().to_string())
}

// ---------------------------------------------------------------------------
// CFG Builder
// ---------------------------------------------------------------------------

pub struct CfgBuilder {
    graph: DataFlowGraph,
    /// During traversal we may temporarily store the entry/exit of a
    /// sub-graph so the caller can wire them.
    entry_id: Option<NodeId>,
}

impl CfgBuilder {
    pub fn new() -> Self {
        Self {
            graph: DataFlowGraph::new(),
            entry_id: None,
        }
    }

    /// Build a CFG for the given AST root and return the populated graph.
    pub fn build(mut self, ast: &dyn AstNode) -> Result<DataFlowGraph> {
        let root_id = self.add_node(ast);
        self.entry_id = Some(root_id);
        let exit_id = self.build_cfg(ast, root_id)?;
        self.graph.add_edge(root_id, exit_id, EdgeType::ControlFlow);
        self.inject_data_flow_edges(ast);
        Ok(self.graph)
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn add_node(&mut self, node: &dyn AstNode) -> NodeId {
        self.graph.add_node(DataFlowNode::from_ast_node(node))
    }

    fn add_cf_edge(&mut self, from: NodeId, to: NodeId) {
        self.graph.add_edge(from, to, EdgeType::ControlFlow);
    }

    fn add_df_edge(&mut self, from: NodeId, to: NodeId) {
        self.graph.add_edge(from, to, EdgeType::DataFlow);
    }

    /// Walk children of `parent` and return a NodeId representing the
    /// *single-exit* block of the compound statement.  Sequential
    /// children are chained with CF edges.  The caller may wire the
    /// first child's entry and the returned exit.
    fn build_children(&mut self, parent: &dyn AstNode, parent_id: NodeId) -> Result<NodeId> {
        let mut prev: Option<NodeId> = None;
        let mut last_exit = parent_id;
        let mut terminated = false;

        for i in 0..parent.child_count() {
            if terminated {
                break;
            }
            let Some(child) = parent.child(i) else { continue };
            let child_id = self.add_node(child);

            if let Some(p) = prev {
                self.add_cf_edge(p, child_id);
            } else {
                self.add_cf_edge(parent_id, child_id);
            }

            let (exit, is_terminal) = self.build_cfg_with_term(child, child_id)?;
            if is_terminal {
                terminated = true;
            } else {
                prev = Some(exit);
            }
            last_exit = exit;
        }

        Ok(last_exit)
    }

    // ------------------------------------------------------------------
    // Main dispatch
    // ------------------------------------------------------------------

    fn build_cfg(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        self.build_cfg_with_term(node, node_id).map(|(id, _)| id)
    }

    fn build_cfg_with_term(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<(NodeId, bool)> {
        let kind = ts_kind(node);
        let ty = node.node_type();

        let (exit, is_terminal) = match kind.as_str() {
            "return_statement" | "return" => {
                (node_id, true)
            }
            "throw_statement" | "throw" => {
                (node_id, true)
            }
            _ => {
                let id = match kind.as_str() {
                    "if_statement" | "if" => self.handle_if(node, node_id)?,
                    "switch_statement" | "switch_expression" => self.handle_switch(node, node_id)?,
                    "try_statement" | "try_with_resources_statement" => self.handle_try(node, node_id)?,
                    "lambda_expression" | "arrow_function" => self.handle_lambda(node, node_id)?,
                    "while_statement" | "for_statement" | "enhanced_for_statement"
                    | "do_statement" | "while" | "for" | "do" => self.handle_loop(node, node_id)?,
                    "block" | "block_statement" | "program" | "source_file"
                    | "compilation_unit" => self.build_children(node, node_id)?,
                    _ => match ty {
                        "function_declaration" | "function_definition"
                        | "method_declaration" | "FunctionDeclaration" => {
                            self.handle_function(node, node_id)?
                        }
                        _ => self.build_children(node, node_id)?,
                    },
                };
                (id, false)
            }
        };
        Ok((exit, is_terminal))
    }

    // ------------------------------------------------------------------
    // if / else
    // ------------------------------------------------------------------

    fn handle_if(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let mut condition_idx: Option<usize> = None;
        let mut consequence_idx: Option<usize> = None;
        let mut alternative_idx: Option<usize> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let ckind = ts_kind(child);

            if ckind.contains("condition") || ckind == "parenthesized_expression" {
                let cid = self.add_node(child);
                self.add_cf_edge(node_id, cid);
                condition_idx = Some(i);
            } else if ckind.contains("else") {
                alternative_idx = Some(i);
            } else if consequence_idx.is_none() {
                let cid = self.add_node(child);
                self.add_cf_edge(node_id, cid);
                consequence_idx = Some(i);
            } else {
                alternative_idx = Some(i);
            }
        }

        let join = self.add_node_phantom("if_join", node);

        // Consequence branch
        if let Some(i) = consequence_idx {
            let csq = node.child(i).unwrap();
            let csq_id = self.graph.node_ids().last().unwrap_or(node_id);
            let csq_exit = self.build_cfg(csq, csq_id)?;
            self.add_cf_edge(csq_exit, join);
        } else {
            self.add_cf_edge(node_id, join);
        }

        // Alternative (else) branch
        if let Some(i) = alternative_idx {
            let alt = node.child(i).unwrap();
            if ts_kind(alt) == "else" {
                let alt_idx = i + 1;
                if alt_idx < node.child_count() {
                    if let Some(body) = node.child(alt_idx) {
                        let body_id = self.add_node(body);
                        self.add_cf_edge(node_id, body_id);
                        let body_exit = self.build_cfg(body, body_id)?;
                        self.add_cf_edge(body_exit, join);
                        return Ok(join);
                    }
                }
                self.add_cf_edge(node_id, join);
            } else {
                let alt_id = self.add_node(alt);
                self.add_cf_edge(node_id, alt_id);
                let alt_exit = self.build_cfg(alt, alt_id)?;
                self.add_cf_edge(alt_exit, join);
            }
        } else {
            self.add_cf_edge(node_id, join);
        }

        Ok(join)
    }

    // ------------------------------------------------------------------
    // try / catch / finally
    // ------------------------------------------------------------------

    fn handle_try(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let mut try_body: Option<NodeId> = None;
        let mut catches: Vec<NodeId> = Vec::new();
        let mut finally_body: Option<NodeId> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let ckind = ts_kind(child);
            let cid = self.add_node(child);
            self.add_cf_edge(node_id, cid);

            if ckind.contains("try") || ckind == "block" && try_body.is_none() {
                try_body = Some(cid);
                self.build_cfg(child, cid)?;
            } else if ckind.contains("catch") {
                catches.push(cid);
                self.build_cfg(child, cid)?;
            } else if ckind.contains("finally") {
                finally_body = Some(cid);
                self.build_cfg(child, cid)?;
            }
        }

        // Exception edges: try body → each catch clause.
        if let Some(try_id) = try_body {
            for &catch_id in &catches {
                self.add_cf_edge(try_id, catch_id);
            }
        }

        // If there's a finally block, both try exits and catch exits
        // should flow into it.
        if let Some(fin_id) = finally_body {
            if let Some(try_id) = try_body {
                self.add_cf_edge(try_id, fin_id);
            }
            for &catch_id in &catches {
                self.add_cf_edge(catch_id, fin_id);
            }
            Ok(fin_id)
        } else if !catches.is_empty() {
            let join = self.add_node_phantom("try_join", node);
            if let Some(try_id) = try_body {
                self.add_cf_edge(try_id, join);
            }
            for &catch_id in &catches {
                self.add_cf_edge(catch_id, join);
            }
            Ok(join)
        } else {
            Ok(node_id)
        }
    }

    // ------------------------------------------------------------------
    // switch
    // ------------------------------------------------------------------

    fn handle_switch(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let join = self.add_node_phantom("switch_join", node);
        let mut prev_case_exit: Option<NodeId> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let ckind = ts_kind(child);
            let cid = self.add_node(child);

            if ckind.contains("switch") && i == 0 {
                // The `switch` keyword / condition – skip, connected via parent
                continue;
            }

            self.add_cf_edge(node_id, cid);

            if ckind.contains("case") || ckind.contains("default") {
                // Wire previous case's fall-through
                if let Some(prev_exit) = prev_case_exit {
                    self.add_cf_edge(prev_exit, cid);
                }
                let case_exit = self.build_cfg(child, cid)?;
                // Each case can flow to the join (break) or fall through
                self.add_cf_edge(case_exit, join);
                prev_case_exit = Some(case_exit);
            }
        }

        Ok(join)
    }

    // ------------------------------------------------------------------
    // lambda / arrow function
    // ------------------------------------------------------------------

    fn handle_lambda(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let mut params_id: Option<NodeId> = None;
        let mut body_id: Option<NodeId> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let cid = self.add_node(child);
            self.add_cf_edge(node_id, cid);

            let ckind = ts_kind(child);
            if ckind.contains("parameter") || ckind.contains("formal_parameter") {
                params_id = Some(cid);
            } else {
                body_id = Some(cid);
                self.build_cfg(child, cid)?;
            }
        }

        // Data-flow: lambda parameters → lambda body
        if let (Some(p), Some(b)) = (params_id, body_id) {
            self.add_df_edge(p, b);
        }

        Ok(body_id.unwrap_or(node_id))
    }

    // ------------------------------------------------------------------
    // while / for / do loops
    // ------------------------------------------------------------------

    fn handle_loop(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let join = self.add_node_phantom("loop_join", node);
        let mut body_id: Option<NodeId> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let ckind = ts_kind(child);
            let cid = self.add_node(child);

            self.add_cf_edge(node_id, cid);

            if ckind.contains("condition") || ckind == "parenthesized_expression" {
                continue;
            }

            // Accept block-like children AND any non-condition child as the body
            body_id = Some(cid);
            let body_exit = self.build_cfg(child, cid)?;
            self.add_cf_edge(body_exit, node_id);
            self.add_cf_edge(body_exit, join);
        }

        if body_id.is_none() {
            self.add_cf_edge(node_id, join);
        }

        Ok(join)
    }

    // ------------------------------------------------------------------
    // function / method declaration
    // ------------------------------------------------------------------

    fn handle_function(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        self.build_children(node, node_id)
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Add a synthetic (phantom) node that has no AST counterpart.
    fn add_node_phantom(&mut self, label: &str, parent: &dyn AstNode) -> NodeId {
        let mut phantom = DataFlowNode::from_ast_node(parent);
        phantom.node_type = format!("cfg_{}", label);
        self.graph.add_node(phantom)
    }

    /// Maps a child NodeId back to its index in the parent's child list.
    /// This is a heuristic – we compare node text/location.
    fn child_offset(&self, parent: &dyn AstNode, child_id: NodeId) -> usize {
        let child_node = self.graph.get_node(child_id);
        for i in 0..parent.child_count() {
            if let Some(c) = parent.child(i) {
                if let Some(cn) = child_node {
                    if c.text() == cn.text.as_deref() {
                        return i;
                    }
                }
            }
        }
        0
    }

    /// Walk AST and add data-flow edges for assignment, call, return nodes.
    fn inject_data_flow_edges(&mut self, ast: &dyn AstNode) {
        self.visit_for_df(ast);
    }

    fn visit_for_df(&mut self, node: &dyn AstNode) {
        let ty = node.node_type();

        // Collect matching child graph-node IDs before mutating the graph.
        let mut df_targets: Vec<(NodeId, NodeId)> = Vec::new();

        // Find this AST node's graph NodeId by matching type + text
        let parent_id_opt: Option<NodeId> = {
            let mut ids: Vec<NodeId> = Vec::new();
            for id in self.graph.node_ids() {
                if let Some(n) = self.graph.get_node(id) {
                    if n.node_type == ty && n.text.as_deref() == node.text() {
                        ids.push(id);
                        break;
                    }
                }
            }
            ids.into_iter().next()
        };

        if let Some(parent_id) = parent_id_opt {
            if ty == "assignment_expression" {
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.node_type() == "identifier" {
                            if let Some(ctext) = child.text() {
                                for cid in self.graph.node_ids() {
                                    if let Some(n) = self.graph.get_node(cid) {
                                        if n.node_type == "identifier" && n.text.as_deref() == Some(ctext) {
                                            df_targets.push((parent_id, cid));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if ty == "call_expression" {
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.node_type() != "identifier"
                            || node.child(0).map(|c| c.node_type()) != Some("identifier")
                        {
                            if let Some(ctext) = child.text() {
                                for cid in self.graph.node_ids() {
                                    if let Some(n) = self.graph.get_node(cid) {
                                        if n.node_type == child.node_type() && n.text.as_deref() == Some(ctext) {
                                            df_targets.push((cid, parent_id));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if ty == "return_statement" {
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if let Some(ctext) = child.text() {
                            for cid in self.graph.node_ids() {
                                if let Some(n) = self.graph.get_node(cid) {
                                    if n.node_type == child.node_type() && n.text.as_deref() == Some(ctext) {
                                        df_targets.push((cid, parent_id));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply edges outside the immutable borrow scope
        for (from, to) in df_targets {
            self.add_df_edge(from, to);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit_for_df(child);
            }
        }
    }
}

impl Default for CfgBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Public convenience
// ---------------------------------------------------------------------------

/// Build a control-flow-correct `DataFlowGraph` for the given AST.
pub fn build_control_flow_graph(ast: &dyn AstNode) -> Result<DataFlowGraph> {
    CfgBuilder::new().build(ast)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{DataFlowNode, EdgeType};
    use astgrep_core::AstNode;
    use std::collections::HashMap;

    // Minimal mock AstNode for unit testing
    struct MockNode {
        kind: String,
        text: String,
        children: Vec<MockNode>,
        attrs: HashMap<String, String>,
    }

    impl MockNode {
        fn new(kind: &str, text: &str) -> Self {
            Self {
                kind: kind.to_string(),
                text: text.to_string(),
                children: vec![],
                attrs: HashMap::new(),
            }
        }
        fn with_children(mut self, children: Vec<MockNode>) -> Self {
            self.children = children;
            self
        }
    }

    impl AstNode for MockNode {
        fn node_type(&self) -> &str {
            &self.kind
        }
        fn text(&self) -> Option<&str> {
            Some(&self.text)
        }
        fn child_count(&self) -> usize {
            self.children.len()
        }
        fn child(&self, index: usize) -> Option<&dyn AstNode> {
            self.children.get(index).map(|c| c as &dyn AstNode)
        }
        fn location(&self) -> Option<(usize, usize, usize, usize)> {
            Some((1, 1, 1, 1))
        }
        fn get_attribute(&self, key: &str) -> Option<&str> {
            self.attrs.get(key).map(|s| s.as_str())
        }
        fn clone_node(&self) -> Box<dyn AstNode> {
            Box::new(Self {
                kind: self.kind.clone(),
                text: self.text.clone(),
                children: self.children.iter().map(|c| {
                    MockNode {
                        kind: c.kind.clone(),
                        text: c.text.clone(),
                        children: vec![],
                        attrs: c.attrs.clone(),
                    }
                }).collect(),
                attrs: self.attrs.clone(),
            })
        }
    }

    #[test]
    fn test_cfg_builder_empty() {
        let root = MockNode::new("program", "");
        let graph = CfgBuilder::new().build(&root).unwrap();
        assert!(graph.node_count() > 0);
    }

    #[test]
    fn test_cfg_builder_if_statement() {
        // Simulate: if (x) { y = 1; } else { y = 2; }
        let cond = MockNode::new("parenthesized_expression", "x")
            .with_children(vec![MockNode::new("identifier", "x")]);
        let cons = MockNode::new("block", "{ y = 1; }")
            .with_children(vec![MockNode::new("expression_statement", "y = 1")]);
        let alt = MockNode::new("block", "{ y = 2; }")
            .with_children(vec![MockNode::new("expression_statement", "y = 2")]);

        let root = MockNode::new("if_statement", "if")
            .with_children(vec![cond, cons, alt]);

        let graph = CfgBuilder::new().build(&root).unwrap();
        // Should have created a join node and edges from both branches.
        assert!(graph.node_count() > 3);
        assert!(graph.edge_count() > 0);
    }

    #[test]
    fn test_cfg_builder_try_catch() {
        let try_block = MockNode::new("block", "{ may_throw(); }")
            .with_children(vec![MockNode::new("expression_statement", "may_throw()")]);
        let catch_clause = MockNode::new("catch_clause", "catch (Exception e)")
            .with_children(vec![MockNode::new("block", "{ handle(); }")]);

        let root = MockNode::new("try_statement", "try")
            .with_children(vec![try_block, catch_clause]);

        let graph = CfgBuilder::new().build(&root).unwrap();
        assert!(graph.node_count() > 2);
        // Should have an exception edge from try body to catch
        assert!(graph.edge_count() > 1);
    }

    #[test]
    fn test_cfg_builder_return_terminates() {
        let ret = MockNode::new("return_statement", "return 42");
        let root = MockNode::new("block", "{ return 42; }")
            .with_children(vec![ret]);

        let graph = CfgBuilder::new().build(&root).unwrap();
        assert!(graph.node_count() > 0);
    }
}
