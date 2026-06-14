//! PolarDB-MySQL dialect parser — delegates to SqlparserAdapter.

use crate::adapter::sqlparser::SqlparserAdapter;
use crate::dialect::{wrap_statements, DialectParseError, SqlDialectParser};
use astgrep_core::{AstNode, SqlDialect};
use std::path::Path;

/// PolarDB-MySQL SQL dialect parser.
///
/// Delegates parsing to [`SqlparserAdapter`] which uses `sqlparser-rs`
/// (Apache DataFusion, v0.62) with `MySqlDialect`.
pub struct PolarDBMySQLDialect;

impl Default for PolarDBMySQLDialect {
    fn default() -> Self {
        Self
    }
}

impl PolarDBMySQLDialect {
    pub fn new() -> Self {
        Self
    }
}

impl SqlDialectParser for PolarDBMySQLDialect {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::PolarDBMySQL
    }

    fn parse(
        &self,
        source: &str,
        _file_path: &Path,
    ) -> Result<Box<dyn AstNode>, DialectParseError> {
        let node = match SqlparserAdapter::parse_to_universal(source) {
            Ok(nodes) => wrap_statements(nodes),
            Err(_) => {
                // sqlparser-rs cannot parse PolarDB-specific syntax (GLOBAL INDEX,
                // DBPARTITION, etc.). Fall back to a bare Program node with source
                // text so literal/text-based pattern matching still works.
                astgrep_ast::UniversalNode::new(astgrep_ast::NodeType::Program)
            }
        };
        let node = node.with_text(source.to_string());
        Ok(Box::new(node))
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "sql" | "ddl" | "dml")
        } else {
            false
        }
    }
}
