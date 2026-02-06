//! Symbolic propagation analysis
//!
//! This module provides symbolic propagation (symbolic execution) for tracking
//! variable aliases and data flow through assignments, enabling detection of
//! taint flows even when variables are reassigned or aliased.
//!
//! Example:
//! ```java
//! ZipEntry nextEntry = super.getNextEntry();  // source
//! ZipEntry c = nextEntry;                      // alias
//! name = c.getName();                          // use through alias
//! ```

use astgrep_core::{AstNode, Result};
use std::collections::{HashMap, HashSet};

/// A symbolic value representing the origin of data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolicValue {
    /// A variable with a specific name
    Variable(String),
    /// A field access (e.g., obj.field)
    FieldAccess { base: Box<SymbolicValue>, field: String },
    /// A method call result
    MethodCall { base: Box<SymbolicValue>, method: String },
    /// A constant value
    Constant(String),
    /// Unknown/untracked value
    Unknown,
}

impl SymbolicValue {
    /// Create a variable symbolic value
    pub fn variable(name: &str) -> Self {
        SymbolicValue::Variable(name.to_string())
    }

    /// Create a field access symbolic value
    pub fn field_access(base: SymbolicValue, field: &str) -> Self {
        SymbolicValue::FieldAccess {
            base: Box::new(base),
            field: field.to_string(),
        }
    }

    /// Create a method call symbolic value
    pub fn method_call(base: SymbolicValue, method: &str) -> Self {
        SymbolicValue::MethodCall {
            base: Box::new(base),
            method: method.to_string(),
        }
    }

    /// Check if this value is derived from another value
    pub fn is_derived_from(&self, other: &SymbolicValue) -> bool {
        match self {
            SymbolicValue::Variable(name) => {
                if let SymbolicValue::Variable(other_name) = other {
                    name == other_name
                } else {
                    false
                }
            }
            SymbolicValue::FieldAccess { base, .. } => {
                base.is_derived_from(other)
            }
            SymbolicValue::MethodCall { base, .. } => {
                base.is_derived_from(other) || base.as_ref() == other
            }
            _ => false,
        }
    }

    /// Get the root variable of this symbolic value
    pub fn root_variable(&self) -> Option<&str> {
        match self {
            SymbolicValue::Variable(name) => Some(name),
            SymbolicValue::FieldAccess { base, .. } => base.root_variable(),
            SymbolicValue::MethodCall { base, .. } => base.root_variable(),
            _ => None,
        }
    }
}

/// Location in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Symbolic state at a program point
#[derive(Debug, Clone)]
pub struct SymbolicState {
    /// Variable name -> symbolic value
    pub variables: HashMap<String, SymbolicValue>,
    /// Track which variables are aliases of each other
    pub aliases: HashMap<String, HashSet<String>>,
}

impl SymbolicState {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Bind a variable to a symbolic value
    pub fn bind(&mut self, var: String, value: SymbolicValue) {
        // Track aliases
        if let SymbolicValue::Variable(src) = &value {
            self.aliases
                .entry(src.clone())
                .or_insert_with(HashSet::new)
                .insert(var.clone());
            self.aliases
                .entry(var.clone())
                .or_insert_with(HashSet::new)
                .insert(src.clone());
        }

        self.variables.insert(var, value);
    }

    /// Get the symbolic value of a variable
    pub fn get(&self, var: &str) -> Option<&SymbolicValue> {
        self.variables.get(var)
    }

