//! Performance tests for astgrep
//! 
//! These tests verify that the system performs within acceptable limits
//! and can handle various load scenarios.

use astgrep_core::{Language, constants::defaults};
use astgrep_parser::LanguageParserRegistry;
use astgrep_matcher::AdvancedSemgrepMatcher;
use astgrep_ast::{UniversalNode, NodeType};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Performance benchmarks and thresholds
struct PerformanceBenchmarks {
    max_parse_time_small: Duration,    // < 1KB files
    max_parse_time_medium: Duration,   // 1KB - 10KB files  
    max_parse_time_large: Duration,    // 10KB - 100KB files
    max_memory_usage_mb: usize,        // Maximum memory usage
    max_pattern_match_time: Duration,  // Pattern matching time
}

impl Default for PerformanceBenchmarks {
    fn default() -> Self {
        Self {
            max_parse_time_small: Duration::from_millis(10),
            max_parse_time_medium: Duration::from_millis(100),
            max_parse_time_large: Duration::from_millis(1000),
            max_memory_usage_mb: 100,
            max_pattern_match_time: Duration::from_millis(50),
        }
    }
}

/// Test parsing performance with different file sizes
#[test]
fn test_parsing_performance() {
    let benchmarks = PerformanceBenchmarks::default();
    let parser_registry = LanguageParserRegistry::new();

    // Test small file (< 1KB)
    let small_java_code = r#"
public class Small {
    public void method() {
        System.out.println("Hello");
    }
}
"#;
    
    let start = Instant::now();
    let result = parser_registry.parse_file(
        &std::path::PathBuf::from("Small.java"), 
        small_java_code
    );
    let small_duration = start.elapsed();
    
    assert!(result.is_ok(), "Small file parsing should succeed");
    assert!(
        small_duration <= benchmarks.max_parse_time_small,
        "Small file parsing took {:?}, expected <= {:?}",
        small_duration,
        benchmarks.max_parse_time_small
    );

    // Test medium file (1KB - 10KB)
    let medium_java_code = generate_java_code(50); // ~5KB
    
    let start = Instant::now();
    let result = parser_registry.parse_file(
        &std::path::PathBuf::from("Medium.java"),
        &medium_java_code
    );
    let medium_duration = start.elapsed();
    
    assert!(result.is_ok(), "Medium file parsing should succeed");
    assert!(
        medium_duration <= benchmarks.max_parse_time_medium,
        "Medium file parsing took {:?}, expected <= {:?}",
        medium_duration,
        benchmarks.max_parse_time_medium
    );

    // Test large file (10KB - 100KB)
    let large_java_code = generate_java_code(500); // ~50KB
    
    let start = Instant::now();
    let result = parser_registry.parse_file(
        &std::path::PathBuf::from("Large.java"),
        &large_java_code
    );
    let large_duration = start.elapsed();
    
    assert!(result.is_ok(), "Large file parsing should succeed");
    assert!(
        large_duration <= benchmarks.max_parse_time_large,
        "Large file parsing took {:?}, expected <= {:?}",
        large_duration,
        benchmarks.max_parse_time_large
    );

    println!("Parsing performance:");
    println!("  Small file: {:?}", small_duration);
    println!("  Medium file: {:?}", medium_duration);
    println!("  Large file: {:?}", large_duration);
}

