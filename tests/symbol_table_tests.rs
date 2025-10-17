//! Tests for symbol table and type inference
//!
//! Tests for symbol management, type tracking, and scope handling.

use cr_dataflow::symbol_table::{SymbolTable, TypeInfo, ScopeType, Symbol};

#[test]
fn test_type_info_primitive() {
    let int_type = TypeInfo::Primitive("int".to_string());
    assert_eq!(int_type.to_string(), "int");
}

#[test]
fn test_type_info_object() {
    let obj_type = TypeInfo::Object("MyClass".to_string());
    assert_eq!(obj_type.to_string(), "MyClass");
}

#[test]
fn test_type_info_array() {
    let array_type = TypeInfo::Array(Box::new(TypeInfo::Primitive("int".to_string())));
    assert_eq!(array_type.to_string(), "int[]");
}

#[test]
fn test_type_info_nested_array() {
    let nested = TypeInfo::Array(Box::new(
        TypeInfo::Array(Box::new(TypeInfo::Primitive("string".to_string())))
    ));
    assert_eq!(nested.to_string(), "string[][]");
}

#[test]
fn test_type_info_function() {
    let func_type = TypeInfo::Function {
        params: vec![
            TypeInfo::Primitive("int".to_string()),
            TypeInfo::Primitive("string".to_string()),
        ],
        return_type: Box::new(TypeInfo::Primitive("bool".to_string())),
    };
    assert_eq!(func_type.to_string(), "(int, string) -> bool");
}

#[test]
fn test_type_info_union() {
    let union_type = TypeInfo::Union(vec![
        TypeInfo::Primitive("int".to_string()),
        TypeInfo::Primitive("string".to_string()),
    ]);
    assert_eq!(union_type.to_string(), "int | string");
}

#[test]
fn test_type_info_unknown() {
    let unknown = TypeInfo::Unknown;
    assert_eq!(unknown.to_string(), "unknown");
}

#[test]
fn test_type_info_compatibility_same_primitive() {
    let int1 = TypeInfo::Primitive("int".to_string());
    let int2 = TypeInfo::Primitive("int".to_string());
    assert!(int1.is_compatible_with(&int2));
}

#[test]
fn test_type_info_compatibility_different_primitive() {
    let int = TypeInfo::Primitive("int".to_string());
    let string = TypeInfo::Primitive("string".to_string());
    assert!(!int.is_compatible_with(&string));
}

#[test]
fn test_type_info_compatibility_with_unknown() {
    let int = TypeInfo::Primitive("int".to_string());
    let unknown = TypeInfo::Unknown;
    assert!(int.is_compatible_with(&unknown));
    assert!(unknown.is_compatible_with(&int));
}

#[test]
fn test_type_info_compatibility_array() {
    let array1 = TypeInfo::Array(Box::new(TypeInfo::Primitive("int".to_string())));
    let array2 = TypeInfo::Array(Box::new(TypeInfo::Primitive("int".to_string())));
    assert!(array1.is_compatible_with(&array2));
}

#[test]
fn test_type_info_compatibility_union() {
    let union = TypeInfo::Union(vec![
        TypeInfo::Primitive("int".to_string()),
        TypeInfo::Primitive("string".to_string()),
    ]);
    let int = TypeInfo::Primitive("int".to_string());
    assert!(union.is_compatible_with(&int));
    assert!(int.is_compatible_with(&union));
}

#[test]
fn test_symbol_table_creation() {
    let table = SymbolTable::new();
    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Global));
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
fn test_symbol_table_resolve_symbol() {
    let mut table = SymbolTable::new();
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    let symbol = table.resolve_symbol("x");
    assert!(symbol.is_some());
    assert_eq!(symbol.unwrap().name, "x");
    assert_eq!(symbol.unwrap().node_id, 1);
}

#[test]
fn test_symbol_table_resolve_nonexistent_symbol() {
    let table = SymbolTable::new();
    assert!(table.resolve_symbol("nonexistent").is_none());
}

#[test]
fn test_symbol_table_get_symbol_type() {
    let mut table = SymbolTable::new();
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    let type_info = table.get_symbol_type("x");
    assert!(type_info.is_some());
    assert_eq!(type_info.unwrap(), &TypeInfo::Primitive("int".to_string()));
}

