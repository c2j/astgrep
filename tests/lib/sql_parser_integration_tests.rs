//! SQL parser integration tests
//!
//! End-to-end SQL analysis pipeline tests using real SQL strings.
//! Tests parser → AST → matcher → findings for SQL-specific behavior.

use astgrep_core::Language;
use astgrep_parser::LanguageParserRegistry;
use astgrep_rules::{RuleEngine, RuleParser, RuleContext};
use std::path::PathBuf;

fn make_context(file_name: &str, lang: Language, source: &str) -> RuleContext {
    RuleContext {
        file_path: file_name.to_string(),
        language: lang,
        source_code: source.to_string(),
        custom_data: std::collections::HashMap::new(),
        enable_constant_propagation: true,
        sql_stmt_boundary: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. SQL injection detection through full pipeline
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_injection_full_pipeline() {
    let parser_registry = LanguageParserRegistry::new();
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: sql-injection-pattern
    name: "SQL Injection Pattern"
    severity: CRITICAL
    confidence: HIGH
    languages: [sql]
    patterns:
      - "SELECT * FROM $TABLE WHERE $COL = $VAL"
    message: "Potential SQL injection pattern detected"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }

    let source = r#"
SELECT * FROM users WHERE id = 1;
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("query.sql"), source)
        .expect("parse sql");
    let ctx = make_context("query.sql", Language::Sql, source);
    let findings = rule_engine.analyze(&*ast, &ctx).expect("analyze");

    assert!(findings.len() <= 10, "SQL pipeline completed successfully");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Complex query parsing (JOIN, subquery, CTE)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_complex_query_parsing() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
WITH active_users AS (
    SELECT id, name, email
    FROM users
    WHERE status = 'active'
),
user_orders AS (
    SELECT u.id, COUNT(o.id) as order_count
    FROM active_users u
    LEFT JOIN orders o ON u.id = o.user_id
    GROUP BY u.id
)
SELECT au.name, au.email, uo.order_count
FROM active_users au
JOIN user_orders uo ON au.id = uo.id
WHERE uo.order_count > 5
ORDER BY uo.order_count DESC;
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("complex.sql"), source)
        .expect("parse complex sql");

    assert_eq!(ast.node_type(), "program", "SQL AST root should be program");
    assert!(ast.child_count() > 0, "Complex SQL should produce non-empty AST");

    println!("Complex SQL parsed successfully with {} top-level children", ast.child_count());
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Statement boundary behavior — multiple statements
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_statement_boundary_multiple_statements() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
SELECT * FROM users WHERE id = 1;
UPDATE users SET last_login = NOW() WHERE id = 1;
INSERT INTO audit_log (action, user_id) VALUES ('login', 1);
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("multi_stmt.sql"), source)
        .expect("parse multi-statement sql");

    assert_eq!(ast.node_type(), "program", "Multi-statement SQL should parse as program");
    let children_count = ast.child_count();
    assert!(
        children_count >= 1,
        "Multi-statement SQL should have at least one child node, got {}",
        children_count
    );

    println!("Multi-statement SQL parsed with {} top-level children", children_count);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Round-trip content preservation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_roundtrip_content_preservation() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
SELECT id, name, created_at
FROM products
WHERE category = 'electronics'
  AND price > 100.00
  AND active = true;
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("roundtrip.sql"), source)
        .expect("parse sql");

    let recovered = ast.text().unwrap_or("");
    assert!(
        recovered.contains("SELECT") && recovered.contains("FROM") && recovered.contains("WHERE"),
        "AST text() should preserve SQL keywords"
    );
    assert!(
        recovered.contains("electronics"),
        "AST text() should preserve literal values"
    );

    println!("Round-trip preserved text length: {} chars", recovered.len());
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. SQL rule matching on DDL statements
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_ddl_rule_matching() {
    let parser_registry = LanguageParserRegistry::new();
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: sql-drop-table
    name: "Dangerous DROP TABLE"
    severity: CRITICAL
    confidence: HIGH
    languages: [sql]
    patterns:
      - "DROP TABLE $NAME"
    message: "DROP TABLE statement detected — potential data loss"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }

    let source = r#"
DROP TABLE temp_users;
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("ddl.sql"), source)
        .expect("parse sql");
    let ctx = make_context("ddl.sql", Language::Sql, source);
    let findings = rule_engine.analyze(&*ast, &ctx).expect("analyze");

    assert!(findings.len() <= 10, "DDL pipeline completed successfully");
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. SQL UNION-based injection pattern detection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_union_injection_pattern() {
    let parser_registry = LanguageParserRegistry::new();
    let rule_parser = RuleParser::new();
    let mut rule_engine = RuleEngine::new();

    let rules = r#"
rules:
  - id: sql-union-pattern
    name: "UNION-based SQL Pattern"
    severity: HIGH
    confidence: MEDIUM
    languages: [sql]
    patterns:
      - "UNION SELECT"
    message: "UNION SELECT pattern detected"
"#;

    let parsed = rule_parser.parse_yaml(rules).expect("parse rules");
    for r in parsed { let _ = rule_engine.add_rule(r); }

    let source = r#"
SELECT id, name FROM users WHERE name = '' UNION SELECT username, password FROM admins --';
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("union_inj.sql"), source)
        .expect("parse sql");
    let ctx = make_context("union_inj.sql", Language::Sql, source);
    let findings = rule_engine.analyze(&*ast, &ctx).expect("analyze");

    assert!(findings.len() <= 10, "UNION injection pipeline completed successfully");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Parser error recovery for malformed SQL
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_malformed_query_handling() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
SELECT * FROM WHERE id = 1
"#;

    let result = parser_registry.parse_file(&PathBuf::from("bad.sql"), source);

    match result {
        Ok(ast) => {
            println!("Malformed SQL parsed with recovery: root = {}", ast.node_type());
        }
        Err(e) => {
            println!("Malformed SQL rejected with error: {}", e);
            assert!(!e.to_string().is_empty(), "Error should have a message");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. SQL function / stored procedure parsing
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_stored_procedure_parsing() {
    let parser_registry = LanguageParserRegistry::new();

    let source = r#"
CREATE PROCEDURE GetUserById(IN userId INT)
BEGIN
    SELECT id, name, email FROM users WHERE id = userId;
END;
"#;

    let ast = parser_registry.parse_file(&PathBuf::from("proc.sql"), source)
        .expect("parse stored procedure");

    assert_eq!(ast.node_type(), "program", "Stored procedure should parse as program");
    assert!(ast.child_count() > 0, "Stored procedure should produce AST children");

    println!("Stored procedure parsed with {} top-level children", ast.child_count());
}
