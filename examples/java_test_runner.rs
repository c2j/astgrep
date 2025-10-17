#!/usr/bin/env cargo +nightly -Zscript
//! Java Test Runner for CR-SemService
//! 
//! This example demonstrates how to test Java files with CR-SemService
//! and compare results with Semgrep.

use cr_semservice::{
    AdvancedSemgrepMatcher, SemgrepPattern, PatternType,
    UniversalNode, NodeType
};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Java Test Runner for CR-SemService");
    println!("=====================================");
    println!();

    // Test different types of Java patterns
    test_java_taint_analysis()?;
    test_java_metavar_comparison()?;
    test_java_symbolic_propagation()?;
    test_java_pattern_matching()?;

    println!("✅ All Java tests completed!");
    Ok(())
}

fn test_java_taint_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 1: Java Taint Analysis");
    println!("---------------------------");

    let java_code = r#"
class Test {
    private String x = source();
    
    void test() {
        //ruleid: tainting
        sink(x);
    }
    
    void safe() {
        String safe = "constant";
        sink(safe);
    }
}
"#;

    // Create taint analysis pattern
    let source_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("source(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let sink_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("sink(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create Java AST
    let ast = create_java_ast(java_code);

    // Test pattern matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    
    let source_matches = matcher.find_matches(&source_pattern, &ast)?;
    let sink_matches = matcher.find_matches(&sink_pattern, &ast)?;

    println!("  Java code analyzed:");
    for (i, line) in java_code.lines().enumerate() {
        if !line.trim().is_empty() {
            println!("    {}: {}", i + 1, line.trim());
        }
    }

    println!("  Source matches: {}", source_matches.len());
    println!("  Sink matches: {}", sink_matches.len());

    // Simple taint analysis: if we have both sources and sinks, it's potentially dangerous
    if source_matches.len() > 0 && sink_matches.len() > 0 {
        println!("  ⚠️  Potential taint flow detected!");
    } else {
        println!("  ✅ No taint flow detected");
    }

    println!();
    Ok(())
}

fn test_java_metavar_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 2: Java Metavariable Comparison");
    println!("------------------------------------");

    let java_code = r#"
public class A {
    public static int test1() {
        int a = 2;
        //ruleid: MSTG-STORAGE-5.1
        return a;
    }
    public static int test2() {
        int a = 3;
        //ok: MSTG-STORAGE-5.1
        return a;
    }
}
"#;

    // Create metavariable pattern for return statements
    let return_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("return $X;".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create Java AST
    let ast = create_java_ast(java_code);

    // Test pattern matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&return_pattern, &ast)?;

    println!("  Java code analyzed:");
    for (i, line) in java_code.lines().enumerate() {
        if !line.trim().is_empty() {
            println!("    {}: {}", i + 1, line.trim());
        }
    }

    println!("  Return statement matches: {}", matches.len());
    
    for (i, m) in matches.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("    Match {}: {}", i + 1, text);
        }
    }

    println!();
    Ok(())
}

fn test_java_symbolic_propagation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 3: Java Symbolic Propagation");
    println!("---------------------------------");

    let java_code = r#"
class Test {
    private String secret = getSecret();
    
    public void process() {
        String data = this.secret;
        output(data);
    }
    
    private String getSecret() {
        return "sensitive";
    }
}
"#;

    // Create patterns for symbolic propagation
    let field_access_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("this.$FIELD".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let assignment_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$VAR = $VALUE".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create Java AST
    let ast = create_java_ast(java_code);

    // Test pattern matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    
    let field_matches = matcher.find_matches(&field_access_pattern, &ast)?;
    let assignment_matches = matcher.find_matches(&assignment_pattern, &ast)?;

    println!("  Java code analyzed:");
    for (i, line) in java_code.lines().enumerate() {
        if !line.trim().is_empty() {
            println!("    {}: {}", i + 1, line.trim());
        }
    }

    println!("  Field access matches: {}", field_matches.len());
    println!("  Assignment matches: {}", assignment_matches.len());

    println!();
    Ok(())
}

