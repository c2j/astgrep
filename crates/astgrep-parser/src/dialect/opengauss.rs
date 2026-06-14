//! OpenGauss dialect parser — shares implementation with GaussDB.

use crate::dialect::gaussdb::GaussDBDialect;
use crate::dialect::{DialectParseError, SqlDialectParser};
use astgrep_core::{AstNode, SqlDialect};
use std::path::Path;

/// OpenGauss SQL dialect parser.
///
/// OpenGauss is the open-source counterpart of GaussDB with near-identical
/// SQL syntax. This parser delegates to [`GaussDBDialect`] internally but
/// reports its [`SqlDialect`] as [`OpenGauss`](SqlDialect::OpenGauss) so that
/// rule filtering via `Rule::applies_to_dialect` works correctly.
pub struct OpenGaussDialect {
    inner: GaussDBDialect,
}

impl OpenGaussDialect {
    /// Create a new OpenGauss dialect parser.
    pub fn new() -> Self {
        Self {
            inner: GaussDBDialect::new(),
        }
    }
}

impl Default for OpenGaussDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialectParser for OpenGaussDialect {
    fn dialect(&self) -> SqlDialect {
        SqlDialect::OpenGauss
    }

    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>, DialectParseError> {
        // Delegate to inner GaussDB parser, but rewrite the dialect tag in errors.
        self.inner.parse(source, file_path).map_err(|e| match e {
            DialectParseError::ParseFailed { reason, .. } => DialectParseError::ParseFailed {
                dialect: SqlDialect::OpenGauss,
                reason,
            },
            other => other,
        })
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        self.inner.supports_file(file_path)
    }
}
