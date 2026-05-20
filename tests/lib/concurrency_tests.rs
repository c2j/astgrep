//! Concurrency and thread-safety integration tests
//!
//! Tests verify that astgrep components are safe under concurrent access
//! across parser registry, rule engine, and analysis pipeline.

use astgrep_core::Language;
use astgrep_parser::LanguageParserRegistry;
use astgrep_rules::{RuleEngine, RuleParser, RuleContext};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;

fn make_context(file_name: &str, lang: Language, source: &str) -> RuleContext {
    RuleContext {
        file_path: file_name.to_string(),
        language: lang,
        source_code: source.to_string(),
        custom_data: std::collections::HashMap::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Parallel analyze same file (100 iterations)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_concurrent_analyze_same_file_100_iterations() {
    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: java-system-out
    name: "System.out.println"
    severity: WARNING
    confidence: HIGH
    languages: [java]
    patterns:
      - "System.out.println"
    message: "Console output detected"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }
    let rule_engine = Arc::new(Mutex::new(rule_engine));

    let source = r#"
public class ConcurrentTest {
    public void run() {
        System.out.println("Hello");
    }
}
"#;

    let path = PathBuf::from("ConcurrentTest.java");
    let source = Arc::new(source.to_string());
    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    for i in 0..100 {
        let parser_registry = Arc::clone(&parser_registry);
        let rule_engine = Arc::clone(&rule_engine);
        let source = Arc::clone(&source);
        let results = Arc::clone(&results);
        let path = path.clone();

        let handle = thread::spawn(move || {
            let ast = parser_registry.parse_file(&path, &source)
                .expect(&format!("parse iteration {}", i));
            let ctx = make_context("ConcurrentTest.java", Language::Java, &source);
            let engine = rule_engine.lock().unwrap();
            let findings = engine.analyze(&*ast, &ctx).expect("analyze");
            let mut results = results.lock().unwrap();
            results.push((i, findings.len()));
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread should complete");
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 100, "All 100 iterations should complete");
    for (i, count) in results.iter() {
        assert!(*count > 0, "Iteration {} should find at least one finding", i);
    }
    println!("✓ 100 concurrent analyses of same file completed without crash");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Parallel analyze different files with shared RuleEngine
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_concurrent_analyze_different_files_shared_rule_engine() {
    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: generic-eval
    name: "Dangerous eval"
    severity: CRITICAL
    confidence: HIGH
    languages: [javascript]
    patterns:
      - "eval($X)"
    message: "eval() usage detected"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }
    let rule_engine = Arc::new(Mutex::new(rule_engine));

    let files: Vec<(&str, &str)> = vec![
        ("file1.js", "function a() { eval('1+1'); }"),
        ("file2.js", "function b() { eval('2+2'); }"),
        ("file3.js", "function c() { eval('3+3'); }"),
        ("file4.js", "function d() { eval('4+4'); }"),
        ("file5.js", "function e() { eval('5+5'); }"),
    ];

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for (name, content) in files {
        let parser_registry = Arc::clone(&parser_registry);
        let rule_engine = Arc::clone(&rule_engine);
        let results = Arc::clone(&results);
        let name = name.to_string();
        let content = content.to_string();

        let handle = thread::spawn(move || {
            let path = PathBuf::from(&name);
            let ast = parser_registry.parse_file(&path, &content)
                .expect(&format!("parse {}", name));
            let ctx = make_context(&name, Language::JavaScript, &content);
            let engine = rule_engine.lock().unwrap();
            let findings = engine.analyze(&*ast, &ctx).expect("analyze");
            let mut results = results.lock().unwrap();
            results.push((name, findings.len()));
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread should complete");
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 5, "All 5 files should be analyzed");
    for (name, count) in results.iter() {
        assert!(*count > 0, "{} should have findings", name);
    }
    println!("✓ Parallel analysis of different files with shared RuleEngine succeeded");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Concurrent parser registry access
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_concurrent_parser_registry_access() {
    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let num_threads = 8;
    let iterations_per_thread = 25;
    let results = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    for thread_id in 0..num_threads {
        let parser_registry = Arc::clone(&parser_registry);
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let mut local_ok = 0usize;
            for i in 0..iterations_per_thread {
                let lang = match (thread_id + i) % 3 {
                    0 => Language::Java,
                    1 => Language::JavaScript,
                    _ => Language::Python,
                };
                let (filename, source) = match lang {
                    Language::Java => (
                        format!("t{}_i{}.java", thread_id, i),
                        "public class T { void m() { System.out.println(1); } }".to_string(),
                    ),
                    Language::JavaScript => (
                        format!("t{}_i{}.js", thread_id, i),
                        "function f() { console.log(1); }".to_string(),
                    ),
                    Language::Python => (
                        format!("t{}_i{}.py", thread_id, i),
                        "def f(): print(1)\n".to_string(),
                    ),
                    _ => unreachable!(),
                };

                let path = PathBuf::from(&filename);
                if parser_registry.parse_file(&path, &source).is_ok() {
                    local_ok += 1;
                }
            }
            let mut results = results.lock().unwrap();
            results.push((thread_id, local_ok));
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread should complete");
    }

    let results = results.lock().unwrap();
    let total_ok: usize = results.iter().map(|(_, ok)| ok).sum();
    let expected = num_threads * iterations_per_thread;
    assert_eq!(total_ok, expected, "All {} parses should succeed", expected);
    println!("✓ Concurrent parser registry access: {}/{} succeeded", total_ok, expected);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Concurrent rule engine execution (parse + analyze in parallel)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_concurrent_rule_engine_execution() {
    let parser_registry = Arc::new(LanguageParserRegistry::new());
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: java-sql-injection
    name: "SQL Injection"
    severity: CRITICAL
    confidence: HIGH
    languages: [java]
    patterns:
      - "executeQuery($QUERY)"
    message: "SQL injection risk"

  - id: js-xss
    name: "XSS"
    severity: CRITICAL
    confidence: HIGH
    languages: [javascript]
    patterns:
      - "innerHTML"
    message: "XSS risk"

  - id: python-os-system
    name: "Command Injection"
    severity: CRITICAL
    confidence: HIGH
    languages: [python]
    patterns:
      - "os.system($CMD)"
    message: "Command injection risk"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }
    let rule_engine = Arc::new(Mutex::new(rule_engine));

    let tasks: Vec<(&str, Language, &str)> = vec![
        (
            "sql.java",
            Language::Java,
            r#"public class A { void m() { stmt.executeQuery("SELECT * FROM t"); } }"#,
        ),
        (
            "xss.js",
            Language::JavaScript,
            r#"function f() { document.body.innerHTML = 'x'; }"#,
        ),
        (
            "cmd.py",
            Language::Python,
            r#"import os; os.system('ls')"#,
        ),
    ];

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for _ in 0..20 {
        for (name, lang, source) in &tasks {
            let parser_registry = Arc::clone(&parser_registry);
            let rule_engine = Arc::clone(&rule_engine);
            let results = Arc::clone(&results);
            let name = name.to_string();
            let lang = *lang;
            let source = source.to_string();

            let handle = thread::spawn(move || {
                let path = PathBuf::from(&name);
                let ast = parser_registry.parse_file(&path, &source)
                    .expect(&format!("parse {}", name));
                let ctx = make_context(&name, lang, &source);
                let engine = rule_engine.lock().unwrap();
                let findings = engine.analyze(&*ast, &ctx).expect("analyze");
                let mut results = results.lock().unwrap();
                results.push((name, findings.len()));
            });
            handles.push(handle);
        }
    }

    for h in handles {
        h.join().expect("thread should complete");
    }

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 60, "All 60 tasks (20 rounds × 3 files) should complete");
    let total_findings: usize = results.iter().map(|(_, c)| c).sum();
    assert!(total_findings > 0, "Should have findings across all runs");
    println!(
        "✓ Concurrent rule engine execution: {} tasks, {} total findings",
        results.len(),
        total_findings
    );
}
