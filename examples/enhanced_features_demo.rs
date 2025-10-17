//! Demonstration of enhanced pattern matching and analysis features

use cr_semservice::matcher::{AdvancedSemgrepMatcher, PreciseExpressionMatcher, MatchingConfig};
use cr_semservice::core::{SemgrepPattern, PatternType, Severity, Confidence};
use cr_semservice::ast::{UniversalNode, NodeType};
use cr_semservice::{PhpOptimizer, JavaScriptOptimizer};
use cr_semservice::dataflow::{EnhancedTaintTracker, TaintAnalysisConfig, Source, Sink, Sanitizer, DataFlowGraph, SourceType, SinkType, SanitizerType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Enhanced Code Review Service - Feature Demonstration");
    println!("========================================================\n");

    // 1. Advanced Pattern Matching
    demo_advanced_pattern_matching()?;
    
    // 2. Precise Expression Matching
    demo_precise_expression_matching()?;
    
    // 3. Language-Specific Optimizations
    demo_language_optimizations()?;
    
    // 4. Enhanced Taint Analysis
    demo_enhanced_taint_analysis()?;

    println!("\n✅ All demonstrations completed successfully!");
    Ok(())
}

fn demo_advanced_pattern_matching() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. 🔍 Advanced Pattern Matching");
    println!("--------------------------------");
    
    let mut matcher = AdvancedSemgrepMatcher::new();
    
    // Create a complex pattern with either/or logic
    let sub_pattern1 = SemgrepPattern {
        pattern_type: PatternType::Simple("eval($CODE)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    let sub_pattern2 = SemgrepPattern {
        pattern_type: PatternType::Simple("exec($CMD)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Either(vec![sub_pattern1, sub_pattern2]),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    // Create test AST nodes
    let eval_ast = UniversalNode::new(NodeType::CallExpression)
        .with_text("eval(userInput)".to_string());
    
    let exec_ast = UniversalNode::new(NodeType::CallExpression)
        .with_text("exec(command)".to_string());
    
    // Test matching
    let eval_results = matcher.find_matches(&pattern, &eval_ast)?;
    let exec_results = matcher.find_matches(&pattern, &exec_ast)?;
    
    println!("   ✓ Pattern-either matching:");
    println!("     - eval() pattern: {} matches found", eval_results.len());
    println!("     - exec() pattern: {} matches found", exec_results.len());
    
    // Test pattern-inside
    let inner_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$VAR".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    let inside_pattern = SemgrepPattern {
        pattern_type: PatternType::Inside(Box::new(inner_pattern)),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    let function_ast = UniversalNode::new(NodeType::FunctionDeclaration)
        .with_text("function test() { return userInput; }".to_string())
        .add_child(
            UniversalNode::new(NodeType::Identifier)
                .with_text("userInput".to_string())
        );
    
    let inside_results = matcher.find_matches(&inside_pattern, &function_ast)?;
    println!("     - pattern-inside: {} matches found", inside_results.len());
    
    println!();
    Ok(())
}

fn demo_precise_expression_matching() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. 🎯 Precise Expression Matching");
    println!("----------------------------------");
    
    // Test different matching configurations
    let configs = vec![
        ("Structural Only", MatchingConfig {
            structural_matching: true,
            semantic_matching: false,
            type_aware_matching: false,
            max_depth: 20,
            allow_partial_matches: false,
            similarity_threshold: 1.0,
        }),
        ("Semantic + Type-Aware", MatchingConfig {
            structural_matching: true,
            semantic_matching: true,
            type_aware_matching: true,
            max_depth: 20,
            allow_partial_matches: true,
            similarity_threshold: 0.8,
        }),
        ("Fuzzy Matching", MatchingConfig {
            structural_matching: true,
            semantic_matching: true,
            type_aware_matching: true,
            max_depth: 20,
            allow_partial_matches: true,
            similarity_threshold: 0.6,
        }),
    ];
    
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$OBJ.$METHOD($...ARGS)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    let ast = UniversalNode::new(NodeType::CallExpression)
        .with_text("user.getName()".to_string());
    
    for (name, config) in configs {
        let mut matcher = PreciseExpressionMatcher::with_config(config);
        let results = matcher.find_precise_matches(&pattern, &ast)?;
        println!("   ✓ {}: {} matches found", name, results.len());
    }
    
    println!();
    Ok(())
}

fn demo_language_optimizations() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. 🔧 Language-Specific Optimizations");
    println!("--------------------------------------");
    
    // PHP Optimization
    println!("   PHP Optimizer:");
    let mut php_optimizer = PhpOptimizer::new();
    
    let php_code = "$user_input = $_GET['data']; echo $user_input;";
    let php_ast = UniversalNode::new(NodeType::Program)
        .with_text(php_code.to_string());
    
    let optimized_php = php_optimizer.optimize_php_ast(php_ast, php_code)?;
    println!("     ✓ Optimized PHP AST with {} attributes", optimized_php.attributes.len());
    
    // Check for specific optimizations
    if optimized_php.attributes.contains_key("language") {
        println!("     ✓ Language detection: {}", optimized_php.attributes.get("language").unwrap());
    }
    
    // JavaScript Optimization
    println!("   JavaScript Optimizer:");
    let mut js_optimizer = JavaScriptOptimizer::new();
    
    let js_code = "element.innerHTML = userInput; eval(code);";
    let js_ast = UniversalNode::new(NodeType::Program)
        .with_text(js_code.to_string());
    
    let optimized_js = js_optimizer.optimize_js_ast(js_ast, js_code)?;
    println!("     ✓ Optimized JavaScript AST with {} attributes", optimized_js.attributes.len());
    
    if optimized_js.attributes.contains_key("language") {
        println!("     ✓ Language detection: {}", optimized_js.attributes.get("language").unwrap());
    }
    
    println!();
    Ok(())
}

fn demo_enhanced_taint_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. 🔬 Enhanced Taint Analysis");
    println!("------------------------------");

    let tracker = EnhancedTaintTracker::new();
    println!("   ✓ Enhanced taint tracker created");

    // Test with custom configuration
    let config = TaintAnalysisConfig {
        max_path_length: 100,
        max_contexts: 50,
        field_sensitive: true,
        context_sensitive: true,
        path_sensitive: true,
        min_confidence: 20,
    };

    let tracker_with_config = EnhancedTaintTracker::with_config(config);
    println!("   ✓ Enhanced taint tracker with custom config created");

    // Create a simple data flow graph
    let graph = DataFlowGraph::new();
    println!("   ✓ Created data flow graph");

    // For demo purposes, we'll just show that the components work
    println!("   ✓ Taint analysis components initialized successfully");
    println!("     - Enhanced field-sensitive analysis: enabled");
    println!("     - Context-sensitive analysis: enabled");
    println!("     - Path-sensitive analysis: enabled");
    println!("     - Maximum path length: 100");
    println!("     - Maximum contexts: 50");

    println!();
    Ok(())
}
