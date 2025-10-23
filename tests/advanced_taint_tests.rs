//! Tests for advanced taint propagation and analysis
//!
//! Tests for transformation tracking, taint merging, and context-aware analysis.

use astgrep_dataflow::advanced_taint::{AdvancedTaintAnalyzer, AdvancedTaintState, TaintTransformation};
use astgrep_dataflow::taint::TaintState;

#[test]
fn test_taint_transformation_identity() {
    let identity = TaintTransformation::Identity;
    assert!(!identity.sanitizes());
    assert_eq!(identity.effectiveness(), 0.0);
}

#[test]
fn test_taint_transformation_hashing() {
    let hash = TaintTransformation::Hashing("SHA256".to_string());
    assert!(hash.sanitizes());
    assert_eq!(hash.effectiveness(), 1.0);
}

#[test]
fn test_taint_transformation_encryption() {
    let encrypt = TaintTransformation::Encryption("AES".to_string());
    assert!(encrypt.sanitizes());
    assert_eq!(encrypt.effectiveness(), 0.95);
}

#[test]
fn test_taint_transformation_encoding() {
    let encoding = TaintTransformation::Encoding("base64".to_string());
    assert!(encoding.sanitizes());
    assert_eq!(encoding.effectiveness(), 0.7);
}

#[test]
fn test_taint_transformation_decoding() {
    let decoding = TaintTransformation::Decoding("base64".to_string());
    assert!(!decoding.sanitizes());
    assert_eq!(decoding.effectiveness(), 0.0);
}

#[test]
fn test_taint_transformation_concatenation() {
    let concat = TaintTransformation::Concatenation;
    assert!(!concat.sanitizes());
    assert_eq!(concat.effectiveness(), 0.0);
}

#[test]
fn test_taint_transformation_method_call() {
    let method = TaintTransformation::MethodCall("substring".to_string());
    assert!(!method.sanitizes());
    assert_eq!(method.effectiveness(), 0.0);
}

#[test]
fn test_advanced_taint_state_creation() {
    let base_taint = TaintState::default();
    let advanced = AdvancedTaintState::new(base_taint);

    assert_eq!(advanced.transformations.len(), 0);
    assert_eq!(advanced.confidence, 1.0);
    assert!(!advanced.is_sanitized());
}

#[test]
fn test_advanced_taint_state_add_single_transformation() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Concatenation);
    assert_eq!(advanced.transformations.len(), 1);
}

#[test]
fn test_advanced_taint_state_add_multiple_transformations() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Concatenation);
    advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));
    advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));

    assert_eq!(advanced.transformations.len(), 3);
}

#[test]
fn test_advanced_taint_state_effective_confidence_no_transformation() {
    let base_taint = TaintState::default();
    let advanced = AdvancedTaintState::new(base_taint);

    assert_eq!(advanced.effective_confidence(), 1.0);
}

#[test]
fn test_advanced_taint_state_effective_confidence_with_encoding() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));
    let effective = advanced.effective_confidence();

    assert!(effective < 1.0);
    assert!(effective > 0.0);
    assert_eq!(effective, 0.3); // 1.0 * (1.0 - 0.7)
}

#[test]
fn test_advanced_taint_state_effective_confidence_with_hashing() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));
    let effective = advanced.effective_confidence();

    assert_eq!(effective, 0.0); // 1.0 * (1.0 - 1.0)
}

#[test]
fn test_advanced_taint_state_is_sanitized_with_hash() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));
    assert!(advanced.is_sanitized());
}

#[test]
fn test_advanced_taint_state_is_sanitized_with_encoding() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));
    assert!(!advanced.is_sanitized()); // 0.3 > 0.1
}

#[test]
fn test_advanced_taint_state_context() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    advanced.set_context("source".to_string(), "user_input".to_string());
    advanced.set_context("sink".to_string(), "sql_query".to_string());

    assert_eq!(advanced.get_context("source"), Some("user_input"));
    assert_eq!(advanced.get_context("sink"), Some("sql_query"));
    assert_eq!(advanced.get_context("unknown"), None);
}

#[test]
fn test_advanced_taint_analyzer_creation() {
    let _analyzer = AdvancedTaintAnalyzer::new();
    // Verify analyzer is created successfully
    // (transformation_rules are private, but we can test functionality)
}

#[test]
fn test_advanced_taint_analyzer_merge_taints_single() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();
    let taint = AdvancedTaintState::new(base_taint);

    let merged = analyzer.merge_taints(&[taint]);
    assert_eq!(merged.transformations.len(), 0);
    assert_eq!(merged.confidence, 1.0);
}

#[test]
fn test_advanced_taint_analyzer_merge_taints_multiple() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();

    let mut taint1 = AdvancedTaintState::new(base_taint.clone());
    taint1.add_transformation(TaintTransformation::Concatenation);
    taint1.confidence = 0.9;

    let mut taint2 = AdvancedTaintState::new(base_taint);
    taint2.add_transformation(TaintTransformation::Encoding("base64".to_string()));
    taint2.confidence = 0.8;

    let merged = analyzer.merge_taints(&[taint1, taint2]);
    assert_eq!(merged.transformations.len(), 2);
    assert_eq!(merged.confidence, 0.85); // (0.9 + 0.8) / 2
}

