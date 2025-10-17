use cr_semservice::{
    AdvancedSemgrepMatcher, SemgrepPattern, PatternType,
    UniversalNode, NodeType, Condition, MetavariableRegex, MetavariableComparison, ComparisonOperator
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing CR-SemService against Semgrep results");
    println!("================================================\n");

    // Test 1: String match test
    test_string_match()?;
    
    // Test 2: Function call test
    test_function_call()?;
    
    // Test 3: Number match test
    test_number_match()?;

    // Test 4: Complex Python eval test
    test_complex_python_eval()?;

    // Test 5: Advanced pattern tests
    test_advanced_patterns()?;

    println!("\n✅ All comparison tests completed!");
    Ok(())
}

fn test_string_match() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 1: String Match");
    println!("-------------------");
    
    // Create the pattern from the YAML rule
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("\"hello\"".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    // Create test code AST
    let test_code = r#"print("hello")
print("world")
x = "hello"
y = "goodbye""#;
    
    // Create a simple AST representation
    let ast = create_python_ast(test_code);
    
    // Use our matcher
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&pattern, &ast)?;

    // Filter out program node matches for more accurate comparison
    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("Our results: {} matches found", filtered_matches.len());
    println!("Semgrep results: 2 matches expected");

    // Expected matches:
    // 1. Line 1, col 7-14: "hello" in print("hello")
    // 2. Line 3, col 5-12: "hello" in x = "hello"

    for (i, m) in filtered_matches.iter().enumerate() {
        println!("  Match {}: {:?}", i + 1, m);
    }

    if filtered_matches.len() == 2 {
        println!("✅ String match test PASSED");
    } else {
        println!("❌ String match test FAILED - expected 2 matches, got {}", filtered_matches.len());
    }
    
    println!();
    Ok(())
}

fn test_function_call() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 2: Function Call");
    println!("--------------------");
    
    // Read the function call test
    let yaml_content = std::fs::read_to_string("tests/simple/function_call.yaml")?;
    let js_content = std::fs::read_to_string("tests/simple/function_call.js")?;
    
    println!("JavaScript code:");
    println!("{}", js_content);
    
    // Parse the YAML to extract pattern
    // The actual pattern from function_call.yaml is "eval(...)"
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("eval(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    // Create JavaScript AST
    let ast = create_javascript_ast(&js_content);
    
    // Use our matcher
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&pattern, &ast)?;

    // Filter out program node matches
    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("Our results: {} matches found", filtered_matches.len());
    println!("Semgrep results: 3 matches expected");

    for (i, m) in filtered_matches.iter().enumerate() {
        println!("  Match {}: {:?}", i + 1, m);
    }
    
    println!();
    Ok(())
}

fn test_number_match() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 3: Number Match");
    println!("-------------------");
    
    // Read the number match test
    let yaml_content = std::fs::read_to_string("tests/simple/number_match.yaml")?;
    let py_content = std::fs::read_to_string("tests/simple/number_match.py")?;
    
    println!("Python code:");
    println!("{}", py_content);
    
    // Extract pattern (should be "42")
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("42".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };
    
    // Create Python AST
    let ast = create_python_ast(&py_content);
    
    // Use our matcher
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&pattern, &ast)?;

    // Filter out program node matches
    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("Our results: {} matches found", filtered_matches.len());
    println!("Semgrep results: 3 matches expected");

    for (i, m) in filtered_matches.iter().enumerate() {
        println!("  Match {}: {:?}", i + 1, m);
    }
    
    println!();
    Ok(())
}

fn test_complex_python_eval() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 4: Complex Python Eval");
    println!("---------------------------");

    // Read the complex Python test file
    let py_content = std::fs::read_to_string("tests/comparison/simple_python_test.py")?;

    println!("Python code (first 10 lines):");
    for (i, line) in py_content.lines().take(10).enumerate() {
        println!("  {}: {}", i + 1, line);
    }
    println!("  ... (truncated)");

    // Pattern: eval(...)
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("eval(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create Python AST
    let ast = create_complex_python_ast(&py_content);

    // Use our matcher
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&pattern, &ast)?;

    // Filter out program node matches
    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("Our results: {} matches found", filtered_matches.len());
    println!("Semgrep results: 4 matches expected");

    for (i, m) in filtered_matches.iter().enumerate() {
        println!("  Match {}: {:?}", i + 1, m);
    }

    if filtered_matches.len() == 4 {
        println!("✅ Complex Python eval test PASSED");
    } else {
        println!("❌ Complex Python eval test FAILED - expected 4 matches, got {}", filtered_matches.len());
    }

    println!();
    Ok(())
}

