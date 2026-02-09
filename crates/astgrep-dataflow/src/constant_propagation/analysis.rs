//! Analysis algorithms for constant propagation
//!
//! This module contains the core analysis algorithms and AST traversal
//! logic for constant propagation.

use crate::graph::{DataFlowGraph, NodeId};
use crate::constant_propagation::state::{ConstantPropagator, ConstantValue, SourceLocation, Scope, VariableDefinition, VisitContext};
use crate::constant_propagation::utils::{
    get_node_location,
    is_static_block_context,
    is_constructor_declaration,
    is_method_declaration,
    extract_variable_name_from_assignment_target,
    extract_constant_from_expression,
};
use astgrep_core::AstNode;
use astgrep_core::Result;
use std::collections::HashMap;

// The ConstantPropagator struct is defined in state.rs
// This module adds impl methods for analysis algorithms

impl ConstantPropagator {
    /// Analyze constants in the data flow graph
    pub fn analyze(&mut self, graph: &DataFlowGraph, symbol_table: &crate::symbol_table::SymbolTable) -> Result<()> {
        // First pass: collect all constant assignments
        self.collect_constants(graph, symbol_table)?;

        // Second pass: propagate constants through the graph
        self.propagate_constants(graph)?;

        Ok(())
    }

    /// Collect constant assignments from the graph
    fn collect_constants(&mut self, graph: &DataFlowGraph, symbol_table: &crate::symbol_table::SymbolTable) -> Result<()> {
        for node_id in graph.get_all_nodes() {
            if let Some(node) = graph.get_node(node_id) {
                // Check if this is a constant assignment
                if let Some(constant) = self.extract_constant_from_node(node) {
                    // Get the variable name from the node
                    if let Some(var_name) = self.get_variable_name_from_node(node) {
                        // Check if variable is reassigned
                        if !self.reassigned.contains(&var_name) {
                            self.constants.insert(var_name, constant.clone());
                            self.node_constants.insert(node_id, constant);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Propagate constants through the graph
    fn propagate_constants(&mut self, graph: &DataFlowGraph) -> Result<()> {
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for node_id in graph.get_all_nodes() {
                // Get predecessors in the data flow graph
                let predecessors = graph.data_flow_predecessors(node_id);

                for pred_id in predecessors {
                    if let Some(pred_constant) = self.node_constants.get(&pred_id).cloned() {
                        if !self.node_constants.contains_key(&node_id) {
                            self.node_constants.insert(node_id, pred_constant);
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract constant value from a node
    fn extract_constant_from_node(&self, node: &dyn std::any::Any) -> Option<ConstantValue> {
        // This is a placeholder - in real implementation, we would parse the node
        // to extract string literals, integer literals, etc.
        None
    }

    /// Get variable name from a node
    fn get_variable_name_from_node(&self, node: &dyn std::any::Any) -> Option<String> {
        // This is a placeholder - in real implementation, we would extract
        // the variable name from assignment nodes
        None
    }

    /// Analyze AST directly to extract constants
    /// This is a simplified version that works without a full symbol table
    pub fn analyze_ast(&mut self, ast: &dyn AstNode) -> crate::Result<HashMap<String, ConstantValue>> {
        self.constants.clear();
        self.node_constants.clear();
        self.reassigned.clear();
        self.current_class_name = None;
        self.constructor_count = 0;
        self.fields_in_constructors.clear();
        self.scope_stack.clear();
        self.variable_definitions.clear();
        self.location_based_constants.clear();

        // Walk the AST to find field declarations with constant initializers
        self.visit_node_for_constants(ast)?;

        // Post-processing: if there are multiple constructors, invalidate fields
        // that are not initialized in ALL constructors
        if self.constructor_count > 1 {
            eprintln!("DEBUG CP: Found {} constructors, checking field initialization consistency", self.constructor_count);
            let mut fields_to_remove = Vec::new();

            for (field, count) in &self.fields_in_constructors {
                if *count < self.constructor_count {
                    eprintln!("DEBUG CP: Field {} initialized in only {} of {} constructors, removing from constants",
                             field, count, self.constructor_count);
                    fields_to_remove.push(field.clone());
                }
            }

            for field in fields_to_remove {
                self.constants.remove(&field);
            }
        }

        Ok(self.constants.clone())
    }

    /// Visit AST node to extract constant field declarations
    fn visit_node_for_constants(
        &mut self,
        node: &dyn AstNode,
    ) -> crate::Result<()> {
        self.visit_node_with_context(node, VisitContext::TopLevel)
    }

    /// Visit AST node with context tracking
    fn visit_node_with_context(
        &mut self,
        node: &dyn AstNode,
        context: VisitContext,
    ) -> crate::Result<()> {
        // Debug: print node type and child count
        eprintln!(
            "DEBUG CP: Visiting node type: {}, child_count: {}, context: {:?}",
            node.node_type(),
            node.child_count(),
            context
        );

        // Check if this is a field declaration or variable declaration with an initializer
        let is_field_or_var = node.node_type() == "field_declaration"
            || node.node_type() == "variable_declaration"
            || node.node_type() == "declaration_statement";

        if is_field_or_var {
            eprintln!("DEBUG CP: Found potential field/variable declaration: {}", node.node_type());

            // Print children for debugging
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    eprintln!("  Child {}: {} - text: {:?}", i, child.node_type(), child.text());
                }
            }

            // For tree-sitter AST, look for variable_declaration children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.node_type() == "variable_declaration" {
                        // Check if it has private modifier by looking at the text
                        let is_private = node.text().map(|t| t.contains("private")).unwrap_or(false);
                        let is_static = node.text().map(|t| t.contains("static")).unwrap_or(false);

                        if is_private {
                            eprintln!("DEBUG CP: Found private variable declaration (static: {})", is_static);

                            // Find the identifier and initializer in the variable_declaration
                            let mut var_name = None;
                            let mut init_value = None;

                            for j in 0..child.child_count() {
                                if let Some(grandchild) = child.child(j) {
                                    eprintln!("    Grandchild {}: {} - text: {:?}", j, grandchild.node_type(), grandchild.text());

                                    if grandchild.node_type() == "identifier" {
                                        var_name = grandchild.text().map(|t| t.to_string());
                                    }

                                    // Look for literal (tree-sitter uses "literal" for numbers)
                                    if grandchild.node_type() == "literal" || grandchild.node_type() == "decimal_integer_literal" {
                                        init_value = grandchild.text().and_then(|t| t.parse::<i64>().ok());
                                    }
                                }
                            }

                            if let (Some(name), Some(value)) = (var_name, init_value) {
                                eprintln!("DEBUG CP: Found constant: {} = {} (direct init)", name, value);
                                self.constants.insert(name, ConstantValue::Integer(value));
                            }
                        }

                        // Handle local variable declarations in methods
                        if context == VisitContext::Method {
                            eprintln!("DEBUG CP: Processing local variable declaration in method");

                            // Find the identifier and initializer in the variable_declaration
                            let mut var_name = None;
                            let mut init_value = None;
                            let location = get_node_location(node);

                            for j in 0..child.child_count() {
                                if let Some(grandchild) = child.child(j) {
                                    eprintln!("    Grandchild {}: {} - text: {:?}", j, grandchild.node_type(), grandchild.text());

                                    if grandchild.node_type() == "identifier" {
                                        var_name = grandchild.text().map(|t| t.to_string());
                                    }

                                    // Look for literal (tree-sitter uses "literal" for numbers)
                                    if grandchild.node_type() == "literal" || grandchild.node_type() == "decimal_integer_literal" {
                                        init_value = grandchild.text().and_then(|t| t.parse::<i64>().ok());
                                    }
                                }
                            }

                            if let (Some(name), Some(value)) = (var_name, init_value) {
                                eprintln!("DEBUG CP: Found local constant: {} = {} at {:?}", name, value, location);
                                if let Some(loc) = location {
                                    self.define_local_variable(name, ConstantValue::Integer(value), loc);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle assignment expressions
        if node.node_type() == "assignment_expression" {
            match context {
                VisitContext::Constructor | VisitContext::StaticBlock => {
                    // Record constant assignments in constructors and static blocks
                    self.process_assignment_expression(node, context)?;
                }
                VisitContext::Method => {
                    // In methods, check if this is a reassignment of a constant field
                    self.check_reassignment_in_method(node)?;

                    // Also process local variable assignments
                    self.process_local_assignment(node)?;
                }
                _ => {
                    eprintln!("DEBUG CP: Skipping assignment in {:?} context", context);
                }
            }
        }

        // Track class name when entering class declarations
        if node.node_type() == "class_declaration" {
            // Reset per-class tracking for new class
            self.constructor_count = 0;
            self.fields_in_constructors.clear();

            // Try to find the class name
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.node_type() == "identifier" {
                        if let Some(name) = child.text() {
                            self.current_class_name = Some(name.to_string());
                            eprintln!("DEBUG CP: Found class name: {}", name);
                        }
                    }
                }
            }
        }

        // Determine context for children
        let is_constructor = node.node_type() == "constructor_declaration"
            || is_constructor_declaration(node, self.current_class_name.as_deref());

        let is_method = node.node_type() == "method_declaration"
            || node.node_type() == "function_definition"
            || node.node_type() == "function_declaration"
            || is_method_declaration(node);

        // Track constructor count
        if is_constructor {
            self.constructor_count += 1;
            eprintln!("DEBUG CP: Found constructor #{} for class {:?}",
                     self.constructor_count, self.current_class_name);
        }

        let child_context = if is_constructor {
            VisitContext::Constructor
        } else if is_method {
            VisitContext::Method
        } else if is_static_block_context(node) {
            VisitContext::StaticBlock
        } else {
            context
        };

        // Push new scope when entering a method
        if is_method {
            let location = get_node_location(node).unwrap_or(SourceLocation::new(0, 0));
            self.push_scope(location);
            eprintln!("DEBUG CP: Pushed new scope for method at {:?}", location);
        }

        // Recursively visit children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.visit_node_with_context(child, child_context)?;
            }
        }

        // Pop scope when exiting a method
        if is_method {
            self.pop_scope();
            eprintln!("DEBUG CP: Popped scope for method");
        }

        Ok(())
    }

    /// Process local variable assignment in methods
    fn process_local_assignment(
        &mut self,
        node: &dyn AstNode,
    ) -> crate::Result<()> {
        eprintln!("DEBUG CP: Processing local assignment in method");

        // assignment_expression typically has 3 children: left, operator, right
        if node.child_count() < 3 {
            return Ok(());
        }

        let left = node.child(0);
        let operator = node.child(1);
        let right = node.child(2);

        // Check if operator is '=' (simple assignment)
        let operator_text = operator
            .as_ref()
            .and_then(|op| op.text());

        let is_simple_assignment = match operator_text {
            Some(text) => text.trim() == "=",
            None => true,
        };

        if !is_simple_assignment {
            return Ok(());
        }

        // Extract variable name from left side
        let var_name = if let Some(left_node) = left {
            extract_variable_name_from_assignment_target(left_node)
        } else {
            None
        };

        // Extract constant value from right side
        let const_value = if let Some(right_node) = right {
            extract_constant_from_expression(right_node, &self.constants)
        } else {
            None
        };

        if let (Some(name), Some(value)) = (var_name, const_value) {
            if let Some(location) = get_node_location(node) {
                eprintln!("DEBUG CP: Found local assignment: {} = {:?} at {:?}", name, value, location);
                self.define_local_variable(name, value, location);
            }
        }

        Ok(())
    }

    /// Process assignment expression to extract constant assignments
    /// Handles patterns like:
    /// - this.field = value (constructor assignments)
    /// - field = value (static block assignments)
    fn process_assignment_expression(
        &mut self,
        node: &dyn AstNode,
        context: VisitContext,
    ) -> crate::Result<()> {
        eprintln!("DEBUG CP: Processing assignment expression");

        // Print children for debugging
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                eprintln!("  Assignment child {}: {} - text: {:?}", i, child.node_type(), child.text());
            }
        }

        // assignment_expression typically has 3 children: left, operator, right
        if node.child_count() < 3 {
            eprintln!("DEBUG CP: Assignment has fewer than 3 children, skipping");
            return Ok(());
        }

        let left = node.child(0);
        let operator = node.child(1);
        let right = node.child(2);

        // Check if operator is '=' (simple assignment, not +=, -=, etc.)
        // The operator may be "unknown" type in tree-sitter, so check the text
        let operator_text = operator
            .as_ref()
            .and_then(|op| op.text());

        eprintln!("DEBUG CP: Operator text: {:?}", operator_text);

        let is_simple_assignment = match operator_text {
            Some(text) => text.trim() == "=",
            None => true, // If we can't determine operator, assume it's simple assignment
        };

        if !is_simple_assignment {
            eprintln!("DEBUG CP: Not a simple assignment operator: {:?}", operator_text);
            return Ok(());
        }

        // Extract variable name from left side
        let var_name = if let Some(left_node) = left {
            extract_variable_name_from_assignment_target(left_node)
        } else {
            None
        };

        eprintln!("DEBUG CP: Extracted variable name: {:?}", var_name);

        // Extract constant value from right side
        let const_value = if let Some(right_node) = right {
            extract_constant_from_expression(right_node, &self.constants)
        } else {
            None
        };

        eprintln!("DEBUG CP: Extracted constant value: {:?}", const_value);

        match (&var_name, &const_value) {
            (Some(name), Some(value)) => {
                eprintln!("DEBUG CP: Found assignment constant: {} = {:?}", name, value);

                // Check if this variable was already assigned
                if self.constants.contains_key(name) {
                    // Variable is being reassigned - mark as non-constant
                    eprintln!("DEBUG CP: Variable {} is reassigned, marking as non-constant", name);
                    self.mark_reassigned(name.clone());
                } else if context == VisitContext::Constructor || context == VisitContext::StaticBlock {
                    // Only record as constant in constructor or static block contexts
                    eprintln!("DEBUG CP: Recording constant: {} = {:?}", name, value);
                    self.constants.insert(name.clone(), value.clone());

                    // Track field initialization in constructors
                    if context == VisitContext::Constructor {
                        *self.fields_in_constructors.entry(name.clone()).or_insert(0) += 1;
                        eprintln!("DEBUG CP: Field {} initialized in constructor (count: {})",
                                 name, self.fields_in_constructors.get(name).unwrap_or(&0));
                    }
                } else {
                    eprintln!("DEBUG CP: Not recording constant in {:?} context: {}", context, name);
                }
            }
            _ => {
                eprintln!("DEBUG CP: Could not extract name or value: name={:?}, value={:?}", var_name, const_value);
            }
        }

        Ok(())
    }

    /// Check if a method assignment reassigns a constant field
    /// If a field was initialized as a constant in the constructor,
    /// but is reassigned in a method, it is no longer a constant
    fn check_reassignment_in_method(
        &mut self,
        node: &dyn AstNode,
    ) -> crate::Result<()> {
        eprintln!("DEBUG CP: Checking reassignment in method");

        // Extract variable name from left side of assignment
        if node.child_count() < 1 {
            return Ok(());
        }

        let left = node.child(0);
        let var_name = if let Some(left_node) = left {
            extract_variable_name_from_assignment_target(left_node)
        } else {
            None
        };

        if let Some(name) = var_name {
            // Check if this variable is currently tracked as a constant
            if self.constants.contains_key(&name) {
                eprintln!("DEBUG CP: Variable {} is reassigned in method, marking as non-constant", name);
                self.mark_reassigned(name);
            }
        }

        Ok(())
    }

    /// Get the constant value for a variable at a specific location
    /// This is the key method for supporting metavariable-comparison
    pub fn get_variable_value_at_location(&self, var_name: &str, location: SourceLocation) -> Option<&ConstantValue> {
        // First check location-based constants (exact match)
        if let Some(value) = self.location_based_constants.get(&(var_name.to_string(), location)) {
            return Some(value);
        }

        // Find the most recent definition that precedes this location
        let mut best_match: Option<&VariableDefinition> = None;

        for def in &self.variable_definitions {
            if def.name == var_name {
                // Check if this definition precedes the given location
                if def.location.line < location.line
                   || (def.location.line == location.line && def.location.column <= location.column) {
                    // This is a candidate - check if it's the best match
                    if let Some(current_best) = best_match {
                        if def.location.line > current_best.location.line
                           || (def.location.line == current_best.location.line && def.location.column > current_best.location.column) {
                            best_match = Some(def);
                        }
                    } else {
                        best_match = Some(def);
                    }
                }
            }
        }

        best_match.map(|def| &def.value)
    }
}
