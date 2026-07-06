//! sqlparser-rs adapter for PolarDB-MySQL dialect.
//!
//! Converts sqlparser's AST into astgrep's UniversalNode. Uses MySqlDialect
//! as the base with PolarDB-specific extensions (Phase 4.3).

use astgrep_ast::{AstBuilder, NodeType, UniversalNode};

/// Error type for sqlparser → UniversalNode conversion.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SqlparserAdapterError {
    #[error("sqlparser failed: {0}")]
    Parse(String),

    #[error("unsupported statement variant: {0}")]
    UnsupportedStatement(&'static str),
}

/// Adapter converting sqlparser-rs's MySQL AST into UniversalNode list.
pub struct SqlparserAdapter;

impl SqlparserAdapter {
    /// Parse MySQL/PolarDB SQL via sqlparser-rs and convert to UniversalNode list.
    ///
    /// # Errors
    /// Returns `SqlparserAdapterError` on parse failure or unsupported statement.
    pub fn parse_to_universal(sql: &str) -> Result<Vec<UniversalNode>, SqlparserAdapterError> {
        use sqlparser::dialect::MySqlDialect;
        use sqlparser::parser::Parser;

        let dialect = MySqlDialect {};
        let statements = Parser::parse_sql(&dialect, sql)
            .map_err(|e| SqlparserAdapterError::Parse(e.to_string()))?;

        statements.iter().map(convert_statement).collect()
    }
}

fn convert_statement(
    stmt: &sqlparser::ast::Statement,
) -> Result<UniversalNode, SqlparserAdapterError> {
    use sqlparser::ast::Statement;
    match stmt {
        Statement::Query(_) => convert_query(),
        Statement::Insert { .. } => Ok(AstBuilder::insert_statement()),
        Statement::Update { .. } => Ok(AstBuilder::update_statement()),
        Statement::Delete { .. } => Ok(AstBuilder::delete_statement()),
        Statement::CreateTable { .. } => Ok(AstBuilder::create_table_statement()),
        Statement::CreateIndex { .. } => Ok(AstBuilder::create_index_statement()),
        Statement::CreateView { .. } => Ok(AstBuilder::create_view_statement()),
        Statement::Drop { .. } => Ok(AstBuilder::drop_statement()),
        Statement::AlterTable { .. } => Ok(AstBuilder::alter_statement()),
        other => Ok(AstBuilder::sql_expression("unsupported_statement")
            .with_metadata("variant".into(), statement_name(other).into())),
    }
}

fn convert_query() -> Result<UniversalNode, SqlparserAdapterError> {
    Ok(AstBuilder::select_statement())
}

fn statement_name(stmt: &sqlparser::ast::Statement) -> &'static str {
    use sqlparser::ast::Statement;
    match stmt {
        Statement::Query(_) => "Query",
        Statement::Insert { .. } => "Insert",
        Statement::Update { .. } => "Update",
        Statement::Delete { .. } => "Delete",
        Statement::CreateTable { .. } => "CreateTable",
        Statement::CreateIndex { .. } => "CreateIndex",
        Statement::CreateView { .. } => "CreateView",
        Statement::Drop { .. } => "Drop",
        Statement::AlterTable { .. } => "AlterTable",
        Statement::Truncate { .. } => "Truncate",
        Statement::StartTransaction { .. } => "StartTransaction",
        Statement::Commit { .. } => "Commit",
        Statement::Rollback { .. } => "Rollback",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_basic() {
        let nodes = SqlparserAdapter::parse_to_universal("SELECT * FROM users").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::SelectStatement);
    }

    #[test]
    fn test_insert_basic() {
        let nodes =
            SqlparserAdapter::parse_to_universal("INSERT INTO users (id) VALUES (1)").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::InsertStatement);
    }

    #[test]
    fn test_update_basic() {
        let nodes =
            SqlparserAdapter::parse_to_universal("UPDATE users SET name = 'Bob' WHERE id = 1")
                .unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::UpdateStatement);
    }

    #[test]
    fn test_delete_basic() {
        let nodes = SqlparserAdapter::parse_to_universal("DELETE FROM users WHERE id = 1").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::DeleteStatement);
    }

    #[test]
    fn test_create_table() {
        let nodes = SqlparserAdapter::parse_to_universal(
            "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100))",
        )
        .unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::CreateTableStatement);
    }

    #[test]
    fn test_drop_table() {
        let nodes = SqlparserAdapter::parse_to_universal("DROP TABLE IF EXISTS temp").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::DropStatement);
    }

    #[test]
    fn test_unsupported_passthrough() {
        // SET is unsupported → passthrough node, not error
        let result = SqlparserAdapter::parse_to_universal("SET @x = 1");
        assert!(
            result.is_ok(),
            "SET should produce passthrough node, got: {result:?}"
        );
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type, NodeType::SqlExpression);
        assert_eq!(
            nodes[0].get_attribute("expression"),
            Some(&"unsupported_statement".to_string())
        );
    }

    #[test]
    fn test_mixed_supported_and_unsupported_polardb() {
        // SELECT (supported) + SET (unsupported) + INSERT (supported)
        let result = SqlparserAdapter::parse_to_universal(
            "SELECT 1; SET @x = 1; INSERT INTO t VALUES (1)",
        );
        assert!(
            result.is_ok(),
            "mixed file must not short-circuit, got: {result:?}"
        );
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].node_type, NodeType::SelectStatement);
        assert_eq!(nodes[1].node_type, NodeType::SqlExpression);
        assert_eq!(nodes[2].node_type, NodeType::InsertStatement);
    }
}
