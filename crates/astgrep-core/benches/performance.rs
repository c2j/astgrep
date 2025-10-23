use criterion::{black_box, criterion_group, criterion_main, Criterion};
use astgrep_core::*;
use astgrep_ast::*;
use astgrep_matcher::*;
use astgrep_dataflow::*;
use astgrep_rules::*;
use std::time::Duration;

fn create_large_ast(size: usize) -> UniversalNode {
    let mut root = AstBuilder::program(Vec::new());
    
    for i in 0..size {
        let function = AstBuilder::function_declaration(
            &format!("function_{}", i),
            vec![
                AstBuilder::parameter(&format!("param_{}", i), "String")
            ],
            AstBuilder::block(vec![
                AstBuilder::variable_declaration(&format!("var_{}", i), Some("String".to_string())),
                AstBuilder::expression_statement(
                    AstBuilder::call_expression(
                        AstBuilder::identifier("executeQuery"),
                        vec![AstBuilder::identifier(&format!("var_{}", i))]
                    )
                )
            ])
        );
        root = root.add_child(function);
    }
    
    root
}

fn create_test_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "sql-injection-test".to_string(),
            name: "SQL Injection Test".to_string(),
            description: "Test rule for SQL injection".to_string(),
            severity: Severity::Error,
            confidence: Confidence::High,
            languages: vec![Language::Java, Language::JavaScript, Language::Python],
            patterns: vec![
                Pattern {
                    pattern: "executeQuery($QUERY)".to_string(),
                    metavariable_pattern: None,
                    focus: None,
                    conditions: vec![],
                }
            ],
            dataflow: Some(DataFlowSpec::new(
                vec!["user_input".to_string()],
                vec!["sql_execution".to_string()],
            )),
            fix: Some("Use prepared statements".to_string()),
            metadata: std::collections::HashMap::new(),
            enabled: true,
        }
    ]
}

fn bench_ast_creation(c: &mut Criterion) {
    c.bench_function("ast_creation_small", |b| {
        b.iter(|| create_large_ast(black_box(10)))
    });
    
    c.bench_function("ast_creation_medium", |b| {
        b.iter(|| create_large_ast(black_box(100)))
    });
    
    c.bench_function("ast_creation_large", |b| {
        b.iter(|| create_large_ast(black_box(1000)))
    });
}

fn bench_pattern_matching(c: &mut Criterion) {
    let ast = create_large_ast(100);
    let mut matcher = PatternMatcher::new();
    
    c.bench_function("pattern_matching_simple", |b| {
        b.iter(|| {
            matcher.matches(black_box("executeQuery"), black_box(&ast))
        })
    });
    
    let advanced_matcher = AdvancedPatternMatcher::new();
    c.bench_function("pattern_matching_advanced", |b| {
        b.iter(|| {
            advanced_matcher.find_matches(
                black_box("executeQuery($VAR)"), 
                black_box(&ast)
            )
        })
    });
}

fn bench_dataflow_analysis(c: &mut Criterion) {
    let ast = create_large_ast(50);
    let mut analyzer = DataFlowAnalyzer::new();
    
    c.bench_function("dataflow_analysis_small", |b| {
        b.iter(|| {
            analyzer.analyze(black_box(&ast))
        })
    });
    
    let large_ast = create_large_ast(200);
    c.bench_function("dataflow_analysis_large", |b| {
        b.iter(|| {
            analyzer.analyze(black_box(&large_ast))
        })
    });
}

fn bench_rule_execution(c: &mut Criterion) {
    let ast = create_large_ast(100);
    let rules = create_test_rules();
    let mut executor = AdvancedRuleExecutor::new();
    
    c.bench_function("rule_execution_single", |b| {
        b.iter(|| {
            executor.execute_comprehensive_analysis(
                black_box(&rules),
                black_box(&ast),
                black_box(Language::Java),
                None
            )
        })
    });
    
    // Test with multiple rules
    let mut multiple_rules = rules.clone();
    for i in 1..10 {
        let mut rule = rules[0].clone();
        rule.id = format!("rule_{}", i);
        multiple_rules.push(rule);
    }
    
    c.bench_function("rule_execution_multiple", |b| {
        b.iter(|| {
            executor.execute_comprehensive_analysis(
                black_box(&multiple_rules),
                black_box(&ast),
                black_box(Language::Java),
                None
            )
        })
    });
}

fn bench_end_to_end(c: &mut Criterion) {
    let ast = create_large_ast(50);
    let rules = create_test_rules();
    let mut engine = RuleEngine::new();
    engine.rules = rules;
    
    let context = RuleContext::new(
        "test.java".to_string(),
        Language::Java,
        "test source".to_string(),
    );
    
    c.bench_function("end_to_end_analysis", |b| {
        b.iter(|| {
            engine.analyze(black_box(&ast), black_box(&context))
        })
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_ast_traversal", |b| {
        let ast = create_large_ast(1000);
        b.iter(|| {
            fn count_nodes(node: &dyn AstNode) -> usize {
                let mut count = 1;
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        count += count_nodes(child);
                    }
                }
                count
            }
            count_nodes(black_box(&ast))
        })
    });
}

criterion_group!(
    benches,
    bench_ast_creation,
    bench_pattern_matching,
    bench_dataflow_analysis,
    bench_rule_execution,
    bench_end_to_end,
    bench_memory_usage
);
criterion_main!(benches);
