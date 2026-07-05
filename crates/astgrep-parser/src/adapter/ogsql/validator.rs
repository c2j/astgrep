//! GaussDB SQL semantic validators — wraps ogsql-parser's built-in validators.

use ogsql_parser::{validate_merge_semantics, MergeSemanticErrorKind, Parser};

/// A validation finding from ogsql-parser's semantic analyzers.
#[non_exhaustive]
pub struct GaussDBValidationFinding {
    pub rule_id: &'static str,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Run GaussDB-specific semantic validation on SQL source.
///
/// Currently checks:
/// - MERGE INTO semantic restrictions (DELETE not supported, ON column updated, DUAL table)
///
/// Returns an empty vec if no violations found or if parsing fails.
pub fn validate_gaussdb_sql(sql: &str) -> Vec<GaussDBValidationFinding> {
    let (stmt_infos, _parse_errors) = Parser::parse_sql(sql);
    if stmt_infos.is_empty() {
        return Vec::new();
    }

    let merge_errors = validate_merge_semantics(&stmt_infos);

    merge_errors
        .into_iter()
        .map(|err| {
            let (rule_id, msg) = match err.kind {
                MergeSemanticErrorKind::DeleteNotSupported => (
                    "GAUSSDB-MERGE-001",
                    "GaussDB does not support MERGE INTO ... WHEN MATCHED THEN DELETE",
                ),
                MergeSemanticErrorKind::OnColumnUpdated => (
                    "GAUSSDB-MERGE-002",
                    "Columns referenced in MERGE INTO ON clause cannot be modified by UPDATE",
                ),
            };
            let message = match err.detail {
                Some(d) => format!("{}: {}", msg, d),
                None => msg.to_string(),
            };
            GaussDBValidationFinding {
                rule_id,
                message,
                line: err.location.line,
                column: err.location.column,
            }
        })
        .collect()
}
