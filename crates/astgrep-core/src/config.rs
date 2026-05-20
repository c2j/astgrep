//! Configuration management for ASTGreP

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

/// Path handler for cross-platform path operations
pub struct PathHandler {
    /// Base directory for operations
    base_dir: PathBuf,
    /// Path separator for the current platform
    _separator: String,
}

impl PathHandler {
    /// Create a new path handler
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            _separator: std::path::MAIN_SEPARATOR.to_string(),
        }
    }

    /// Create a path handler with a specific base directory
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            _separator: std::path::MAIN_SEPARATOR.to_string(),
        }
    }

    /// Normalize a path to use forward slashes (for cross-platform compatibility)
    pub fn normalize_path(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let normalized = path_str.replace('\\', "/");
        PathBuf::from(normalized)
    }

    /// Convert a path to be relative to the base directory
    pub fn make_relative(&self, path: &PathBuf) -> Result<PathBuf> {
        pathdiff::diff_paths(path, &self.base_dir).ok_or_else(|| {
            crate::error::AnalysisError::parse_error(format!(
                "Cannot make path relative to base directory: {:?}",
                path
            ))
        })
    }

    /// Join path components using the platform separator
    pub fn join(&self, components: &[&str]) -> PathBuf {
        let mut path = PathBuf::new();
        for component in components {
            path.push(component);
        }
        path
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }
}

impl Default for PathHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for ASTGreP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstGrepConfig {
    /// Default timeout for operations
    pub default_timeout: std::time::Duration,
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: usize,
    /// Whether to enable debug logging
    pub debug_logging: bool,
    /// Default output format
    pub default_output_format: String,
    /// Custom configuration values
    pub custom_settings: HashMap<String, String>,
}

impl Default for AstGrepConfig {
    fn default() -> Self {
        Self {
            default_timeout: std::time::Duration::from_secs(300),
            max_concurrent_operations: 4,
            debug_logging: false,
            default_output_format: "json".to_string(),
            custom_settings: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_handler_new_defaults_to_cwd() {
        let handler = PathHandler::new();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        assert_eq!(handler.base_dir(), &cwd);
    }

    #[test]
    fn test_path_handler_with_base_dir_sets_base() {
        let base = PathBuf::from("/some/base/dir");
        let handler = PathHandler::with_base_dir(base.clone());
        assert_eq!(handler.base_dir(), &base);
    }

    #[test]
    fn test_path_handler_normalize_backslashes() {
        let handler = PathHandler::new();
        let path = PathBuf::from("foo\\bar\\baz");
        let normalized = handler.normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("foo/bar/baz"));
    }

    #[test]
    fn test_path_handler_normalize_already_forward_slashes() {
        let handler = PathHandler::new();
        let path = PathBuf::from("foo/bar/baz");
        let normalized = handler.normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("foo/bar/baz"));
    }

    #[test]
    fn test_path_handler_normalize_empty_path() {
        let handler = PathHandler::new();
        let path = PathBuf::from("");
        let normalized = handler.normalize_path(&path);
        assert_eq!(normalized, PathBuf::from(""));
    }

    #[test]
    fn test_path_handler_make_relative_subpath() {
        let base = PathBuf::from("/home/user");
        let handler = PathHandler::with_base_dir(base);
        let path = PathBuf::from("/home/user/projects/foo");
        let relative = handler.make_relative(&path).expect("should resolve");
        assert_eq!(relative, PathBuf::from("projects/foo"));
    }

    #[test]
    fn test_path_handler_make_relative_same_path() {
        let base = PathBuf::from("/home/user");
        let handler = PathHandler::with_base_dir(base.clone());
        let relative = handler.make_relative(&base).expect("should resolve");
        assert_eq!(relative, PathBuf::from(""));
    }

    #[test]
    fn test_path_handler_join_components() {
        let handler = PathHandler::new();
        let joined = handler.join(&["a", "b", "c"]);
        assert_eq!(joined, PathBuf::from("a/b/c"));
    }

    #[test]
    fn test_path_handler_join_empty_components() {
        let handler = PathHandler::new();
        let joined = handler.join(&[]);
        assert_eq!(joined, PathBuf::from(""));
    }

    #[test]
    fn test_path_handler_join_single_component() {
        let handler = PathHandler::new();
        let joined = handler.join(&["standalone"]);
        assert_eq!(joined, PathBuf::from("standalone"));
    }

    #[test]
    fn test_path_handler_default_matches_new() {
        let default_handler = PathHandler::default();
        let new_handler = PathHandler::new();
        assert_eq!(default_handler.base_dir(), new_handler.base_dir());
    }

    #[test]
    fn test_ast_grep_config_default_values() {
        let config = AstGrepConfig::default();
        assert_eq!(config.default_timeout, std::time::Duration::from_secs(300));
        assert_eq!(config.max_concurrent_operations, 4);
        assert_eq!(config.debug_logging, false);
        assert_eq!(config.default_output_format, "json");
        assert!(config.custom_settings.is_empty());
    }
}
