//! Main discovery orchestration and file analysis
//!
//! This module handles the core discovery workflow including file scanning,
//! content analysis, test case creation, and relationship detection.

use super::{LanguageDiscoveryConfig, DiscoveryResult, DiscoverySummary};
use super::detection::{LanguagePattern, classify_test_file, is_test_file, ContentAnalysis, calculate_classification_confidence};
use super::extensions::detect_language;
use super::content_analysis::analyze_content;
use super::test_case_creation::{FileAnalysis, create_test_case_from_analysis, generate_target_path, determine_category};
use astgrep_core::{
    models::{TestCase, TestType, TestComplexity, TestCategory, TestCaseMetadata, TestPriority, LanguageConfig},
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
use sha2::Sha256;

/// Language-specific test discovery engine
pub struct LanguageDiscovery {
    config: LanguageDiscoveryConfig,
    /// Pre-compiled regex patterns for different languages
    language_patterns: HashMap<String, Vec<LanguagePattern>>,
    /// File analysis cache to avoid redundant work
    analysis_cache: HashMap<PathBuf, FileAnalysis>,
    /// Exclude patterns as compiled regex
    compiled_exclude_patterns: Vec<Regex>,
}

impl LanguageDiscovery {
    /// Create a new language discovery engine
    pub fn new(config: LanguageDiscoveryConfig) -> Result<Self> {
        let mut discovery = Self {
            config,
            language_patterns: HashMap::new(),
            analysis_cache: HashMap::new(),
            compiled_exclude_patterns: Vec::new(),
        };

        discovery.initialize_language_patterns()?;
        discovery.compile_exclude_patterns()?;

        Ok(discovery)
    }

    /// Initialize language-specific patterns for test file identification
    fn initialize_language_patterns(&mut self) -> Result<()> {
        self.language_patterns = super::detection::initialize_language_patterns()?;
        Ok(())
    }

    /// Compile exclude patterns to regex
    fn compile_exclude_patterns(&mut self) -> Result<()> {
        self.compiled_exclude_patterns = self.config.exclude_patterns
            .iter()
            .map(|pattern| {
                let regex_pattern = pattern.replace('*', ".*").replace('?', ".");
                Regex::new(&regex_pattern)
                    .map_err(|e| anyhow!("Failed to compile exclude pattern '{}': {}", pattern, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    /// Discover test cases from configured root directory
    #[instrument(skip(self))]
    pub async fn discover_test_cases(&mut self) -> Result<DiscoveryResult> {
        info!("Starting language-specific test case discovery");

        let start_time = std::time::Instant::now();
        let mut test_cases = Vec::new();
        let mut excluded_files = Vec::new();
        let mut warnings = Vec::new();
        let mut total_files_scanned = 0usize;
        let mut total_bytes_analyzed = 0u64;
        let mut language_distribution = HashMap::new();
        let mut type_distribution = HashMap::new();

        debug!("Scanning directory: {}", self.config.root_directory.display());

        let walker = if self.config.recursive_search {
            if self.config.max_depth > 0 {
                WalkDir::new(&self.config.root_directory)
                    .max_depth(self.config.max_depth)
            } else {
                WalkDir::new(&self.config.root_directory)
            }
        } else {
            WalkDir::new(&self.config.root_directory).max_depth(1)
        };

        for entry in walker {
            let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;

            total_files_scanned += 1;

            // Skip directories for now
            if entry.file_type().is_dir() {
                continue;
            }

            let file_path = entry.path();

            // Check if file should be excluded
            if self.should_exclude_file(&file_path) {
                excluded_files.push((file_path, "Matched exclude pattern".to_string()));
                continue;
            }

            // Check file size limits
            if let Some((min_size, max_size)) = self.config.file_size_limits {
                if let Ok(metadata) = entry.metadata() {
                    let file_size = metadata.len();
                    if file_size < min_size || file_size > max_size {
                        excluded_files.push((file_path, format!("File size {} bytes outside range {}-{}", file_size, min_size, max_size)));
                        continue;
                    }
                }
            }

            // Detect language and test type
            let analysis = self.analyze_file(&file_path).await?;
            total_bytes_analyzed += analysis.file_size;

            // Only include if it's identified as a test file
            if is_test_file(&analysis.file_path, &analysis.detected_language, &self.language_patterns, &analysis.content_analysis) {
                let test_case = create_test_case_from_analysis(&analysis, &self.config).await?;
                test_cases.push(test_case);

                // Update distributions
                *language_distribution.entry(analysis.detected_language.clone()).or_insert(0) += 1;
                *type_distribution.entry(format!("{:?}", analysis.test_type)).or_insert(0) += 1;
            }
        }

        let discovery_duration = start_time.elapsed();

        info!("Discovery completed: {} test cases found from {} files", test_cases.len(), total_files_scanned);

        Ok(DiscoveryResult {
            test_cases,
            summary: DiscoverySummary {
                total_files_scanned,
                test_files_found: test_cases.len(),
                non_test_files_found: total_files_scanned - test_cases.len() - excluded_files.len(),
                files_excluded: excluded_files.len(),
                unique_languages_detected: language_distribution.len(),
                total_bytes_analyzed,
                discovery_duration_ms: discovery_duration.as_millis() as u64,
            },
            language_distribution,
            type_distribution,
            excluded_files,
            warnings,
            discovered_at: chrono::Utc::now(),
        })
    }

    /// Check if a file should be excluded based on patterns
    fn should_exclude_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        self.compiled_exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(&path_str))
    }

    /// Analyze a file for test case classification
    #[instrument(skip(self, file_path))]
    async fn analyze_file(&mut self, file_path: &Path) -> Result<FileAnalysis> {
        // Check cache first
        if let Some(cached_analysis) = self.analysis_cache.get(file_path) {
            return Ok(cached_analysis.clone());
        }

        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len();
        let last_modified = metadata.modified()?;

        // Detect language
        let content = if self.config.analyze_content {
            fs::read_to_string(file_path).ok()
        } else {
            None
        };

        let detected_language = if let Some(ref content) = content {
            self.config.language_mapping.detect_language(file_path, Some(content))
        } else {
            self.config.language_mapping.detect_language(file_path, None)
        };

        // Classify as test file
        let (test_type, complexity) = classify_test_file(file_path, &detected_language, &self.language_patterns);

        // Analyze content if enabled
        let content_analysis = if self.config.analyze_content {
            if let Some(ref content_str) = content {
                analyze_content(file_path, content_str)?
            } else {
                ContentAnalysis::default()
            }
        } else {
            ContentAnalysis::default()
        };

        // Calculate checksum if enabled
        let checksum = if self.config.calculate_checksums {
            if let Some(content) = content {
                Some(format!("{:x}", Sha256::digest(content.as_bytes())))
            } else {
                None
            }
        } else {
            None
        };

        // Detect relationships if enabled
        let relationships = if self.config.detect_relationships {
            self.detect_relationships(file_path, content.as_deref()).await?
        } else {
            Vec::new()
        };

        let analysis = FileAnalysis {
            file_path: file_path.to_path_buf(),
            detected_language,
            file_size,
            last_modified,
            checksum,
            test_type,
            complexity,
            relationships,
            content_analysis,
        };

        // Cache analysis
        self.analysis_cache.insert(file_path.to_path_buf(), analysis.clone());

        Ok(analysis)
    }

    /// Detect relationships between test files
    async fn detect_relationships(
        &self,
        file_path: &Path,
        content: &str,
    ) -> Result<Vec<String>> {
        let mut relationships = Vec::new();
        let filename = file_path.file_stem()
            .and_then(|s| s.to_string())
            .unwrap_or_default();

        // Look for references to other test files
        for line in content.lines() {
            if line.contains(filename) && line.contains("test") {
                // Extract referenced test names
                let words: Vec<&str> = line.split_whitespace().collect();
                for word in words {
                    if word != filename && (word.contains("Test") || word.ends_with("Test")) {
                        relationships.push(word.to_string());
                    }
                }
            }
        }

        Ok(relationships)
    }

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_language_discovery_config_default() {
        let config = LanguageDiscoveryConfig::default();
        assert_eq!(config.root_directory, std::path::PathBuf::from("."));
        assert!(config.recursive_search);
        assert_eq!(config.max_depth, 0);
        assert!(config.analyze_content);
        assert!(config.calculate_checksums);
    }

    #[tokio::test]
    async fn test_file_analysis() {
        let config = LanguageDiscoveryConfig::default();
        let mut discovery = LanguageDiscovery::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("SecurityTest.java");
        let content = r#"
package com.example;

import org.junit.Test;
import static org.junit.Assert.*;

/**
 * Security test class for authentication
 */
public class SecurityTest {

    @Test
    public void testAuthentication() {
        Assert.assertTrue(true);
    }
}
"#;
        fs::write(&test_file, content).unwrap();

        let analysis = discovery.analyze_file(&test_file).await.unwrap();
        assert_eq!(analysis.detected_language, "java");
        assert_eq!(analysis.test_type, TestType::Security);
        assert_eq!(analysis.complexity, TestComplexity::Expert);
        assert!(analysis.content_analysis.frameworks.contains(&"JUnit".to_string()));
    }

    #[tokio::test]
    async fn test_python_test_detection() {
        let config = LanguageDiscoveryConfig::default();
        let mut discovery = LanguageDiscovery::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_auth.py");
        let content = r#"
import unittest
import pytest

class TestAuthentication:
    def test_login(self):
        assert True
"#;
        fs::write(&test_file, content).unwrap();

        let analysis = discovery.analyze_file(&test_file).await.unwrap();
        assert_eq!(analysis.detected_language, "python");
        assert!(analysis.content_analysis.frameworks.contains(&"unittest".to_string()) ||
                   analysis.content_analysis.frameworks.contains(&"pytest".to_string()));
        assert!(analysis.content_analysis.test_annotations.len() > 0);
    }

    #[test]
    fn test_exclude_patterns() {
        let config = LanguageDiscoveryConfig::default();
        let discovery = LanguageDiscovery::new(config).unwrap();

        assert!(discovery.should_exclude_file(&std::path::PathBuf::from("target/test.java")));
        assert!(discovery.should_exclude_file(&std::path::PathBuf::from("node_modules/test.py")));
        assert!(discovery.should_exclude_file(&std::path::PathBuf::from("test.tmp")));
    }

    #[test]
    fn test_classification_confidence() {
        // High confidence test
        let high_confidence = calculate_classification_confidence(
            &vec!["import os".to_string()],
            &vec!["JUnit".to_string()],
            &vec!["@Test".to_string()],
            &vec!["assert".to_string(), "test".to_string(), "should".to_string()],
        );
        assert!(high_confidence > 0.8);

        // Low confidence test
        let low_confidence = calculate_classification_confidence(
            &Vec::new(),
            &Vec::new(),
            &Vec::new(),
            &vec!["code".to_string()],
        );
        assert!(low_confidence < 0.3);
    }

    #[tokio::test]
    async fn test_target_path_generation() {
        let config = LanguageDiscoveryConfig {
            root_directory: std::path::PathBuf::from("/project"),
            language_mapping: astgrep_core::models::LanguageMapping::new(),
            ..Default::default()
        };
        let mut discovery = LanguageDiscovery::new(config).unwrap();

        let analysis = FileAnalysis {
            file_path: std::path::PathBuf::from("/tests/SecurityTest.java"),
            detected_language: "java".to_string(),
            file_size: 1024,
            last_modified: SystemTime::now(),
            checksum: Some("abc123".to_string()),
            test_type: TestType::Security,
            complexity: TestComplexity::Expert,
            relationships: Vec::new(),
            content_analysis: ContentAnalysis::default(),
        };

        let target_path = discovery.generate_target_path(&analysis).await.unwrap();

        assert!(target_path.starts_with("/project/newtest/testcases/java/security/"));
        assert!(target_path.ends_with(".java"));
    }
}
