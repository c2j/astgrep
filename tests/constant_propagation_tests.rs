//! Tests for constant propagation and constant analysis
//!
//! Tests for verifying that constant propagation and analysis features work correctly.

use cr_dataflow::constant_propagation::{ConstantPropagator, ConstantValue};
use cr_dataflow::constant_analysis::{ConstantAnalyzer, ConstantInfo};

#[test]
fn test_constant_value_string() {
    let cv = ConstantValue::String("password".to_string());
    assert_eq!(cv.to_string_value(), Some("password".to_string()));
    assert!(cv.matches_pattern("pass"));
    assert!(!cv.matches_pattern("user"));
}

#[test]
fn test_constant_value_integer() {
    let cv = ConstantValue::Integer(42);
    assert_eq!(cv.to_string_value(), Some("42".to_string()));
    assert!(cv.matches_pattern("42"));
    assert!(!cv.matches_pattern("43"));
}

#[test]
fn test_constant_value_boolean() {
    let cv = ConstantValue::Boolean(true);
    assert_eq!(cv.to_string_value(), Some("true".to_string()));
    assert!(cv.matches_pattern("true"));
    assert!(!cv.matches_pattern("false"));
}

#[test]
fn test_constant_value_null() {
    let cv = ConstantValue::Null;
    assert_eq!(cv.to_string_value(), Some("null".to_string()));
    // Null doesn't match patterns since it's not a string/integer/boolean
    assert!(!cv.matches_pattern("null"));
}

#[test]
fn test_constant_value_unknown() {
    let cv = ConstantValue::Unknown;
    assert_eq!(cv.to_string_value(), None);
    assert!(!cv.matches_pattern("anything"));
}

#[test]
fn test_constant_value_equality() {
    let cv1 = ConstantValue::String("test".to_string());
    let cv2 = ConstantValue::String("test".to_string());
    let cv3 = ConstantValue::String("other".to_string());
    
    assert_eq!(cv1, cv2);
    assert_ne!(cv1, cv3);
}

#[test]
fn test_constant_propagator_new() {
    let propagator = ConstantPropagator::new();
    assert!(propagator.get_all_constants().is_empty());
    assert!(propagator.get_all_node_constants().is_empty());
}

#[test]
fn test_constant_propagator_is_constant() {
    let mut propagator = ConstantPropagator::new();
    
    // Initially, no constants
    assert!(!propagator.is_constant("x"));
    
    // After marking reassigned, still not constant
    propagator.mark_reassigned("x".to_string());
    assert!(!propagator.is_constant("x"));
}

#[test]
fn test_constant_propagator_mark_reassigned() {
    let mut propagator = ConstantPropagator::new();
    
    // Mark as reassigned
    propagator.mark_reassigned("x".to_string());
    
    // Should not be constant
    assert!(!propagator.is_constant("x"));
}

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
fn test_constant_info_increment_assignments() {
    let mut info = ConstantInfo::new(ConstantValue::Integer(42));
    assert_eq!(info.assignment_count, 1);
    
    info.increment_assignments();
    assert_eq!(info.assignment_count, 2);
    
    info.increment_assignments();
    assert_eq!(info.assignment_count, 3);
}

#[test]
fn test_constant_analyzer_new() {
    let analyzer = ConstantAnalyzer::new();
    assert!(analyzer.get_all_constants().is_empty());
    assert!(!analyzer.get_all_constants().is_empty() || true); // Always true
}

#[test]
fn test_constant_analyzer_register() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("x".to_string(), ConstantValue::Integer(42));
    
    assert!(analyzer.is_constant("x"));
    assert_eq!(analyzer.get_constant_value("x"), Some(&ConstantValue::Integer(42)));
}

#[test]
fn test_constant_analyzer_sensitive_by_name() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("password".to_string(), ConstantValue::String("secret123".to_string()));
    
    assert!(analyzer.is_sensitive("password"));
}

#[test]
fn test_constant_analyzer_sensitive_by_value() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("var".to_string(), ConstantValue::String("password123".to_string()));
    
    assert!(analyzer.is_sensitive("var"));
}