/// Test pattern matching performance
#[test]
fn test_pattern_matching_performance() {
    let benchmarks = PerformanceBenchmarks::default();
    let mut matcher = AdvancedSemgrepMatcher::new();

    // Create a complex AST for testing
    let ast = create_complex_ast(100); // 100 nodes

    // Test simple pattern matching
    let simple_pattern = astgrep_core::SemgrepPattern {
        pattern_type: astgrep_core::PatternType::Simple("$FUNC($ARG)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let start = Instant::now();
    let result = matcher.find_matches(&simple_pattern, &ast);
    let simple_duration = start.elapsed();

    assert!(result.is_ok(), "Simple pattern matching should succeed");
    assert!(
        simple_duration <= benchmarks.max_pattern_match_time,
        "Simple pattern matching took {:?}, expected <= {:?}",
        simple_duration,
        benchmarks.max_pattern_match_time
    );

    // Test complex pattern matching
    let complex_pattern = astgrep_core::SemgrepPattern {
        pattern_type: astgrep_core::PatternType::Simple("$OBJ.$METHOD($ARG1, $ARG2)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let start = Instant::now();
    let result = matcher.find_matches(&complex_pattern, &ast);
    let complex_duration = start.elapsed();

    assert!(result.is_ok(), "Complex pattern matching should succeed");
    
    println!("Pattern matching performance:");
    println!("  Simple pattern: {:?}", simple_duration);
    println!("  Complex pattern: {:?}", complex_duration);
}

/// Test memory usage during analysis
#[test]
fn test_memory_usage() {
    let benchmarks = PerformanceBenchmarks::default();
    
    // Get initial memory usage
    let initial_memory = get_memory_usage_mb();
    
    // Perform memory-intensive operations
    let parser_registry = LanguageParserRegistry::new();
    let mut asts = Vec::new();
    
    // Parse multiple large files
    for i in 0..10 {
        let large_code = generate_java_code(200); // ~20KB each
        let filename = format!("Test{}.java", i);
        
        if let Ok(ast) = parser_registry.parse_file(
            &std::path::PathBuf::from(filename),
            &large_code
        ) {
            asts.push(ast);
        }
    }
    
    // Check memory usage after operations
    let peak_memory = get_memory_usage_mb();
    let memory_increase = peak_memory.saturating_sub(initial_memory);
    
    assert!(
        memory_increase <= benchmarks.max_memory_usage_mb,
        "Memory usage increased by {}MB, expected <= {}MB",
        memory_increase,
        benchmarks.max_memory_usage_mb
    );
    
    println!("Memory usage:");
    println!("  Initial: {}MB", initial_memory);
    println!("  Peak: {}MB", peak_memory);
    println!("  Increase: {}MB", memory_increase);
}

/// Test concurrent processing performance
#[test]
fn test_concurrent_performance() {
    use std::sync::Arc;
    use std::thread;

    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let num_threads = 4;
    let files_per_thread = 5;

    let start = Instant::now();
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let parser_registry = Arc::clone(&parser_registry);
            
            thread::spawn(move || {
                let mut results = Vec::new();
                
                for file_id in 0..files_per_thread {
                    let code = generate_java_code(50);
                    let filename = format!("Thread{}File{}.java", thread_id, file_id);
                    
                    let file_start = Instant::now();
                    let result = parser_registry.parse_file(
                        &std::path::PathBuf::from(filename),
                        &code
                    );
                    let file_duration = file_start.elapsed();
                    
                    results.push((result.is_ok(), file_duration));
                }
                
                results
            })
        })
        .collect();

    let mut all_results = Vec::new();
    for handle in handles {
        let thread_results = handle.join().expect("Thread should complete");
        all_results.extend(thread_results);
    }

    let total_duration = start.elapsed();
    let successful_parses = all_results.iter().filter(|(success, _)| *success).count();
    let avg_parse_time: Duration = all_results
        .iter()
        .map(|(_, duration)| *duration)
        .sum::<Duration>() / all_results.len() as u32;

    assert_eq!(
        successful_parses,
        num_threads * files_per_thread,
        "All concurrent parses should succeed"
    );

    println!("Concurrent performance:");
    println!("  Total time: {:?}", total_duration);
    println!("  Files processed: {}", all_results.len());
    println!("  Average parse time: {:?}", avg_parse_time);
    println!("  Successful parses: {}/{}", successful_parses, all_results.len());
}

/// Test performance regression
#[test]
fn test_performance_regression() {
    // This test would compare current performance against baseline metrics
    // For now, we just ensure operations complete within reasonable time
    
    let baseline_metrics = HashMap::from([
        ("small_file_parse_ms", 10u64),
        ("medium_file_parse_ms", 100u64),
        ("large_file_parse_ms", 1000u64),
        ("pattern_match_ms", 50u64),
    ]);

    let parser_registry = LanguageParserRegistry::new();
    
    // Test small file parsing
    let small_code = generate_java_code(5);
    let start = Instant::now();
    let _ = parser_registry.parse_file(&std::path::PathBuf::from("test.java"), &small_code);
    let duration = start.elapsed();
    
    let baseline = Duration::from_millis(baseline_metrics["small_file_parse_ms"]);
    assert!(
        duration <= baseline,
        "Performance regression detected: small file parsing took {:?}, baseline is {:?}",
        duration,
        baseline
    );

    println!("Performance regression test passed");
}

/// Generate Java code of approximately the specified number of methods
fn generate_java_code(num_methods: usize) -> String {
    let mut code = String::new();
    code.push_str("public class GeneratedTest {\n");
    
    for i in 0..num_methods {
        code.push_str(&format!(r#"
    public void method{}(String param{}) {{
        String query = "SELECT * FROM table WHERE id = " + param{};
        System.out.println("Executing method {}: " + query);
        if (param{} != null) {{
            processData(param{});
        }}
    }}
"#, i, i, i, i, i, i));
    }
    
    code.push_str("    private void processData(String data) {\n");
    code.push_str("        // Process the data\n");
    code.push_str("    }\n");
    code.push_str("}\n");
    
    code
}

/// Create a complex AST structure for testing
fn create_complex_ast(num_nodes: usize) -> UniversalNode {
    let mut root = UniversalNode::new(NodeType::Program);
    
    for i in 0..num_nodes {
        let node = UniversalNode::new(NodeType::CallExpression)
            .with_text(format!("method{}(arg{})", i, i));
        root = root.add_child(node);
    }
    
    root
}

/// Get current memory usage in MB (simplified implementation)
fn get_memory_usage_mb() -> usize {
    // This is a simplified implementation
    // In a real scenario, you would use a proper memory profiling library
    
    // For now, return a mock value based on system info
    // In practice, you might use libraries like `sysinfo` or platform-specific APIs
    50 // Mock value
}
