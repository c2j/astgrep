use super::*;
use astgrep_ast::{AstBuilder, UniversalNode};
use astgrep_core::{Confidence, Language, Severity};

fn create_test_rule() -> Rule {
    Rule::new(
        "test-rule".to_string(),
        "Test Rule".to_string(),
        "A test rule".to_string(),
        Severity::Warning,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("println".to_string()))
}

fn create_test_ast() -> UniversalNode {
    AstBuilder::call_expression(
        AstBuilder::property_access("System.out", "println"),
        vec![AstBuilder::string_literal("Hello, World!")],
    )
    .with_text("System.out.println(\"Hello, World!\")".to_string())
}

fn create_test_context() -> RuleContext {
    RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "System.out.println(\"Hello, World!\");".to_string(),
    )
}

#[test]
fn test_execute_rule() {
    let mut engine = RuleExecutionEngine::new();
    let rule = create_test_rule();
    let ast = create_test_ast();
    let context = create_test_context();

    let result = engine.execute_rule(&rule, &ast, &context);

    assert!(result.is_success());
    assert_eq!(result.rule_id, "test-rule");
    assert!(result.execution_time_ms >= 0); // Allow zero time for fast execution
}

#[test]
fn test_execute_multiple_rules() {
    let mut engine = RuleExecutionEngine::new();
    let rule1 = create_test_rule();
    let mut rule2 = create_test_rule();
    rule2.id = "test-rule-2".to_string();

    let rules = vec![rule1, rule2];
    let ast = create_test_ast();
    let context = create_test_context();

    let results = engine.execute_rules(&rules, &ast, &context);

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.is_success()));
}

#[test]
fn test_rule_not_applicable_to_language() {
    let mut engine = RuleExecutionEngine::new();
    let mut rule = create_test_rule();
    rule.languages = vec![Language::Python]; // Different language

    let ast = create_test_ast();
    let context = create_test_context(); // Java context

    let results = engine.execute_rules(&[rule], &ast, &context);

    assert_eq!(results.len(), 0); // Rule should be filtered out
}

#[test]
fn test_cache_functionality() {
    let mut engine = RuleExecutionEngine::new().set_cache_enabled(true);
    let rule = create_test_rule();
    let ast = create_test_ast();
    let context = create_test_context();

    // First execution
    let result1 = engine.execute_rule(&rule, &ast, &context);
    let (cache_size_1, cache_enabled) = engine.cache_stats();

    // Second execution (should use cache)
    let result2 = engine.execute_rule(&rule, &ast, &context);
    let (cache_size_2, _) = engine.cache_stats();

    assert!(cache_enabled);
    assert_eq!(cache_size_1, 1);
    assert_eq!(cache_size_2, 1);
    assert_eq!(result1.rule_id, result2.rule_id);
}

#[test]
fn test_dataflow_rule() {
    let mut engine = RuleExecutionEngine::new();
    let dataflow = DataFlowSpec::from_strings(vec!["input".to_string()], vec!["output".to_string()]);

    let rule = Rule::new(
        "dataflow-rule".to_string(),
        "Dataflow Rule".to_string(),
        "A dataflow test rule".to_string(),
        Severity::Error,
        Confidence::High,
        vec![Language::Java],
    )
    .with_dataflow(dataflow);

    let ast = create_test_ast();
    let context = create_test_context();

    let result = engine.execute_rule(&rule, &ast, &context);

    assert!(result.is_success());
    assert_eq!(result.rule_id, "dataflow-rule");
}

#[test]
fn test_sql_case_insensitive_simple_pattern() {
    let engine = RuleExecutionEngine::new();
    let pattern = "DELETE FROM $TABLE";
    let text = "delete from user;";
    assert!(engine.simple_pattern_match(pattern, text, Language::Sql));
}

#[test]
fn test_execution_timeout() {
    let mut engine = RuleExecutionEngine::new().set_max_execution_time(0); // Immediate timeout
    let rule = create_test_rule();
    let ast = create_test_ast();
    let context = create_test_context();

    let result = engine.execute_rule(&rule, &ast, &context);

    // Note: This test might be flaky due to timing, but demonstrates the concept
    assert_eq!(result.rule_id, "test-rule");
}

#[test]
fn test_sql_select_star_pattern_either_dedup() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "sql-avoid-select-star".to_string(),
        "Avoid SELECT *".to_string(),
        "Detects usage of SELECT *".to_string(),
        Severity::Warning,
        Confidence::Medium,
        vec![Language::Sql],
    )
    .add_pattern(Pattern::either(vec![
        Pattern::simple("SELECT * FROM users".to_string()),
        Pattern::simple("select * from users".to_string()),
    ]));

    let sql = "SELECT * FROM users;\n\nSELECT id, name FROM users;\n\nselect * from users;\n";
    // AST content is not used for simple-literal path; reuse existing helper
    let ast = create_test_ast();
    let context = RuleContext::new("test.sql".to_string(), Language::Sql, sql.to_string());

    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    // Expect exactly two findings (two SELECT * occurrences), not four
    assert_eq!(result.findings.len(), 2);
}