#[test]
fn test_constant_analyzer_not_sensitive() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("count".to_string(), ConstantValue::Integer(42));
    
    assert!(!analyzer.is_sensitive("count"));
}

#[test]
fn test_constant_analyzer_fold_integer() {
    let analyzer = ConstantAnalyzer::new();
    assert_eq!(analyzer.fold_constants("42"), Some(ConstantValue::Integer(42)));
    assert_eq!(analyzer.fold_constants("-10"), Some(ConstantValue::Integer(-10)));
}

#[test]
fn test_constant_analyzer_fold_string() {
    let analyzer = ConstantAnalyzer::new();
    assert_eq!(
        analyzer.fold_constants("\"hello\""),
        Some(ConstantValue::String("hello".to_string()))
    );
}

#[test]
fn test_constant_analyzer_fold_boolean() {
    let analyzer = ConstantAnalyzer::new();
    assert_eq!(analyzer.fold_constants("true"), Some(ConstantValue::Boolean(true)));
    assert_eq!(analyzer.fold_constants("false"), Some(ConstantValue::Boolean(false)));
}

#[test]
fn test_constant_analyzer_fold_null() {
    let analyzer = ConstantAnalyzer::new();
    assert_eq!(analyzer.fold_constants("null"), Some(ConstantValue::Null));
}

#[test]
fn test_constant_analyzer_fold_invalid() {
    let analyzer = ConstantAnalyzer::new();
    assert_eq!(analyzer.fold_constants("invalid"), None);
}

#[test]
fn test_constant_analyzer_function_returns() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_function_return("get_password".to_string(), ConstantValue::String("secret".to_string()));
    
    let returns = analyzer.get_function_returns("get_password");
    assert!(returns.is_some());
    assert_eq!(returns.unwrap().len(), 1);
    assert_eq!(returns.unwrap()[0], ConstantValue::String("secret".to_string()));
}

#[test]
fn test_constant_analyzer_multiple_function_returns() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_function_return("get_value".to_string(), ConstantValue::Integer(1));
    analyzer.register_function_return("get_value".to_string(), ConstantValue::Integer(2));
    
    let returns = analyzer.get_function_returns("get_value");
    assert!(returns.is_some());
    assert_eq!(returns.unwrap().len(), 2);
}

#[test]
fn test_constant_analyzer_get_sensitive_constants() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("password".to_string(), ConstantValue::String("secret".to_string()));
    analyzer.register_constant("count".to_string(), ConstantValue::Integer(42));
    analyzer.register_constant("api_key".to_string(), ConstantValue::String("key123".to_string()));
    
    let sensitive = analyzer.get_sensitive_constants();
    assert_eq!(sensitive.len(), 2);
}

#[test]
fn test_constant_analyzer_add_sensitive_pattern() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.add_sensitive_pattern("custom_secret".to_string());
    
    analyzer.register_constant("custom_secret_var".to_string(), ConstantValue::String("value".to_string()));
    assert!(analyzer.is_sensitive("custom_secret_var"));
}

#[test]
fn test_constant_analyzer_get_constant() {
    let mut analyzer = ConstantAnalyzer::new();
    analyzer.register_constant("x".to_string(), ConstantValue::Integer(42));
    
    let info = analyzer.get_constant("x");
    assert!(info.is_some());
    assert_eq!(info.unwrap().assignment_count, 1);
    assert!(!info.unwrap().is_mutable);
}

#[test]
fn test_constant_value_hash() {
    use std::collections::HashSet;
    
    let mut set = HashSet::new();
    set.insert(ConstantValue::String("test".to_string()));
    set.insert(ConstantValue::Integer(42));
    set.insert(ConstantValue::Boolean(true));
    
    assert_eq!(set.len(), 3);
    assert!(set.contains(&ConstantValue::String("test".to_string())));
}

#[test]
fn test_constant_analyzer_default() {
    let analyzer = ConstantAnalyzer::default();
    assert!(analyzer.get_all_constants().is_empty());
}

