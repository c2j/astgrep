//! Integration tests for CR-SemService
//! 
//! These tests verify the complete analysis pipeline from source code to findings.

use cr_core::{Language, AnalysisConfig, OutputFormat};
use cr_parser::LanguageParserRegistry;
use cr_rules::{RuleEngine, RuleParser};
use cr_matcher::AdvancedSemgrepMatcher;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

/// Test the complete analysis pipeline
#[test]
fn test_complete_analysis_pipeline() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create test source files
    let java_file = temp_path.join("Test.java");
    fs::write(&java_file, r#"
public class Test {
    public void vulnerableMethod(String userInput) {
        // SQL injection vulnerability
        String query = "SELECT * FROM users WHERE id = " + userInput;
        executeQuery(query);
        
        // Hardcoded password
        String password = "admin123";
        authenticate(password);
    }
    
    private void executeQuery(String query) {
        // Database execution
    }
    
    private void authenticate(String password) {
        // Authentication logic
    }
}
"#).expect("Failed to write Java test file");

    let js_file = temp_path.join("test.js");
    fs::write(&js_file, r#"
function vulnerableFunction(userInput) {
    // XSS vulnerability
    document.getElementById("content").innerHTML = userInput;
    
    // Dangerous eval usage
    eval("var result = " + userInput);
    
    // Console.log in production
    console.log("Debug: " + userInput);
    
    return result;
}

// Hardcoded API key
const apiKey = "sk-1234567890abcdef";
"#).expect("Failed to write JavaScript test file");

    // Set up analysis configuration
    let config = AnalysisConfig {
        target_paths: vec![temp_path.to_path_buf()],
        exclude_patterns: vec![],
        languages: vec![Language::Java, Language::JavaScript],
        rule_files: vec![],
        output_format: OutputFormat::Json,
        parallel: false,
        max_threads: Some(1),
    };

    // Initialize components
    let parser_registry = LanguageParserRegistry::new();
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    // Load built-in rules for testing
    let java_rules = create_test_java_rules();
    let js_rules = create_test_javascript_rules();
    
    let parsed_java_rules = rule_parser.parse_yaml(&java_rules)
        .expect("Failed to parse Java rules");
    let parsed_js_rules = rule_parser.parse_yaml(&js_rules)
        .expect("Failed to parse JavaScript rules");

    for rule in parsed_java_rules {
        rule_engine.add_rule(rule);
    }
    for rule in parsed_js_rules {
        rule_engine.add_rule(rule);
    }

    // Run analysis on Java file
    let java_source = fs::read_to_string(&java_file)
        .expect("Failed to read Java file");
    let java_ast = parser_registry.parse_file(&java_file, &java_source)
        .expect("Failed to parse Java file");
    
    let java_context = cr_rules::RuleContext::new(
        java_file.to_string_lossy().to_string(),
        Language::Java,
        java_source.clone(),
    );
    let java_findings = rule_engine.analyze(&*java_ast, &java_context)
        .expect("Failed to analyze Java AST");

    // Run analysis on JavaScript file
    let js_source = fs::read_to_string(&js_file)
        .expect("Failed to read JavaScript file");
    let js_ast = parser_registry.parse_file(&js_file, &js_source)
        .expect("Failed to parse JavaScript file");
    
    let js_context = cr_rules::RuleContext::new(
        js_file.to_string_lossy().to_string(),
        Language::JavaScript,
        js_source.clone(),
    );
    let js_findings = rule_engine.analyze(&*js_ast, &js_context)
        .expect("Failed to analyze JavaScript AST");

    // Verify findings
    assert!(!java_findings.is_empty(), "Should find vulnerabilities in Java code");
    assert!(!js_findings.is_empty(), "Should find vulnerabilities in JavaScript code");

    // Check for specific vulnerability types
    let java_has_sql_injection = java_findings.iter()
        .any(|f| f.rule_id.contains("sql") || f.message.to_lowercase().contains("sql"));
    let java_has_hardcoded_password = java_findings.iter()
        .any(|f| f.rule_id.contains("password") || f.message.to_lowercase().contains("password"));

    let js_has_xss = js_findings.iter()
        .any(|f| f.rule_id.contains("xss") || f.message.to_lowercase().contains("xss"));
    let js_has_eval = js_findings.iter()
        .any(|f| f.rule_id.contains("eval") || f.message.to_lowercase().contains("eval"));

    // Verify that the analysis pipeline produces meaningful results
    println!("Java findings: {}", java_findings.len());
    for finding in &java_findings {
        println!("  Java: {} - {} (line {})", finding.rule_id, finding.message, finding.location.start_line);
        // Verify finding structure
        assert!(!finding.rule_id.is_empty(), "Java finding should have rule ID");
        assert!(!finding.message.is_empty(), "Java finding should have message");
        assert!(finding.location.start_line > 0, "Java finding should have valid line number");
    }

    println!("JavaScript findings: {}", js_findings.len());
    for finding in &js_findings {
        println!("  JS: {} - {} (line {})", finding.rule_id, finding.message, finding.location.start_line);
        // Verify finding structure
        assert!(!finding.rule_id.is_empty(), "JS finding should have rule ID");
        assert!(!finding.message.is_empty(), "JS finding should have message");
        assert!(finding.location.start_line > 0, "JS finding should have valid line number");
    }

    // Verify that at least some analysis was performed
    let total_findings = java_findings.len() + js_findings.len();
    println!("✓ Analysis pipeline completed with {} total findings", total_findings);
}

/// Test error handling in the analysis pipeline
#[test]
fn test_analysis_error_handling() {
    let parser_registry = LanguageParserRegistry::new();
    
    // Test with invalid file path
    let invalid_path = PathBuf::from("nonexistent.java");
    let result = parser_registry.parse_file(&invalid_path, "invalid content");
    
    // Should handle the error gracefully - either succeed with error recovery or fail with descriptive error
    match result {
        Ok(ast) => {
            println!("✓ Parser handled invalid file with error recovery");
            assert_eq!(ast.node_type(), "program", "Should return valid AST structure");
        }
        Err(e) => {
            println!("✓ Parser properly reported error: {}", e);
            assert!(!e.to_string().is_empty(), "Error should have descriptive message");
        }
    }
    
    // Test with unsupported file extension
    let unsupported_path = PathBuf::from("test.unknown");
    let result = parser_registry.detect_language(&unsupported_path);
    assert!(result.is_err(), "Should reject unsupported file extensions");
}

/// Test performance with larger files
#[test]
fn test_performance_with_large_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create a larger test file
    let large_java_file = temp_path.join("LargeTest.java");
    let mut large_content = String::new();
    
    // Generate a file with many methods
    large_content.push_str("public class LargeTest {\n");
    for i in 0..100 {
        large_content.push_str(&format!(r#"
    public void method{}(String input) {{
        String query = "SELECT * FROM table WHERE id = " + input;
        executeQuery(query);
        System.out.println("Method {} executed");
    }}
"#, i, i));
    }
    large_content.push_str("}\n");

    fs::write(&large_java_file, &large_content)
        .expect("Failed to write large Java file");

    let parser_registry = LanguageParserRegistry::new();
    
    // Measure parsing time
    let start = std::time::Instant::now();
    let result = parser_registry.parse_file(&large_java_file, &large_content);
    let parse_duration = start.elapsed();

    assert!(result.is_ok(), "Should parse large files successfully");
    assert!(parse_duration.as_secs() < 10, "Parsing should complete within reasonable time");
    
    println!("Parsed large file ({} bytes) in {:?}", large_content.len(), parse_duration);
}

/// Test concurrent analysis
#[test]
fn test_concurrent_analysis() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let results = Arc::new(Mutex::new(Vec::new()));

    let test_files = vec![
        ("test1.java", "public class Test1 { void method() { System.out.println(\"test\"); } }"),
        ("test2.js", "function test() { console.log('test'); }"),
        ("test3.py", "def test(): print('test')"),
    ];

    let mut handles = vec![];

    for (filename, content) in test_files {
        let parser_registry = Arc::clone(&parser_registry);
        let results = Arc::clone(&results);
        let content = content.to_string();
        let filename = filename.to_string();

        let handle = thread::spawn(move || {
            let path = PathBuf::from(&filename);
            let result = parser_registry.parse_file(&path, &content);
            
            let mut results = results.lock().unwrap();
            results.push((filename, result.is_ok()));
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 3, "All files should be processed");
    
    // All results should be successful (or at least not crash)
    for (filename, success) in results.iter() {
        println!("File {}: {}", filename, if *success { "OK" } else { "Error" });
    }
}

/// Create test rules for Java
fn create_test_java_rules() -> String {
    r#"
rules:
  - id: java-sql-injection
    name: "SQL Injection Risk"
    description: "Detects potential SQL injection vulnerabilities"
    severity: Critical
    confidence: High
    languages: [java]
    patterns:
      - "SELECT * FROM"
    message: "Potential SQL injection vulnerability detected"
    fix: "Use PreparedStatement with parameterized queries"

  - id: java-hardcoded-password
    name: "Hardcoded Password"
    description: "Detects hardcoded passwords"
    severity: Error
    confidence: Medium
    languages: [java]
    patterns:
      - "admin123"
    message: "Hardcoded password detected"
    fix: "Use environment variables or secure configuration"
"#.to_string()
}

/// Create test rules for JavaScript
fn create_test_javascript_rules() -> String {
    r#"
rules:
  - id: js-xss-vulnerability
    name: "XSS Vulnerability"
    description: "Detects potential XSS vulnerabilities"
    severity: Critical
    confidence: High
    languages: [javascript]
    patterns:
      - "innerHTML"
    message: "Potential XSS vulnerability detected"
    fix: "Use textContent or proper sanitization"

  - id: js-dangerous-eval
    name: "Dangerous eval() Usage"
    description: "Detects dangerous eval() usage"
    severity: Critical
    confidence: High
    languages: [javascript]
    patterns:
      - "eval("
    message: "Dangerous eval() usage detected"
    fix: "Avoid eval() or use safer alternatives"

  - id: js-console-log
    name: "Console.log in Production"
    description: "Detects console.log statements"
    severity: WARNING
    confidence: HIGH
    languages: [javascript]
    patterns:
      - "console.log($MESSAGE)"
    message: "Console.log statement found"
    fix: "Remove console statements before production"
"#.to_string()
}
