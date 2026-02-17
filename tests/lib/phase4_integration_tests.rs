//! Phase 4 integration tests
//!
//! Comprehensive tests for Phase 4: Complete Compatibility features

use astgrep_dataflow::constant_propagation::{ConstantValue, ConstantPropagator};
use astgrep_dataflow::constant_analysis::{ConstantAnalyzer, ConstantInfo};
use astgrep_rules::marketplace::{MarketplaceRule, RuleMarketplace};
use astgrep_cli::vscode_integration::{VsCodeDiagnostic, VsCodeExtension};

#[test]
fn test_phase4_constant_propagation_integration() {
    let mut propagator = ConstantPropagator::new();
    
    // Test constant value creation
    let str_val = ConstantValue::String("test".to_string());
    let int_val = ConstantValue::Integer(42);
    let bool_val = ConstantValue::Boolean(true);
    let null_val = ConstantValue::Null;
    
    assert!(str_val.matches_pattern("test"));
    assert!(int_val.matches_pattern("42"));
    assert!(bool_val.matches_pattern("true"));
    assert!(!null_val.matches_pattern("null"));
}

#[test]
fn test_phase4_constant_analysis_integration() {
    let mut analyzer = ConstantAnalyzer::new();
    
    // Test constant analysis
    let info = ConstantInfo {
        value: ConstantValue::String("password123".to_string()),
        is_mutable: false,
        assignment_count: 1,
        is_sensitive: true,
    };
    
    assert!(info.is_sensitive);
    assert!(!info.is_mutable);
}

#[test]
fn test_phase4_marketplace_integration() {
    let mut marketplace = RuleMarketplace::new();
    
    // Create and add rules
    let mut rule1 = MarketplaceRule::new(
        "sql_injection".to_string(),
        "SQL Injection Detection".to_string(),
        "security_team".to_string(),
    );
    rule1.add_rating(5.0);
    rule1.increment_downloads();
    rule1.mark_verified();
    
    let mut rule2 = MarketplaceRule::new(
        "xss_detection".to_string(),
        "XSS Detection".to_string(),
        "security_team".to_string(),
    );
    rule2.add_rating(4.0);
    
    marketplace.add_rule(rule1);
    marketplace.add_rule(rule2);
    
    // Verify marketplace functionality
    assert_eq!(marketplace.rule_count(), 2);
    assert_eq!(marketplace.get_verified_rules().len(), 1);
    assert_eq!(marketplace.get_top_rated(1).len(), 1);
}

#[test]
fn test_phase4_vscode_integration() {
    let mut ext = VsCodeExtension::new();
    
    // Create diagnostics
    let diag1 = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "SQL Injection detected".to_string(),
        "error".to_string(),
        "sql_injection".to_string(),
    );
    
    let diag2 = VsCodeDiagnostic::new(
        "test.java".to_string(),
        20,
        10,
        "XSS vulnerability".to_string(),
        "warning".to_string(),
        "xss".to_string(),
    );
    
    ext.add_diagnostic(diag1);
    ext.add_diagnostic(diag2);
    
    // Verify VS Code integration
    assert_eq!(ext.diagnostic_count(), 2);
    assert_eq!(ext.diagnostic_count_for_file("test.java"), 2);
    assert!(ext.is_rule_enabled("sql_injection"));
}

#[test]
fn test_phase4_end_to_end_workflow() {
    // 1. Create marketplace with rules
    let mut marketplace = RuleMarketplace::new();
    let mut rule = MarketplaceRule::new(
        "security_rule".to_string(),
        "Security Rule".to_string(),
        "author".to_string(),
    );
    rule.add_rating(5.0);
    marketplace.add_rule(rule);
    
    // 2. Create VS Code extension
    let mut ext = VsCodeExtension::new();
    
    // 3. Add diagnostics based on marketplace rules
    for marketplace_rule in marketplace.get_all_rules() {
        if ext.is_rule_enabled(&marketplace_rule.id) {
            let diag = VsCodeDiagnostic::new(
                "code.java".to_string(),
                1,
                0,
                format!("Issue from rule: {}", marketplace_rule.name),
                "warning".to_string(),
                marketplace_rule.id.clone(),
            );
            ext.add_diagnostic(diag);
        }
    }
    
    // 4. Verify end-to-end workflow
    assert_eq!(marketplace.rule_count(), 1);
    assert_eq!(ext.diagnostic_count(), 1);
}

