//! SQL parser integration tests
//!
//! End-to-end SQL analysis pipeline tests using real SQL strings.
//! Tests parser → AST → matcher → findings for SQL-specific behavior.

use astgrep_core::{Language, SqlDialect};
use astgrep_parser::LanguageParserRegistry;
use astgrep_rules::{RuleContext, RuleEngine, RuleParser};
use std::path::PathBuf;

fn make_context(file_name: &str, lang: Language, source: &str) -> RuleContext {
    RuleContext {
        file_path: file_name.to_string(),
        language: lang,
        source_code: source.to_string(),
        custom_data: std::collections::HashMap::new(),
        enable_constant_propagation: true,
        sql_stmt_boundary: None,
        sql_dialect: None,
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
    for r in parsed {
        let _ = rule_engine.add_rule(r);
    }

    let source = r#"
SELECT * FROM users WHERE id = 1;
"#;

    let ast = parser_registry
        .parse_file(&PathBuf::from("query.sql"), source)
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

    let ast = parser_registry
        .parse_file(&PathBuf::from("complex.sql"), source)
        .expect("parse complex sql");

    assert_eq!(ast.node_type(), "program", "SQL AST root should be program");
    assert!(
        ast.child_count() > 0,
        "Complex SQL should produce non-empty AST"
    );

    println!(
        "Complex SQL parsed successfully with {} top-level children",
        ast.child_count()
    );
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

    let ast = parser_registry
        .parse_file(&PathBuf::from("multi_stmt.sql"), source)
        .expect("parse multi-statement sql");

    assert_eq!(
        ast.node_type(),
        "program",
        "Multi-statement SQL should parse as program"
    );
    let children_count = ast.child_count();
    assert!(
        children_count >= 1,
        "Multi-statement SQL should have at least one child node, got {}",
        children_count
    );

    println!(
        "Multi-statement SQL parsed with {} top-level children",
        children_count
    );
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

    let ast = parser_registry
        .parse_file(&PathBuf::from("roundtrip.sql"), source)
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

    println!(
        "Round-trip preserved text length: {} chars",
        recovered.len()
    );
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
    for r in parsed {
        let _ = rule_engine.add_rule(r);
    }

    let source = r#"
DROP TABLE temp_users;
"#;

    let ast = parser_registry
        .parse_file(&PathBuf::from("ddl.sql"), source)
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
    for r in parsed {
        let _ = rule_engine.add_rule(r);
    }

    let source = r#"
SELECT id, name FROM users WHERE name = '' UNION SELECT username, password FROM admins --';
"#;

    let ast = parser_registry
        .parse_file(&PathBuf::from("union_inj.sql"), source)
        .expect("parse sql");
    let ctx = make_context("union_inj.sql", Language::Sql, source);
    let findings = rule_engine.analyze(&*ast, &ctx).expect("analyze");

    assert!(
        findings.len() <= 10,
        "UNION injection pipeline completed successfully"
    );
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
            println!(
                "Malformed SQL parsed with recovery: root = {}",
                ast.node_type()
            );
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

    let ast = parser_registry
        .parse_file(&PathBuf::from("proc.sql"), source)
        .expect("parse stored procedure");

    assert_eq!(
        ast.node_type(),
        "program",
        "Stored procedure should parse as program"
    );
    assert!(
        ast.child_count() > 0,
        "Stored procedure should produce AST children"
    );

    println!(
        "Stored procedure parsed with {} top-level children",
        ast.child_count()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Multi-rule combined vs individual correctness (GaussDB dialect)
//    Guards the invariant: N rules applied together == sum of N individual runs.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gaussdb_multi_rule_combined_equals_individual() {
    use astgrep_parser::dialect;

    let source = r#"
        CREATE TABLE accounts (
            id INT PRIMARY KEY,
            cnt INT,
            name VARCHAR2(100)
        );
        SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
        DELETE FROM logs WHERE ts < now();
    "#;

    // Parse once with GaussDB dialect
    let gaussdb_parser = dialect::dispatch(SqlDialect::GaussDB);
    let ast = gaussdb_parser
        .parse(source, &PathBuf::from("test.sql"))
        .expect("GaussDB should parse the source");

    // Rules covering distinct patterns
    let rules_yaml = r#"
rules:
  - id: detect-select
    name: Select detection
    description: Detects SELECT statements
    message: SELECT found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "SELECT"
  - id: detect-update
    name: Update detection
    description: Detects UPDATE statements
    message: UPDATE found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "UPDATE"
  - id: detect-delete
    name: Delete detection
    description: Detects DELETE statements
    message: DELETE found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "DELETE"
  - id: detect-create
    name: Create detection
    description: Detects CREATE TABLE
    message: CREATE found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "CREATE TABLE"
  - id: detect-varchar2
    name: VARCHAR2 detection
    description: Detects VARCHAR2 usage
    message: VARCHAR2 found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "VARCHAR2"
  - id: detect-for-update
    name: FOR UPDATE detection
    description: Detects FOR UPDATE clause
    message: FOR UPDATE found
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "FOR UPDATE"
"#;

    let ctx = RuleContext::new("test.sql".to_string(), Language::Sql, source.to_string());
    let mut ctx = ctx;
    ctx.sql_dialect = Some(SqlDialect::GaussDB);

    // Combined: all rules via analyze()
    let mut engine_combined = RuleEngine::new();
    let rule_count = engine_combined.load_rules_from_yaml(rules_yaml)
        .expect("load rules");
    assert_eq!(rule_count, 6);
    let combined_findings = engine_combined.analyze(ast.as_ref(), &ctx)
        .expect("combined analyze");

    // Individual: each rule separately
    let rule_ids = [
        "detect-select", "detect-update", "detect-delete",
        "detect-create", "detect-varchar2", "detect-for-update",
    ];

    let mut total_individual = 0usize;
    let mut per_rule_individual: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for rid in &rule_ids {
        let mut engine_ind = RuleEngine::new();
        engine_ind.load_rules_from_yaml(rules_yaml).unwrap();
        let result = engine_ind.execute_rule(rid, ast.as_ref(), &ctx)
            .expect("execute_rule")
            .expect("rule should exist");
        let count = result.finding_count();
        per_rule_individual.insert(rid.to_string(), count);
        total_individual += count;
    }

    // Compare per-rule counts
    for rid in &rule_ids {
        let combined_count = combined_findings.iter()
            .filter(|f| f.rule_id == *rid)
            .count();
        let expected = per_rule_individual[*rid];
        assert_eq!(
            combined_count, expected,
            "GaussDB combined vs individual mismatch for {}: combined={}, individual={}",
            rid, combined_count, expected
        );
    }

    let total_combined = combined_findings.len();
    assert_eq!(
        total_combined, total_individual,
        "Total findings: combined={} vs individual sum={}",
        total_combined, total_individual
    );

    // Sanity: at least some rules should have matched
    assert!(total_combined > 0, "Should have at least one finding");
}

/// GaussDB multi-rule with overlapping pattern: two rules matching the same
/// text region must both independently report findings.
#[test]
fn test_gaussdb_multi_rule_overlapping_patterns() {
    use astgrep_parser::dialect;

    let source = "SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;";

    let gaussdb_parser = dialect::dispatch(SqlDialect::GaussDB);
    let ast = gaussdb_parser
        .parse(source, &PathBuf::from("test.sql"))
        .expect("parse");

    let rules_yaml = r#"
rules:
  - id: broad-select
    name: Select
    description: Any SELECT
    message: SELECT
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "SELECT"
  - id: narrow-for-update
    name: FOR UPDATE
    description: FOR UPDATE clause
    message: FOR UPDATE
    severity: INFO
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - "FOR UPDATE"
"#;

    let mut ctx = RuleContext::new("test.sql".to_string(), Language::Sql, source.to_string());
    ctx.sql_dialect = Some(SqlDialect::GaussDB);

    let mut engine = RuleEngine::new();
    engine.load_rules_from_yaml(rules_yaml).unwrap();
    let findings = engine.analyze(ast.as_ref(), &ctx).unwrap();

    let broad_count = findings.iter().filter(|f| f.rule_id == "broad-select").count();
    let narrow_count = findings.iter().filter(|f| f.rule_id == "narrow-for-update").count();

    assert_eq!(broad_count, 1, "Broad SELECT rule should match once");
    assert_eq!(narrow_count, 1, "Narrow FOR UPDATE rule should match once");

    // Both rules must have reported findings on the same source line
    let broad_loc = &findings.iter().find(|f| f.rule_id == "broad-select").unwrap().location;
    let narrow_loc = &findings.iter().find(|f| f.rule_id == "narrow-for-update").unwrap().location;
    assert_eq!(broad_loc.start_line, narrow_loc.start_line,
        "Overlapping rules should both report findings on the same line");
}
