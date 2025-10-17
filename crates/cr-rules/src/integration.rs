//! Integration module for demonstrating rule execution engine capabilities
//!
//! This module provides examples and integration tests for the rule execution engine.

use crate::*;
use cr_core::{AstNode, Language, Severity, Confidence};
use cr_ast::{AstBuilder, UniversalNode};
use std::path::Path;

/// Integration example for rule execution
pub struct RuleExecutionExample;

impl RuleExecutionExample {
    /// Create a sample rule for SQL injection detection
    pub fn create_sql_injection_rule() -> Rule {
        Rule {
            id: "sql-injection-001".to_string(),
            name: "SQL Injection Detection".to_string(),
            description: "Detects potential SQL injection vulnerabilities".to_string(),
            severity: Severity::Error,
            confidence: Confidence::High,
            languages: vec![Language::Java, Language::JavaScript, Language::Python],
            patterns: vec![
                Pattern::simple("executeQuery($QUERY)".to_string())
                    .add_condition(Condition::MetavariableRegex(MetavariableRegex::new(
                        "QUERY".to_string(),
                        r".*\+.*".to_string()
                    )))
            ],
            dataflow: Some(DataFlowSpec::new(
                vec!["user_input".to_string(), "request_parameter".to_string()],
                vec!["sql_execution".to_string(), "database_query".to_string()],
            ).with_sanitizers(vec!["sql_escape".to_string(), "prepared_statement".to_string()])),
            fix: Some("Use prepared statements instead of string concatenation".to_string()),
            fix_regex: None,
            paths: None,
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("category".to_string(), "security".to_string());
                metadata.insert("cwe".to_string(), "89".to_string());
                metadata.insert("owasp".to_string(), "A03:2021".to_string());
                metadata
            },
            enabled: true,
        }
    }

    /// Create a sample rule for XSS detection
    pub fn create_xss_rule() -> Rule {
        Rule {
            id: "xss-001".to_string(),
            name: "Cross-Site Scripting Detection".to_string(),
            description: "Detects potential XSS vulnerabilities".to_string(),
            severity: Severity::Warning,
            confidence: Confidence::Medium,
            languages: vec![Language::JavaScript, Language::Java],
            patterns: vec![
                Pattern::simple("innerHTML = $VALUE".to_string())
                    .add_condition(Condition::MetavariableRegex(MetavariableRegex::new(
                        "VALUE".to_string(),
                        r".*user.*".to_string()
                    )))
            ],
            dataflow: Some(DataFlowSpec::new(
                vec!["user_input".to_string(), "url_parameter".to_string()],
                vec!["html_output".to_string(), "dom_manipulation".to_string()],
            ).with_sanitizers(vec!["html_encode".to_string(), "sanitize_html".to_string()])),
            fix: Some("Encode HTML output to prevent XSS".to_string()),
            fix_regex: None,
            paths: None,
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("category".to_string(), "security".to_string());
                metadata.insert("cwe".to_string(), "79".to_string());
                metadata.insert("owasp".to_string(), "A03:2021".to_string());
                metadata
            },
            enabled: true,
        }
    }

    /// Create a sample AST for testing
    pub fn create_sample_ast() -> UniversalNode {
        // Create a simple method call that could be vulnerable
        AstBuilder::call_expression(
            AstBuilder::identifier("executeQuery"),
            vec![AstBuilder::identifier("userInput")]
        ).with_text("executeQuery(userInput)".to_string())
    }

    /// Demonstrate rule execution
    pub fn demonstrate_rule_execution() -> Result<()> {
        println!("=== Rule Execution Engine Demonstration ===\n");

        // Create rule engine
        let mut engine = RuleEngine::new();

        // Create sample rules (for demonstration, we'll create YAML and load it)
        let sql_rule = Self::create_sql_injection_rule();
        let xss_rule = Self::create_xss_rule();

        // For this demo, we'll manually add rules to the engine's rules vector
        // In practice, rules would be loaded from YAML files
        engine.rules.push(sql_rule.clone());
        engine.rules.push(xss_rule.clone());

        println!("Added {} rules to the engine", engine.rule_count());

        // Create sample AST
        let ast = Self::create_sample_ast();
        println!("Created sample AST with {} nodes", Self::count_nodes(&ast));

        // Create rule context
        let context = RuleContext::new(
            "example.java".to_string(),
            Language::Java,
            "public class Example { ... }".to_string(),
        );

        // Execute rules
        println!("\nExecuting rules...");
        let findings = engine.analyze(&ast, &context)?;

        // Display results
        println!("\nAnalysis Results:");
        println!("Found {} findings", findings.len());

        for (i, finding) in findings.iter().enumerate() {
            println!("\nFinding {}:", i + 1);
            println!("  Rule ID: {}", finding.rule_id);
            println!("  Message: {}", finding.message);
            println!("  Severity: {:?}", finding.severity);
            println!("  Confidence: {:?}", finding.confidence);
            println!("  Location: {}:{}-{}:{}", 
                finding.location.start_line, 
                finding.location.start_column,
                finding.location.end_line, 
                finding.location.end_column
            );
            
            if !finding.metadata.is_empty() {
                println!("  Metadata:");
                for (key, value) in &finding.metadata {
                    println!("    {}: {}", key, value);
                }
            }
        }

        Ok(())
    }

    /// Count nodes in an AST
    fn count_nodes(node: &dyn AstNode) -> usize {
        let mut count = 1;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += Self::count_nodes(child);
            }
        }
        count
    }

    /// Demonstrate advanced rule execution with data flow analysis
    pub fn demonstrate_advanced_execution() -> Result<()> {
        println!("\n=== Advanced Rule Execution with Data Flow Analysis ===\n");

        // Create advanced executor
        let mut executor = AdvancedRuleExecutor::new();

        // Create rules
        let rules = vec![
            Self::create_sql_injection_rule(),
            Self::create_xss_rule(),
        ];

        // Create sample AST
        let ast = Self::create_sample_ast();

        // Execute comprehensive analysis
        let result = executor.execute_comprehensive_analysis(
            &rules,
            &ast,
            Language::Java,
            Some(Path::new("example.java"))
        )?;

        // Display comprehensive results
        println!("Comprehensive Analysis Results:");
        println!("Execution time: {:?}", result.execution_time);
        println!("Total findings: {}", result.findings.len());
        println!("Rules executed: {}", result.rule_results.len());

        // Display summary
        let summary = result.summary();
        println!("\nSummary:");
        println!("  Total findings: {}", summary.total_findings);
        println!("  Errors: {}", summary.error_count);
        println!("  Warnings: {}", summary.warning_count);
        println!("  Info: {}", summary.info_count);
        println!("  Rules executed: {}", summary.rules_executed);

        // Display rule execution details
        println!("\nRule Execution Details:");
        for rule_result in &result.rule_results {
            println!("  Rule {}: {} findings in {:?}", 
                rule_result.rule_id, 
                rule_result.findings.len(),
                rule_result.execution_time
            );
            if !rule_result.success {
                if let Some(ref error) = rule_result.error {
                    println!("    Error: {}", error);
                }
            }
        }

        // Display data flow analysis if available
        if let Some(ref dataflow) = result.dataflow_analysis {
            println!("\nData Flow Analysis:");
            let stats = dataflow.statistics();
            println!("  Nodes: {}", stats.node_count);
            println!("  Edges: {}", stats.edge_count);
            println!("  Sources: {}", stats.source_count);
            println!("  Sinks: {}", stats.sink_count);
            println!("  Sanitizers: {}", stats.sanitizer_count);
            println!("  Taint flows: {}", stats.flow_count);
            println!("  Vulnerable flows: {}", stats.vulnerable_flow_count);
        }

        Ok(())
    }

    /// Run all demonstrations
    pub fn run_all_demonstrations() -> Result<()> {
        Self::demonstrate_rule_execution()?;
        Self::demonstrate_advanced_execution()?;
        Ok(())
    }
}

