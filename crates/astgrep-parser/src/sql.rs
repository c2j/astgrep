//! SQL language parser and adapter
//!
//! This module provides SQL-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use astgrep_ast::{AstBuilder, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// SQL AST adapter
pub struct SqlAdapter;

impl Default for SqlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlAdapter {
    /// Create a new SQL adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse SQL-specific constructs
    fn parse_sql_construct(
        &self,
        source: &str,
        _context: &AdapterContext,
    ) -> Result<UniversalNode> {
        let trimmed = source.trim().to_uppercase();

        if trimmed.starts_with("SELECT ") {
            self.parse_select_statement(source)
        } else if trimmed.starts_with("INSERT ") {
            self.parse_insert_statement(source)
        } else if trimmed.starts_with("UPDATE ") {
            self.parse_update_statement(source)
        } else if trimmed.starts_with("DELETE ") {
            self.parse_delete_statement(source)
        } else if trimmed.starts_with("CREATE ") {
            self.parse_create_statement(source)
        } else if trimmed.starts_with("DROP ") {
            self.parse_drop_statement(source)
        } else if trimmed.starts_with("ALTER ") {
            self.parse_alter_statement(source)
        } else {
            // Default to SQL expression
            Ok(AstBuilder::sql_expression(source.trim()).with_text(source.to_string()))
        }
    }

    /// Parse SELECT statement
    fn parse_select_statement(&self, source: &str) -> Result<UniversalNode> {
        let mut select_node = AstBuilder::select_statement();

        // Extract SELECT columns (simplified)
        if let Some(from_pos) = source.to_uppercase().find(" FROM ") {
            let select_part = &source[6..from_pos].trim(); // Skip "SELECT"
            let from_part = &source[from_pos + 6..].trim(); // Skip " FROM "

            // Parse columns
            for column in select_part.split(',') {
                let column = column.trim();
                if !column.is_empty() {
                    select_node = select_node.with_column(column.to_string());
                }
            }

            // Parse FROM clause
            let table_part = if let Some(where_pos) = from_part.to_uppercase().find(" WHERE ") {
                &from_part[..where_pos]
            } else if let Some(group_pos) = from_part.to_uppercase().find(" GROUP BY ") {
                &from_part[..group_pos]
            } else if let Some(order_pos) = from_part.to_uppercase().find(" ORDER BY ") {
                &from_part[..order_pos]
            } else {
                from_part
            };

            select_node = select_node.with_table(table_part.trim().to_string());

            // Parse WHERE clause
            if let Some(where_pos) = from_part.to_uppercase().find(" WHERE ") {
                let where_part = &from_part[where_pos + 7..]; // Skip " WHERE "
                let condition =
                    if let Some(group_pos) = where_part.to_uppercase().find(" GROUP BY ") {
                        &where_part[..group_pos]
                    } else if let Some(order_pos) = where_part.to_uppercase().find(" ORDER BY ") {
                        &where_part[..order_pos]
                    } else {
                        where_part
                    };

                select_node = select_node.with_where(condition.trim().to_string());
            }
        }

        Ok(select_node.with_text(source.to_string()))
    }

    /// Parse INSERT statement
    fn parse_insert_statement(&self, source: &str) -> Result<UniversalNode> {
        let mut insert_node = AstBuilder::insert_statement();

        // INSERT INTO table_name (columns) VALUES (values)
        if let Some(into_pos) = source.to_uppercase().find(" INTO ") {
            let after_into = &source[into_pos + 6..]; // Skip " INTO "

            if let Some(paren_pos) = after_into.find('(') {
                let table_name = after_into[..paren_pos].trim();
                insert_node = insert_node.with_table(table_name.to_string());

                // Extract columns
                if let Some(close_paren) = after_into.find(')') {
                    let columns_str = &after_into[paren_pos + 1..close_paren];
                    for column in columns_str.split(',') {
                        let column = column.trim();
                        if !column.is_empty() {
                            insert_node = insert_node.with_column(column.to_string());
                        }
                    }
                }
            }
        }

        Ok(insert_node.with_text(source.to_string()))
    }

    /// Parse UPDATE statement
    fn parse_update_statement(&self, source: &str) -> Result<UniversalNode> {
        let mut update_node = AstBuilder::update_statement();

        // UPDATE table_name SET column = value WHERE condition
        if let Some(set_pos) = source.to_uppercase().find(" SET ") {
            let table_part = &source[7..set_pos].trim(); // Skip "UPDATE "
            update_node = update_node.with_table(table_part.to_string());

            let after_set = &source[set_pos + 5..]; // Skip " SET "

            // Parse SET clause
            let set_part = if let Some(where_pos) = after_set.to_uppercase().find(" WHERE ") {
                &after_set[..where_pos]
            } else {
                after_set
            };

            for assignment in set_part.split(',') {
                let assignment = assignment.trim();
                if !assignment.is_empty() {
                    update_node = update_node.with_assignment(assignment.to_string());
                }
            }

            // Parse WHERE clause
            if let Some(where_pos) = after_set.to_uppercase().find(" WHERE ") {
                let where_part = &after_set[where_pos + 7..]; // Skip " WHERE "
                update_node = update_node.with_where(where_part.trim().to_string());
            }
        }

        Ok(update_node.with_text(source.to_string()))
    }

