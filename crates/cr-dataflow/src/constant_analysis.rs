//! Advanced constant analysis for improved taint tracking
//!
//! This module provides advanced constant analysis capabilities including:
//! - String constant tracking
//! - Numeric constant tracking
//! - Constant folding
//! - Constant propagation through function calls

use crate::constant_propagation::ConstantValue;
use cr_core::Result;
use std::collections::HashMap;

/// Represents a constant value with metadata
#[derive(Debug, Clone)]
pub struct ConstantInfo {
    /// The constant value
    pub value: ConstantValue,
    /// Whether this constant is mutable
    pub is_mutable: bool,
    /// Number of assignments to this constant
    pub assignment_count: usize,
    /// Whether this constant is used in a sensitive context
    pub is_sensitive: bool,
}

impl ConstantInfo {
    /// Create a new constant info
    pub fn new(value: ConstantValue) -> Self {
        Self {
            value,
            is_mutable: false,
            assignment_count: 1,
            is_sensitive: false,
        }
    }

    /// Mark this constant as mutable
    pub fn mark_mutable(mut self) -> Self {
        self.is_mutable = true;
        self
    }

    /// Mark this constant as sensitive
    pub fn mark_sensitive(mut self) -> Self {
        self.is_sensitive = true;
        self
    }

    /// Increment assignment count
    pub fn increment_assignments(&mut self) {
        self.assignment_count += 1;
    }
}

/// Constant analysis engine
pub struct ConstantAnalyzer {
    /// Map from variable name to constant info
    constants: HashMap<String, ConstantInfo>,
    /// Map from function name to return constants
    function_returns: HashMap<String, Vec<ConstantValue>>,
    /// Sensitive patterns to detect
    sensitive_patterns: Vec<String>,
}

impl ConstantAnalyzer {
    /// Create a new constant analyzer
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            function_returns: HashMap::new(),
            sensitive_patterns: vec![
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "api_key".to_string(),
                "private_key".to_string(),
                "credential".to_string(),
            ],
        }
    }

    /// Register a constant
    pub fn register_constant(&mut self, name: String, value: ConstantValue) {
        let mut info = ConstantInfo::new(value.clone());

        // Check if this is a sensitive constant
        if self.is_sensitive_constant(&name, &value) {
            info = info.mark_sensitive();
        }

        self.constants.insert(name, info);
    }

    /// Check if a constant is sensitive
    fn is_sensitive_constant(&self, name: &str, value: &ConstantValue) -> bool {
        let name_lower = name.to_lowercase();
        
        // Check name patterns
        for pattern in &self.sensitive_patterns {
            if name_lower.contains(pattern) {
                return true;
            }
        }

        // Check value patterns
        if let Some(str_val) = value.to_string_value() {
            let str_lower = str_val.to_lowercase();
            for pattern in &self.sensitive_patterns {
                if str_lower.contains(pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Get constant info for a variable
    pub fn get_constant(&self, name: &str) -> Option<&ConstantInfo> {
        self.constants.get(name)
    }

    /// Get constant value for a variable
    pub fn get_constant_value(&self, name: &str) -> Option<&ConstantValue> {
        self.constants.get(name).map(|info| &info.value)
    }

    /// Check if a variable is a constant
    pub fn is_constant(&self, name: &str) -> bool {
        self.constants.contains_key(name) && !self.constants[name].is_mutable
    }

    /// Check if a constant is sensitive
    pub fn is_sensitive(&self, name: &str) -> bool {
        self.constants.get(name).map(|info| info.is_sensitive).unwrap_or(false)
    }

    /// Register a function return constant
    pub fn register_function_return(&mut self, func_name: String, return_value: ConstantValue) {
        self.function_returns
            .entry(func_name)
            .or_insert_with(Vec::new)
            .push(return_value);
    }

    /// Get function return constants
    pub fn get_function_returns(&self, func_name: &str) -> Option<&Vec<ConstantValue>> {
        self.function_returns.get(func_name)
    }

    /// Perform constant folding on an expression
    pub fn fold_constants(&self, expr: &str) -> Option<ConstantValue> {
        // Simple constant folding for common patterns
        if let Ok(num) = expr.parse::<i64>() {
            return Some(ConstantValue::Integer(num));
        }

        if expr.starts_with('"') && expr.ends_with('"') {
            let string = expr[1..expr.len() - 1].to_string();
            return Some(ConstantValue::String(string));
        }

        if expr == "true" {
            return Some(ConstantValue::Boolean(true));
        }

        if expr == "false" {
            return Some(ConstantValue::Boolean(false));
        }

        if expr == "null" {
            return Some(ConstantValue::Null);
        }

        None
    }

    /// Get all constants
    pub fn get_all_constants(&self) -> &HashMap<String, ConstantInfo> {
        &self.constants
    }

    /// Get all sensitive constants
    pub fn get_sensitive_constants(&self) -> Vec<(&String, &ConstantInfo)> {
        self.constants
            .iter()
            .filter(|(_, info)| info.is_sensitive)
            .collect()
    }

    /// Add a sensitive pattern
    pub fn add_sensitive_pattern(&mut self, pattern: String) {
        self.sensitive_patterns.push(pattern);
    }
}

impl Default for ConstantAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_info_new() {
        let info = ConstantInfo::new(ConstantValue::String("test".to_string()));
        assert_eq!(info.assignment_count, 1);
        assert!(!info.is_mutable);
        assert!(!info.is_sensitive);
    }

    #[test]
    fn test_constant_info_mark_mutable() {
        let info = ConstantInfo::new(ConstantValue::Integer(42))
            .mark_mutable();
        assert!(info.is_mutable);
    }

    #[test]
    fn test_constant_info_mark_sensitive() {
        let info = ConstantInfo::new(ConstantValue::String("secret".to_string()))
            .mark_sensitive();
        assert!(info.is_sensitive);
    }

    #[test]
    fn test_constant_analyzer_new() {
        let analyzer = ConstantAnalyzer::new();
        assert!(analyzer.constants.is_empty());
        assert!(!analyzer.sensitive_patterns.is_empty());
    }

    #[test]
    fn test_constant_analyzer_register() {
        let mut analyzer = ConstantAnalyzer::new();
        analyzer.register_constant("x".to_string(), ConstantValue::Integer(42));
        
        assert!(analyzer.is_constant("x"));
        assert_eq!(analyzer.get_constant_value("x"), Some(&ConstantValue::Integer(42)));
    }

    #[test]
    fn test_constant_analyzer_sensitive() {
        let mut analyzer = ConstantAnalyzer::new();
        analyzer.register_constant("password".to_string(), ConstantValue::String("secret123".to_string()));
        
        assert!(analyzer.is_sensitive("password"));
    }

    #[test]
    fn test_constant_analyzer_fold() {
        let analyzer = ConstantAnalyzer::new();
        
        assert_eq!(analyzer.fold_constants("42"), Some(ConstantValue::Integer(42)));
        assert_eq!(analyzer.fold_constants("\"hello\""), Some(ConstantValue::String("hello".to_string())));
        assert_eq!(analyzer.fold_constants("true"), Some(ConstantValue::Boolean(true)));
        assert_eq!(analyzer.fold_constants("null"), Some(ConstantValue::Null));
    }

    #[test]
    fn test_constant_analyzer_function_returns() {
        let mut analyzer = ConstantAnalyzer::new();
        analyzer.register_function_return("get_password".to_string(), ConstantValue::String("secret".to_string()));
        
        let returns = analyzer.get_function_returns("get_password");
        assert!(returns.is_some());
        assert_eq!(returns.unwrap().len(), 1);
    }
}

