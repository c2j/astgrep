//! Metavariable handling
//! 
//! This module provides functionality for handling metavariables in patterns.

use cr_core::{AstNode, Result};
use regex::Regex;
use std::collections::HashMap;

/// Metavariable constraint types
#[derive(Debug, Clone)]
pub enum MetavarConstraint {
    /// Regex pattern constraint
    Regex(Regex),
    /// Type constraint
    Type(String),
    /// Value constraint
    Value(String),
    /// Custom constraint function
    Custom(String),
}

impl PartialEq for MetavarConstraint {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MetavarConstraint::Regex(a), MetavarConstraint::Regex(b)) => a.as_str() == b.as_str(),
            (MetavarConstraint::Type(a), MetavarConstraint::Type(b)) => a == b,
            (MetavarConstraint::Value(a), MetavarConstraint::Value(b)) => a == b,
            (MetavarConstraint::Custom(a), MetavarConstraint::Custom(b)) => a == b,
            _ => false,
        }
    }
}

/// Metavariable binding with constraints
#[derive(Debug, Clone)]
pub struct MetavarBinding {
    pub name: String,
    pub value: String,
    pub node_type: String,
    pub constraints: Vec<MetavarConstraint>,
}

impl MetavarBinding {
    /// Create a new metavariable binding
    pub fn new(name: String, value: String, node_type: String) -> Self {
        Self {
            name,
            value,
            node_type,
            constraints: Vec::new(),
        }
    }

    /// Add a constraint to this binding
    pub fn add_constraint(mut self, constraint: MetavarConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Check if this binding satisfies all constraints
    pub fn satisfies_constraints(&self) -> bool {
        for constraint in &self.constraints {
            if !self.satisfies_constraint(constraint) {
                return false;
            }
        }
        true
    }

    /// Check if this binding satisfies a specific constraint
    fn satisfies_constraint(&self, constraint: &MetavarConstraint) -> bool {
        match constraint {
            MetavarConstraint::Regex(regex) => regex.is_match(&self.value),
            MetavarConstraint::Type(expected_type) => self.node_type == *expected_type,
            MetavarConstraint::Value(expected_value) => self.value == *expected_value,
            MetavarConstraint::Custom(_) => {
                // Custom constraints would be evaluated by external functions
                // For now, we assume they pass
                true
            }
        }
    }
}

/// Metavariable manager
pub struct MetavarManager {
    bindings: HashMap<String, MetavarBinding>,
    constraints: HashMap<String, Vec<MetavarConstraint>>,
}

impl MetavarManager {
    /// Create a new metavariable manager
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            constraints: HashMap::new(),
        }
    }

    /// Add a constraint for a metavariable
    pub fn add_constraint(&mut self, metavar_name: String, constraint: MetavarConstraint) {
        self.constraints
            .entry(metavar_name)
            .or_insert_with(Vec::new)
            .push(constraint);
    }

    /// Bind a metavariable to a value
    pub fn bind(&mut self, name: String, value: String, node: &dyn AstNode) -> Result<bool> {
        let node_type = node.node_type().to_string();
        
        // Check if this metavariable is already bound
        if let Some(existing_binding) = self.bindings.get(&name) {
            // Check if the new binding is consistent with the existing one
            return Ok(existing_binding.value == value);
        }

        // Create new binding with constraints
        let mut binding = MetavarBinding::new(name.clone(), value, node_type);
        
        if let Some(constraints) = self.constraints.get(&name) {
            for constraint in constraints {
                binding = binding.add_constraint(constraint.clone());
            }
        }

        // Check if the binding satisfies all constraints
        if !binding.satisfies_constraints() {
            return Ok(false);
        }

        self.bindings.insert(name, binding);
        Ok(true)
    }

    /// Get a metavariable binding
    pub fn get_binding(&self, name: &str) -> Option<&MetavarBinding> {
        self.bindings.get(name)
    }

    /// Get all bindings
    pub fn get_all_bindings(&self) -> &HashMap<String, MetavarBinding> {
        &self.bindings
    }

    /// Get binding values as a simple map
    pub fn get_binding_values(&self) -> HashMap<String, String> {
        self.bindings
            .iter()
            .map(|(name, binding)| (name.clone(), binding.value.clone()))
            .collect()
    }

    /// Clear all bindings
    pub fn clear_bindings(&mut self) {
        self.bindings.clear();
    }

    /// Check if a metavariable is bound
    pub fn is_bound(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Unbind a metavariable
    pub fn unbind(&mut self, name: &str) -> Option<MetavarBinding> {
        self.bindings.remove(name)
    }

    /// Get the number of bound metavariables
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Validate all current bindings
    pub fn validate_bindings(&self) -> bool {
        self.bindings.values().all(|binding| binding.satisfies_constraints())
    }

    /// Create a snapshot of current bindings
    pub fn snapshot(&self) -> HashMap<String, MetavarBinding> {
        self.bindings.clone()
    }

    /// Restore bindings from a snapshot
    pub fn restore(&mut self, snapshot: HashMap<String, MetavarBinding>) {
        self.bindings = snapshot;
    }
}

