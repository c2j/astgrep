//! SQL dialect dispatcher.
//!
//! Provides a unified `SqlDialectParser` trait that future phases will implement
//! per dialect. Phase 1 only wires up the `Standard` path (delegating to existing
//! `SqlParser`); other dialects return a clear "not yet implemented" error.

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
/// Phase 1 behavior:
/// - `Standard` → returns `StandardDialectParser` (delegates to existing `SqlParser`)
/// - `GaussDB` / `OpenGauss` → returns parser but `parse()` returns `NotYetImplemented` (Phase 2)
/// - `PolarDBMySQL` → same (Phase 4)
pub fn dispatch(dialect: SqlDialect) -> Box<dyn SqlDialectParser> {
    match dialect {
        SqlDialect::Standard => Box::new(StandardDialectParser::new()),
        SqlDialect::GaussDB => Box::new(StubDialectParser::new(SqlDialect::GaussDB, "Phase 2")),
        SqlDialect::OpenGauss => Box::new(StubDialectParser::new(SqlDialect::OpenGauss, "Phase 2")),
        SqlDialect::PolarDBMySQL => {
            Box::new(StubDialectParser::new(SqlDialect::PolarDBMySQL, "Phase 4"))
        }
        _ => Box::new(StubDialectParser::new(dialect, "TBD")),
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
    fn test_dispatch_gaussdb_returns_stub_with_phase2_message() {
        let parser = dispatch(SqlDialect::GaussDB);
        assert_eq!(parser.dialect(), SqlDialect::GaussDB);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        match result {
            Err(DialectParseError::NotYetImplemented {
                dialect,
                planned_phase,
            }) => {
                assert_eq!(dialect, SqlDialect::GaussDB);
                assert_eq!(planned_phase, "Phase 2");
            }
            _ => panic!("expected NotYetImplemented, got unexpected result"),
        }
    }

    #[test]
    fn test_dispatch_opengauss_returns_stub_with_phase2_message() {
        let parser = dispatch(SqlDialect::OpenGauss);
        assert_eq!(parser.dialect(), SqlDialect::OpenGauss);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_polardb_returns_stub_with_phase4_message() {
        let parser = dispatch(SqlDialect::PolarDBMySQL);
        assert_eq!(parser.dialect(), SqlDialect::PolarDBMySQL);
        let result = parser.parse("SELECT 1", Path::new("test.sql"));
        match result {
            Err(DialectParseError::NotYetImplemented {
                dialect,
                planned_phase,
            }) => {
                assert_eq!(dialect, SqlDialect::PolarDBMySQL);
                assert_eq!(planned_phase, "Phase 4");
            }
            _ => panic!("expected NotYetImplemented, got unexpected result"),
        }
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
        let parser = dispatch(SqlDialect::GaussDB);
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