#[test]
fn test_symbol_table_update_symbol_type() {
    let mut table = SymbolTable::new();
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    table.update_symbol_type("x".to_string(), TypeInfo::Primitive("string".to_string()));

    let type_info = table.get_symbol_type("x");
    assert_eq!(type_info.unwrap(), &TypeInfo::Primitive("string".to_string()));
}

#[test]
fn test_symbol_table_enter_scope() {
    let mut table = SymbolTable::new();
    let scope_id = table.enter_scope(ScopeType::Function("foo".to_string()));

    assert_eq!(scope_id, 1);
    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Function("foo".to_string())));
}

#[test]
fn test_symbol_table_exit_scope() {
    let mut table = SymbolTable::new();
    table.enter_scope(ScopeType::Function("foo".to_string()));

    let result = table.exit_scope();
    assert!(result.is_ok());
    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Global));
}

#[test]
fn test_symbol_table_exit_global_scope_fails() {
    let mut table = SymbolTable::new();
    let result = table.exit_scope();
    assert!(result.is_err());
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
fn test_symbol_table_deeply_nested_scopes() {
    let mut table = SymbolTable::new();

    // Global scope
    table.define_symbol("global".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    // Function scope
    table.enter_scope(ScopeType::Function("foo".to_string()));
    table.define_symbol("func_var".to_string(), 2, TypeInfo::Primitive("string".to_string())).ok();

    // Block scope
    table.enter_scope(ScopeType::Block);
    table.define_symbol("block_var".to_string(), 3, TypeInfo::Primitive("bool".to_string())).ok();

    // Should find all three
    assert!(table.resolve_symbol("global").is_some());
    assert!(table.resolve_symbol("func_var").is_some());
    assert!(table.resolve_symbol("block_var").is_some());

    // Exit block scope
    table.exit_scope().ok();
    assert!(table.resolve_symbol("block_var").is_none());
    assert!(table.resolve_symbol("func_var").is_some());

    // Exit function scope
    table.exit_scope().ok();
    assert!(table.resolve_symbol("func_var").is_none());
    assert!(table.resolve_symbol("global").is_some());
}

#[test]
fn test_symbol_table_get_current_scope_symbols() {
    let mut table = SymbolTable::new();
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();
    table.define_symbol("y".to_string(), 2, TypeInfo::Primitive("string".to_string())).ok();

    let symbols = table.get_current_scope_symbols();
    assert!(symbols.is_some());
    assert_eq!(symbols.unwrap().len(), 2);
}

#[test]
fn test_symbol_table_clear() {
    let mut table = SymbolTable::new();
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    table.clear();

    assert!(table.resolve_symbol("x").is_none());
    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Global));
}

#[test]
fn test_symbol_table_class_scope() {
    let mut table = SymbolTable::new();

    // Enter class scope
    table.enter_scope(ScopeType::Class("MyClass".to_string()));
    table.define_symbol("member".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Class("MyClass".to_string())));
    assert!(table.resolve_symbol("member").is_some());
}

#[test]
fn test_symbol_table_loop_scope() {
    let mut table = SymbolTable::new();

    // Enter loop scope
    table.enter_scope(ScopeType::Loop);
    table.define_symbol("loop_var".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    assert_eq!(table.get_current_scope_type(), Some(&ScopeType::Loop));
    assert!(table.resolve_symbol("loop_var").is_some());
}

#[test]
fn test_symbol_shadowing() {
    let mut table = SymbolTable::new();

    // Define in global scope
    table.define_symbol("x".to_string(), 1, TypeInfo::Primitive("int".to_string())).ok();

    // Enter function scope and shadow
    table.enter_scope(ScopeType::Function("foo".to_string()));
    table.define_symbol("x".to_string(), 2, TypeInfo::Primitive("string".to_string())).ok();

    // Should resolve to the local one
    let symbol = table.resolve_symbol("x");
    assert_eq!(symbol.unwrap().node_id, 2);

    // Exit scope
    table.exit_scope().ok();

    // Should resolve to the global one
    let symbol = table.resolve_symbol("x");
    assert_eq!(symbol.unwrap().node_id, 1);
}