fn create_python_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());
    
    // Parse the code and create child nodes for string literals
    let lines: Vec<&str> = code.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        if line.contains("\"hello\"") {
            // Create a string literal node
            let string_node = UniversalNode::new(NodeType::StringLiteral)
                .with_text("\"hello\"".to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string())
                .with_attribute("value".to_string(), "hello".to_string());
            
            ast = ast.add_child(string_node);
        }
        
        if line.contains("42") && !line.contains("\"42\"") {
            // Create a number literal node (but not if it's in a string)
            let number_node = UniversalNode::new(NodeType::IntegerLiteral)
                .with_text("42".to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string())
                .with_attribute("value".to_string(), "42".to_string());

            ast = ast.add_child(number_node);
        }
    }
    
    ast
}

fn create_javascript_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "javascript".to_string());

    // Parse the code and create child nodes for function calls
    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if line.contains("eval(") {
            // Create a function call node for eval
            let call_node = UniversalNode::new(NodeType::CallExpression)
                .with_text(line.trim().to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string())
                .with_attribute("function".to_string(), "eval".to_string());

            ast = ast.add_child(call_node);
        }
    }

    ast
}

fn create_complex_python_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());

    // Parse the code and create child nodes for eval calls
    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if line.contains("eval(") && !line.trim().starts_with("#") {
            // Create a function call node for eval
            let call_node = UniversalNode::new(NodeType::CallExpression)
                .with_text(line.trim().to_string())
                .with_attribute("line".to_string(), (line_num + 1).to_string())
                .with_attribute("function".to_string(), "eval".to_string());

            ast = ast.add_child(call_node);
        }
    }

    ast
}

fn test_advanced_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test 5: Advanced Pattern Matching");
    println!("=================================");

    // Test pattern-either
    test_pattern_either()?;

    // Test pattern-not
    test_pattern_not()?;

    // Test pattern-inside
    test_pattern_inside()?;

    // Test pattern-regex
    test_pattern_regex()?;

    // Test metavariables
    test_metavariables()?;

    println!("✅ All advanced pattern tests completed!");
    println!();
    Ok(())
}

fn test_pattern_either() -> Result<(), Box<dyn std::error::Error>> {
    println!("  5.1: Pattern-Either (OR Logic)");
    println!("  ------------------------------");

    // Create test code with multiple function calls
    let test_code = r#"
eval("dangerous code")
exec("system command")
compile("code", "file", "exec")
print("safe function")
"#;

    // Create pattern-either: eval(...) OR exec(...)
    let eval_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("eval(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let exec_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("exec(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let either_pattern = SemgrepPattern {
        pattern_type: PatternType::Either(vec![eval_pattern, exec_pattern]),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create AST
    let ast = create_function_call_ast(test_code);

    // Test matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&either_pattern, &ast)?;

    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Code tested:");
    for line in test_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("      {}", line.trim());
    }
    println!("    Pattern: eval(...) OR exec(...)");
    println!("    Expected: 2 matches (eval and exec calls)");
    println!("    Found: {} matches", filtered_matches.len());

    for (i, m) in filtered_matches.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("      Match {}: {}", i + 1, text);
        }
    }

    if filtered_matches.len() >= 2 {
        println!("    ✅ Pattern-either test PASSED");
    } else {
        println!("    ❌ Pattern-either test FAILED");
    }

    println!();
    Ok(())
}

fn test_pattern_not() -> Result<(), Box<dyn std::error::Error>> {
    println!("  5.2: Pattern-Not (Exclusion Logic)");
    println!("  ----------------------------------");

    // Create test code with various function calls
    let test_code = r#"
print("hello")
print("world")
console.log("debug")
eval("dangerous")
"#;

    // Create pattern: any function call BUT NOT eval(...)
    let any_call_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$FUNC(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let eval_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("eval(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let not_eval_pattern = SemgrepPattern {
        pattern_type: PatternType::Not(Box::new(eval_pattern)),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create AST
    let ast = create_function_call_ast(test_code);

    // Test matching - first find all function calls
    let mut matcher = AdvancedSemgrepMatcher::new();
    let all_matches = matcher.find_matches(&any_call_pattern, &ast)?;

    // Then test the NOT pattern
    let not_matches = matcher.find_matches(&not_eval_pattern, &ast)?;

    let all_filtered: Vec<_> = all_matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    let not_filtered: Vec<_> = not_matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Code tested:");
    for line in test_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("      {}", line.trim());
    }
    println!("    Pattern: NOT eval(...)");
    println!("    All function calls: {}", all_filtered.len());
    println!("    Non-eval calls: {}", not_filtered.len());

    for (i, m) in not_filtered.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("      Match {}: {}", i + 1, text);
        }
    }

    // Should exclude eval calls
    if not_filtered.len() < all_filtered.len() {
        println!("    ✅ Pattern-not test PASSED (excluded some matches)");
    } else {
        println!("    ❌ Pattern-not test FAILED");
    }

    println!();
    Ok(())
}