    /// Check if a variable is an alias of another
    pub fn is_alias(&self, var1: &str, var2: &str) -> bool {
        if var1 == var2 {
            return true;
        }
        
        if let Some(aliases) = self.aliases.get(var1) {
            if aliases.contains(var2) {
                return true;
            }
        }

        // Check transitive aliases
        if let Some(aliases1) = self.aliases.get(var1) {
            for alias in aliases1 {
                if let Some(aliases2) = self.aliases.get(alias) {
                    if aliases2.contains(var2) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all aliases of a variable (including transitive)
    pub fn get_all_aliases(&self, var: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut visited = HashSet::new();
        let mut queue = vec![var.to_string()];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            if current != var {
                result.insert(current.clone());
            }

            if let Some(aliases) = self.aliases.get(&current) {
                for alias in aliases {
                    if !visited.contains(alias) {
                        queue.push(alias.clone());
                    }
                }
            }
        }

        result
    }

    /// Merge two symbolic states (used for control flow joins)
    pub fn merge(&self, other: &SymbolicState) -> SymbolicState {
        let mut merged = SymbolicState::new();

        // Merge variables
        for (var, value) in &self.variables {
            if let Some(other_value) = other.variables.get(var) {
                // Variable exists in both states
                if value == other_value {
                    merged.variables.insert(var.clone(), value.clone());
                } else {
                    // Conflict - mark as unknown or keep the value
                    merged.variables.insert(var.clone(), value.clone());
                }
            } else {
                merged.variables.insert(var.clone(), value.clone());
            }
        }

        // Add variables from other state
        for (var, value) in &other.variables {
            if !merged.variables.contains_key(var) {
                merged.variables.insert(var.clone(), value.clone());
            }
        }

        // Merge aliases
        for (var, aliases) in &self.aliases {
            let mut merged_aliases = aliases.clone();
            if let Some(other_aliases) = other.aliases.get(var) {
                merged_aliases.extend(other_aliases.iter().cloned());
            }
            merged.aliases.insert(var.clone(), merged_aliases);
        }

        merged
    }
}

impl Default for SymbolicState {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbolic propagation analyzer
pub struct SymbolicPropagator {
    /// Current symbolic state
    state: SymbolicState,
    /// State at each program location (for flow-sensitive analysis)
    location_states: HashMap<SourceLocation, SymbolicState>,
    /// Enable deep propagation through field accesses
    enable_deep_propagation: bool,
}

impl SymbolicPropagator {
    /// Create a new symbolic propagator
    pub fn new() -> Self {
        Self {
            state: SymbolicState::new(),
            location_states: HashMap::new(),
            enable_deep_propagation: true,
        }
    }

    /// Enable or disable deep propagation
    pub fn with_deep_propagation(mut self, enabled: bool) -> Self {
        self.enable_deep_propagation = enabled;
        self
    }

    /// Analyze the AST and build symbolic state
    pub fn analyze(&mut self, ast: &dyn AstNode) -> Result<()> {
        self.analyze_node(ast)?;
        Ok(())
    }

    /// Analyze a node and update symbolic state
    fn analyze_node(&mut self, node: &dyn AstNode) -> Result<()> {
        match node.node_type() {
            "local_variable_declaration" | "field_declaration" => {
                self.analyze_declaration(node)?;
            }
            "assignment_expression" => {
                self.analyze_assignment(node)?;
            }
            "method_invocation" | "call_expression" => {
                self.analyze_method_call(node)?;
            }
            "field_access" => {
                self.analyze_field_access(node)?;
            }
            _ => {
                // Recursively analyze children
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.analyze_node(child)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze a variable declaration
    fn analyze_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
        // Try to extract variable name and initializer
        let mut var_name = None;
        let mut initializer = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "variable_declarator" | "declarator" => {
                        // Analyze the declarator
                        self.analyze_declarator(child, &mut var_name, &mut initializer)?;
                    }
                    "identifier" => {
                        if var_name.is_none() {
                            var_name = child.text().map(|s| s.to_string());
                        }
                    }
                    _ => {
                        // Check if it's an initializer
                        if let Some(text) = child.text() {
                            if !text.is_empty() && initializer.is_none() {
                                initializer = Some(child.clone_node());
                            }
                        }
                    }
                }
            }
        }

        if let (Some(name), Some(init)) = (var_name, initializer) {
            let symbolic_value = self.node_to_symbolic_value(init.as_ref());
            self.state.bind(name, symbolic_value);
        }

        Ok(())
    }

    /// Analyze a declarator node
    fn analyze_declarator(
        &self,
        node: &dyn AstNode,
        var_name: &mut Option<String>,
        initializer: &mut Option<Box<dyn AstNode>>,
    ) -> Result<()> {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        if var_name.is_none() {
                            *var_name = child.text().map(|s| s.to_string());
                        }
                    }
                    "field_access" | "method_invocation" | "call_expression" => {
                        *initializer = Some(child.clone_node());
                    }
                    _ => {
                        // Check for initializer by looking for assignment-like patterns
                        let text = child.text().unwrap_or("");
                        if text.contains("=") || text.contains("(") {
                            *initializer = Some(child.clone_node());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze an assignment expression
    fn analyze_assignment(&mut self, node: &dyn AstNode) -> Result<()> {
        let mut left = None;
        let mut right = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        if left.is_none() {
                            left = child.text().map(|s| s.to_string());
                        }
                    }
                    _ => {
                        if right.is_none() {
                            right = Some(child.clone_node());
                        }
                    }
                }
            }
        }

        if let (Some(var_name), Some(rhs)) = (left, right) {
            let symbolic_value = self.node_to_symbolic_value(rhs.as_ref());
            self.state.bind(var_name, symbolic_value);
        }

        Ok(())
    }

    /// Analyze a method call
    fn analyze_method_call(&mut self, node: &dyn AstNode) -> Result<()> {
        // Try to extract receiver and method name
        if self.enable_deep_propagation {
            // Just process children for now
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    self.analyze_node(child)?;
                }
            }
        }
        Ok(())
    }

    /// Analyze a field access
    fn analyze_field_access(&mut self, node: &dyn AstNode) -> Result<()> {
        // Field accesses are handled during value extraction
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.analyze_node(child)?;
            }
        }
        Ok(())
    }

    /// Convert an AST node to a symbolic value
    fn node_to_symbolic_value(
        &self, node: &dyn AstNode
    ) -> SymbolicValue {
        // Convert to dyn AstNode and delegate
        self.extract_field_access_dyn(node)
    }

    /// Helper to extract method call from any AstNode reference
    fn extract_method_call_from_ref<T: AstNode>(
        &self, node: &T
    ) -> SymbolicValue {
        // Convert to dyn AstNode and delegate
        self.extract_method_call_dyn(node)
    }

    /// Extract a field access symbolic value
    fn extract_field_access<T: AstNode>(
        &self, node: &T
    ) -> SymbolicValue {
        self.extract_field_access_dyn(node)
    }

    /// Internal implementation using dyn trait object
    fn extract_field_access_dyn(&self, node: &dyn AstNode) -> SymbolicValue {
        let mut base = None;
        let mut field = None;

        for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        if base.is_none() {
                            base = child.text().map(|s| SymbolicValue::variable(&s));
                        } else if field.is_none() {
                            field = child.text().map(|s| s.to_string());
                        }
                    }
                    "field_access" => {
                        base = Some(self.extract_field_access_dyn(child));
                    }
                    _ => {
                        if field.is_none() {
                            field = child.text().map(|s| s.to_string());
                        }
                    }
                }
            }
        }

        if let (Some(base), Some(field)) = (base, field) {
            SymbolicValue::field_access(base, &field)
        } else {
            SymbolicValue::Unknown
        }
    }

    /// Extract a method call symbolic value
    fn extract_method_call<T: AstNode>(
        &self, node: &T
    ) -> SymbolicValue {
        self.extract_method_call_dyn(node)
    }

    /// Internal implementation using dyn trait object
    fn extract_method_call_dyn(&self, node: &dyn AstNode) -> SymbolicValue {
        let mut base = None;
        let mut method = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        if base.is_none() {
                            base = child.text().map(|s| SymbolicValue::variable(&s));
                        } else if method.is_none() {
                            method = child.text().map(|s| s.to_string());
                        }
                    }
                    "field_access" => {
                        base = Some(self.extract_field_access_dyn(child));
                    }
                    _ => {
                        // Try to extract method name from text
                        if let Some(text) = child.text() {
                            if text.contains("(") && method.is_none() {
                                let parts: Vec<&str> = text.split('(').collect();
                                if !parts.is_empty() {
                                    method = Some(parts[0].to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(base), Some(method)) = (base, method) {
            SymbolicValue::method_call(base, &method)
        } else {
            SymbolicValue::Unknown
        }
    }

    /// Get the current symbolic state
    pub fn state(&self) -> &SymbolicState {
        &self.state
    }

    /// Check if a variable is derived from a source
    pub fn is_derived_from(&self, var: &str, source: &SymbolicValue) -> bool {
        if let Some(value) = self.state.get(var) {
            value.is_derived_from(source)
        } else {
            false
        }
    }

    /// Check if a value contains any alias of a variable
    pub fn contains_alias(&self, value: &str, var: &str) -> bool {
        let aliases = self.state.get_all_aliases(var);
        
        // Check if the value text contains any alias
        for alias in &aliases {
            if value.contains(alias) {
                return true;
            }
        }

        // Also check the original variable
        value.contains(var)
    }

    /// Reset the propagator state
    pub fn reset(&mut self) {
        self.state = SymbolicState::new();
        self.location_states.clear();
    }
}

impl Default for SymbolicPropagator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbolic_value_creation() {
        let var = SymbolicValue::variable("x");
        assert!(matches!(var, SymbolicValue::Variable(name) if name == "x"));

        let field = SymbolicValue::field_access(SymbolicValue::variable("obj"), "field");
        assert!(matches!(field, SymbolicValue::FieldAccess { field: f, .. } if f == "field"));
    }

    #[test]
    fn test_symbolic_value_derivation() {
        let x = SymbolicValue::variable("x");
        let field_x = SymbolicValue::field_access(x.clone(), "field");
        let method_x = SymbolicValue::method_call(x.clone(), "getName");

        assert!(field_x.is_derived_from(&x));
        assert!(method_x.is_derived_from(&x));
        assert!(!x.is_derived_from(&field_x));
    }

    #[test]
    fn test_symbolic_state_binding() {
        let mut state = SymbolicState::new();
        state.bind("a".to_string(), SymbolicValue::variable("b"));
        
        assert!(state.is_alias("a", "b"));
        assert!(state.is_alias("b", "a"));
        
        let aliases = state.get_all_aliases("a");
        assert!(aliases.contains("b"));
    }

    #[test]
    fn test_symbolic_state_transitive_aliases() {
        let mut state = SymbolicState::new();
        state.bind("a".to_string(), SymbolicValue::variable("b"));
        state.bind("c".to_string(), SymbolicValue::variable("b"));
        
        // a and c should be transitively aliased through b
        assert!(state.is_alias("a", "c"));
        
        let aliases_a = state.get_all_aliases("a");
        assert!(aliases_a.contains("b"));
        assert!(aliases_a.contains("c"));
    }
}
