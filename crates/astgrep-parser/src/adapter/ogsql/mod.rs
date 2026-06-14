//! Adapter converting ogsql-parser's AST into astgrep's UniversalNode.
//!
//! `ogsql-parser` is a hand-written recursive descent SQL parser for
//! openGauss/GaussDB (537 commits, 1646 unit tests, 1409 openGauss regression
//! tests passing).
//!
//! Phase 2.1 (this file): scaffolding + SELECT POC. Full DML/DDL mapping is
//! split into sibling files (dml.rs, ddl.rs, features.rs) in Tasks 2.2–2.4 to
//! comply with M-ARCH-03 (file size ≤ 600 lines).

use astgrep_ast::{AstBuilder, UniversalNode};

/// Error type for ogsql → UniversalNode conversion.
///
/// Uses thiserror (not anyhow) per M-ERR-01 (library code must define concrete
/// error types).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OgsqlAdapterError {
    /// ogsql tokenizer failed.
    #[error("ogsql tokenizer failed: {0}")]
    Tokenize(#[from] ogsql_parser::token::tokenizer::TokenizerError),

    /// ogsql parser failed.
    #[error("ogsql parser failed: {0}")]
    Parse(#[from] ogsql_parser::parser::ParserError),

    /// Statement variant not yet supported by the adapter.
    /// Phase 2.2–2.4 progressively reduces these.
    #[error("unsupported statement variant: {variant}")]
    UnsupportedStatement {
        /// The name of the unsupported statement variant.
        variant: &'static str,
    },

    /// Conversion failed for a supported variant (e.g. missing required field).
    #[error("conversion failed for {node}: {reason}")]
    ConversionFailed {
        /// The node that failed conversion.
        node: &'static str,
        /// The reason for the failure.
        reason: String,
    },
}

/// Adapter converting ogsql-parser's GaussDB/openGauss SQL into UniversalNode list.
///
/// Phase 2.1: only SELECT is converted; other variants return
/// `UnsupportedStatement`.  Phase 2.2–2.4 progressively adds
/// INSERT/UPDATE/DELETE/MERGE/CREATE/etc.
pub struct OgsqlAdapter;

impl OgsqlAdapter {
    /// Parse SQL source and convert each statement to a UniversalNode.
    ///
    /// # Errors
    ///
    /// Returns `OgsqlAdapterError` when:
    /// - Tokenization fails (`Tokenize`)
    /// - Parsing fails (`Parse`)
    /// - Statement variant is not yet supported (`UnsupportedStatement`)
    pub fn parse_to_universal(sql: &str) -> Result<Vec<UniversalNode>, OgsqlAdapterError> {
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql).tokenize()?;
        // Parser::parse() returns Vec<Statement> directly (not a Result).
        let statements = ogsql_parser::parser::Parser::new(tokens).parse();
        statements.iter().map(Self::convert_statement).collect()
    }

    fn convert_statement(
        stmt: &ogsql_parser::ast::Statement,
    ) -> Result<UniversalNode, OgsqlAdapterError> {
        match stmt {
            ogsql_parser::ast::Statement::Select(ref spanned) => {
                // Spanned<T> implements Deref<Target = T>, so we can pass
                // &SelectStatement directly to convert_select.
                Self::convert_select(spanned)
            }
            other => {
                // POC: map known variant names for visible error messages
                // so we can track mapping work needed in Tasks 2.2–2.4.
                #[allow(clippy::match_same_arms)]
                let variant = match other {
                    ogsql_parser::ast::Statement::Insert(_) => "Insert",
                    ogsql_parser::ast::Statement::InsertAll(_) => "InsertAll",
                    ogsql_parser::ast::Statement::InsertFirst(_) => "InsertFirst",
                    ogsql_parser::ast::Statement::Update(_) => "Update",
                    ogsql_parser::ast::Statement::Delete(_) => "Delete",
                    ogsql_parser::ast::Statement::Merge(_) => "Merge",
                    ogsql_parser::ast::Statement::CreateTable(_) => "CreateTable",
                    ogsql_parser::ast::Statement::CreateTableAs(_) => "CreateTableAs",
                    ogsql_parser::ast::Statement::CreateIndex(_) => "CreateIndex",
                    ogsql_parser::ast::Statement::CreateView(_) => "CreateView",
                    ogsql_parser::ast::Statement::CreateMaterializedView(_) => {
                        "CreateMaterializedView"
                    }
                    ogsql_parser::ast::Statement::CreateSequence(_) => "CreateSequence",
                    ogsql_parser::ast::Statement::CreateFunction(_) => "CreateFunction",
                    ogsql_parser::ast::Statement::CreateProcedure(_) => "CreateProcedure",
                    ogsql_parser::ast::Statement::CreatePackage(_) => "CreatePackage",
                    ogsql_parser::ast::Statement::CreateTrigger(_) => "CreateTrigger",
                    ogsql_parser::ast::Statement::AlterTable(_) => "AlterTable",
                    ogsql_parser::ast::Statement::Drop(_) => "Drop",
                    ogsql_parser::ast::Statement::Truncate(_) => "Truncate",
                    ogsql_parser::ast::Statement::Copy(_) => "Copy",
                    ogsql_parser::ast::Statement::Explain(_) => "Explain",
                    // PredictBy is not a Statement variant; use the actual name.
                    _ => "Other",
                };
                Err(OgsqlAdapterError::UnsupportedStatement { variant })
            }
        }
    }

    fn convert_select(
        select: &ogsql_parser::ast::SelectStatement,
    ) -> Result<UniversalNode, OgsqlAdapterError> {
        // POC: minimal conversion — just wrap in a SelectStatement node.
        // Full field mapping (columns, from, where, group_by, etc.) is Task 2.2.
        //
        // Use Debug formatting to capture the structure for inspection.
        let debug_repr = format!("{select:#?}");
        let node = AstBuilder::select_statement().with_text(debug_repr);
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[test]
    fn test_select_single_statement() {
        let result = OgsqlAdapter::parse_to_universal("SELECT * FROM users");
        assert!(result.is_ok(), "expected ok, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1, "expected 1 node, got {}", nodes.len());
        assert_eq!(
            nodes[0].node_type(),
            "select_statement",
            "expected select_statement node"
        );
        // Should have text from the debug representation
        assert!(
            nodes[0].text().is_some(),
            "expected text on the select node"
        );
    }

    #[test]
    fn test_select_multiple_statements() {
        let result = OgsqlAdapter::parse_to_universal("SELECT 1; SELECT 2");
        assert!(result.is_ok(), "expected ok, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 2, "expected 2 nodes, got {}", nodes.len());
        assert_eq!(nodes[0].node_type(), "select_statement");
        assert_eq!(nodes[1].node_type(), "select_statement");
    }

    #[test]
    fn test_insert_returns_unsupported() {
        let result = OgsqlAdapter::parse_to_universal("INSERT INTO t VALUES (1)");
        assert!(
            result.is_err(),
            "expected error for unsupported Insert, got: {result:?}"
        );
        match result.unwrap_err() {
            OgsqlAdapterError::UnsupportedStatement { variant } => {
                assert_eq!(variant, "Insert");
            }
            other => panic!("expected UnsupportedStatement, got: {other:?}"),
        }
    }

    #[test]
    fn test_invalid_sql_returns_error() {
        // ogsql-parser is resilient: it tokenizes and parses most inputs into
        // some statement variant.  We verify that either:
        // - The parser actually fails (Tokenize/Parse error), OR
        // - It parses but our adapter returns UnsupportedStatement (unknown variant)
        let result = OgsqlAdapter::parse_to_universal("INVALID SQL %%%");
        assert!(
            result.is_err(),
            "expected error for invalid SQL, got: {result:?}"
        );
        // Any OgsqlAdapterError is acceptable — the test verifies error handling works.
        match result.unwrap_err() {
            OgsqlAdapterError::Tokenize(_)
            | OgsqlAdapterError::Parse(_)
            | OgsqlAdapterError::UnsupportedStatement { .. }
            | OgsqlAdapterError::ConversionFailed { .. } => {} // all acceptable
        }
    }

    #[test]
    fn test_merge_returns_unsupported() {
        let result = OgsqlAdapter::parse_to_universal(
            "MERGE INTO t USING s ON t.id = s.id \
             WHEN MATCHED THEN UPDATE SET t.x = s.x",
        );
        assert!(
            result.is_err(),
            "expected error for unsupported Merge, got: {result:?}"
        );
        match result.unwrap_err() {
            OgsqlAdapterError::UnsupportedStatement { variant } => {
                assert_eq!(variant, "Merge");
            }
            other => panic!("expected UnsupportedStatement, got: {other:?}"),
        }
    }
}