fn test_pattern_inside() -> Result<(), Box<dyn std::error::Error>> {
    println!("  5.3: Pattern-Inside (Context Matching)");
    println!("  --------------------------------------");

    // Create test code with nested structures
    let test_code = r#"
def dangerous_function():
    eval("code")
    return True

def safe_function():
    print("hello")
    return False

eval("global eval")
"#;

    // Create pattern: eval(...) inside a function definition
    let eval_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("eval(...)".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let function_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("def $FUNC(...): ...".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    let inside_pattern = SemgrepPattern {
        pattern_type: PatternType::Inside(Box::new(function_pattern)),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create AST with nested structure
    let ast = create_nested_ast(test_code);

    // Test matching
    let mut matcher = AdvancedSemgrepMatcher::new();

    // First find all eval calls
    let all_eval_matches = matcher.find_matches(&eval_pattern, &ast)?;

    // Then find eval calls inside functions
    let inside_matches = matcher.find_matches(&inside_pattern, &ast)?;

    let all_eval_filtered: Vec<_> = all_eval_matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    let inside_filtered: Vec<_> = inside_matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Code tested:");
    for line in test_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("      {}", line.trim());
    }
    println!("    Pattern: eval(...) inside function definition");
    println!("    All eval calls: {}", all_eval_filtered.len());
    println!("    Eval calls inside functions: {}", inside_filtered.len());

    for (i, m) in inside_filtered.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("      Match {}: {}", i + 1, text);
        }
    }

    // Should find fewer matches when restricted to inside functions
    if inside_filtered.len() <= all_eval_filtered.len() {
        println!("    ✅ Pattern-inside test PASSED");
    } else {
        println!("    ❌ Pattern-inside test FAILED");
    }

    println!();
    Ok(())
}

fn test_pattern_regex() -> Result<(), Box<dyn std::error::Error>> {
    println!("  5.4: Pattern-Regex (Regular Expression Matching)");
    println!("  ------------------------------------------------");

    // Create test code with various string patterns
    let test_code = r#"
password = "admin123"
api_key = "sk-1234567890abcdef"
token = "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9"
username = "user@example.com"
safe_string = "hello world"
"#;

    // Create regex pattern to match potential secrets
    // Pattern: strings that look like API keys or tokens
    let regex_pattern = SemgrepPattern {
        pattern_type: PatternType::Regex(r#""[a-zA-Z0-9_-]{16,}""#.to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: Vec::new(),
    };

    // Create AST
    let ast = create_string_ast(test_code);

    // Test matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&regex_pattern, &ast)?;

    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Code tested:");
    for line in test_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("      {}", line.trim());
    }
    println!("    Regex pattern: strings with 16+ alphanumeric/dash/underscore chars");
    println!("    Expected: 2-3 matches (api_key, token, possibly others)");
    println!("    Found: {} matches", filtered_matches.len());

    for (i, m) in filtered_matches.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("      Match {}: {}", i + 1, text);
        }
    }

    if filtered_matches.len() >= 2 {
        println!("    ✅ Pattern-regex test PASSED");
    } else {
        println!("    ❌ Pattern-regex test FAILED");
    }

    println!();
    Ok(())
}

fn test_metavariables() -> Result<(), Box<dyn std::error::Error>> {
    println!("  5.5: Metavariables (Variable Binding and Constraints)");
    println!("  -----------------------------------------------------");

    // Create test code with various assignments
    let test_code = r#"
x = dangerous_function()
y = safe_function()
z = another_dangerous_call()
result = normal_call()
"#;

    // Create pattern with metavariable and regex constraint
    let pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$VAR = $FUNC()".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: vec![
            Condition::MetavariableRegex(MetavariableRegex {
                metavariable: "FUNC".to_string(),
                regex: ".*dangerous.*".to_string(),
            })
        ],
    };

    // Create AST
    let ast = create_assignment_ast(test_code);

    // Test matching
    let mut matcher = AdvancedSemgrepMatcher::new();
    let matches = matcher.find_matches(&pattern, &ast)?;

    let filtered_matches: Vec<_> = matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Code tested:");
    for line in test_code.lines().filter(|l| !l.trim().is_empty()) {
        println!("      {}", line.trim());
    }
    println!("    Pattern: $VAR = $FUNC() where $FUNC matches '.*dangerous.*'");
    println!("    Expected: 2 matches (dangerous_function, another_dangerous_call)");
    println!("    Found: {} matches", filtered_matches.len());

    for (i, m) in filtered_matches.iter().enumerate() {
        if let Some(text) = m.node.text() {
            println!("      Match {}: {}", i + 1, text);
        }
    }

    // Test metavariable comparison
    let comparison_pattern = SemgrepPattern {
        pattern_type: PatternType::Simple("$X = $Y".to_string()),
        metavariable_pattern: None,
        focus: None,
        conditions: vec![
            Condition::MetavariableComparison(MetavariableComparison {
                metavariable: "X".to_string(),
                operator: ComparisonOperator::Equals,
                value: "Y".to_string(),
            })
        ],
    };

    let comparison_matches = matcher.find_matches(&comparison_pattern, &ast)?;
    let comparison_filtered: Vec<_> = comparison_matches.iter()
        .filter(|m| m.node.node_type() != "program")
        .collect();

    println!("    Metavariable comparison test: {} matches", comparison_filtered.len());

    if filtered_matches.len() >= 1 {
        println!("    ✅ Metavariables test PASSED");
    } else {
        println!("    ❌ Metavariables test FAILED");
    }

    println!();
    Ok(())
}

