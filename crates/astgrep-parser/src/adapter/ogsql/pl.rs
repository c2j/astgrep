//! PL/pgSQL block and statement conversion: ogsql PL AST → UniversalNode.
//!
//! Converts anonymous blocks (DECLARE...BEGIN...END), DO blocks, and
//! individual PL/pgSQL statements (assignment, flow control, etc.) into
//! structured UniversalNode trees for static analysis.

use super::OgsqlAdapterError;
use astgrep_ast::{AstBuilder, NodeType, UniversalNode};

use ogsql_parser::ast::plpgsql::*;

// ── Top-level entry points ──

pub fn convert_anony_block(
    anony: &ogsql_parser::ast::AnonyBlockStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    convert_pl_block(&anony.block, NodeType::BlockStatement)
}

pub fn convert_do_block(
    do_stmt: &ogsql_parser::ast::DoStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    if let Some(ref block) = do_stmt.block {
        let mut node = convert_pl_block(block, NodeType::BlockStatement)?;
        if let Some(ref lang) = do_stmt.language {
            node = node.with_metadata("pl_language".into(), lang.clone());
        }
        node = node.with_metadata("pl_block_type".into(), "do_block".into());
        Ok(node)
    } else {
        Ok(AstBuilder::sql_expression("DO")
            .with_metadata("code".into(), do_stmt.code.clone()))
    }
}

// ── Block conversion ──

pub(super) fn convert_pl_block(
    block: &PlBlock,
    node_type: NodeType,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = UniversalNode::new(node_type);

    if let Some(ref label) = block.label {
        node = node.with_metadata("label".into(), label.clone());
    }

    // DECLARE section
    for decl in &block.declarations {
        node = node.add_child(convert_pl_declaration(decl));
    }

    // BEGIN...END body
    for stmt in &block.body {
        node = node.add_child(convert_pl_statement(stmt)?);
    }

    Ok(node)
}

// ── Statement conversion ──

fn convert_pl_statement(
    stmt: &PlStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    match stmt {
        PlStatement::Assignment { target, expression } => {
            let target_str = extract_variable_name(target);
            let mut node = UniversalNode::new(NodeType::AssignmentExpression)
                .with_metadata("target".into(), target_str.clone())
                .with_metadata("operator".into(), ":=".into());
            node = node.add_child(super::expr::convert_expr(expression));
            Ok(node)
        }

        PlStatement::SqlStatement { statement, .. } => {
            // Recurse: convert the inner SQL statement using existing DML/DDL converters
            super::OgsqlAdapter::convert_statement(statement)
        }

        PlStatement::Perform { query, .. } => {
            Ok(AstBuilder::sql_expression("perform_statement")
                .with_metadata("query".into(), query.clone()))
        }

        PlStatement::Execute(stmt) => {
            Ok(AstBuilder::sql_expression("execute_statement")
                .with_metadata("immediate".into(), stmt.immediate.to_string()))
        }

        PlStatement::Return { expression } => {
            let mut node = AstBuilder::sql_expression("return_statement");
            if let Some(expr) = expression {
                node = node.add_child(super::expr::convert_expr(expr));
            }
            Ok(node)
        }

        PlStatement::If(_) => Ok(AstBuilder::sql_expression("if_statement")),
        PlStatement::Loop(_) => Ok(AstBuilder::sql_expression("loop_statement")),
        PlStatement::While(_) => Ok(AstBuilder::sql_expression("while_statement")),
        PlStatement::For(_) => Ok(AstBuilder::sql_expression("for_statement")),
        PlStatement::Case(_) => Ok(AstBuilder::sql_expression("case_statement")),
        PlStatement::Raise(_) => Ok(AstBuilder::sql_expression("raise_statement")),
        PlStatement::Null => Ok(AstBuilder::sql_expression("null_statement")),

        PlStatement::Block(inner) => convert_pl_block(&inner, NodeType::BlockStatement),

        // Remaining statement types get a generic wrapper
        _ => Ok(AstBuilder::sql_expression("pl_statement")),
    }
}

// ── Declaration conversion ──

fn convert_pl_declaration(decl: &PlDeclaration) -> UniversalNode {
    match decl {
        PlDeclaration::Variable(v) => {
            UniversalNode::new(NodeType::VariableDeclaration)
                .with_metadata("name".into(), v.name.clone())
                .with_metadata("data_type".into(), pl_data_type_str(&v.data_type))
        }
        PlDeclaration::Cursor(c) => {
            AstBuilder::sql_expression("CURSOR")
                .with_metadata("name".into(), c.name.clone())
        }
        PlDeclaration::Record(r) => {
            AstBuilder::sql_expression("RECORD")
                .with_metadata("name".into(), r.name.clone())
        }
        PlDeclaration::Type(_) => AstBuilder::sql_expression("TYPE_DECL"),
        PlDeclaration::Pragma { name, .. } => {
            AstBuilder::sql_expression("PRAGMA")
                .with_metadata("name".into(), name.clone())
        }
        _ => AstBuilder::sql_expression("PL_DECL"),
    }
}

// ── Helpers ──