impl Default for MetavarManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for metavariable operations
pub mod utils {
    use super::*;

    /// Extract metavariable names from a pattern string
    pub fn extract_metavar_names(pattern: &str) -> Vec<String> {
        let mut metavars = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut name = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if !name.is_empty() {
                    metavars.push(name);
                }
            }
        }

        metavars
    }

    /// Check if a string is a valid metavariable name
    pub fn is_valid_metavar_name(name: &str) -> bool {
        !name.is_empty()
            && name.chars().next().unwrap().is_uppercase()
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Normalize metavariable name (ensure it starts with $)
    pub fn normalize_metavar_name(name: &str) -> String {
        if name.starts_with('$') {
            name.to_string()
        } else {
            format!("${}", name)
        }
    }

    /// Create a regex constraint from a pattern string
    pub fn create_regex_constraint(pattern: &str) -> Result<MetavarConstraint> {
        let regex = Regex::new(pattern)
            .map_err(|e| cr_core::AnalysisError::pattern_match_error(format!("Invalid regex: {}", e)))?;
        Ok(MetavarConstraint::Regex(regex))
    }

    /// Create a type constraint
    pub fn create_type_constraint(type_name: &str) -> MetavarConstraint {
        MetavarConstraint::Type(type_name.to_string())
    }

    /// Create a value constraint
    pub fn create_value_constraint(value: &str) -> MetavarConstraint {
        MetavarConstraint::Value(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_ast::{AstBuilder, NodeType};

    #[test]
    fn test_metavar_binding_creation() {
        let binding = MetavarBinding::new(
            "VAR".to_string(),
            "test_value".to_string(),
            "identifier".to_string(),
        );

        assert_eq!(binding.name, "VAR");
        assert_eq!(binding.value, "test_value");
        assert_eq!(binding.node_type, "identifier");
        assert!(binding.constraints.is_empty());
    }

    #[test]
    fn test_metavar_binding_with_constraints() {
        let regex_constraint = utils::create_regex_constraint(r"test_.*").unwrap();
        let type_constraint = utils::create_type_constraint("identifier");

        let binding = MetavarBinding::new(
            "VAR".to_string(),
            "test_value".to_string(),
            "identifier".to_string(),
        )
        .add_constraint(regex_constraint)
        .add_constraint(type_constraint);

        assert_eq!(binding.constraints.len(), 2);
        assert!(binding.satisfies_constraints());
    }

    #[test]
    fn test_metavar_binding_constraint_failure() {
        let regex_constraint = utils::create_regex_constraint(r"fail_.*").unwrap();

        let binding = MetavarBinding::new(
            "VAR".to_string(),
            "test_value".to_string(),
            "identifier".to_string(),
        )
        .add_constraint(regex_constraint);

        assert!(!binding.satisfies_constraints());
    }

    #[test]
    fn test_metavar_manager_binding() {
        let mut manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        let result = manager.bind("VAR".to_string(), "test_var".to_string(), &node);
        assert!(result.unwrap());
        assert!(manager.is_bound("VAR"));
        assert_eq!(manager.binding_count(), 1);
    }

    #[test]
    fn test_metavar_manager_consistent_binding() {
        let mut manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        // First binding
        let result1 = manager.bind("VAR".to_string(), "test_var".to_string(), &node);
        assert!(result1.unwrap());

        // Second binding with same value should succeed
        let result2 = manager.bind("VAR".to_string(), "test_var".to_string(), &node);
        assert!(result2.unwrap());

        // Third binding with different value should fail
        let result3 = manager.bind("VAR".to_string(), "different_var".to_string(), &node);
        assert!(!result3.unwrap());
    }

    #[test]
    fn test_metavar_manager_with_constraints() {
        let mut manager = MetavarManager::new();
        let regex_constraint = utils::create_regex_constraint(r"test_.*").unwrap();
        manager.add_constraint("VAR".to_string(), regex_constraint);

        let node = AstBuilder::identifier("test_var");

        // Binding that satisfies constraint should succeed
        let result1 = manager.bind("VAR".to_string(), "test_var".to_string(), &node);
        assert!(result1.unwrap());

        manager.clear_bindings();

        // Binding that doesn't satisfy constraint should fail
        let result2 = manager.bind("VAR".to_string(), "other_var".to_string(), &node);
        assert!(!result2.unwrap());
    }

    #[test]
    fn test_metavar_manager_snapshot_restore() {
        let mut manager = MetavarManager::new();
        let node = AstBuilder::identifier("test_var");

        manager.bind("VAR1".to_string(), "value1".to_string(), &node).unwrap();
        manager.bind("VAR2".to_string(), "value2".to_string(), &node).unwrap();

        let snapshot = manager.snapshot();
        assert_eq!(snapshot.len(), 2);

        manager.clear_bindings();
        assert_eq!(manager.binding_count(), 0);

        manager.restore(snapshot);
        assert_eq!(manager.binding_count(), 2);
        assert!(manager.is_bound("VAR1"));
        assert!(manager.is_bound("VAR2"));
    }

    #[test]
    fn test_extract_metavar_names() {
        let pattern = "function $NAME($PARAM1, $PARAM2) { return $RESULT; }";
        let metavars = utils::extract_metavar_names(pattern);
        assert_eq!(metavars, vec!["NAME", "PARAM1", "PARAM2", "RESULT"]);
    }

    #[test]
    fn test_is_valid_metavar_name() {
        assert!(utils::is_valid_metavar_name("VAR"));
        assert!(utils::is_valid_metavar_name("VAR_NAME"));
        assert!(utils::is_valid_metavar_name("VAR123"));
        assert!(!utils::is_valid_metavar_name("var")); // lowercase
        assert!(!utils::is_valid_metavar_name("123VAR")); // starts with number
        assert!(!utils::is_valid_metavar_name("")); // empty
    }

    #[test]
    fn test_normalize_metavar_name() {
        assert_eq!(utils::normalize_metavar_name("VAR"), "$VAR");
        assert_eq!(utils::normalize_metavar_name("$VAR"), "$VAR");
    }

    #[test]
    fn test_get_binding_values() {
        let mut manager = MetavarManager::new();
        let node = AstBuilder::identifier("test");

        manager.bind("VAR1".to_string(), "value1".to_string(), &node).unwrap();
        manager.bind("VAR2".to_string(), "value2".to_string(), &node).unwrap();

        let values = manager.get_binding_values();
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("VAR1"), Some(&"value1".to_string()));
        assert_eq!(values.get("VAR2"), Some(&"value2".to_string()));
    }
}
