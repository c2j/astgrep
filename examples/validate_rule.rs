//! Rule validation example
//!
//! This example demonstrates how to validate a rule file using the CR-SemService rule engine.

use cr_core::{Language, Severity, Confidence};
use cr_rules::{RuleEngine, RuleValidator, RuleContext};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CR-SemService Rule Validation Tool ===\n");

    // Rule file path
    let rule_file = "examples/cs-eh-08-system-out-logging.yaml";
    
    if !Path::new(rule_file).exists() {
        eprintln!("Error: Rule file not found: {}", rule_file);
        std::process::exit(1);
    }

    println!("📁 Loading rule file: {}", rule_file);
    
    // Read the YAML content
    let yaml_content = fs::read_to_string(rule_file)?;
    println!("✅ Successfully read {} bytes", yaml_content.len());

    // Create rule engine and validator
    let mut engine = RuleEngine::new();
    let validator = RuleValidator::new();

    println!("\n🔍 Parsing YAML content...");
    
    // Parse the rule
    match engine.load_rules_from_yaml(&yaml_content) {
        Ok(count) => {
            println!("✅ Successfully parsed {} rule(s)", count);
            
            // Get the parsed rules
            let rules = engine.rules();
            
            for (i, rule) in rules.iter().enumerate() {
                println!("\n📋 Rule {} Details:", i + 1);
                println!("  ID: {}", rule.id);
                println!("  Name: {}", rule.name);
                println!("  Description: {}", rule.description);
                println!("  Severity: {:?}", rule.severity);
                println!("  Confidence: {:?}", rule.confidence);
                println!("  Languages: {:?}", rule.languages);
                println!("  Enabled: {}", rule.enabled);
                println!("  Patterns: {} pattern(s)", rule.patterns.len());
                
                if let Some(fix) = &rule.fix {
                    println!("  Fix suggestion available: {} characters", fix.len());
                }
                
                println!("  Metadata: {} entries", rule.metadata.len());
                
                // Display patterns
                for (j, pattern) in rule.patterns.iter().enumerate() {
                    if let Some(pattern_str) = pattern.get_pattern_string() {
                        println!("    Pattern {}: {}", j + 1, pattern_str);
                    } else {
                        println!("    Pattern {}: <complex pattern>", j + 1);
                    }
                    if let Some(ref focus) = pattern.focus {
                        println!("      Focus: {:?}", focus);
                    }
                    println!("      Conditions: {} condition(s)", pattern.conditions.len());
                }
                
                // Validate the rule
                println!("\n🔬 Validating rule...");
                match validator.validate_rule(rule) {
                    Ok(()) => {
                        println!("✅ Rule validation passed!");
                        
                        // Check rule applicability
                        println!("\n🎯 Rule Applicability:");
                        for language in &[Language::Java, Language::JavaScript, Language::Python] {
                            let applies = rule.applies_to(*language);
                            println!("  {:?}: {}", language, if applies { "✅" } else { "❌" });
                        }
                        
                        // Check if rule requires dataflow analysis
                        let requires_dataflow = rule.requires_dataflow();
                        println!("  Requires dataflow analysis: {}", if requires_dataflow { "✅" } else { "❌" });
                        
                    }
                    Err(e) => {
                        println!("❌ Rule validation failed: {}", e);
                        println!("   Category: {}", e.category());
                        println!("   Severity: {:?}", e.severity());
                        println!("   Suggested action: {}", e.suggested_action());
                        println!("   Recoverable: {}", e.is_recoverable());
                    }
                }
            }
            
            // Test basic rule engine functionality
            println!("\n🧪 Testing rule engine configuration...");
            test_rule_engine_config(&engine)?;
            
        }
        Err(e) => {
            println!("❌ Failed to parse rule: {}", e);
            println!("   Error category: {}", e.category());
            println!("   Severity: {:?}", e.severity());
            println!("   Suggested action: {}", e.suggested_action());
            return Err(e.into());
        }
    }

    println!("\n🎉 Rule validation completed successfully!");
    Ok(())
}

fn test_rule_engine_config(engine: &RuleEngine) -> Result<(), Box<dyn std::error::Error>> {
    println!("  📊 Rule engine configuration:");
    println!("    Total rules loaded: {}", engine.rule_count());
    println!("    Java-applicable rules: {}", engine.rules_for_language(Language::Java).len());
    println!("    JavaScript-applicable rules: {}", engine.rules_for_language(Language::JavaScript).len());
    println!("    Python-applicable rules: {}", engine.rules_for_language(Language::Python).len());

    // Test rule context creation
    let context = RuleContext::new(
        "TestClass.java".to_string(),
        Language::Java,
        "System.out.println(\"Debug: Processing user\");".to_string(),
    );

    println!("  🔍 Created rule context:");
    println!("    File: {}", context.file_path);
    println!("    Language: {:?}", context.language);
    println!("    Source length: {} characters", context.source_code.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_validation() {
        // This test would run the validation in a test environment
        let yaml_content = r#"
id: test-rule
name: "Test Rule"
description: "A test rule"
severity: warning
confidence: medium
enabled: true
languages:
  - java
patterns:
  - pattern: "test()"
"#;

        let mut engine = RuleEngine::new();
        let result = engine.load_rules_from_yaml(yaml_content);
        assert!(result.is_ok(), "Rule should be valid");
        assert_eq!(result.unwrap(), 1, "Should load one rule");
    }
}
