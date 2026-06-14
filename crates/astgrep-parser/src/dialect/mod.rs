//! SQL dialect dispatcher.
//!
//! Provides a unified `SqlDialectParser` trait that each dialect implements.
//!
//! - `Standard` → delegates to existing `SqlParser` (tree-sitter-sequel)
//! - `GaussDB` / `OpenGauss` → delegates to `OgsqlAdapter` (ogsql-parser v0.6.20)
//! - `PolarDBMySQL` → stub returning `NotYetImplemented` (planned Phase 4)

pub mod gaussdb;
pub mod opengauss;
pub mod polardb_mysql;

use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::{LanguageParser, SqlDialect};
use std::path::Path;

/// SQL dialect parser trait. Each dialect (Standard / GaussDB / PolarDB-MySQL)
/// implements this trait.
///
/// Phase 1 only `Standard` has a concrete implementation (delegating to the
/// existing `SqlParser`); other dialects will be implemented in Phase 2/4.
pub trait SqlDialectParser: Send + Sync {
    /// The dialect this parser handles.
    fn dialect(&self) -> SqlDialect;

    /// Parse SQL source code into an AST.
    ///
    /// # Errors
    ///
    /// Returns `DialectParseError` when:
    /// - The dialect is not yet implemented (`NotYetImplemented`)
    /// - The underlying parser fails (`ParseFailed`)
    fn parse(
        &self,
        source: &str,
        file_path: &Path,
    ) -> std::result::Result<Box<dyn astgrep_core::AstNode>, DialectParseError>;

    /// Whether this parser handles the given file path.
    fn supports_file(&self, file_path: &Path) -> bool;
}

/// Dialect parse error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DialectParseError {
    /// The dialect will be implemented in the specified Phase.
    #[error(
        "dialect `{dialect:?}` not yet implemented (planned for {planned_phase}); \
         use `--dialect standard` as fallback"
    )]
    NotYetImplemented {
        /// The dialect that is not yet implemented.
        dialect: SqlDialect,
        /// The phase in which this dialect will be implemented.
        planned_phase: &'static str,
    },

    /// The underlying parser failed.
    #[error("parse failed for dialect `{dialect:?}`: {reason}")]
    ParseFailed {
        /// The dialect that failed to parse.
        dialect: SqlDialect,
        /// The error reason.
        reason: String,
    },
}

/// Dispatch to the appropriate parser implementation based on dialect.
///
/// - `Standard` → `StandardDialectParser` (existing `SqlParser` via tree-sitter-sequel)
/// - `GaussDB` / `OpenGauss` → real dialect parsers backed by `OgsqlAdapter`
/// - `PolarDBMySQL` → stub returning `NotYetImplemented` (planned Phase 4)
pub fn dispatch(dialect: SqlDialect) -> Box<dyn SqlDialectParser> {
    match dialect {
        SqlDialect::Standard => Box::new(StandardDialectParser::new()),
        SqlDialect::GaussDB => Box::new(gaussdb::GaussDBDialect::new()),
        SqlDialect::OpenGauss => Box::new(opengauss::OpenGaussDialect::new()),
        SqlDialect::PolarDBMySQL => Box::new(polardb_mysql::PolarDBMySQLDialect::new()),
        _ => Box::new(StubDialectParser::new(dialect, "TBD")),
    }
}

/// Wrap a vec of per-statement UniversalNodes into a single node suitable
/// for `Box<dyn AstNode>`.
///
/// - Empty vec → empty `Program` node.
/// - Single statement → return it directly (no artificial wrapper).
/// - Multiple statements → `Program` node with each statement as a child.
pub(crate) fn wrap_statements(nodes: Vec<UniversalNode>) -> UniversalNode {
    match nodes.len() {
        0 => UniversalNode::new(NodeType::Program),
        1 => nodes.into_iter().next().expect("len == 1 checked above"),
        _ => UniversalNode::new(NodeType::Program).add_children(nodes),
    }
}

// ---- Internal implementations ----

/// Standard dialect parser — delegates to the existing `crate::sql::SqlParser`.
struct StandardDialectParser {
    inner: crate::sql::SqlParser,
}

impl StandardDialectParser {
    fn new() -> Self {
        Self {
            inner: crate::sql::SqlParser::new(),
        }
    }
}

