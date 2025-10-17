//! Symbol table and type inference for enhanced data flow analysis
//!
//! This module provides:
//! - Symbol table management
//! - Type inference and tracking
//! - Scope management
//! - Symbol resolution

use std::collections::HashMap;
use cr_core::{Result, AnalysisError};

/// Represents a symbol in the program
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub scope_id: usize,
    pub node_id: usize,
}

/// Type information for a symbol
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    /// Primitive types
    Primitive(String), // "int", "string", "bool", etc.
    /// Object/class types
    Object(String),
    /// Array types
    Array(Box<TypeInfo>),
    /// Function types
    Function {
        params: Vec<TypeInfo>,
        return_type: Box<TypeInfo>,
    },
    /// Union types
    Union(Vec<TypeInfo>),
    /// Unknown type
    Unknown,
}

impl TypeInfo {
    /// Check if this type is compatible with another
    pub fn is_compatible_with(&self, other: &TypeInfo) -> bool {
        match (self, other) {
            (TypeInfo::Unknown, _) | (_, TypeInfo::Unknown) => true,
            (TypeInfo::Primitive(a), TypeInfo::Primitive(b)) => a == b,
            (TypeInfo::Object(a), TypeInfo::Object(b)) => a == b,
            (TypeInfo::Array(a), TypeInfo::Array(b)) => a.is_compatible_with(b),
            (TypeInfo::Union(types), other) => types.iter().any(|t| t.is_compatible_with(other)),
            (this, TypeInfo::Union(types)) => types.iter().any(|t| this.is_compatible_with(t)),
            _ => false,
        }
    }

    /// Get a string representation
    pub fn to_string(&self) -> String {
        match self {
            TypeInfo::Primitive(name) => name.clone(),
            TypeInfo::Object(name) => name.clone(),
            TypeInfo::Array(inner) => format!("{}[]", inner.to_string()),
            TypeInfo::Function { params, return_type } => {
                let param_str = params
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) -> {}", param_str, return_type.to_string())
            }
            TypeInfo::Union(types) => {
                types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            TypeInfo::Unknown => "unknown".to_string(),
        }
    }
}

/// Scope information
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub symbols: HashMap<String, Symbol>,
    pub scope_type: ScopeType,
}

/// Type of scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeType {
    Global,
    Function(String),
    Block,
    Class(String),
    Loop,
}

impl Scope {
    /// Create a new scope
    pub fn new(id: usize, scope_type: ScopeType) -> Self {
        Self {
            id,
            parent_id: None,
            symbols: HashMap::new(),
            scope_type,
        }
    }

    /// Add a symbol to this scope
    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    /// Get a symbol from this scope
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Check if symbol exists in this scope
    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }
}

/// Symbol table for managing symbols and types
pub struct SymbolTable {
    scopes: HashMap<usize, Scope>,
    current_scope_id: usize,
    scope_counter: usize,
    symbol_types: HashMap<String, TypeInfo>,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        let mut table = Self {
            scopes: HashMap::new(),
            current_scope_id: 0,
            scope_counter: 1,
            symbol_types: HashMap::new(),
        };

        // Create global scope
        let global_scope = Scope::new(0, ScopeType::Global);
        table.scopes.insert(0, global_scope);