fn test_java_pattern_matching() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 4: Java General Pattern Matching");
    println!("-------------------------------------");

    let java_code = r#"
public class Example {
    public void method1() {
        System.out.println("Hello");
        log.info("Information");
        logger.debug("Debug info");
    }
    
    public void method2() {
        String msg = "test";
        System.err.println(msg);
    }
}
"#;

    // Test various Java patterns
    let patterns = vec![
        ("System.out.println(...)", "System output calls"),
        ("log.$METHOD(...)", "Logging calls"),
        ("$OBJ.println(...)", "Any println calls"),
        ("String $VAR = $VALUE", "String declarations"),
    ];

    // Create Java AST
    let ast = create_java_ast(java_code);
    let mut matcher = AdvancedSemgrepMatcher::new();

    println!("  Java code analyzed:");
    for (i, line) in java_code.lines().enumerate() {
        if !line.trim().is_empty() {
            println!("    {}: {}", i + 1, line.trim());
        }
    }
    println!();

    for (pattern_str, description) in patterns {
        let pattern = SemgrepPattern {
            pattern_type: PatternType::Simple(pattern_str.to_string()),
            metavariable_pattern: None,
            focus: None,
            conditions: Vec::new(),
        };

        let matches = matcher.find_matches(&pattern, &ast)?;
        println!("  Pattern: {} ({})", pattern_str, description);
        println!("  Matches: {}", matches.len());
        
        for (i, m) in matches.iter().enumerate() {
            if let Some(text) = m.node.text() {
                println!("    {}: {}", i + 1, text);
            }
        }
        println!();
    }

    Ok(())
}

// Helper function to create a Java AST
fn create_java_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "java".to_string());

    let lines: Vec<&str> = code.lines().collect();
    let mut current_class: Option<UniversalNode> = None;
    let mut current_method: Option<UniversalNode> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Class declaration
        if trimmed.starts_with("class ") || trimmed.starts_with("public class ") {
            if let Some(class_node) = current_class.take() {
                ast = ast.add_child(class_node);
            }
            current_class = Some(
                UniversalNode::new(NodeType::ClassDeclaration)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
            );
        }
        // Method declaration
        else if trimmed.contains("(") && (trimmed.contains("void") || trimmed.contains("int") || trimmed.contains("String")) {
            if let Some(method_node) = current_method.take() {
                if let Some(ref mut class_node) = current_method {
                    *class_node = class_node.clone().add_child(method_node);
                }
            }
            current_method = Some(
                UniversalNode::new(NodeType::FunctionDeclaration)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
            );
        }
        // Method calls and other statements
        else if trimmed.contains("(") && trimmed.contains(")") {
            let call_node = UniversalNode::new(NodeType::CallExpression)
                .with_text(trimmed.to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string());

            if let Some(ref mut method_node) = current_method {
                *method_node = method_node.clone().add_child(call_node);
            } else if let Some(ref mut class_node) = current_class {
                *class_node = class_node.clone().add_child(call_node);
            } else {
                ast = ast.add_child(call_node);
            }
        }
        // Assignments and other statements
        else if trimmed.contains("=") {
            let assign_node = UniversalNode::new(NodeType::AssignmentExpression)
                .with_text(trimmed.to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string());

            if let Some(ref mut method_node) = current_method {
                *method_node = method_node.clone().add_child(assign_node);
            } else if let Some(ref mut class_node) = current_class {
                *class_node = class_node.clone().add_child(assign_node);
            } else {
                ast = ast.add_child(assign_node);
            }
        }
    }

    // Add remaining nodes
    if let Some(method_node) = current_method {
        if let Some(ref mut class_node) = current_class {
            *class_node = class_node.clone().add_child(method_node);
        }
    }
    if let Some(class_node) = current_class {
        ast = ast.add_child(class_node);
    }

    ast
}