#[test]
fn test_advanced_taint_analyzer_merge_taints_empty() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let merged = analyzer.merge_taints(&[]);

    assert_eq!(merged.transformations.len(), 0);
    assert_eq!(merged.confidence, 1.0);
}

#[test]
fn test_advanced_taint_analyzer_split_taint() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();
    let taint = AdvancedTaintState::new(base_taint);

    let (true_branch, false_branch) = analyzer.split_taint(&taint, "x > 0");

    assert_eq!(true_branch.get_context("branch"), Some("true"));
    assert_eq!(true_branch.get_context("condition"), Some("x > 0"));

    assert_eq!(false_branch.get_context("branch"), Some("false"));
    assert_eq!(false_branch.get_context("condition"), Some("x > 0"));
}

#[test]
fn test_advanced_taint_analyzer_split_taint_preserves_transformations() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();
    let mut taint = AdvancedTaintState::new(base_taint);

    taint.add_transformation(TaintTransformation::Concatenation);
    taint.confidence = 0.8;

    let (true_branch, false_branch) = analyzer.split_taint(&taint, "x > 0");

    assert_eq!(true_branch.transformations.len(), 1);
    assert_eq!(true_branch.confidence, 0.8);

    assert_eq!(false_branch.transformations.len(), 1);
    assert_eq!(false_branch.confidence, 0.8);
}

#[test]
fn test_advanced_taint_analyzer_set_and_get_taint_state() {
    let mut analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();
    let taint = AdvancedTaintState::new(base_taint);

    analyzer.set_taint_state(0, taint);
    assert!(analyzer.get_taint_state(0).is_some());
    assert!(analyzer.get_taint_state(1).is_none());
}

#[test]
fn test_advanced_taint_analyzer_clear() {
    let mut analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();
    let taint = AdvancedTaintState::new(base_taint);

    analyzer.set_taint_state(0, taint);
    assert!(analyzer.get_taint_state(0).is_some());

    analyzer.clear();
    assert!(analyzer.get_taint_state(0).is_none());
}

#[test]
fn test_complex_transformation_chain() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    // Simulate: user_input -> concatenation -> encoding -> hashing
    advanced.add_transformation(TaintTransformation::Concatenation);
    advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));
    advanced.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));

    // Effective confidence: 1.0 * (1.0 - 0.0) * (1.0 - 0.7) * (1.0 - 1.0) = 0.0
    assert_eq!(advanced.effective_confidence(), 0.0);
    assert!(advanced.is_sanitized());
}

#[test]
fn test_partial_sanitization() {
    let base_taint = TaintState::default();
    let mut advanced = AdvancedTaintState::new(base_taint);

    // Simulate: user_input -> encoding (partial sanitization)
    advanced.add_transformation(TaintTransformation::Encoding("base64".to_string()));

    // Effective confidence: 1.0 * (1.0 - 0.7) = 0.3
    assert_eq!(advanced.effective_confidence(), 0.3);
    assert!(!advanced.is_sanitized()); // 0.3 > 0.1
}

#[test]
fn test_filter_by_context() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();

    let mut taint1 = AdvancedTaintState::new(base_taint.clone());
    taint1.set_context("branch".to_string(), "true".to_string());

    let mut taint2 = AdvancedTaintState::new(base_taint);
    taint2.set_context("branch".to_string(), "false".to_string());

    let filtered = analyzer.filter_by_context(&[taint1, taint2], "branch", "true");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].get_context("branch"), Some("true"));
}

#[test]
fn test_combine_with_context_union() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();

    let mut taint1 = AdvancedTaintState::new(base_taint.clone());
    taint1.add_transformation(TaintTransformation::Concatenation);

    let mut taint2 = AdvancedTaintState::new(base_taint);
    taint2.add_transformation(TaintTransformation::Encoding("base64".to_string()));

    let combined = analyzer.combine_with_context(&[taint1, taint2], "union");
    assert_eq!(combined.transformations.len(), 2);
}

#[test]
fn test_combine_with_context_intersection() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();

    let mut taint1 = AdvancedTaintState::new(base_taint.clone());
    taint1.add_transformation(TaintTransformation::Concatenation);
    taint1.add_transformation(TaintTransformation::Encoding("base64".to_string()));

    let mut taint2 = AdvancedTaintState::new(base_taint);
    taint2.add_transformation(TaintTransformation::Encoding("base64".to_string()));

    let combined = analyzer.combine_with_context(&[taint1, taint2], "intersection");
    assert_eq!(combined.transformations.len(), 1);
}

#[test]
fn test_combine_with_context_most_restrictive() {
    let analyzer = AdvancedTaintAnalyzer::new();
    let base_taint = TaintState::default();

    let mut taint1 = AdvancedTaintState::new(base_taint.clone());
    taint1.add_transformation(TaintTransformation::Encoding("base64".to_string()));

    let mut taint2 = AdvancedTaintState::new(base_taint);
    taint2.add_transformation(TaintTransformation::Hashing("SHA256".to_string()));

    let combined = analyzer.combine_with_context(&[taint1, taint2], "most_restrictive");
    // Most restrictive should be the one with hashing (effectiveness 1.0)
    assert_eq!(combined.transformations.len(), 1);
}

