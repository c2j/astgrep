use astgrep_core::{AstNode, Language};
use astgrep_dataflow::constant_propagation::{ConstantPropagator, ConstantValue};
use astgrep_matcher::{AdvancedSemgrepMatcher, PatternParser};
use astgrep_parser::TreeSitterParser;
use std::collections::HashMap;

fn main() {
    println!("Testing the fix for 'return 5;' pattern matching with constant propagation...");

    // Test source code with a variable that has a constant value of 5
    let source = r#"
function example() {
    const x = 5;
    return x; // This should match "return 5;"
}
"#;

    // Parse the source code to AST
    let parser = TreeSitterParser::new(Language::JavaScript);
    let ast = parser.parse(source).unwrap();

    // Perform constant propagation
    let mut propagator = ConstantPropagator::new();
    let constants = propagator.analyze_ast(&ast).unwrap();

    println!("Found {} constant values:", constants.len());
    for (name, value) in &constants {
        println!("  {} = {:?}", name, value);
    }

    // Test pattern "return 5;"
    let pattern_str = "return 5;";
    println!("\nTesting pattern: '{}'", pattern_str);

    // Parse the pattern
    let parser = PatternParser::new();
    let pattern = parser.parse(pattern_str).unwrap();
    println!("Parsed pattern: {:?}", pattern);

    // Create matcher with constant values
    let mut matcher = AdvancedSemgrepMatcher::new();
    matcher.set_constant_values(constants);

    // Find matches
    let matches = matcher.find_matches(&pattern, &ast).unwrap();

    println!("\nFound {} matches:", matches.len());
    for (i, m) in matches.iter().enumerate() {
        println!("  Match {} at {:?}", i + 1, m.location());
        if let Some(text) = m.node().text() {
            println!("  Text: '{}'", text);
        }
    }

    // Verify the fix
    if matches.is_empty() {
        println!("\n❌ FAILURE: No matches found for pattern 'return 5;'");
    } else {
        println!("\n✅ SUCCESS: Pattern 'return 5;' matched 'return x;' where x = 5");
    }
}
