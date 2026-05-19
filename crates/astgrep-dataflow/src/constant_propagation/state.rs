//! State management for constant propagation analysis
//!
//! This module contains all data structures and state-related logic
//! for constant propagation analysis.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Source location for a variable definition or use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Represents a constant value in the program
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Represents a scope for variable tracking
#[derive(Debug, Clone)]
pub struct Scope {
    /// Variables defined in this scope
    pub variables: HashMap<String, ConstantValue>,
    /// Source location where scope starts
    pub start_location: SourceLocation,
    /// Source location where scope ends
    pub end_location: SourceLocation,
}

impl Scope {
    pub fn new(start: SourceLocation) -> Self {
        Self {
            variables: HashMap::new(),
            start_location: start,
            end_location: start,
        }
    }

    pub fn define_variable(&mut self, name: String, value: ConstantValue) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&ConstantValue> {
        self.variables.get(name)
    }

    pub fn update_location(&mut self, location: SourceLocation) {
        if location.line > self.end_location.line
            || (location.line == self.end_location.line
                && location.column > self.end_location.column)
        {
            self.end_location = location;
        }
    }
}

/// Variable definition with location information
#[derive(Debug, Clone)]
pub struct VariableDefinition {
    pub name: String,
    pub value: ConstantValue,
    pub location: SourceLocation,
    pub scope_depth: usize,
}

/// Context for tracking where we are in the AST
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisitContext {
    TopLevel,
    StaticBlock,
    Constructor,
    Method,
    Other,
}

/// Constant propagation analyzer
pub struct ConstantPropagator {
    /// Map from variable name to constant value
    pub constants: HashMap<String, ConstantValue>,
    /// Map from node ID to constant value
    pub node_constants: HashMap<crate::graph::NodeId, ConstantValue>,
    /// Set of variables that are reassigned (not constant)
    pub reassigned: HashSet<String>,
    /// Current class name for detecting constructors
    pub current_class_name: Option<String>,
    /// Number of constructors in current class
    pub constructor_count: usize,
    /// Fields initialized in constructors (to detect partial initialization)
    pub fields_in_constructors: HashMap<String, usize>,
    /// Scope stack for local variable tracking
    pub scope_stack: Vec<Scope>,
    /// All variable definitions with location information
    pub variable_definitions: Vec<VariableDefinition>,
    /// Map from (variable_name, location) to constant value
    /// Used for efficient lookup of variable values at specific locations
    pub location_based_constants: HashMap<(String, SourceLocation), ConstantValue>,
}

impl ConstantPropagator {
    /// Push a new scope onto the stack
    pub fn push_scope(&mut self, location: SourceLocation) {
        self.scope_stack.push(Scope::new(location));
    }

    /// Pop the current scope from the stack
    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Define a local variable in the current scope
    pub fn define_local_variable(
        &mut self,
        name: String,
        value: ConstantValue,
        location: SourceLocation,
    ) {
        // Record the definition
        let def = VariableDefinition {
            name: name.clone(),
            value: value.clone(),
            location,
            scope_depth: self.scope_stack.len(),
        };
        self.variable_definitions.push(def);

        // Store location-based constant for efficient lookup
        self.location_based_constants
            .insert((name.clone(), location), value.clone());

        // Also define in current scope if we have one
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.define_variable(name, value);
        }
    }

    /// Look up a variable in the current scope chain
    pub fn lookup_variable(&self, name: &str) -> Option<&ConstantValue> {
        // Search from innermost to outermost scope
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.get_variable(name) {
                return Some(value);
            }
        }
        None
    }

    /// Update scope end location
    pub fn update_scope_location(&mut self, location: SourceLocation) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.update_location(location);
        }
    }

    /// Get constant value for a variable
    pub fn get_constant(&self, var_name: &str) -> Option<&ConstantValue> {
        self.constants.get(var_name)
    }

    /// Get constant value for a node
    pub fn get_node_constant(&self, node_id: crate::graph::NodeId) -> Option<&ConstantValue> {
        self.node_constants.get(&node_id)
    }

    /// Check if a variable is constant
    pub fn is_constant(&self, var_name: &str) -> bool {
        self.constants.contains_key(var_name) && !self.reassigned.contains(var_name)
    }

    /// Get all constants
    pub fn get_all_constants(&self) -> &HashMap<String, ConstantValue> {
        &self.constants
    }

    /// Get all node constants
    pub fn get_all_node_constants(&self) -> &HashMap<crate::graph::NodeId, ConstantValue> {
        &self.node_constants
    }

    /// Get all location-based constants
    pub fn get_location_based_constants(
        &self,
    ) -> &HashMap<(String, SourceLocation), ConstantValue> {
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
        for ((name, location), value) in &self.location_based_constants {
            let key = format!("{}@{}:{}", name, location.line, location.column);
            result.insert(key, value.clone());
        }
        result
    }

    /// Mark a variable as reassigned (not constant)
    pub fn mark_reassigned(&mut self, var_name: String) {
        self.reassigned.insert(var_name.clone());
        self.constants.remove(&var_name);
    }
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
}

impl Default for ConstantPropagator {
    fn default() -> Self {
        Self::new()
    }
}