        table
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, scope_type: ScopeType) -> usize {
        let new_scope_id = self.scope_counter;
        self.scope_counter += 1;

        let mut new_scope = Scope::new(new_scope_id, scope_type);
        new_scope.parent_id = Some(self.current_scope_id);

        self.scopes.insert(new_scope_id, new_scope);
        self.current_scope_id = new_scope_id;

        new_scope_id
    }

    /// Exit current scope
    pub fn exit_scope(&mut self) -> Result<()> {
        if let Some(scope) = self.scopes.get(&self.current_scope_id) {
            if let Some(parent_id) = scope.parent_id {
                self.current_scope_id = parent_id;
                return Ok(());
            }
        }
        Err(AnalysisError::internal_error("Cannot exit global scope"))
    }

    /// Define a symbol in current scope
    pub fn define_symbol(&mut self, name: String, node_id: usize, type_info: TypeInfo) -> Result<()> {
        let symbol = Symbol {
            name: name.clone(),
            scope_id: self.current_scope_id,
            node_id,
        };

        if let Some(scope) = self.scopes.get_mut(&self.current_scope_id) {
            scope.add_symbol(symbol);
            self.symbol_types.insert(name, type_info);
            Ok(())
        } else {
            Err(AnalysisError::internal_error("Current scope not found"))
        }
    }

    /// Resolve a symbol (search in current scope and parent scopes)
    pub fn resolve_symbol(&self, name: &str) -> Option<&Symbol> {
        let mut current_scope_id = self.current_scope_id;

        loop {
            if let Some(scope) = self.scopes.get(&current_scope_id) {
                if let Some(symbol) = scope.get_symbol(name) {
                    return Some(symbol);
                }

                if let Some(parent_id) = scope.parent_id {
                    current_scope_id = parent_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        None
    }

    /// Get type information for a symbol
    pub fn get_symbol_type(&self, name: &str) -> Option<&TypeInfo> {
        self.symbol_types.get(name)
    }

    /// Update type information for a symbol
    pub fn update_symbol_type(&mut self, name: String, type_info: TypeInfo) {
        self.symbol_types.insert(name, type_info);
    }

    /// Get all symbols in current scope
    pub fn get_current_scope_symbols(&self) -> Option<Vec<&Symbol>> {
        self.scopes
            .get(&self.current_scope_id)
            .map(|scope| scope.symbols.values().collect())
    }

    /// Get current scope type
    pub fn get_current_scope_type(&self) -> Option<&ScopeType> {
        self.scopes.get(&self.current_scope_id).map(|s| &s.scope_type)
    }

    /// Clear all scopes and reset
    pub fn clear(&mut self) {
        self.scopes.clear();
        self.current_scope_id = 0;
        self.scope_counter = 1;
        self.symbol_types.clear();

        let global_scope = Scope::new(0, ScopeType::Global);
        self.scopes.insert(0, global_scope);
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_info_primitive() {
        let int_type = TypeInfo::Primitive("int".to_string());
        assert_eq!(int_type.to_string(), "int");
    }

    #[test]
    fn test_type_info_array() {
        let array_type = TypeInfo::Array(Box::new(TypeInfo::Primitive("int".to_string())));
        assert_eq!(array_type.to_string(), "int[]");
    }

    #[test]
    fn test_type_info_compatibility() {
        let int1 = TypeInfo::Primitive("int".to_string());
        let int2 = TypeInfo::Primitive("int".to_string());
        let string = TypeInfo::Primitive("string".to_string());

        assert!(int1.is_compatible_with(&int2));
        assert!(!int1.is_compatible_with(&string));
    }

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();
        assert_eq!(table.current_scope_id, 0);
    }

    #[test]
    fn test_symbol_table_define_symbol() {
        let mut table = SymbolTable::new();
        let result = table.define_symbol(
            "x".to_string(),
            1,
            TypeInfo::Primitive("int".to_string()),
        );

        assert!(result.is_ok());
        assert!(table.resolve_symbol("x").is_some());
    }

    #[test]
    fn test_symbol_table_scope_management() {
        let mut table = SymbolTable::new();

        // Enter function scope
        table.enter_scope(ScopeType::Function("foo".to_string()));
        table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

        // Exit scope
        assert!(table.exit_scope().is_ok());

        // Symbol should not be found in global scope
        assert!(table.resolve_symbol("x").is_none());
    }

    #[test]
    fn test_symbol_table_nested_scopes() {
        let mut table = SymbolTable::new();

        // Define in global scope
        table.define_symbol("global_x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

        // Enter function scope
        table.enter_scope(ScopeType::Function("foo".to_string()));
        table.define_symbol("local_x".to_string(), 2, TypeInfo::Primitive("string".to_string())).ok();

        // Should find both
        assert!(table.resolve_symbol("global_x").is_some());
        assert!(table.resolve_symbol("local_x").is_some());

        // Exit scope
        table.exit_scope().ok();

        // Should only find global
        assert!(table.resolve_symbol("global_x").is_some());
        assert!(table.resolve_symbol("local_x").is_none());
    }

    #[test]
    fn test_symbol_table_type_tracking() {
        let mut table = SymbolTable::new();

        table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

        let type_info = table.get_symbol_type("x");
        assert!(type_info.is_some());
        assert_eq!(type_info.unwrap(), &TypeInfo::Primitive("int".to_string()));
    }
}

