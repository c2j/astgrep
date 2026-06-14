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

fn is_operator_node(node_type: &str, node_text: Option<&str>) -> bool {
    // Check by known operator types
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

    // Check by node text (handles cases where node_type is unknown)
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

/// A symbolic value representing the origin of data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolicValue {
    /// A variable with a specific name
    Variable(String),
    /// A field access (e.g., obj.field)
    FieldAccess {
        base: Box<SymbolicValue>,
        field: String,
    },
    /// A method call result
    MethodCall {
        base: Box<SymbolicValue>,
        method: String,
    },
    /// A constructor invocation (e.g., new B())
    ConstructorCall { class: String },
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

    /// Create a constructor call symbolic value
    pub fn constructor_call(class: &str) -> Self {
        SymbolicValue::ConstructorCall {
            class: class.to_string(),
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
            SymbolicValue::FieldAccess { base, .. } => base.is_derived_from(other),
            SymbolicValue::MethodCall { base, .. } => {
                base.is_derived_from(other) || base.as_ref() == other
            }
            SymbolicValue::ConstructorCall { .. } => false,
            _ => false,
        }
    }

    /// Get the root variable of this symbolic value
    pub fn root_variable(&self) -> Option<&str> {
        match self {
            SymbolicValue::Variable(name) => Some(name),
            SymbolicValue::FieldAccess { base, .. } => base.root_variable(),
            SymbolicValue::MethodCall { base, .. } => base.root_variable(),
            SymbolicValue::ConstructorCall { .. } => None,
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
                .or_default()
                .insert(var.clone());
            self.aliases
                .entry(var.clone())
                .or_default()
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
#[derive(Clone)]
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

    /// Get the symbolic value for a given variable name
    pub fn get_symbolic_value(&self, var_name: &str) -> Option<&SymbolicValue> {
        self.state.get(var_name)
    }

    /// Analyze a node and update symbolic state
    fn analyze_node(&mut self, node: &dyn AstNode) -> Result<()> {
        let node_type = node.node_type();

        match node_type {
            "local_variable_declaration" | "variable_declaration" | "field_declaration" => {
                self.analyze_declaration(node)?;
            }
            "assignment_expression" => {
                self.analyze_assignment(node)?;
            }
            "method_invocation" | "call_expression" => {
                self.analyze_method_call(node)?;
            }
            "field_access" | "member_expression" => {
                self.analyze_field_access(node)?;
            }
            _ => {}
        }

        // Always recursively analyze children, even for special node types
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.analyze_node(child)?;
            }
        }

        Ok(())
    }

    /// Analyze a variable declaration
    fn analyze_declaration(&mut self, node: &dyn AstNode) -> Result<()> {
        // Skip if this is just a wrapper node (contains another variable_declaration)
        // For example, "String userName = req.xyz;" might have a child "userName = req.xyz"
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.node_type() == "variable_declaration"
                    || child.node_type() == "variable_declarator"
                    || child.node_type() == "declarator"
                {
                    // This is a nested declarator - real declaration is inside
                    // Don't analyze this node as a whole, let's recursion handle the child
                    return Ok(());
                }
            }
        }

        // Try to extract variable name and initializer
        let mut var_name = None;
        let mut initializer = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        // The first identifier is likely to be variable name
                        if var_name.is_none() {
                            var_name = child.text().map(|s| s.to_string());
                        }
                    }
                    "field_access" | "member_expression" => {
                        // This is likely to be initializer
                        if initializer.is_none() {
                            initializer = Some(child.clone_node());
                        }
                    }
                    _ => {
                        // Check if it's an initializer by looking for field access or assignment patterns
                        let text = child.text().unwrap_or("");
                        if (text.contains(".") || text.contains("(")) && initializer.is_none() {
                            initializer = Some(child.clone_node());
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
    #[allow(dead_code)]
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
                let child_type = child.node_type();
                let child_text = child.text();
                match child_type {
                    "identifier" => {
                        if left.is_none() {
                            left = child_text.map(|s| s.to_string());
                        }
                    }
                    _ => {
                        // Skip operator nodes like "="
                        if right.is_none() && !is_operator_node(child_type, child_text) {
                            eprintln!("DEBUG analyze_assignment: Setting right from child {} (type={}, text='{}')",
                                     i, child_type, child_text.unwrap_or(""));
                            right = Some(child.clone_node());
                        }
                    }
                }
            }
        }

        // Bind the left variable to the symbolic value of the right side
        if let (Some(var_name), Some(right_node)) = (left, right) {
            let symbolic_value = self.node_to_symbolic_value(right_node.as_ref());
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
    fn node_to_symbolic_value(&self, node: &dyn AstNode) -> SymbolicValue {
        let node_type = node.node_type();
        let node_text = node.text();

        match node_type {
            "constructor_invocation" | "object_creation_expression" | "class_creator" => {
                self.extract_constructor_call_dyn(node)
            }
            "method_invocation" | "call_expression" => self.extract_method_call_dyn(node),
            "field_access" | "member_expression" => self.extract_field_access_dyn(node),
            _ => {
                // Check for 'new' keyword to detect constructor calls
                if let Some(text) = node_text {
                    if text.starts_with("new ") && text.contains("(") {
                        return self.extract_constructor_call_dyn(node);
                    }
                }
                // For other nodes, try to extract as method call or field access
                let method_result = self.extract_method_call_dyn(node);
                if !matches!(method_result, SymbolicValue::Unknown) {
                    method_result
                } else {
                    self.extract_field_access_dyn(node)
                }
            }
        }
    }

    /// Helper to extract method call from any AstNode reference
    #[allow(dead_code)]
    fn extract_method_call_from_ref<T: AstNode>(&self, node: &T) -> SymbolicValue {
        // Convert to dyn AstNode and delegate
        self.extract_method_call_dyn(node)
    }

    /// Extract a field access symbolic value
    #[allow(dead_code)]
    fn extract_field_access<T: AstNode>(&self, node: &T) -> SymbolicValue {
        self.extract_field_access_dyn(node)
    }

    /// Internal implementation using dyn trait object
    #[allow(dead_code)]
    fn extract_field_access_dyn(&self, node: &dyn AstNode) -> SymbolicValue {
        let mut base = None;
        let mut field = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.node_type() {
                    "identifier" => {
                        if base.is_none() {
                            base = child.text().map(SymbolicValue::variable);
                        } else if field.is_none() {
                            field = child.text().map(|s| s.to_string());
                        }
                    }
                    "field_access" => {
                        base = Some(self.extract_field_access_dyn(child));
                    }
                    _ => {
                        // Skip operators like ".", "->", etc.
                        let text = child.text().unwrap_or("");
                        if field.is_none() && !text.starts_with(".") && !text.starts_with("->") {
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
    #[allow(dead_code)]
    fn extract_method_call<T: AstNode>(&self, node: &T) -> SymbolicValue {
        self.extract_method_call_dyn(node)
    }

    /// Internal implementation using dyn trait object
    #[allow(dead_code)]
    fn extract_method_call_dyn(&self, node: &dyn AstNode) -> SymbolicValue {
        let mut base = None;
        let mut method = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_type = child.node_type();
                let child_text = child.text();

                match child_type {
                    "identifier" => {
                        if base.is_none() {
                            base = child_text.map(SymbolicValue::variable);
                        } else if method.is_none() {
                            method = child_text.map(|s| s.to_string());
                        }
                    }
                    "field_access" => {
                        base = Some(self.extract_field_access_dyn(child));
                    }
                    _ => {
                        // Try to extract method name from text
                        if let Some(text) = child_text {
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

        if let Some(method) = method {
            if let Some(base) = base {
                SymbolicValue::method_call(base, &method)
            } else {
                SymbolicValue::Unknown
            }
        } else if let Some(base) = base {
            // For calls like x() without an explicit method name,
            // treat as a method call with empty method name
            SymbolicValue::method_call(base, "")
        } else {
            SymbolicValue::Unknown
        }
    }

    /// Extract a constructor call symbolic value
    fn extract_constructor_call_dyn(&self, node: &dyn AstNode) -> SymbolicValue {
        let mut class = None;
        let node_text = node.text();

        // Try to extract the class name from the constructor call
        // The pattern should be: new ClassName(...)
        if let Some(text) = node_text {
            if let Some(rest) = text.strip_prefix("new ") {
                let rest = rest.trim();
                if let Some(paren_pos) = rest.find('(') {
                    let class_name = rest[..paren_pos].trim();
                    if !class_name.is_empty() {
                        class = Some(class_name.to_string());
                    }
                }
            }
        }

        // Fallback: look for identifier child nodes
        if class.is_none() {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.node_type() {
                        "identifier" | "type_identifier" => {
                            let child_text = child.text();
                            // Skip "new" keyword if it appears as a child
                            if let Some(ct) = child_text {
                                if ct != "new" {
                                    class = Some(ct.to_string());
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(class_name) = class {
            SymbolicValue::constructor_call(&class_name)
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

        let constructor = SymbolicValue::constructor_call("MyClass");
        assert!(
            matches!(constructor, SymbolicValue::ConstructorCall { class } if class == "MyClass")
        );
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
    fn test_constructor_call_no_derivation() {
        let constructor = SymbolicValue::constructor_call("MyClass");
        let x = SymbolicValue::variable("x");
        assert!(!constructor.is_derived_from(&x));
        assert!(!x.is_derived_from(&constructor));
        assert!(constructor.root_variable().is_none());
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

        assert!(state.is_alias("a", "c"));

        let aliases_a = state.get_all_aliases("a");
        assert!(aliases_a.contains("b"));
        assert!(aliases_a.contains("c"));
    }

    #[test]
    fn test_symbolic_state_merge() {
        let mut state1 = SymbolicState::new();
        state1.bind("x".to_string(), SymbolicValue::variable("a"));
        state1.bind("y".to_string(), SymbolicValue::variable("b"));

        let mut state2 = SymbolicState::new();
        state2.bind("x".to_string(), SymbolicValue::variable("a"));
        state2.bind("z".to_string(), SymbolicValue::variable("c"));

        let merged = state1.merge(&state2);
        assert!(merged.get("x").is_some());
        assert!(merged.get("y").is_some());
        assert!(merged.get("z").is_some());
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(10, 5);
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_symbolic_propagator_new() {
        let propagator = SymbolicPropagator::new();
        assert!(propagator.state().variables.is_empty());
    }

    #[test]
    fn test_symbolic_propagator_with_deep_propagation() {
        let propagator = SymbolicPropagator::new().with_deep_propagation(false);
        assert!(!propagator.enable_deep_propagation);
    }

    #[test]
    fn test_symbolic_propagator_reset() {
        let mut propagator = SymbolicPropagator::new();
        propagator
            .state
            .bind("x".to_string(), SymbolicValue::variable("y"));
        propagator.reset();
        assert!(propagator.state().variables.is_empty());
    }

    #[test]
    fn test_symbolic_propagator_is_derived_from() {
        let mut propagator = SymbolicPropagator::new();
        propagator
            .state
            .bind("a".to_string(), SymbolicValue::variable("b"));

        assert!(propagator.is_derived_from("a", &SymbolicValue::variable("b")));
        assert!(!propagator.is_derived_from("a", &SymbolicValue::variable("c")));
    }

    #[test]
    fn test_symbolic_propagator_contains_alias() {
        let mut propagator = SymbolicPropagator::new();
        propagator
            .state
            .bind("a".to_string(), SymbolicValue::variable("b"));
        propagator
            .state
            .bind("c".to_string(), SymbolicValue::variable("b"));

        assert!(propagator.contains_alias("value_of_b", "b"));
        assert!(propagator.contains_alias("value_of_a", "b"));
        assert!(!propagator.contains_alias("value_of_x", "z"));
    }

    #[test]
    fn test_symbolic_value_root_variable() {
        let var = SymbolicValue::variable("x");
        assert_eq!(var.root_variable(), Some("x"));

        let field = SymbolicValue::field_access(SymbolicValue::variable("obj"), "field");
        assert_eq!(field.root_variable(), Some("obj"));

        let constructor = SymbolicValue::constructor_call("MyClass");
        assert_eq!(constructor.root_variable(), None);
    }
}
