//! GaussDB dialect parser — delegates to OgsqlAdapter.

use crate::adapter::ogsql::OgsqlAdapter;
use crate::dialect::{wrap_statements, DialectParseError, SqlDialectParser};
use astgrep_core::{AstNode, SqlDialect};
use std::path::Path;

/// GaussDB deployment mode.
///
/// Affects MERGE INTO subquery support: per GaussDB documentation,
/// subqueries in WHEN clauses are only available in Centralized mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GaussDBMode {
    /// Centralized deployment — full MERGE INTO subquery support.
    #[default]
    Centralized,
    /// Distributed deployment — MERGE INTO subquery restrictions apply.
    Distributed,
}

/// GaussDB SQL dialect parser.
///
/// Delegates parsing to [`OgsqlAdapter`] which uses the hand-written
/// `ogsql-parser` crate (v0.6.20) supporting full openGauss/GaussDB syntax
/// including MERGE INTO, CREATE PACKAGE, PREDICT BY, TIMECAPSULE, SHRINK,
/// and plan hints.
pub struct GaussDBDialect {
    mode: GaussDBMode,
}

impl GaussDBDialect {
    /// Create a new GaussDB dialect parser with default (Centralized) mode.
    pub fn new() -> Self {
        Self {
            mode: GaussDBMode::default(),
        }
    }

    /// Create a new GaussDB dialect parser with the specified deployment mode.
    pub fn with_mode(mode: GaussDBMode) -> Self {
        Self { mode }
    }

    /// Current deployment mode.
    pub fn mode(&self) -> GaussDBMode {
        self.mode
    }

    /// Extract PL/pgSQL body from dollar-quoted string and parse as anonym block.
    fn extract_and_parse_dollar_body(source: &str) -> Vec<astgrep_ast::UniversalNode> {
        let start = match source.find("$$") {
            Some(s) => s,
            None => return vec![],
        };
        let after_open = &source[start + 2..];
        let end = match after_open.find("$$") {
            Some(e) => e,
            None => return vec![],
        };
        let body = after_open[..end].trim();
        if body.is_empty() {
            return vec![];
        }
        // Wrap body in DO block for ogsql-parser to handle PL/pgSQL syntax
        let wrapped = format!("DO $$ {} END $$;", body);
        OgsqlAdapter::parse_to_universal(&wrapped).unwrap_or_default()
    }
}

impl Default for GaussDBDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialectParser for GaussDBDialect {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::GaussDB
    }

    fn parse(
        &self,
        source: &str,
        _file_path: &Path,
    ) -> Result<Box<dyn AstNode>, DialectParseError> {
        let mut nodes = OgsqlAdapter::parse_to_universal(source)
            .or_else(|e| {
                // If direct parsing fails and the source contains PL/pgSQL syntax
                // (:= assignment), retry with DO block wrapping so ogsql-parser
                // can handle the procedural statements.
                if source.contains(":=") {
                    let wrapped = format!("DO $$ BEGIN {} END $$;", source);
                    OgsqlAdapter::parse_to_universal(&wrapped)
                } else {
                    Err(e)
                }
            })
            .map_err(|e| DialectParseError::ParseFailed {
                dialect: SqlDialect::GaussDB,
                reason: e.to_string(),
            })?;

        // Attach PL/pgSQL block bodies from dollar-quoted strings as child nodes
        let body_nodes = Self::extract_and_parse_dollar_body(source);
        if !body_nodes.is_empty() {
            let root = nodes.get_mut(0).unwrap();
            root.children.extend(body_nodes);
        }

        Ok(Box::new(
            wrap_statements(nodes).with_text(source.to_string()),
        ))
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "sql" | "ddl" | "dml")
        } else {
            false
        }
    }
}
