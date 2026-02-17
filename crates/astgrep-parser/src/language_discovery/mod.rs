//! Language-specific test discovery for ASTGreP
//!
//! This module provides functionality to discover and categorize test files
//! by programming language, supporting hierarchical test organization
//! structure defined in migration plan.
//!
//! ## Module Structure
//! - `detection`: Language detection and test classification logic
//! - `extensions`: File extension mapping and language configuration
//! - `content_analysis`: Language-specific content analysis
//! - `test_case_creation`: Test case creation and path generation
//! - `discovery`: Main discovery orchestration and file analysis

pub mod detection;
pub mod extensions;
pub mod content_analysis;
pub mod test_case_creation;
pub mod discovery;

// Re-export public types for backward compatibility
pub use detection::{LanguagePattern, classify_test_file, is_test_file};
pub use extensions::*;
pub use discovery::{
    LanguageDiscovery,
    LanguageDiscoveryConfig,
    DiscoveryResult,
    DiscoverySummary,
};

use astgrep_core::{
    models::{TestCase, TestType, TestComplexity, TestCaseStatus, LanguageMapping, TestCategory, TestCaseMetadata, TestPriority},
    error::Result as AstGrepResult,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{info, warn, debug, error, instrument};
use walkdir::{WalkDir, DirEntry};
use regex::Regex;
use anyhow::{Result, anyhow};

/// Configuration for test case discovery
#[derive(Debug, Clone)]
pub struct LanguageDiscoveryConfig {
    /// Root directory to search for test cases
    pub root_directory: PathBuf,
    /// Language mapping configuration
    pub language_mapping: LanguageMapping,
    /// Whether to search recursively through subdirectories
    pub recursive_search: bool,
    /// Maximum depth for recursive search (0 = unlimited)
    pub max_depth: usize,
    /// File size limits (min, max bytes)
    pub file_size_limits: Option<(u64, u64)>,
    /// Patterns to exclude from discovery
    pub exclude_patterns: Vec<String>,
    /// Whether to analyze file content for additional classification
    pub analyze_content: bool,
    /// Whether to calculate checksums for integrity verification
    pub calculate_checksums: bool,
    /// Whether to detect test case relationships and dependencies
    pub detect_relationships: bool,
}

impl Default for LanguageDiscoveryConfig {
    fn default() -> Self {
        Self {
            root_directory: PathBuf::from("."),
            language_mapping: LanguageMapping::new(),
            recursive_search: true,
            max_depth: 0,
            file_size_limits: Some((10, 10_000_000)), // 10 bytes to 10MB
            exclude_patterns: vec![
                ".git/*".to_string(),
                "node_modules/*".to_string(),
                "target/*".to_string(),
                "build/*".to_string(),
                "dist/*".to_string(),
                "*.tmp".to_string(),
                "*.bak".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
            ],
            analyze_content: true,
            calculate_checksums: true,
            detect_relationships: true,
        }
    }
}

/// Result of discovering test cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// All discovered test cases
    pub test_cases: Vec<TestCase>,
    /// Discovery summary statistics
    pub summary: DiscoverySummary,
    /// Language distribution
    pub language_distribution: HashMap<String, usize>,
    /// Test type distribution
    pub type_distribution: HashMap<String, usize>,
    /// Files that were excluded and why
    pub excluded_files: Vec<(PathBuf, String)>,
    /// Any warnings or issues discovered
    pub warnings: Vec<String>,
    /// Discovery timestamp
    pub discovered_at: chrono::DateTime<chrono::Utc>,
}

/// Summary of discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySummary {
    /// Total files scanned
    pub total_files_scanned: usize,
    /// Test files found
    pub test_files_found: usize,
    /// Non-test files found
    pub non_test_files_found: usize,
    /// Files excluded by patterns
    pub files_excluded: usize,
    /// Unique languages detected
    pub unique_languages_detected: usize,
    /// Total bytes analyzed
    pub total_bytes_analyzed: u64,
    /// Discovery duration in milliseconds
    pub discovery_duration_ms: u64,
}
