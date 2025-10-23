//! SQL language parser and adapter
//! 
//! This module provides SQL-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use astgrep_ast::{AstBuilder, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// SQL AST adapter
pub struct SqlAdapter;

impl SqlAdapter {
    /// Create a new SQL adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse SQL-specific constructs
    fn parse_sql_construct(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
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
            Ok(AstBuilder::sql_expression(source.trim())
                .with_text(source.to_string()))
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
                let condition = if let Some(group_pos) = where_part.to_uppercase().find(" GROUP BY ") {
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
        } else {
            Ok(AstBuilder::create_statement("unknown")
                .with_text(source.to_string()))
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
                            create_table_node = create_table_node.with_column_definition(column_def.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(create_table_node.with_text(source.to_string()))
    }

    /// Parse CREATE INDEX statement
    fn parse_create_index(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::create_index_statement()
            .with_text(source.to_string()))
    }

    /// Parse CREATE VIEW statement
    fn parse_create_view(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::create_view_statement()
            .with_text(source.to_string()))
    }

    /// Parse DROP statement
    fn parse_drop_statement(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::drop_statement()
            .with_text(source.to_string()))
    }

    /// Parse ALTER statement
    fn parse_alter_statement(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::alter_statement()
            .with_text(source.to_string()))
    }
}

impl AstAdapter for SqlAdapter {
    fn adapt_node(&self, _node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
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
    fn test_parse_select_statement() {
        let adapter = SqlAdapter::new();
        
        let result = adapter.parse_select_statement("SELECT id, name FROM users WHERE age > 18");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "select_statement");
    }

    #[test]
    fn test_parse_insert_statement() {
        let adapter = SqlAdapter::new();
        
        let result = adapter.parse_insert_statement("INSERT INTO users (name, email) VALUES ('John', 'john@example.com')");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "insert_statement");
    }

    #[test]
    fn test_parse_update_statement() {
        let adapter = SqlAdapter::new();
        
        let result = adapter.parse_update_statement("UPDATE users SET name = 'Jane' WHERE id = 1");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "update_statement");
    }

    #[test]
    fn test_parse_delete_statement() {
        let adapter = SqlAdapter::new();
        
        let result = adapter.parse_delete_statement("DELETE FROM users WHERE age < 18");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "delete_statement");
    }

    #[test]
    fn test_parse_create_table() {
        let adapter = SqlAdapter::new();
        
        let result = adapter.parse_create_table("CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100))");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "create_table_statement");
    }

    #[test]
    fn test_sql_adapter_metadata() {
        let adapter = SqlAdapter::new();
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "SqlAdapter");
        assert!(metadata.supported_features.contains(&"select_statements".to_string()));
        assert!(metadata.supported_features.contains(&"create_statements".to_string()));
    }
}
