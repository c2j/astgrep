//! Configuration management for ASTGreP

use crate::Result;
use serde::{Deserialize, Serialize};
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
    pub fn normalize_path(&self, path: &PathBuf) -> PathBuf {
        let path_str = path.to_string_lossy();
        let normalized = path_str.replace('\\', "/");
        PathBuf::from(normalized)
    }

    /// Convert a path to be relative to the base directory
    pub fn make_relative(&self, path: &PathBuf) -> Result<PathBuf> {
        pathdiff::diff_paths(path, &self.base_dir)
            .ok_or_else(|| crate::error::AnalysisError::parse_error(
                format!("Cannot make path relative to base directory: {:?}", path)
            ))
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