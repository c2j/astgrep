//! LanguageParser wrapper around TreeSitterParser
//!
//! Provides a LanguageParser implementation backed by tree-sitter that
//! returns a UniversalNode AST with precise locations and full children.

use crate::tree_sitter_parser::TreeSitterParser;
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;
use std::sync::Mutex;

/// LanguageParser implemented using TreeSitterParser
pub struct TreeSitterLanguageParser {
    language: Language,
    parser: Mutex<TreeSitterParser>,
}

impl TreeSitterLanguageParser {
    /// Create a new tree-sitter based language parser for a specific language
    pub fn new(language: Language) -> Result<Self> {
        let ts = TreeSitterParser::new()?;
        Ok(Self {
            language,
            parser: Mutex::new(ts),
        })
    }
}

impl LanguageParser for TreeSitterLanguageParser {
    fn parse(&self, source: &str, _file_path: &Path) -> Result<Box<dyn AstNode>> {
        // TreeSitterParser::parse requires &mut self, so we guard it with a Mutex
        let mut ts = self
            .parser
            .lock()
            .map_err(|_| astgrep_core::AnalysisError::parse_error("TreeSitter parser lock poisoned".to_string()))?;

        if let Some(tree) = ts.parse(source, self.language)? {
            let root = ts.tree_to_universal_ast(&tree, source)?;
            Ok(Box::new(root))
        } else {
            Err(astgrep_core::AnalysisError::parse_error(
                format!("Tree-sitter does not support language: {:?}", self.language),
            ))
        }
    }

    fn language(&self) -> Language {
        self.language
    }
}

