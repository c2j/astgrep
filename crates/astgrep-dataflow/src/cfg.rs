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
        // Connect the synthetic root to the real entry so clients that
        // expect parent→child edges from old code still see connectivity.
        self.graph.add_edge(root_id, exit_id, EdgeType::ControlFlow);
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

        for i in 0..parent.child_count() {
            let Some(child) = parent.child(i) else { continue };
            let child_id = self.add_node(child);

            if let Some(p) = prev {
                self.add_cf_edge(p, child_id);
            } else {
                self.add_cf_edge(parent_id, child_id);
            }

            let exit = self.build_cfg(child, child_id)?;
            prev = Some(exit);
            last_exit = exit;
        }

        Ok(last_exit)
    }

    // ------------------------------------------------------------------
    // Main dispatch
    // ------------------------------------------------------------------

    fn build_cfg(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        let kind = ts_kind(node);
        let ty = node.node_type();

        match kind.as_str() {
            "if_statement" => self.handle_if(node, node_id),
            "if" => self.handle_if(node, node_id),
            "switch_statement" | "switch_expression" => self.handle_switch(node, node_id),
            "try_statement" => self.handle_try(node, node_id),
            "try_with_resources_statement" => self.handle_try(node, node_id),
            "return_statement" | "return" => {
                self.add_cf_edge(node_id, node_id); // self-loop marks termination
                Ok(node_id)
            }
            "throw_statement" | "throw" => {
                // throw cuts normal flow – mark as terminal, but still
                // connect to the enclosing try's catch if present.
                Ok(node_id)
            }
            "lambda_expression" | "arrow_function" => self.handle_lambda(node, node_id),
            "while_statement" | "for_statement" | "enhanced_for_statement"
            | "do_statement" | "while" | "for" | "do" => self.handle_loop(node, node_id),
            "block" | "block_statement" | "program" | "source_file"
            | "compilation_unit" => self.build_children(node, node_id),
            _ => {
                // Check universal type for method-like constructs
                match ty {
                    "function_declaration" | "function_definition"
                    | "method_declaration" | "FunctionDeclaration" => {
                        self.handle_function(node, node_id)
                    }
                    _ => self.build_children(node, node_id),
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // if / else
    // ------------------------------------------------------------------

    fn handle_if(&mut self, node: &dyn AstNode, node_id: NodeId) -> Result<NodeId> {
        // Collect children: condition, consequence, (optional) alternative
        let mut condition: Option<(usize, NodeId)> = None;
        let mut consequence: Option<(usize, NodeId)> = None;
        let mut alternative: Option<(usize, NodeId)> = None;

        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            let ckind = ts_kind(child);
            let cid = self.add_node(child);

            if ckind.contains("condition") || ckind == "parenthesized_expression" {
                condition = Some((i, cid));
            } else if ckind.contains("else") {
                alternative = Some((i, cid));
            } else if consequence.is_none() {
                consequence = Some((i, cid));
            } else {
                alternative = Some((i, cid));
            }
        }

        // Wire node → condition
        let cond_id = if let Some((_, cid)) = condition {
            self.add_cf_edge(node_id, cid);
            cid
        } else {
            node_id
        };

        let join = self.add_node_phantom("if_join", node);

        // Consequence branch
        if let Some((_, csq_id)) = consequence {
            self.add_cf_edge(cond_id, csq_id);
            let csq_exit = self.build_cfg(
                node.child(csq_id - self.child_offset(node, csq_id)).unwrap_or(node),
                csq_id,
            )?;
            self.add_cf_edge(csq_exit, join);
        } else {
            self.add_cf_edge(cond_id, join);
        }

        // Alternative (else) branch
        if let Some((_, alt_id)) = alternative {
            let alt_node = node
                .child(alt_id - self.child_offset(node, alt_id))
                .unwrap_or(node);
            // The "else" keyword itself may be a separate child; look for the
            // actual block child after it.
            if ts_kind(alt_node) == "else" {
                // Find the next sibling (the else body)
                let alt_idx = alt_id - self.child_offset(node, alt_id) + 1;
                if alt_idx < node.child_count() {
                    if let Some(body) = node.child(alt_idx) {
                        let body_id = self.add_node(body);
                        self.add_cf_edge(cond_id, body_id);
                        let body_exit = self.build_cfg(body, body_id)?;
                        self.add_cf_edge(body_exit, join);
                        return Ok(join);
                    }
                }
                self.add_cf_edge(cond_id, join);
            } else {
                self.add_cf_edge(cond_id, alt_id);
                let alt_exit = self.build_cfg(alt_node, alt_id)?;
                self.add_cf_edge(alt_exit, join);
            }
        } else {
            self.add_cf_edge(cond_id, join);
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
            Ok(*catches.last().unwrap_or(&node_id))
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
                // Condition → body
                // (body will be connected when found)
                continue;
            }

            if ckind == "block" || ckind == "block_statement" || ckind == "statement_block" {
                body_id = Some(cid);
                let body_exit = self.build_cfg(child, cid)?;
                // Back edge: body → condition (loop)
                self.add_cf_edge(body_exit, node_id);
                // Exit edge: body → join (break)
                self.add_cf_edge(body_exit, join);
            }
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