fn extract_variable_name(expr: &ogsql_parser::ast::Expr) -> String {
    match expr {
        ogsql_parser::ast::Expr::ColumnRef(name) => name.join("."),
        ogsql_parser::ast::Expr::PlVariable(name) => name.join("."),
        _ => format!("{:?}", expr),
    }
}

fn pl_data_type_str(dt: &PlDataType) -> String {
    match dt {
        PlDataType::TypeName(s) => s.clone(),
        PlDataType::PercentType { table, column } => format!("{}.{}%TYPE", table, column),
        PlDataType::PercentRowType(t) => format!("{}%ROWTYPE", t),
        PlDataType::Record => "RECORD".to_string(),
        PlDataType::Cursor => "CURSOR".to_string(),
        PlDataType::RefCursor => "REFCURSOR".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::OgsqlAdapter;
    use astgrep_core::AstNode;

    #[test]
    fn test_anony_block_basic() {
        let result = OgsqlAdapter::parse_to_universal(
            "DECLARE v INTEGER; BEGIN v := 1; END;",
        );
        assert!(result.is_ok(), "expected ok, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type(), "block_statement");
        // Should have 1 declaration + 1 assignment child
        assert_eq!(nodes[0].child_count(), 2);
    }

    #[test]
    fn test_do_block_with_select_into_for_update() {
        let result = OgsqlAdapter::parse_to_universal(
            "DO $$ DECLARE v_cnt INTEGER; BEGIN \
             SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE; \
             v_cnt := v_cnt + 1; \
             UPDATE accounts SET cnt = v_cnt WHERE id = 1; \
             END $$;",
        );
        assert!(result.is_ok(), "DO block should parse: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1, "expected 1 do_block node");
        let block = &nodes[0];
        assert_eq!(block.node_type(), "block_statement");

        // Check children: declaration + 3 body statements (SELECT, assignment, UPDATE)
        assert!(
            block.child_count() >= 4,
            "expected >= 4 children, got {}",
            block.child_count()
        );

        // Find the select_statement child
        let select_child = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "select_statement" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have a select_statement child");

        // Check SELECT metadata populated (Phase A)
        assert_eq!(
            select_child.get_attribute("has_lock").as_deref(), Some("true"),
            "SELECT should have lock metadata"
        );
        assert_eq!(
            select_child.get_attribute("lock_type").as_deref(), Some("Update"),
            "lock_type should be Update"
        );
        assert_eq!(
            select_child.get_attribute("has_into").as_deref(), Some("true"),
            "SELECT INTO should have has_into in PL context"
        );
        assert!(
            select_child.get_attribute("into_vars").unwrap().contains("v_cnt"),
            "into_vars should contain v_cnt"
        );

        // Find assignment_statement
        let assign_child = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "assignment_expression" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have an assignment_statement child");
        assert_eq!(
            assign_child.get_attribute("target").as_deref(), Some("v_cnt"),
        );

        // Find update_statement
        let update_child = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "update_statement" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have an update_statement child");
        assert!(update_child.get_attribute("tables").unwrap().contains("accounts"));
    }

    #[test]
    fn test_assignment_statement_metadata() {
        let result = OgsqlAdapter::parse_to_universal(
            "DECLARE v INTEGER; BEGIN v := v + 1; END;",
        );
        assert!(result.is_ok());
        let nodes = result.unwrap();
        let block = &nodes[0];
        let assign = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "assignment_expression" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have assignment_statement");
        assert_eq!(assign.get_attribute("target").as_deref(), Some("v"));
        assert_eq!(assign.get_attribute("operator").as_deref(), Some(":="));
    }

    #[test]
    fn test_select_into_without_lock_in_pl_context() {
        let result = OgsqlAdapter::parse_to_universal(
            "DO $$ DECLARE v_cnt INTEGER; BEGIN \
             SELECT cnt INTO v_cnt FROM accounts WHERE id = 1; \
             END $$;",
        );
        assert!(result.is_ok());
        let nodes = result.unwrap();
        let block = &nodes[0];
        let select_child = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "select_statement" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have select_statement");
        assert_eq!(
            select_child.get_attribute("has_into").as_deref(), Some("true")
        );
        assert!(select_child.get_attribute("has_lock").is_none());
    }

    #[test]
    fn test_select_for_share_in_do_block() {
        let result = OgsqlAdapter::parse_to_universal(
            "DO $$ BEGIN SELECT cnt INTO v FROM t FOR SHARE; END $$;",
        );
        assert!(result.is_ok());
        let nodes = result.unwrap();
        let block = &nodes[0];
        let select_child = (0..block.child_count())
            .find_map(|i| {
                let c = block.child(i).unwrap();
                if c.node_type() == "select_statement" {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("should have select_statement");
        assert_eq!(
            select_child.get_attribute("lock_type").as_deref(), Some("Share")
        );
    }

    #[test]
    fn test_unsupported_pl_statement_does_not_crash() {
        // Test that unknown PL statements get a generic wrapper, not a crash
        let result = OgsqlAdapter::parse_to_universal(
            "DECLARE v INTEGER; BEGIN NULL; END;",
        );
        assert!(result.is_ok(), "NULL statement should parse ok");
    }
}