impl SqlDialectParser for StandardDialectParser {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::Standard
    }

    fn parse(
        &self,
        source: &str,
        file_path: &Path,
    ) -> std::result::Result<Box<dyn astgrep_core::AstNode>, DialectParseError> {
        self.inner
            .parse(source, file_path)
            .map_err(|e| DialectParseError::ParseFailed {
                dialect: SqlDialect::Standard,
                reason: e.to_string(),
            })
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        self.inner.supports_file(file_path)
    }
}

/// Stub parser for dialects not yet implemented.
/// All parsing methods return `NotYetImplemented`.
struct StubDialectParser {
    dialect: SqlDialect,
    planned_phase: &'static str,
}

impl StubDialectParser {
    fn new(dialect: SqlDialect, planned_phase: &'static str) -> Self {
        Self {
            dialect,
            planned_phase,
        }
    }
}

impl SqlDialectParser for StubDialectParser {
    fn dialect(&self) -> SqlDialect {
        self.dialect
    }

    fn parse(
        &self,
        _source: &str,
        _file_path: &Path,
    ) -> std::result::Result<Box<dyn astgrep_core::AstNode>, DialectParseError> {
        Err(DialectParseError::NotYetImplemented {
            dialect: self.dialect,
            planned_phase: self.planned_phase,
        })
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "sql" | "ddl" | "dml")
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_standard_returns_standard_parser() {
        let parser = dispatch(SqlDialect::Standard);
        assert_eq!(parser.dialect(), SqlDialect::Standard);
    }

    #[test]
    fn test_dispatch_gaussdb_returns_real_parser() {
        let parser = dispatch(SqlDialect::GaussDB);
        assert_eq!(parser.dialect(), SqlDialect::GaussDB);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        assert!(
            result.is_ok(),
            "GaussDB should parse SELECT: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_dispatch_opengauss_returns_real_parser() {
        let parser = dispatch(SqlDialect::OpenGauss);
        assert_eq!(parser.dialect(), SqlDialect::OpenGauss);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        assert!(
            result.is_ok(),
            "OpenGauss should parse SELECT: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_gaussdb_parses_predict_by() {
        let parser = dispatch(SqlDialect::GaussDB);
        let result = parser.parse("PREDICT BY model FEATURES (col1)", Path::new("t.sql"));
        assert!(
            result.is_ok(),
            "PREDICT BY should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_gaussdb_parses_merge_into() {
        let parser = dispatch(SqlDialect::GaussDB);
        let sql = "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.x = s.x";
        let result = parser.parse(sql, Path::new("t.sql"));
        assert!(
            result.is_ok(),
            "MERGE INTO should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_gaussdb_mode_default_centralized() {
        let dialect = gaussdb::GaussDBDialect::new();
        assert_eq!(dialect.mode(), gaussdb::GaussDBMode::Centralized);
    }

    #[test]
    fn test_gaussdb_mode_distributed() {
        let dialect = gaussdb::GaussDBDialect::with_mode(gaussdb::GaussDBMode::Distributed);
        assert_eq!(dialect.mode(), gaussdb::GaussDBMode::Distributed);
    }

    #[test]
    fn test_dispatch_polardb_returns_real_parser() {
        let parser = dispatch(SqlDialect::PolarDBMySQL);
        assert_eq!(parser.dialect(), SqlDialect::PolarDBMySQL);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        assert!(
            result.is_ok(),
            "PolarDB should parse SELECT: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_standard_parser_supports_sql_files() {
        let parser = dispatch(SqlDialect::Standard);
        assert!(parser.supports_file(Path::new("query.sql")));
        assert!(parser.supports_file(Path::new("schema.ddl")));
        assert!(parser.supports_file(Path::new("data.dml")));
        assert!(!parser.supports_file(Path::new("code.java")));
    }

    #[test]
    fn test_stub_parser_supports_sql_files() {
        let parser = StubDialectParser::new(SqlDialect::Standard, "TBD");
        assert!(parser.supports_file(Path::new("query.sql")));
        assert!(!parser.supports_file(Path::new("code.java")));
    }

    #[test]
    fn test_standard_parser_can_parse_simple_sql() {
        // Smoke test: ensure Standard path delegates to existing SqlParser successfully
        let parser = dispatch(SqlDialect::Standard);
        let result = parser.parse("SELECT * FROM users", Path::new("test.sql"));
        assert!(
            result.is_ok(),
            "Standard parser should handle basic SELECT: {:?}",
            result.err()
        );
    }
}
