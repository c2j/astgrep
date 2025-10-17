//! Tests for VS Code IDE integration
//!
//! Tests for verifying that VS Code integration features work correctly.

use cr_cli::vscode_integration::{VsCodeDiagnostic, VsCodeConfig, VsCodeExtension};

#[test]
fn test_vscode_diagnostic_new() {
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "SQL Injection detected".to_string(),
        "error".to_string(),
        "sql_injection".to_string(),
    );

    assert_eq!(diag.file, "test.java");
    assert_eq!(diag.line, 10);
    assert_eq!(diag.column, 5);
    assert_eq!(diag.message, "SQL Injection detected");
    assert_eq!(diag.severity, "error");
    assert_eq!(diag.rule_id, "sql_injection");
    assert_eq!(diag.end_line, 10);
    assert_eq!(diag.end_column, 6);
}

#[test]
fn test_vscode_diagnostic_with_end_position() {
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    )
    .with_end_position(10, 20);

    assert_eq!(diag.end_line, 10);
    assert_eq!(diag.end_column, 20);
}

#[test]
fn test_vscode_diagnostic_to_vscode_format() {
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );

    let json = diag.to_vscode_format();
    assert!(json.get("range").is_some());
    assert!(json.get("message").is_some());
    assert!(json.get("severity").is_some());
    assert!(json.get("source").is_some());
    assert!(json.get("code").is_some());
}

#[test]
fn test_vscode_config_default() {
    let config = VsCodeConfig::default();
    assert!(config.enabled);
    assert!(config.auto_run_on_save);
    assert!(config.show_inline_diagnostics);
    assert_eq!(config.highlight_severity, "warning");
    assert!(!config.file_patterns.is_empty());
    assert!(!config.exclude_patterns.is_empty());
}

#[test]
fn test_vscode_extension_new() {
    let ext = VsCodeExtension::new();
    assert!(ext.get_config().enabled);
    assert_eq!(ext.diagnostic_count(), 0);
}

#[test]
fn test_vscode_extension_with_config() {
    let mut config = VsCodeConfig::default();
    config.enabled = false;
    
    let ext = VsCodeExtension::with_config(config);
    assert!(!ext.get_config().enabled);
}

#[test]
fn test_vscode_extension_add_diagnostic() {
    let mut ext = VsCodeExtension::new();
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );

    ext.add_diagnostic(diag);
    assert_eq!(ext.diagnostic_count(), 1);
}

#[test]
fn test_vscode_extension_add_multiple_diagnostics() {
    let mut ext = VsCodeExtension::new();
    
    for i in 0..5 {
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            i,
            0,
            format!("Error {}", i),
            "error".to_string(),
            format!("rule{}", i),
        );
        ext.add_diagnostic(diag);
    }

    assert_eq!(ext.diagnostic_count(), 5);
}

#[test]
fn test_vscode_extension_get_diagnostics() {
    let mut ext = VsCodeExtension::new();
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );

    ext.add_diagnostic(diag);
    
    let diags = ext.get_diagnostics("test.java");
    assert!(diags.is_some());
    assert_eq!(diags.unwrap().len(), 1);
}

#[test]
fn test_vscode_extension_clear_diagnostics() {
    let mut ext = VsCodeExtension::new();
    let diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        10,
        5,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );

    ext.add_diagnostic(diag);
    assert_eq!(ext.diagnostic_count(), 1);

    ext.clear_diagnostics("test.java");
    assert_eq!(ext.diagnostic_count(), 0);
}

#[test]
fn test_vscode_extension_clear_all_diagnostics() {
    let mut ext = VsCodeExtension::new();
    
    for i in 0..3 {
        let diag = VsCodeDiagnostic::new(
            format!("test{}.java", i),
            10,
            5,
            "Error".to_string(),
            "error".to_string(),
            "rule1".to_string(),
        );
        ext.add_diagnostic(diag);
    }

    assert_eq!(ext.diagnostic_count(), 3);
    ext.clear_all_diagnostics();
    assert_eq!(ext.diagnostic_count(), 0);
}

