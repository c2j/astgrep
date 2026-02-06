//! Constant propagation analysis for data flow
//!
//! This module implements constant propagation to track constant values
//! through the program and enable more precise taint analysis.

use crate::graph::{DataFlowGraph, NodeId};
use crate::symbol_table::SymbolTable;
use astgrep_core::Result;
use std::collections::{HashMap, HashSet};

/// Source location for a variable definition or use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Represents a scope for variable tracking
#[derive(Debug, Clone)]
struct Scope {
    /// Variables defined in this scope
    variables: HashMap<String, ConstantValue>,
    /// Source location where scope starts
    start_location: SourceLocation,
    /// Source location where scope ends
    end_location: SourceLocation,
}

impl Scope {
    fn new(start: SourceLocation) -> Self {
        Self {
            variables: HashMap::new(),
            start_location: start,
            end_location: start,
        }
    }

    fn define_variable(&mut self, name: String, value: ConstantValue) {
        self.variables.insert(name, value);
    }

    fn get_variable(&self, name: &str) -> Option<&ConstantValue> {
        self.variables.get(name)
    }

    fn update_location(&mut self, location: SourceLocation) {
        if location.line > self.end_location.line ||
           (location.line == self.end_location.line && location.column > self.end_location.column) {
            self.end_location = location;
        }
    }
}

/// Variable definition with location information
#[derive(Debug, Clone)]
struct VariableDefinition {
    pub name: String,
    pub value: ConstantValue,
    pub location: SourceLocation,
    pub scope_depth: usize,
}

/// Represents a constant value in the program
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConstantValue {
    /// String constant
    String(String),
    /// Integer constant
    Integer(i64),
    /// Boolean constant
    Boolean(bool),
    /// Null constant
    Null,
    /// Unknown constant
    Unknown,
}

impl ConstantValue {
    /// Check if this constant matches a pattern
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        match self {
            ConstantValue::String(s) => s.contains(pattern),
            ConstantValue::Integer(i) => i.to_string().contains(pattern),
            ConstantValue::Boolean(b) => b.to_string().contains(pattern),
            _ => false,
        }
    }

    /// Convert to string representation
    pub fn to_string_value(&self) -> Option<String> {
        match self {
            ConstantValue::String(s) => Some(s.clone()),
            ConstantValue::Integer(i) => Some(i.to_string()),
            ConstantValue::Boolean(b) => Some(b.to_string()),
            ConstantValue::Null => Some("null".to_string()),
            ConstantValue::Unknown => None,
        }
    }
}

/// Constant propagation analyzer
pub struct ConstantPropagator {
    /// Map from variable name to constant value
    constants: HashMap<String, ConstantValue>,
    /// Map from node ID to constant value
    node_constants: HashMap<NodeId, ConstantValue>,
    /// Set of variables that are reassigned (not constant)
    reassigned: HashSet<String>,
    /// Current class name for detecting constructors
    current_class_name: Option<String>,
    /// Number of constructors in current class
    constructor_count: usize,
    /// Fields initialized in constructors (to detect partial initialization)
    fields_in_constructors: HashMap<String, usize>,
    /// Scope stack for local variable tracking
    scope_stack: Vec<Scope>,
    /// All variable definitions with location information
    variable_definitions: Vec<VariableDefinition>,
    /// Map from (variable_name, location) to constant value
    /// Used for efficient lookup of variable values at specific locations
    location_based_constants: HashMap<(String, SourceLocation), ConstantValue>,
}

/// Context for tracking where we are in the AST
#[derive(Debug, Clone, Copy, PartialEq)]
enum VisitContext {
    TopLevel,
    StaticBlock,
    Constructor,
    Method,
    Other,
}