    /// Parse DELETE statement
    fn parse_delete_statement(&self, source: &str) -> Result<UniversalNode> {
        let mut delete_node = AstBuilder::delete_statement();

        // DELETE FROM table_name WHERE condition
        if let Some(from_pos) = source.to_uppercase().find(" FROM ") {
            let after_from = &source[from_pos + 6..]; // Skip " FROM "

            let table_part = if let Some(where_pos) = after_from.to_uppercase().find(" WHERE ") {
                &after_from[..where_pos]
            } else {
                after_from
            };

            delete_node = delete_node.with_table(table_part.trim().to_string());

            // Parse WHERE clause
            if let Some(where_pos) = after_from.to_uppercase().find(" WHERE ") {
                let where_part = &after_from[where_pos + 7..]; // Skip " WHERE "
                delete_node = delete_node.with_where(where_part.trim().to_string());
            }
        }

        Ok(delete_node.with_text(source.to_string()))
    }

    /// Parse CREATE statement
    fn parse_create_statement(&self, source: &str) -> Result<UniversalNode> {
        let upper_source = source.to_uppercase();

        if upper_source.contains("CREATE TABLE ") {
            self.parse_create_table(source)
        } else if upper_source.contains("CREATE INDEX ") {
            self.parse_create_index(source)
        } else if upper_source.contains("CREATE VIEW ") {
            self.parse_create_view(source)
        } else if upper_source.contains("CREATE SEQUENCE ") {
            self.parse_create_sequence(source)
        } else {
            Ok(AstBuilder::create_statement("unknown").with_text(source.to_string()))
        }
    }

    /// Parse CREATE TABLE statement
    fn parse_create_table(&self, source: &str) -> Result<UniversalNode> {
        let mut create_table_node = AstBuilder::create_table_statement();

        if let Some(table_pos) = source.to_uppercase().find("CREATE TABLE ") {
            let after_table = &source[table_pos + 13..]; // Skip "CREATE TABLE "

            if let Some(paren_pos) = after_table.find('(') {
                let table_name = after_table[..paren_pos].trim();
                create_table_node = create_table_node.with_table(table_name.to_string());

                // Extract column definitions
                if let Some(close_paren) = after_table.rfind(')') {
                    let columns_str = &after_table[paren_pos + 1..close_paren];
                    for column_def in columns_str.split(',') {
                        let column_def = column_def.trim();
                        if !column_def.is_empty() {
                            create_table_node =
                                create_table_node.with_column_definition(column_def.to_string());
                        }
                    }
                }
            }
        }

        Ok(create_table_node.with_text(source.to_string()))
    }

    /// Parse CREATE INDEX statement
    fn parse_create_index(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::create_index_statement().with_text(source.to_string()))
    }

    /// Parse CREATE VIEW statement
    fn parse_create_view(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::create_view_statement().with_text(source.to_string()))
    }

    /// Parse CREATE SEQUENCE statement
    fn parse_create_sequence(&self, source: &str) -> Result<UniversalNode> {
        let mut sequence_node = AstBuilder::create_sequence_statement();

        // Extract sequence name
        if let Some(after_sequence) = source.to_uppercase().find("CREATE SEQUENCE ") {
            let after_create = &source[after_sequence + 17..]; // Skip "CREATE SEQUENCE "

            // Extract sequence name (first word after SEQUENCE)
            let sequence_name = if let Some(space_pos) = after_create.trim().find(' ') {
                after_create[..space_pos].trim().to_string()
            } else {
                after_create.trim().to_string()
            };

            sequence_node = sequence_node.with_sequence_name(sequence_name);

            // Extract options (everything after the sequence name)
            let options_part = if let Some(space_pos) = after_create.trim().find(' ') {
                after_create[space_pos..].trim().to_string()
            } else {
                String::new()
            };

            // Check for CYCLE option
            let has_cycle = options_part.to_uppercase().contains("CYCLE");

            // Add options as attribute for pattern matching
            sequence_node = sequence_node
                .with_attribute("options".to_string(), options_part)
                .with_attribute("has_cycle".to_string(), has_cycle.to_string());
        }

        Ok(sequence_node.with_text(source.to_string()))
    }

    /// Parse DROP statement
    fn parse_drop_statement(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::drop_statement().with_text(source.to_string()))
    }

    /// Parse ALTER statement
    fn parse_alter_statement(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::alter_statement().with_text(source.to_string()))
    }
}

