//! PolarDB-MySQL dialect parser — delegates to SqlparserAdapter.

use crate::adapter::sqlparser::{SqlparserAdapter, SqlparserAdapterError};
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
            Err(SqlparserAdapterError::Parse(reason)) => {
                return Err(DialectParseError::ParseFailed {
                    dialect: SqlDialect::PolarDBMySQL,
                    reason,
                });
            }
            Err(SqlparserAdapterError::UnsupportedStatement(variant)) => {
                return Err(DialectParseError::ParseFailed {
                    dialect: SqlDialect::PolarDBMySQL,
                    reason: format!(
                        "unexpected unsupported statement variant (passthrough expected): {variant}"
                    ),
                });
            }
            #[allow(unreachable_patterns)]
            Err(_) => {
                return Err(DialectParseError::ParseFailed {
                    dialect: SqlDialect::PolarDBMySQL,
                    reason: "unknown sqlparser error".to_string(),
                });
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