#[test]
fn test_vscode_extension_should_analyze_file() {
    let ext = VsCodeExtension::new();

    // Files in excluded directories should not be analyzed
    assert!(!ext.should_analyze_file("node_modules/test.js"));
    assert!(!ext.should_analyze_file(".git/test.java"));
    assert!(!ext.should_analyze_file("target/test.java"));
}

#[test]
fn test_vscode_extension_should_analyze_file_disabled() {
    let mut config = VsCodeConfig::default();
    config.enabled = false;
    
    let ext = VsCodeExtension::with_config(config);
    assert!(!ext.should_analyze_file("test.java"));
}

#[test]
fn test_vscode_extension_is_rule_enabled() {
    let ext = VsCodeExtension::new();
    assert!(ext.is_rule_enabled("any_rule"));
}

#[test]
fn test_vscode_extension_is_rule_disabled() {
    let mut config = VsCodeConfig::default();
    config.disabled_rules = vec!["sql_injection".to_string()];
    
    let ext = VsCodeExtension::with_config(config);
    assert!(!ext.is_rule_enabled("sql_injection"));
    assert!(ext.is_rule_enabled("other_rule"));
}

#[test]
fn test_vscode_extension_is_rule_enabled_whitelist() {
    let mut config = VsCodeConfig::default();
    config.enabled_rules = vec!["sql_injection".to_string(), "xss".to_string()];
    
    let ext = VsCodeExtension::with_config(config);
    assert!(ext.is_rule_enabled("sql_injection"));
    assert!(ext.is_rule_enabled("xss"));
    assert!(!ext.is_rule_enabled("other_rule"));
}

#[test]
fn test_vscode_extension_diagnostic_count_for_file() {
    let mut ext = VsCodeExtension::new();
    
    for i in 0..3 {
        let diag = VsCodeDiagnostic::new(
            "test.java".to_string(),
            i,
            0,
            "Error".to_string(),
            "error".to_string(),
            "rule1".to_string(),
        );
        ext.add_diagnostic(diag);
    }

    assert_eq!(ext.diagnostic_count_for_file("test.java"), 3);
    assert_eq!(ext.diagnostic_count_for_file("other.java"), 0);
}

#[test]
fn test_vscode_extension_get_all_diagnostics() {
    let mut ext = VsCodeExtension::new();
    
    let diag1 = VsCodeDiagnostic::new(
        "test1.java".to_string(),
        10,
        5,
        "Error 1".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );
    
    let diag2 = VsCodeDiagnostic::new(
        "test2.java".to_string(),
        20,
        10,
        "Error 2".to_string(),
        "warning".to_string(),
        "rule2".to_string(),
    );

    ext.add_diagnostic(diag1);
    ext.add_diagnostic(diag2);
    
    let all = ext.get_all_diagnostics();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_vscode_extension_update_config() {
    let mut ext = VsCodeExtension::new();
    assert!(ext.get_config().enabled);
    
    let mut new_config = VsCodeConfig::default();
    new_config.enabled = false;
    
    ext.update_config(new_config);
    assert!(!ext.get_config().enabled);
}

#[test]
fn test_vscode_extension_default() {
    let ext = VsCodeExtension::default();
    assert!(ext.get_config().enabled);
    assert_eq!(ext.diagnostic_count(), 0);
}

#[test]
fn test_vscode_diagnostic_severity_levels() {
    let error_diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        0,
        0,
        "Error".to_string(),
        "error".to_string(),
        "rule1".to_string(),
    );
    
    let warning_diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        0,
        0,
        "Warning".to_string(),
        "warning".to_string(),
        "rule1".to_string(),
    );
    
    let info_diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        0,
        0,
        "Info".to_string(),
        "information".to_string(),
        "rule1".to_string(),
    );
    
    let hint_diag = VsCodeDiagnostic::new(
        "test.java".to_string(),
        0,
        0,
        "Hint".to_string(),
        "hint".to_string(),
        "rule1".to_string(),
    );

    assert_eq!(error_diag.to_vscode_format()["severity"], 1);
    assert_eq!(warning_diag.to_vscode_format()["severity"], 2);
    assert_eq!(info_diag.to_vscode_format()["severity"], 3);
    assert_eq!(hint_diag.to_vscode_format()["severity"], 4);
}

