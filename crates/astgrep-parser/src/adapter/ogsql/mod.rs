//! Adapter converting ogsql-parser's AST into astgrep's UniversalNode.
//!
//! `ogsql-parser` is a hand-written recursive descent SQL parser for
//! openGauss/GaussDB (537 commits, 1646 unit tests, 1409 openGauss regression
//! tests passing).
//!
//! Phase 2.2: DML statements (SELECT/INSERT/UPDATE/DELETE/MERGE) are mapped in
//! sibling files `dml.rs` and `expr.rs`.
//! Phase 2.3: DDL statements (CREATE TABLE/INDEX/VIEW/FUNCTION/PROCEDURE/PACKAGE,
//! DROP, ALTER TABLE) are mapped in `ddl.rs`.

mod ddl;
mod dml;
mod expr;
mod features;
mod pl;
pub mod validator;

use astgrep_ast::UniversalNode;

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

    /// Convert a single ogsql statement to UniversalNode.
    /// Public for use by submodule tests (features.rs, dml.rs, ddl.rs).
    pub fn convert_statement_for_test(
        stmt: &ogsql_parser::ast::Statement,
    ) -> Result<UniversalNode, OgsqlAdapterError> {
        Self::convert_statement(stmt)
    }

    pub(super) fn convert_statement(
        stmt: &ogsql_parser::ast::Statement,
    ) -> Result<UniversalNode, OgsqlAdapterError> {
        let result = match stmt {
            // DML: dispatched to dml.rs
            ogsql_parser::ast::Statement::Select(ref s) => {
                let span = s.span.clone();
                dml::convert_select(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Insert(ref s) => {
                let span = s.span.clone();
                dml::convert_insert(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Update(ref s) => {
                let span = s.span.clone();
                dml::convert_update(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Delete(ref s) => {
                let span = s.span.clone();
                dml::convert_delete(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Merge(ref s) => {
                let span = s.span.clone();
                dml::convert_merge(s).map(|node| apply_span(node, span))
            }
            // Multi-table insert not yet supported
            ogsql_parser::ast::Statement::InsertAll(_) => {
                Err(OgsqlAdapterError::UnsupportedStatement {
                    variant: "InsertAll",
                })
            }
            ogsql_parser::ast::Statement::InsertFirst(_) => {
                Err(OgsqlAdapterError::UnsupportedStatement {
                    variant: "InsertFirst",
                })
            }

            // ── DDL (Phase 2.3) ──
            ogsql_parser::ast::Statement::CreateTable(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_table(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreateIndex(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_index(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreateGlobalIndex(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_global_index(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreateView(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_view(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreateFunction(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_function(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreateProcedure(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_procedure(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreatePackage(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_package(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::CreatePackageBody(ref s) => {
                let span = s.span.clone();
                ddl::convert_create_package_body(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Drop(ref s) => {
                let span = s.span.clone();
                ddl::convert_drop(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::AlterTable(ref s) => {
                let span = s.span.clone();
                ddl::convert_alter_table(s).map(|node| apply_span(node, span))
            }

            // GaussDB-specific features (Phase 2.4)
            ogsql_parser::ast::Statement::PredictBy(ref s) => {
                let span = s.span.clone();
                features::convert_predict_by(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::TimeCapsule(ref s) => {
                let span = s.span.clone();
                features::convert_timecapsule(s).map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Shrink(ref s) => {
                let span = s.span.clone();
                features::convert_shrink(s).map(|node| apply_span(node, span))
            }

            // PL/pgSQL blocks (Phase 2.5)
            ogsql_parser::ast::Statement::AnonyBlock(ref s) => {
                let span = s.span.clone();
                pl::convert_anony_block(s, span.as_ref())
                    .map(|node| apply_span(node, span))
            }
            ogsql_parser::ast::Statement::Do(ref s) => {
                let span = s.span.clone();
                pl::convert_do_block(s, span.as_ref())
                    .map(|node| apply_span(node, span))
            }

            // Still unsupported
            other => {
                let variant = match other {
                    ogsql_parser::ast::Statement::Replace(_) => "Replace",
                    ogsql_parser::ast::Statement::CreateTableAs(_) => "CreateTableAs",
                    ogsql_parser::ast::Statement::CreateMaterializedView(_) => {
                        "CreateMaterializedView"
                    }
                    ogsql_parser::ast::Statement::CreateSequence(_) => "CreateSequence",
                    ogsql_parser::ast::Statement::CreateTrigger(_) => "CreateTrigger",
                    ogsql_parser::ast::Statement::Truncate(_) => "Truncate",
                    ogsql_parser::ast::Statement::Copy(_) => "Copy",
                    ogsql_parser::ast::Statement::Explain(_) => "Explain",
                    _ => "Other",
                };
                Err(OgsqlAdapterError::UnsupportedStatement { variant })
            }
        };
        result
    }
}

/// Apply ogsql-parser source span to a UniversalNode so match results
/// report correct line/column numbers instead of default (1,1).
/// Propagates the span to all descendants that don't have their own location.
fn apply_span(
    mut node: UniversalNode,
    span: Option<ogsql_parser::ast::SourceSpan>,
) -> UniversalNode {
    if let Some(s) = span {
        let loc = (s.start.line, s.start.column, s.end.line, s.end.column);
        node = node.with_location(loc.0, loc.1, loc.2, loc.3);
        propagate_location(&mut node, loc);
    }
    node
}

fn propagate_location(node: &mut UniversalNode, loc: (usize, usize, usize, usize)) {
    for child in node.children.iter_mut() {
        if child.location.is_none() {
            child.location = Some(loc);
        }
        propagate_location(child, loc);
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
        // Should have tables attribute (not just debug text)
        assert!(
            nodes[0].get_attribute("tables").is_some(),
            "expected tables attribute on select node"
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
    fn test_insert_now_supported() {
        let result = OgsqlAdapter::parse_to_universal("INSERT INTO t VALUES (1)");
        assert!(result.is_ok(), "expected ok for Insert, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type(), "insert_statement");
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
    fn test_merge_now_supported() {
        let result = OgsqlAdapter::parse_to_universal(
            "MERGE INTO t USING s ON t.id = s.id \
             WHEN MATCHED THEN UPDATE SET t.x = s.x",
        );
        assert!(result.is_ok(), "expected ok for Merge, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type(), "merge_statement");
    }

    #[test]
    fn test_update_now_supported() {
        let result = OgsqlAdapter::parse_to_universal("UPDATE t SET x = 1");
        assert!(result.is_ok(), "expected ok for Update, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type(), "update_statement");
    }

    #[test]
    fn test_delete_now_supported() {
        let result = OgsqlAdapter::parse_to_universal("DELETE FROM t");
        assert!(result.is_ok(), "expected ok for Delete, got: {result:?}");
        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_type(), "delete_statement");
    }

    #[test]
    fn test_insert_all_still_unsupported() {
        // Multi-table INSERT is not yet supported
        let result = OgsqlAdapter::parse_to_universal(
            "INSERT ALL INTO t1 VALUES (1) INTO t2 VALUES (2) SELECT * FROM dual",
        );
        assert!(result.is_err(), "expected error for InsertAll");
        match result.unwrap_err() {
            OgsqlAdapterError::UnsupportedStatement { variant } => {
                assert_eq!(variant, "InsertAll");
            }
            other => panic!("expected UnsupportedStatement, got: {other:?}"),
        }
    }
}