impl AstAdapter for SqlAdapter {
    fn adapt_node(
        &self,
        _node: &dyn std::any::Any,
        context: &AdapterContext,
    ) -> Result<UniversalNode> {
        self.parse_sql_construct(&context.source_code, context)
    }

    fn language(&self) -> Language {
        Language::Sql
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "SqlAdapter".to_string(),
            "1.0.0".to_string(),
            "SQL AST adapter with DDL and DML support".to_string(),
        )
        .with_feature("select_statements".to_string())
        .with_feature("insert_statements".to_string())
        .with_feature("update_statements".to_string())
        .with_feature("delete_statements".to_string())
        .with_feature("create_statements".to_string())
        .with_feature("drop_statements".to_string())
        .with_feature("alter_statements".to_string())
    }
}

/// SQL language parser
pub struct SqlParser {
    adapter: SqlAdapter,
}

impl SqlParser {
    /// Create a new SQL parser
    pub fn new() -> Self {
        Self {
            adapter: SqlAdapter::new(),
        }
    }
}

impl LanguageParser for SqlParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        // Prefer tree-sitter (tree-sitter-sequel) by default; allow override via env: ASTGREP_SQL_PARSER=manual
        #[cfg(feature = "sql-tree-sitter")]
        {
            if std::env::var("ASTGREP_SQL_PARSER").as_deref() != Ok("manual") {
                if let Ok(ts_parser) = crate::tree_sitter_parser::TreeSitterParser::new() {
                    if let Ok(Some(tree)) = ts_parser.parse(source, Language::Sql) {
                        if let Ok(universal_node) = ts_parser.tree_to_universal_ast(&tree, source) {
                            return Ok(Box::new(universal_node));
                        }
                    }
                }
            }
        }

        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Sql,
        );

        let universal_node = self.adapter.parse_sql_construct(source, &context)?;
        Ok(Box::new(universal_node))
    }

    fn language(&self) -> Language {
        Language::Sql
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "sql" | "ddl" | "dml")
        } else {
            false
        }
    }
}