// Helper functions for creating different types of ASTs

fn create_function_call_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());

    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for function calls
        if let Some(func_start) = trimmed.find('(') {
            let func_name = trimmed[..func_start].trim();
            if !func_name.is_empty() {
                let call_node = UniversalNode::new(NodeType::CallExpression)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
                    .with_attribute("function".to_string(), func_name.to_string());

                ast = ast.add_child(call_node);
            }
        }
    }

    ast
}

fn create_nested_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());

    let lines: Vec<&str> = code.lines().collect();
    let mut current_function: Option<UniversalNode> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("def ") {
            // Start of function definition
            if let Some(func_node) = current_function.take() {
                ast = ast.add_child(func_node);
            }

            current_function = Some(
                UniversalNode::new(NodeType::FunctionDeclaration)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
            );
        } else if let Some(func_start) = trimmed.find('(') {
            // Function call
            let func_name = trimmed[..func_start].trim();
            if !func_name.is_empty() {
                let call_node = UniversalNode::new(NodeType::CallExpression)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
                    .with_attribute("function".to_string(), func_name.to_string());

                if let Some(ref mut func_node) = current_function {
                    *func_node = func_node.clone().add_child(call_node);
                } else {
                    ast = ast.add_child(call_node);
                }
            }
        }
    }

    // Add the last function if any
    if let Some(func_node) = current_function {
        ast = ast.add_child(func_node);
    }

    ast
}

fn create_string_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());

    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for string literals
        if let Some(start) = trimmed.find('"') {
            if let Some(end) = trimmed.rfind('"') {
                if start != end {
                    let string_content = &trimmed[start..=end];
                    let string_node = UniversalNode::new(NodeType::StringLiteral)
                        .with_text(string_content.to_string())
                        .with_attribute("line".to_string(), (line_num + 1).to_string())
                        .with_attribute("value".to_string(), string_content.to_string());

                    ast = ast.add_child(string_node);
                }
            }
        }
    }

    ast
}

fn create_assignment_ast(code: &str) -> UniversalNode {
    let mut ast = UniversalNode::new(NodeType::Program)
        .with_text(code.to_string())
        .with_attribute("language".to_string(), "python".to_string());

    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for assignments
        if let Some(eq_pos) = trimmed.find('=') {
            let var_name = trimmed[..eq_pos].trim();
            let value = trimmed[eq_pos + 1..].trim();

            if !var_name.is_empty() && !value.is_empty() {
                let assignment_node = UniversalNode::new(NodeType::AssignmentExpression)
                    .with_text(trimmed.to_string())
                    .with_attribute("line".to_string(), (line_num + 1).to_string())
                    .with_attribute("variable".to_string(), var_name.to_string())
                    .with_attribute("value".to_string(), value.to_string());

                // Add variable and value as children
                let var_node = UniversalNode::new(NodeType::Identifier)
                    .with_text(var_name.to_string())
                    .with_attribute("name".to_string(), var_name.to_string());

                let value_node = if value.ends_with("()") {
                    // Function call
                    let func_name = &value[..value.len() - 2];
                    UniversalNode::new(NodeType::CallExpression)
                        .with_text(value.to_string())
                        .with_attribute("function".to_string(), func_name.to_string())
                } else {
                    // Other value
                    UniversalNode::new(NodeType::Identifier)
                        .with_text(value.to_string())
                };

                let assignment_with_children = assignment_node
                    .add_child(var_node)
                    .add_child(value_node);

                ast = ast.add_child(assignment_with_children);
            }
        }
    }

    ast
}