impl ConstantPropagator {
    /// Create a new constant propagator
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            node_constants: HashMap::new(),
            reassigned: HashSet::new(),
            current_class_name: None,
            constructor_count: 0,
            fields_in_constructors: HashMap::new(),
            scope_stack: Vec::new(),
            variable_definitions: Vec::new(),
            location_based_constants: HashMap::new(),
        }
    }

    /// Get the source location of a node
    fn get_node_location(node: &dyn astgrep_core::AstNode) -> Option<SourceLocation> {
        node.location().map(|(start_line, start_col, _, _)| {
            SourceLocation::new(start_line, start_col)
        })
    }

    /// Push a new scope onto the stack
    fn push_scope(&mut self, location: SourceLocation) {
        self.scope_stack.push(Scope::new(location));
    }

    /// Pop the current scope from the stack
    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Define a local variable in the current scope
    fn define_local_variable(&mut self, name: String, value: ConstantValue, location: SourceLocation) {
        // Record the definition
        let def = VariableDefinition {
            name: name.clone(),
            value: value.clone(),
            location,
            scope_depth: self.scope_stack.len(),
        };
        self.variable_definitions.push(def);

        // Store location-based constant for efficient lookup
        self.location_based_constants.insert((name.clone(), location), value.clone());

        // Also define in current scope if we have one
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.define_variable(name, value);
        }
    }

    /// Look up a variable in the current scope chain
    fn lookup_variable(&self, name: &str) -> Option<&ConstantValue> {
        // Search from innermost to outermost scope
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.get_variable(name) {
                return Some(value);
            }
        }
        None
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
                if def.location.line < location.line ||
                   (def.location.line == location.line && def.location.column <= location.column) {
                    // This is a candidate - check if it's the best match
                    if let Some(current_best) = best_match {
                        if def.location.line > current_best.location.line ||
                           (def.location.line == current_best.location.line && def.location.column > current_best.location.column) {
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

    /// Update scope end location
    fn update_scope_location(&mut self, location: SourceLocation) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.update_location(location);
        }
    }

    /// Analyze constants in the data flow graph
    pub fn analyze(&mut self, graph: &DataFlowGraph, symbol_table: &SymbolTable) -> Result<()> {
        // First pass: collect all constant assignments
        self.collect_constants(graph, symbol_table)?;

        // Second pass: propagate constants through the graph
        self.propagate_constants(graph)?;

        Ok(())
    }

    /// Collect constant assignments from the graph
    fn collect_constants(&mut self, graph: &DataFlowGraph, symbol_table: &SymbolTable) -> Result<()> {
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

    /// Get constant value for a variable
    pub fn get_constant(&self, var_name: &str) -> Option<&ConstantValue> {
        self.constants.get(var_name)
    }

    /// Get constant value for a node
    pub fn get_node_constant(&self, node_id: NodeId) -> Option<&ConstantValue> {
        self.node_constants.get(&node_id)
    }

    /// Analyze AST directly to extract constants
    /// This is a simplified version that works without a full symbol table
    pub fn analyze_ast(&mut self, ast: &dyn astgrep_core::AstNode) -> crate::Result<HashMap<String, ConstantValue>> {
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
        node: &dyn astgrep_core::AstNode,
    ) -> crate::Result<()> {
        self.visit_node_with_context(node, VisitContext::TopLevel)
    }

    /// Visit AST node with context tracking
    fn visit_node_with_context(
        &mut self,
        node: &dyn astgrep_core::AstNode,
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
                            let location = Self::get_node_location(node);
                            
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
            || Self::is_constructor_declaration(node, self.current_class_name.as_deref());
        
        let is_method = node.node_type() == "method_declaration"
            || node.node_type() == "function_definition"
            || node.node_type() == "function_declaration"
            || Self::is_method_declaration(node);
        
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
        } else if Self::is_static_block_context(node) {
            VisitContext::StaticBlock
        } else {
            context
        };
        
        // Push new scope when entering a method
        if is_method {
            let location = Self::get_node_location(node).unwrap_or(SourceLocation::new(0, 0));
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
        node: &dyn astgrep_core::AstNode,
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
            self.extract_variable_name_from_assignment_target(left_node)
        } else {
            None
        };
        
        // Extract constant value from right side
        let const_value = if let Some(right_node) = right {
            self.extract_constant_from_expression(right_node)
        } else {
            None
        };
        
        if let (Some(name), Some(value)) = (var_name, const_value) {
            if let Some(location) = Self::get_node_location(node) {
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
        node: &dyn astgrep_core::AstNode,
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
            self.extract_variable_name_from_assignment_target(left_node)
        } else {
            None
        };

        eprintln!("DEBUG CP: Extracted variable name: {:?}", var_name);

        // Extract constant value from right side
        let const_value = if let Some(right_node) = right {
            self.extract_constant_from_expression(right_node)
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
        node: &dyn astgrep_core::AstNode,
    ) -> crate::Result<()> {
        eprintln!("DEBUG CP: Checking reassignment in method");

        // Extract variable name from left side of assignment
        if node.child_count() < 1 {
            return Ok(());
        }

        let left = node.child(0);
        let var_name = if let Some(left_node) = left {
            self.extract_variable_name_from_assignment_target(left_node)
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

    /// Extract variable name from assignment target
    /// Handles: identifier, field_access (this.field), etc.
    fn extract_variable_name_from_assignment_target(
        &self,
        node: &dyn astgrep_core::AstNode,
    ) -> Option<String> {
        match node.node_type() {
            "identifier" => {
                // Direct variable: x = ...
                node.text().map(|t| t.to_string())
            }
            "field_access" => {
                // Field access: this.field = ... or obj.field = ...
                // Extract the field name (last identifier child)
                let mut field_name = None;
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.node_type() == "identifier" {
                            field_name = child.text().map(|t| t.to_string());
                        }
                    }
                }
                field_name
            }
            _ => {
                // Try to find identifier in other node types
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if let Some(name) = self.extract_variable_name_from_assignment_target(child) {
                            return Some(name);
                        }
                    }
                }
                None
            }
        }
    }

    /// Extract constant value from expression node
    fn extract_constant_from_expression(
        &self,
        node: &dyn astgrep_core::AstNode,
    ) -> Option<ConstantValue> {
        match node.node_type() {
            "literal" | "decimal_integer_literal" | "integer_literal" => {
                // Integer literal
                node.text()
                    .and_then(|t| t.parse::<i64>().ok())
                    .map(ConstantValue::Integer)
            }
            "string_literal" | "literal_string" => {
                // String literal
                node.text()
                    .map(|t| t.trim_matches('"').to_string())
                    .map(ConstantValue::String)
            }
            "true" | "false" => {
                // Boolean literal
                node.text()
                    .map(|t| t == "true")
                    .map(ConstantValue::Boolean)
            }
            "null_literal" | "null" => {
                Some(ConstantValue::Null)
            }
            "identifier" => {
                // If identifier refers to a known constant, propagate it
                if let Some(var_name) = node.text() {
                    self.constants.get(var_name).cloned()
                } else {
                    None
                }
            }
            _ => {
                // For other node types, check children
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if let Some(value) = self.extract_constant_from_expression(child) {
                            return Some(value);
                        }
                    }
                }
                None
            }
        }
    }

    /// Check if a variable is constant
    pub fn is_constant(&self, var_name: &str) -> bool {
        self.constants.contains_key(var_name) && !self.reassigned.contains(var_name)
    }

    /// Check if this is a static block context
    fn is_static_block_context(node: &dyn astgrep_core::AstNode) -> bool {
        // Check for static block patterns:
        // 1. The node itself contains "static {" pattern
        // 2. Or it's inside a static initialization block
        if let Some(text) = node.text() {
            let text_trimmed = text.trim();
            // Match patterns like "static {" or "static{"
            if text_trimmed.starts_with("static") && text_trimmed.contains("{") {
                return true;
            }
        }
        
        // Check children recursively for static block patterns
        // This handles cases where static keyword and block are separate children
        let mut has_static = false;
        let mut has_block = false;
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_type = child.node_type();
                
                // Look for static keyword
                if child_type == "static" || child.text().map(|t| t.trim() == "static").unwrap_or(false) {
                    has_static = true;
                }
                
                // Look for block
                if child_type == "block_statement" || child_type == "block" {
                    has_block = true;
                }
                
                // If we see both static and a block in the same node context, it's likely a static block
                if has_static && has_block {
                    return true;
                }
            }
        }
        
        // Also check if parent node is a static block
        if node.node_type() == "static_initializer" 
            || node.node_type() == "static_block" 
            || node.node_type() == "static_initialization" {
            return true;
        }
        
        false
    }

    /// Check if this is a constructor declaration
    /// Java constructors appear as declaration_statement with:
    /// - 4 children: [modifiers, identifier(class_name), params, body]
    /// - No return type (unlike methods which have 5 children with return type)
    fn is_constructor_declaration(node: &dyn astgrep_core::AstNode, class_name: Option<&str>) -> bool {
        // Must be a declaration_statement
        if node.node_type() != "declaration_statement" {
            return false;
        }

        // Constructors have exactly 4 children (no return type)
        // Methods have 5 children (with return type)
        if node.child_count() != 4 {
            return false;
        }

        // Find the identifier (should be the class name for constructors)
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.node_type() == "identifier" {
                    if let Some(name) = child.text() {
                        // If class_name is provided, check if they match
                        if let Some(class) = class_name {
                            if name == class {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if this is a method declaration
    /// Java methods appear as declaration_statement with:
    /// - 5 children: [modifiers, return_type, identifier(method_name), params, body]
    fn is_method_declaration(node: &dyn astgrep_core::AstNode) -> bool {
        // Must be a declaration_statement
        if node.node_type() != "declaration_statement" {
            return false;
        }

        // Methods have 5 children (with return type)
        // Check if second child (after modifiers) is a return type, not an identifier
        if node.child_count() == 5 {
            // Check if the 3rd child (index 2) is an identifier (method name)
            if let Some(child) = node.child(2) {
                if child.node_type() == "identifier" {
                    // Check if 2nd child (index 1) is NOT an identifier
                    // (it would be the return type for methods)
                    if let Some(second_child) = node.child(1) {
                        if second_child.node_type() != "identifier" {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Mark a variable as reassigned (not constant)
    pub fn mark_reassigned(&mut self, var_name: String) {
        self.reassigned.insert(var_name.clone());
        self.constants.remove(&var_name);
    }

    /// Get all constants
    pub fn get_all_constants(&self) -> &HashMap<String, ConstantValue> {
        &self.constants
    }

    /// Get all node constants
    pub fn get_all_node_constants(&self) -> &HashMap<NodeId, ConstantValue> {
        &self.node_constants
    }

    /// Get all location-based constants
    pub fn get_location_based_constants(&self) -> &HashMap<(String, SourceLocation), ConstantValue> {
        &self.location_based_constants
    }

    /// Get all variable definitions
    pub fn get_variable_definitions(&self) -> &[VariableDefinition] {
        &self.variable_definitions
    }

    /// Get a combined map of all constants (fields and locals)
    /// Returns a map where keys are "variable_name@line:col" and values are ConstantValue
    pub fn get_all_constants_with_locations(&self) -> HashMap<String, ConstantValue> {
        let mut result = self.constants.clone();
        
        // Add location-based constants with their locations as part of the key
        for ((name, location), value) in &self.location_based_constants {
            let key = format!("{}@{}:{}", name, location.line, location.column);
            result.insert(key, value.clone());
        }
        
        result
    }
}

impl Default for ConstantPropagator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_value_string() {
        let cv = ConstantValue::String("password".to_string());
        assert_eq!(cv.to_string_value(), Some("password".to_string()));
        assert!(cv.matches_pattern("pass"));
    }

    #[test]
    fn test_constant_value_integer() {
        let cv = ConstantValue::Integer(42);
        assert_eq!(cv.to_string_value(), Some("42".to_string()));
        assert!(cv.matches_pattern("42"));
    }

    #[test]
    fn test_constant_value_boolean() {
        let cv = ConstantValue::Boolean(true);
        assert_eq!(cv.to_string_value(), Some("true".to_string()));
        assert!(cv.matches_pattern("true"));
    }

    #[test]
    fn test_constant_value_null() {
        let cv = ConstantValue::Null;
        assert_eq!(cv.to_string_value(), Some("null".to_string()));
    }

    #[test]
    fn test_constant_propagator_new() {
        let propagator = ConstantPropagator::new();
        assert!(propagator.constants.is_empty());
        assert!(propagator.node_constants.is_empty());
        assert!(propagator.reassigned.is_empty());
    }

    #[test]
    fn test_constant_propagator_mark_reassigned() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("x".to_string(), ConstantValue::Integer(42));
        
        assert!(propagator.is_constant("x"));
        
        propagator.mark_reassigned("x".to_string());
        
        assert!(!propagator.is_constant("x"));
        assert!(!propagator.constants.contains_key("x"));
    }

    #[test]
    fn test_constant_propagator_get_constant() {
        let mut propagator = ConstantPropagator::new();
        propagator.constants.insert("password".to_string(), ConstantValue::String("secret".to_string()));
        
        assert_eq!(
            propagator.get_constant("password"),
            Some(&ConstantValue::String("secret".to_string()))
        );
        assert_eq!(propagator.get_constant("unknown"), None);
    }

    #[test]
    fn test_constant_value_equality() {
        let cv1 = ConstantValue::String("test".to_string());
        let cv2 = ConstantValue::String("test".to_string());
        let cv3 = ConstantValue::String("other".to_string());
        
        assert_eq!(cv1, cv2);
        assert_ne!(cv1, cv3);
    }
}