impl Default for SqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[test]
    fn test_sql_adapter_new() {
        let adapter = SqlAdapter::new();
        assert_eq!(adapter.language(), Language::Sql);
    }

    #[test]
    fn test_sql_adapter_default() {
        let adapter: SqlAdapter = Default::default();
        assert_eq!(adapter.language(), Language::Sql);
    }

    #[test]
    fn test_sql_adapter_language() {
        let adapter = SqlAdapter::new();
        assert_eq!(adapter.language(), Language::Sql);
    }

    #[test]
    fn test_parse_simple_select() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_select_statement("SELECT * FROM table");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.text(), Some("SELECT * FROM table"));
    }

    #[test]
    fn test_parse_select_with_columns() {
        let adapter = SqlAdapter::new();
        let result =
            adapter.parse_select_statement("SELECT col1, col2 FROM table WHERE condition");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(
            node.get_attribute("columns").map(|s| s.as_str()),
            Some("col1,col2"),
            "expected columns attribute"
        );
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("table"));
        assert_eq!(node.get_attribute("where").map(|s| s.as_str()), Some("condition"));
    }

    #[test]
    fn test_parse_insert() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_insert_statement("INSERT INTO users (name, email) VALUES ('John', 'john@example.com')");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "insert_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("users"));
        assert_eq!(node.get_attribute("columns").map(|s| s.as_str()), Some("name,email"));
    }

    #[test]
    fn test_parse_update() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_update_statement("UPDATE table SET col=val WHERE condition");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "update_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("table"));
        assert_eq!(node.get_attribute("assignments").map(|s| s.as_str()), Some("col=val"));
        assert_eq!(node.get_attribute("where").map(|s| s.as_str()), Some("condition"));
    }

    #[test]
    fn test_parse_delete() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_delete_statement("DELETE FROM table WHERE condition");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "delete_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("table"));
        assert_eq!(node.get_attribute("where").map(|s| s.as_str()), Some("condition"));
    }

    #[test]
    fn test_parse_create_table() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_create_table("CREATE TABLE name (col INT, col2 VARCHAR(255))");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "create_table_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("name"));
        assert_eq!(node.get_attribute("column_definitions").map(|s| s.as_str()), Some("col INT,col2 VARCHAR(255)"));
    }

    #[test]
    fn test_parse_join() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_select_statement("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("t1 JOIN t2 ON t1.id = t2.id"));
    }

    #[test]
    fn test_parse_subquery() {
        let adapter = SqlAdapter::new();
        let result =
            adapter.parse_select_statement("SELECT * FROM (SELECT id FROM inner_table) AS sub");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
    }

    #[test]
    fn test_parse_group_by_having() {
        let adapter = SqlAdapter::new();
        let result = adapter
            .parse_select_statement("SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 1");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("employees"));
    }

    #[test]
    fn test_parse_order_limit() {
        let adapter = SqlAdapter::new();
        let result = adapter
            .parse_select_statement("SELECT * FROM users ORDER BY age LIMIT 10");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("users"));
    }

    #[test]
    fn test_parse_union() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_select_statement("SELECT a FROM t1 UNION SELECT b FROM t2");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("table").map(|s| s.as_str()), Some("t1 UNION SELECT b FROM t2"));
    }

    #[test]
    fn test_parse_expression() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_sql_construct("1 + 2 * 3", &AdapterContext::new("expr.sql".to_string(), "1 + 2 * 3".to_string(), Language::Sql));
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(node.get_attribute("expression").map(|s| s.as_str()), Some("1 + 2 * 3"));
    }

    #[test]
    fn test_parse_function_call() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_sql_construct("COUNT(*)", &AdapterContext::new("func.sql".to_string(), "COUNT(*)".to_string(), Language::Sql));
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(node.get_attribute("expression").map(|s| s.as_str()), Some("COUNT(*)"));
    }

    #[test]
    fn test_parse_malformed_sql() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_select_statement("SELECT");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");

        let result = adapter.parse_insert_statement("INSERT INTO");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_empty_input() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_sql_construct("", &AdapterContext::new("empty.sql".to_string(), "".to_string(), Language::Sql));
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "sql_expression");
    }

    #[test]
    fn test_parse_whitespace_only() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_sql_construct("   \n\t   ", &AdapterContext::new("ws.sql".to_string(), "   \n\t   ".to_string(), Language::Sql));
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "sql_expression");
    }

    #[test]
    fn test_statement_boundary_default() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_select_statement("SELECT 1; SELECT 2");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
    }

    #[test]
    fn test_statement_boundary_enabled() {
        let adapter = SqlAdapter::new();
        let result = adapter.parse_sql_construct(
            "SELECT 1; SELECT 2",
            &AdapterContext::new("multi.sql".to_string(), "SELECT 1; SELECT 2".to_string(), Language::Sql),
        );
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
    }

    #[test]
    fn test_sql_parser_creation() {
        let parser = SqlParser::new();
        assert_eq!(parser.language(), Language::Sql);
    }

    #[test]
    fn test_sql_parser_supports_file() {
        let parser = SqlParser::new();
        assert!(parser.supports_file(Path::new("query.sql")));
        assert!(parser.supports_file(Path::new("schema.ddl")));
        assert!(parser.supports_file(Path::new("data.dml")));
        assert!(!parser.supports_file(Path::new("test.py")));
        assert!(!parser.supports_file(Path::new("test.js")));
    }

    #[test]
    fn test_sql_adapter_metadata() {
        let adapter = SqlAdapter::new();
        let metadata = adapter.metadata();

        assert_eq!(metadata.name, "SqlAdapter");
        assert!(metadata
            .supported_features
            .contains(&"select_statements".to_string()));
        assert!(metadata
            .supported_features
            .contains(&"create_statements".to_string()));
    }

    #[test]
    fn test_sql_parser_default_uses_tree_sitter() {
        // Ensure default path (no manual override)
        std::env::remove_var("ASTGREP_SQL_PARSER");
        let parser = SqlParser::new();
        let source = "SELECT id, name FROM users WHERE age > 18";
        let node = parser
            .parse(source, Path::new("query.sql"))
            .expect("parse ok");
        // Tree-sitter path attaches original ts_kind metadata on the root node
        assert!(
            node.get_attribute("ts_kind").is_some(),
            "expected ts_kind metadata when using tree-sitter path"
        );
    }

    #[test]
    fn test_sql_parser_manual_override() {
        // Force manual parser
        std::env::set_var("ASTGREP_SQL_PARSER", "manual");
        let parser = SqlParser::new();
        let source = "SELECT id FROM users";
        let node = parser
            .parse(source, Path::new("query.sql"))
            .expect("parse ok");
        // Manual path should not have ts_kind metadata and should produce SQL-specific node types
        assert!(
            node.get_attribute("ts_kind").is_none(),
            "manual path should not include ts_kind metadata"
        );
        assert_eq!(node.node_type(), "select_statement");
        // Cleanup
        std::env::remove_var("ASTGREP_SQL_PARSER");
    }
}