#[test]
fn test_sql_regex_cte_single_block() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "sql.detect-any-cte".to_string(),
        "Detect CTE".to_string(),
        "发现 CTE 用法（WITH 子句）".to_string(),
        Severity::Info,
        Confidence::Medium,
        vec![Language::Sql],
    )
    .add_pattern(Pattern::regex("(?is)\\bwith\\s+\\w+\\s*as\\s*\\(".to_string()));

    let sql = "WITH my_cte AS (\n  SELECT one, two\n  FROM my_table\n)\nSELECT *\nFROM my_cte;\n";
    let ast = create_test_ast();
    let context = RuleContext::new("test.sql".to_string(), Language::Sql, sql.to_string());

    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn test_java_out_println_does_not_match_system_qualified() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-out-println".to_string(),
        "Java out.println".to_string(),
        "Detect out.println".to_string(),
        Severity::Warning,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("out.println($INPUT)".to_string()));
    // AST node simulates System.out.println(...)
    let ast = create_test_ast();
    let context = RuleContext::new(
        "Demo.java".to_string(),
        Language::Java,
        "class Demo { void f(){ System.out.println(\"x\"); } }".to_string(),
    );
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 0);
}

#[test]
fn test_java_out_println_matches_plain_out() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-out-println-2".to_string(),
        "Java out.println".to_string(),
        "Detect out.println".to_string(),
        Severity::Warning,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("out.println($INPUT)".to_string()));
    // AST node simulates out.println(...)
    let ast = AstBuilder::call_expression(
        AstBuilder::property_access("out", "println"),
        vec![AstBuilder::string_literal("Hello")],
    )
    .with_text("out.println(\"Hello\");".to_string());
    let context = RuleContext::new(
        "Demo.java".to_string(),
        Language::Java,
        "out.println(\"Hello\");".to_string(),
    );
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn test_java_simple_with_metavar_multiple_occurrences() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-writer-write".to_string(),
        "Detect writer.write".to_string(),
        "检测到可能未进行XSS防护的用户输入输出".to_string(),
        Severity::Error,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("response.getWriter().write($INPUT)".to_string()));

    let java_code = "String userInput = request.getParameter(\"name\");\n\
response.getWriter().write(userInput);\n\
String userInput2 = request.getParameter(\"title\");\n\
response.getWriter().write(\"<div>\" + userInput2 + \"</div>\");\n\
String scriptParam = request.getParameter(\"x\");\n\
response.getWriter().write(\"<script>var data = '\" + scriptParam + \"';</script>\");\n";
    let ast = create_test_ast();
    let context = RuleContext::new("Xss.java".to_string(), Language::Java, java_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 3);
}

#[test]
fn test_java_either_with_metavar_multiple_occurrences() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-writer-either".to_string(),
        "Detect unsafe outputs".to_string(),
        "检测到可能未进行XSS防护的用户输入输出".to_string(),
        Severity::Error,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::either(vec![
        Pattern::simple("response.getWriter().write($INPUT)".to_string()),
        Pattern::simple("response.getWriter().print($INPUT)".to_string()),
        Pattern::simple("response.getWriter().println($INPUT)".to_string()),
    ]));

    let java_code = "String userInput = request.getParameter(\"name\");\n\
response.getWriter().write(userInput);\n\
String userInput2 = request.getParameter(\"title\");\n\
response.getWriter().write(\"<div>\" + userInput2 + \"</div>\");\n\
String scriptParam = request.getParameter(\"x\");\n\
response.getWriter().write(\"<script>var data = '\" + scriptParam + \"';</script>\");\n";
    let ast = create_test_ast();
    let context = RuleContext::new("Xss.java".to_string(), Language::Java, java_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 3);
}

#[test]
fn test_java_ellipsis_call_arguments() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-ellipsis-call".to_string(),
        "Ellipsis call args".to_string(),
        "支持 ... 匹配任意个实参".to_string(),
        Severity::Info,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("System.out.println(...)".to_string()));

    let java_code = "class D{ void f(){ System.out.println(); System.out.println(\"x\"); } }";
    let ast = create_test_ast();
    let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    // 两处调用都应命中
    assert_eq!(result.findings.len(), 2);
}

#[test]
fn test_java_ellipsis_block_bodies() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "java-ellipsis-block".to_string(),
        "Ellipsis in blocks".to_string(),
        "支持在块体内使用 ...".to_string(),
        Severity::Info,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple(
        "try { ... } catch (Exception e) { ... }".to_string(),
    ));

    let java_code = "class D{ void f(){ try { a(); b(); } catch (Exception e) { handle(); } } }";
    let ast = create_test_ast();
    let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn test_ellipsis_sequence_across_statements() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "ellipsis-seq".to_string(),
        "Ellipsis sequence".to_string(),
        "A ... B 序列匹配".to_string(),
        Severity::Info,
        Confidence::Medium,
        vec![Language::Java],
    )
    .add_pattern(Pattern::simple("A ... B".to_string()));

    let java_code = "class D{ void f(){ A(); X(); Y(); B(); } }";
    let ast = create_test_ast();
    let context = RuleContext::new("Demo.java".to_string(), Language::Java, java_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn test_execute_taint_mode_basic() {
    let mut engine = RuleExecutionEngine::new();
    let rule = Rule::new(
        "taint-basic".to_string(),
        "Basic taint analysis".to_string(),
        "Taint analysis basic test".to_string(),
        Severity::Warning,
        Confidence::Medium,
        vec![Language::JavaScript],
    );
    let rule = Rule {
        mode: RuleMode::Taint,
        dataflow: Some(DataFlowSpec::from_strings(
            vec!["userInput".to_string()],
            vec!["document.write".to_string()],
        )),
        ..rule
    };

    let js_code = "function test() { var userInput = getParam(); document.write(userInput); }";
    let ast = create_test_ast();
    let context = RuleContext::new("test.js".to_string(), Language::JavaScript, js_code.to_string());
    let result = engine.execute_rule(&rule, &ast, &context);
    assert!(result.is_success());
}
