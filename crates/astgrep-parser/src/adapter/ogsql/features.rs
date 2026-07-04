//! GaussDB-specific feature conversion: PREDICT BY, TIMECAPSULE, SHRINK, Plan Hints.
//!
//! ogsql-parser v0.6.20 fully parses all four features. This module converts
//! them into UniversalNode representations for static analysis.
//!
//! | Feature        | ogsql Statement variant  | ogsql struct              | Target NodeType           |
//! |----------------|--------------------------|---------------------------|---------------------------|
//! | PREDICT BY     | `Statement::PredictBy`   | `PredictByStatement`      | `PredictStatement`        |
//! | TIMECAPSULE    | `Statement::TimeCapsule` | `TimeCapsuleStatement`    | `TimecapsuleStatement`    |
//! | SHRINK         | `Statement::Shrink`      | `ShrinkStatement`         | `ShrinkStatement`         |
//! | Plan Hints     | DML stmt `.hints` field  | `Vec<String>`             | metadata `plan_hints`     |

use super::OgsqlAdapterError;
use astgrep_ast::{nodes::NodeType, UniversalNode};

/// Convert a PREDICT BY statement into a UniversalNode.
///
/// GaussDB syntax: `PREDICT BY model_name (FEATURES col1, col2, ...) [USING ...]`
///
/// The returned node has node_type `predict_statement` with:
/// - `model` attribute: the model name
/// - `features` attribute: comma-separated feature columns
/// - `using_clause` metadata: the optional USING clause (if present)
pub fn convert_predict_by(
    stmt: &ogsql_parser::ast::PredictByStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = UniversalNode::new(NodeType::PredictStatement)
        .with_attribute("model".into(), stmt.model.clone())
        .with_text(format!("PREDICT BY {}", stmt.model));

    if !stmt.features.is_empty() {
        node = node.with_attribute("features".into(), stmt.features.join(", "));
    }
    if let Some(ref using) = stmt.using_clause {
        node = node.with_metadata("using_clause".into(), using.clone());
    }

    Ok(node)
}

/// Convert a TIMECAPSULE TABLE statement into a UniversalNode.
///
/// GaussDB syntax: `TIMECAPSULE TABLE table_name {TO {TIMESTAMP|CSN} expr | ...}`
///
/// The returned node has node_type `timecapsule_statement` with:
/// - `table` attribute: the target table name
/// - `action` attribute: the timecapsule action text (e.g. "TO TIMESTAMP '2024-01-01'")
/// - `raw_rest` metadata: the full remaining clause text
pub fn convert_timecapsule(
    stmt: &ogsql_parser::ast::TimeCapsuleStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let table_name = stmt.table_name.join(".");
    let mut node = UniversalNode::new(NodeType::TimecapsuleStatement)
        .with_attribute("table".into(), table_name)
        .with_attribute("action".into(), stmt.action.clone())
        .with_text(format!("TIMECAPSULE TABLE {}", stmt.table_name.join(".")));

    if !stmt.raw_rest.is_empty() {
        node = node.with_metadata("raw_rest".into(), stmt.raw_rest.clone());
    }

    Ok(node)
}

/// Convert a SHRINK statement into a UniversalNode.
///
/// GaussDB syntax: `SHRINK TABLE table_name` or `SHRINK INDEX index_name`
///
/// The returned node has node_type `shrink_statement` with:
/// - `target` attribute: the target name (TABLE or INDEX keyword, or raw text)
/// - `raw_rest` metadata: the full remaining clause text
pub fn convert_shrink(
    stmt: &ogsql_parser::ast::ShrinkStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = UniversalNode::new(NodeType::ShrinkStatement).with_text("SHRINK".to_string());

    if let Some(ref target) = stmt.target {
        node = node.with_attribute("target".into(), target.clone());
    }
    if !stmt.raw_rest.is_empty() {
        node = node.with_metadata("raw_rest".into(), stmt.raw_rest.clone());
    }

    Ok(node)
}