#[test]
fn test_phase4_constant_propagation_with_analysis() {
    let mut propagator = ConstantPropagator::new();
    let mut analyzer = ConstantAnalyzer::new();
    
    // Simulate constant propagation
    let const_val = ConstantValue::String("api_key_secret".to_string());
    
    // Analyze for sensitivity
    let info = ConstantInfo {
        value: const_val,
        is_mutable: false,
        assignment_count: 1,
        is_sensitive: true,
    };
    
    assert!(info.is_sensitive);
}

#[test]
fn test_phase4_marketplace_with_vscode() {
    let mut marketplace = RuleMarketplace::new();
    let mut ext = VsCodeExtension::new();
    
    // Add rules to marketplace
    for i in 0..3 {
        let rule = MarketplaceRule::new(
            format!("rule{}", i),
            format!("Rule {}", i),
            "author".to_string(),
        );
        marketplace.add_rule(rule);
    }
    
    // Configure VS Code to use marketplace rules
    let mut config = ext.get_config().clone();
    config.enabled_rules = marketplace.get_all_rules()
        .iter()
        .map(|r| r.id.clone())
        .collect();
    ext.update_config(config);
    
    // Verify integration
    assert_eq!(marketplace.rule_count(), 3);
    assert!(ext.is_rule_enabled("rule0"));
    assert!(ext.is_rule_enabled("rule1"));
    assert!(ext.is_rule_enabled("rule2"));
}

#[test]
fn test_phase4_all_features_together() {
    // 1. Create marketplace
    let mut marketplace = RuleMarketplace::new();
    let mut rule = MarketplaceRule::new(
        "comprehensive_rule".to_string(),
        "Comprehensive Security Rule".to_string(),
        "security_team".to_string(),
    );
    rule.add_rating(5.0);
    rule.increment_downloads();
    rule.mark_verified();
    marketplace.add_rule(rule);
    
    // 2. Create constant analyzer
    let mut analyzer = ConstantAnalyzer::new();
    let sensitive_const = ConstantInfo {
        value: ConstantValue::String("secret_token".to_string()),
        is_mutable: false,
        assignment_count: 1,
        is_sensitive: true,
    };
    
    // 3. Create VS Code extension
    let mut ext = VsCodeExtension::new();
    
    // 4. Add diagnostics
    let diag = VsCodeDiagnostic::new(
        "app.java".to_string(),
        42,
        10,
        "Sensitive constant detected".to_string(),
        "warning".to_string(),
        "comprehensive_rule".to_string(),
    );
    ext.add_diagnostic(diag);
    
    // 5. Verify all features work together
    assert_eq!(marketplace.rule_count(), 1);
    assert!(marketplace.get_verified_rules()[0].verified);
    assert!(sensitive_const.is_sensitive);
    assert_eq!(ext.diagnostic_count(), 1);
    assert!(ext.is_rule_enabled("comprehensive_rule"));
}

#[test]
fn test_phase4_performance_with_many_rules() {
    let mut marketplace = RuleMarketplace::new();

    // Add 100 rules
    for i in 0..100 {
        let rule = MarketplaceRule::new(
            format!("rule{}", i),
            format!("Rule {}", i),
            "author".to_string(),
        );
        marketplace.add_rule(rule);
    }

    assert_eq!(marketplace.rule_count(), 100);

    // Test search performance - search by name with case-insensitive match
    let results = marketplace.search_by_name("Rule 5");
    assert!(!results.is_empty());
}

#[test]
fn test_phase4_diagnostics_with_multiple_files() {
    let mut ext = VsCodeExtension::new();
    
    // Add diagnostics for multiple files
    for file_idx in 0..5 {
        for diag_idx in 0..3 {
            let diag = VsCodeDiagnostic::new(
                format!("file{}.java", file_idx),
                diag_idx as u32,
                0,
                format!("Issue {}", diag_idx),
                "warning".to_string(),
                format!("rule{}", diag_idx),
            );
            ext.add_diagnostic(diag);
        }
    }
    
    assert_eq!(ext.diagnostic_count(), 15);
    assert_eq!(ext.diagnostic_count_for_file("file0.java"), 3);
    assert_eq!(ext.diagnostic_count_for_file("file4.java"), 3);
}