// RuleContext is already defined in types.rs, so we don't need to redefine it here

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sql_injection_rule() {
        let rule = RuleExecutionExample::create_sql_injection_rule();
        assert_eq!(rule.id, "sql-injection-001");
        assert_eq!(rule.severity, Severity::Error);
        assert!(rule.applies_to(Language::Java));
        assert!(rule.requires_dataflow());
    }

    #[test]
    fn test_create_xss_rule() {
        let rule = RuleExecutionExample::create_xss_rule();
        assert_eq!(rule.id, "xss-001");
        assert_eq!(rule.severity, Severity::Warning);
        assert!(rule.applies_to(Language::JavaScript));
    }

    #[test]
    fn test_create_sample_ast() {
        let ast = RuleExecutionExample::create_sample_ast();
        assert_eq!(ast.node_type(), "call_expression");
        assert!(ast.child_count() > 0);
    }

    #[test]
    fn test_rule_context() {
        let context = RuleContext::new(
            "test.java".to_string(),
            Language::Java,
            "public class Test {}".to_string(),
        ).add_data("test".to_string(), "value".to_string());

        assert_eq!(context.language, Language::Java);
        assert_eq!(context.file_path, "test.java");
        assert_eq!(context.custom_data.get("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_rule_execution_integration() {
        // This is a basic integration test
        let result = RuleExecutionExample::demonstrate_rule_execution();
        assert!(result.is_ok());
    }
}