/// Extract plan hints from a DML statement's hints field and add them as metadata.
///
/// GaussDB plan hints are embedded as `/*+ hint1 hint2 */` comments in SQL.
/// ogsql-parser stores them as raw hint strings (e.g. `"tablescan(t1)"`,
/// `"hashjoin(t1 t2)"`) on all DML statement structs.
///
/// This function is called from the DML conversion functions (`convert_select`,
/// `convert_insert`, etc.) to attach hints to the resulting UniversalNode.
///
/// # Returns
///
/// The same `UniversalNode` with a `plan_hints` metadata attribute (comma-separated
/// hint strings) if `hints` is non-empty, or unchanged otherwise.
pub fn add_plan_hints(node: UniversalNode, hints: &[String]) -> UniversalNode {
    if hints.is_empty() {
        return node;
    }
    node.with_metadata("plan_hints".into(), hints.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    fn parse_one(sql: &str) -> UniversalNode {
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        assert_eq!(stmts.len(), 1, "expected 1 statement from: {sql}");
        let node = crate::adapter::ogsql::OgsqlAdapter::convert_statement_for_test(&stmts[0])
            .expect("conversion should succeed");
        node
    }

    // ── PREDICT BY ──

    #[test]
    fn test_predict_by_simple() {
        // Syntax: PREDICT BY model FEATURES (col1, col2) — FEATURES keyword precedes parens
        let node = parse_one("PREDICT BY model1 FEATURES (col1, col2)");
        assert_eq!(node.node_type(), "predict_statement");
        assert_eq!(
            node.get_attribute("model").map(String::as_str),
            Some("model1")
        );
        assert_eq!(
            node.get_attribute("features").map(String::as_str),
            Some("col1, col2")
        );
    }

    #[test]
    fn test_predict_by_with_using() {
        let node = parse_one("PREDICT BY model1 FEATURES (col1) USING WITH (learning_rate = 0.1)");
        assert_eq!(node.node_type(), "predict_statement");
        assert_eq!(
            node.get_attribute("model").map(String::as_str),
            Some("model1")
        );
        assert_eq!(
            node.get_attribute("features").map(String::as_str),
            Some("col1")
        );
        assert!(node.get_attribute("using_clause").is_some());
    }

    #[test]
    fn test_predict_by_no_features() {
        let node = parse_one("PREDICT BY model1");
        assert_eq!(node.node_type(), "predict_statement");
        assert_eq!(
            node.get_attribute("model").map(String::as_str),
            Some("model1")
        );
        assert!(node.get_attribute("features").is_none());
    }

    // ── TIMECAPSULE ──

    #[test]
    fn test_timecapsule_timestamp() {
        let node = parse_one("TIMECAPSULE TABLE t1 TO TIMESTAMP '2024-01-01 00:00:00'");
        assert_eq!(node.node_type(), "timecapsule_statement");
        assert_eq!(node.get_attribute("table").map(String::as_str), Some("t1"));
        assert!(node.get_attribute("action").is_some());
    }

    #[test]
    fn test_timecapsule_csn() {
        let node = parse_one("TIMECAPSULE TABLE my_schema.my_table TO CSN 12345");
        assert_eq!(node.node_type(), "timecapsule_statement");
        assert_eq!(
            node.get_attribute("table").map(String::as_str),
            Some("my_schema.my_table")
        );
    }

    #[test]
    fn test_timecapsule_no_action() {
        let node = parse_one("TIMECAPSULE TABLE t1");
        assert_eq!(node.node_type(), "timecapsule_statement");
        assert_eq!(node.get_attribute("table").map(String::as_str), Some("t1"));
    }

    // ── SHRINK ──

    #[test]
    fn test_shrink_table() {
        let node = parse_one("SHRINK TABLE t1");
        assert_eq!(node.node_type(), "shrink_statement");
    }

    #[test]
    fn test_shrink_index() {
        let node = parse_one("SHRINK INDEX idx1");
        assert_eq!(node.node_type(), "shrink_statement");
        // ogsql-parser lowercases identifiers; "INDEX" becomes "index"
        assert_eq!(
            node.get_attribute("target").map(String::as_str),
            Some("index")
        );
    }

    #[test]
    fn test_shrink_bare() {
        let node = parse_one("SHRINK");
        assert_eq!(node.node_type(), "shrink_statement");
    }

    // ── Plan Hints ──

    #[test]
    fn test_plan_hints_on_select() {
        let sql = "SELECT /*+ tablescan(t1) */ * FROM t1";
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        assert_eq!(stmts.len(), 1);

        // Verify hints are captured in the parsed statement
        if let ogsql_parser::ast::Statement::Select(ref s) = stmts[0] {
            assert!(!s.hints.is_empty(), "expected hints to be parsed");
            assert!(s.hints[0].name.contains("tablescan"), "expected tablescan hint");
        } else {
            panic!("expected Select statement");
        }

        let node = crate::adapter::ogsql::OgsqlAdapter::convert_statement_for_test(&stmts[0])
            .expect("conversion should succeed");
        assert_eq!(node.node_type(), "select_statement");
        let hints = node.get_attribute("plan_hints");
        assert!(hints.is_some(), "expected plan_hints metadata");
        assert!(
            hints.unwrap().contains("tablescan"),
            "expected tablescan in plan_hints"
        );

        // Verify hints also exist via add_plan_hints
        let node2 = add_plan_hints(
            UniversalNode::new(NodeType::SelectStatement),
            &["tablescan(t1)".to_string()],
        );
        assert!(node2.get_attribute("plan_hints").is_some());
    }

    #[test]
    fn test_plan_hints_on_insert() {
        let sql = "INSERT /*+ redistribute(t1) */ INTO t1 VALUES (1)";
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        assert_eq!(stmts.len(), 1);

        if let ogsql_parser::ast::Statement::Insert(ref s) = stmts[0] {
            assert!(!s.hints.is_empty(), "expected hints on Insert");
        } else {
            panic!("expected Insert statement");
        }

        let node = crate::adapter::ogsql::OgsqlAdapter::convert_statement_for_test(&stmts[0])
            .expect("conversion should succeed");
        let hints = node.get_attribute("plan_hints");
        assert!(hints.is_some(), "expected plan_hints on Insert");
        assert!(
            hints.unwrap().contains("redistribute"),
            "expected redistribute in plan_hints"
        );
    }

    #[test]
    fn test_add_plan_hints_empty() {
        let node = UniversalNode::new(NodeType::SelectStatement);
        let result = add_plan_hints(node, &[]);
        assert!(result.get_attribute("plan_hints").is_none());
    }

    #[test]
    fn test_add_plan_hints_non_empty() {
        let node = UniversalNode::new(NodeType::SelectStatement);
        let hints = vec!["tablescan(t1)".to_string(), "hashjoin(t1 t2)".to_string()];
        let result = add_plan_hints(node, &hints);
        let h = result
            .get_attribute("plan_hints")
            .expect("plan_hints should be present");
        assert!(h.contains("tablescan(t1)"));
        assert!(h.contains("hashjoin(t1 t2)"));
    }
}
