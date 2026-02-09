//! Language-specific test discovery for ASTGreP
//!
//! This module provides functionality to discover and categorize test files
//! by programming language, supporting the hierarchical test organization
//! structure defined in the migration plan.

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

/// Pattern for identifying test files of a specific language
#[derive(Debug, Clone)]
struct LanguagePattern {
    /// Regex pattern to match file paths
    path_pattern: Regex,
    /// Language this pattern belongs to
    language: String,
    /// Test type inferred from pattern
    test_type: TestType,
    /// Complexity level inferred from pattern
    complexity: TestComplexity,
    /// Description of the pattern
    description: String,
}

/// Analysis of a file for test case classification
#[derive(Debug, Clone)]
struct FileAnalysis {
    /// File path
    file_path: PathBuf,
    /// Detected language
    detected_language: String,
    /// File size in bytes
    file_size: u64,
    /// Last modification time
    last_modified: SystemTime,
    /// File checksum (if calculated)
    checksum: Option<String>,
    /// Detected test type
    test_type: TestType,
    /// Detected complexity
    complexity: TestComplexity,
    /// Relationships to other files
    relationships: Vec<String>,
    /// Content analysis results
    content_analysis: ContentAnalysis,
}

/// Analysis of file content for classification
#[derive(Debug, Clone)]
struct ContentAnalysis {
    /// Lines of code
    line_count: usize,
    /// Dependencies detected (imports, includes, etc.)
    dependencies: Vec<String>,
    /// Test framework usage detected
    frameworks: Vec<String>,
    /// Test annotations or decorators
    test_annotations: Vec<String>,
    /// Keywords indicating test type
    test_keywords: Vec<String>,
    /// Confidence in classification (0.0-1.0)
    classification_confidence: f64,
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
        // Java test patterns
        let java_patterns = vec![
            // JUnit-style test classes
            LanguagePattern {
                path_pattern: Regex::new(r".*Test.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "JUnit test class".to_string(),
            },
            LanguagePattern {
                path_pattern: Regex::new(r".*TestCase.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "Test case class".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*IT.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::Integration,
                complexity: TestComplexity::Complex,
                description: "Integration test interface".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*[Ss]ecurity.*Test.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::Security,
                complexity: TestComplexity::Expert,
                description: "Security test class".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*[Pp]erformance.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::Performance,
                complexity: TestComplexity::Complex,
                description: "Performance test".to_string(),
            },
            // Maven test directory patterns
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*/src/test/java/.*\.java$")?,
                language: "java".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "Maven test directory".to_string(),
            },
        ];

        // Python test patterns
        let python_patterns = vec![
            LanguagePattern {
                path_pattern: regex::Regex::new(r"test_.*\.py$")?,
                language: "python".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "pytest test file".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*_test\.py$")?,
                language: "python".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "unittest test file".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r"tests?/.*test.*\.py$")?,
                language: "python".to_string(),
                test_type: TestType::Integration,
                complexity: TestComplexity::Medium,
                description: "Python test directory".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*_integration.*\.py$")?,
                language: "python".to_string(),
                test_type: TestType::Integration,
                complexity: TestComplexity::Complex,
                description: "Integration test".to_string(),
            },
        ];

        // SQL test patterns
        let sql_patterns = vec![
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*_test.*\.sql$")?,
                language: "sql".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "SQL test file".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*_validate.*\.sql$")?,
                language: "sql".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "SQL validation file".to_string(),
            },
        ];

        // JavaScript/TypeScript test patterns
        let js_patterns = vec![
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*\.test\.js$")?,
                language: "javascript".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "JavaScript test file".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*\.spec\.js$")?,
                language: "javascript".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "Jasmine/JavaScript test spec".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*\.test\.ts$")?,
                language: "typescript".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "TypeScript test file".to_string(),
            },
            LanguagePattern {
                path_pattern: regex::Regex::new(r".*\.spec\.ts$")?,
                language: "typescript".to_string(),
                test_type: TestType::RuleValidation,
                complexity: TestComplexity::Medium,
                description: "TypeScript test spec".to_string(),
            },
        ];

        self.language_patterns.insert("java".to_string(), java_patterns);
        self.language_patterns.insert("python".to_string(), python_patterns);
        self.language_patterns.insert("sql".to_string(), sql_patterns);
        self.language_patterns.insert("javascript".to_string(), js_patterns);
        self.language_patterns.insert("typescript".to_string(), js_patterns);

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

    /// Discover test cases from the configured root directory
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
            if self.is_test_file(&analysis) {
                let test_case = self.create_test_case_from_analysis(&analysis).await?;
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
        let (test_type, complexity) = self.classify_test_file(file_path, &detected_language, content.as_deref())?;

        // Analyze content if enabled
        let content_analysis = if self.config.analyze_content {
            self.analyze_content(file_path, content.as_deref()).await?
        } else {
            ContentAnalysis::default()
        };

        // Calculate checksum if enabled
        let checksum = if self.config.calculate_checksums {
            if let Some(content) = content {
                Some(format!("{:x}", sha2::Sha256::digest(content.as_bytes())))
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

        // Cache the analysis
        self.analysis_cache.insert(file_path.to_path_buf(), analysis.clone());

        Ok(analysis)
    }

    /// Check if a file is identified as a test file
    fn is_test_file(&self, analysis: &FileAnalysis) -> bool {
        // Check if file extension suggests it's a test file
        if let Some(extension) = analysis.file_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "test" | "spec" | "it" | "re" => return true,
                _ => {}
            }
        }

        // Check if filename contains test indicators
        if let Some(filename) = analysis.file_path.file_stem().and_then(|s| s.to_str()) {
            if filename.to_lowercase().contains("test") ||
               filename.to_lowercase().contains("spec") ||
               filename.to_lowercase().contains("validate") ||
               filename.to_lowercase().contains("check") {
                return true;
            }
        }

        // Check if language patterns classify it as a test file
        if let Some(patterns) = self.language_patterns.get(&analysis.detected_language) {
            for pattern in patterns {
                if pattern.path_pattern.is_match(&analysis.file_path.to_string_lossy()) {
                    return true;
                }
            }
        }

        // Check content analysis
        if analysis.content_analysis.classification_confidence > 0.7 {
            return true;
        }

        false
    }

    /// Classify a file as a test case
    fn classify_test_file(&self, file_path: &Path, language: &str, content: &str) -> (TestType, TestComplexity) {
        let filename_lower = file_path.file_stem()
            .and_then(|s| s.to_string())
            .unwrap_or_default()
            .to_lowercase();

        // Use language-specific patterns first
        if let Some(patterns) = self.language_patterns.get(language) {
            for pattern in patterns {
                if pattern.path_pattern.is_match(&file_path.to_string_lossy()) {
                    return (pattern.test_type.clone(), pattern.complexity.clone());
                }
            }
        }

        // Generic classification based on filename patterns
        if filename_lower.contains("test") {
            if filename_lower.contains("security") {
                return (TestType::Security, TestComplexity::Expert);
            } else if filename_lower.contains("performance") || filename_lower.contains("perf") {
                return (TestType::Performance, TestComplexity::Complex);
            } else if filename_lower.contains("integration") {
                return (TestType::Integration, TestComplexity::Complex);
            } else if filename_lower.contains("parsing") || filename_lower.contains("parse") {
                return (TestType::Parsing, TestComplexity::Medium);
            } else if filename_lower.contains("basic") {
                return (TestType::Basic, TestComplexity::Simple);
            } else {
                return (TestType::RuleValidation, TestComplexity::Medium);
            }
        } else if filename_lower.contains("spec") {
            return (TestType::RuleValidation, TestType::Medium);
        }

        // Default classification
        (TestType::RuleValidation, TestComplexity::Medium)
    }

    /// Analyze file content for additional classification
    async fn analyze_content(&self, file_path: &Path, content: &str) -> Result<ContentAnalysis> {
        let lines: Vec<&str> = content.lines().collect();
        let mut dependencies = Vec::new();
        let mut frameworks = Vec::new();
        let mut test_annotations = Vec::new();
        let mut test_keywords = Vec::new();

        // Java-specific analysis
        if file_path.extension().and_then(|e| e.to_str()) == Some("java") {
            self.analyze_java_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
        }

        // Python-specific analysis
        if file_path.extension().and_then(|e| e.to_str()) == Some("py") {
            self.analyze_python_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
        }

        // JavaScript/TypeScript analysis
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            if ext == "js" || ext == "ts" || ext == "jsx" || ext == "tsx" {
                self.analyze_javascript_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
            }
        }

        // SQL analysis
        if file_path.extension().and_then(|e| e.to_str()) == Some("sql") {
            self.analyze_sql_content(&lines, &mut dependencies, &mut test_annotations);
        }

        // Generic test keyword detection
        for line in &lines {
            let line_lower = line.to_lowercase();
            if line_lower.contains("test") || line_lower.contains("assert") {
                test_keywords.push(line.trim().to_string());
            }
        }

        let line_count = lines.len();
        let classification_confidence = self.calculate_classification_confidence(
            &dependencies, &frameworks, &test_annotations, &test_keywords
        );

        Ok(ContentAnalysis {
            line_count,
            dependencies,
            frameworks,
            test_annotations,
            test_keywords,
            classification_confidence,
        })
    }

    /// Analyze Java file content
    fn analyze_java_content(
        &self,
        lines: &[&str],
        dependencies: &mut Vec<String>,
        frameworks: &mut Vec<String>,
        test_annotations: &mut Vec<String>,
    ) {
        for line in lines {
            let line = line.trim();

            // Import statements
            if line.starts_with("import ") {
                if let Some(import_path) = line.split_whitespace().nth(1) {
                    dependencies.push(import_path.to_string());
                }
            }

            // Framework detection
            if line.contains("@Test") {
                frameworks.push("JUnit".to_string());
            }

            // Test annotations
            if line.starts_with("@") {
                test_annotations.push(line.to_string());
            }

            // Dependency keywords
            if line.contains("import ") || line.contains("package ") {
                dependencies.push(line.to_string());
            }
        }
    }

    /// Analyze Python file content
    fn analyze_python_content(
        &self,
        lines: &[&str],
        dependencies: &mut Vec<String>,
        frameworks: &mut Vec<String>>,
        test_annotations: &mut Vec<String>,
    ) {
        for line in lines {
            let line = line.trim();

            // Import statements
            if line.starts_with("import ") || line.starts_with("from ") {
                dependencies.push(line.to_string());
            }

            // Framework detection
            if line.contains("unittest.") || line.contains("unittest.main") {
                frameworks.push("unittest".to_string());
            } else if line.contains("pytest.") || line.contains("pytest.fixture") {
                frameworks.push("pytest".to_string());
            }

            // Test functions
            if line.starts_with("def test_") {
                test_annotations.push(line.to_string());
            }

            // Decorators
            if line.starts_with("@") {
                test_annotations.push(line.to_string());
            }
        }
    }

    /// Analyze JavaScript/TypeScript file content
    fn analyze_javascript_content(
        &self,
        lines: &[&str],
        dependencies: &mut Vec<String>,
        frameworks: &mut Vec<String>,
        test_annotations: &mut Vec<String>,
    ) {
        for line in lines {
            let line = line.trim();

            // Import statements
            if line.starts_with("import ") || line.starts_with("const ") {
                dependencies.push(line.to_string());
            }

            // Framework detection
            if line.contains("describe(") {
                frameworks.push("Jest".to_string());
            } else if line.contains("it(") {
                frameworks.push("Jest".to_string());
            } else if line.contains("test(") {
                test_annotations.push(line.to_string());
            }

            // Require statements
            if line.starts_with("require(") {
                dependencies.push(line.to_string());
            }
        }
    }

    /// Analyze SQL file content
    fn analyze_sql_content(
        &self,
        lines: &[&str],
        dependencies: &mut Vec<String>,
        test_annotations: &mut Vec<String>,
    ) {
        for line in lines {
            let line = line.trim();

            // SQL keywords and statements
            if line.starts_with("SELECT ") || line.starts_with("INSERT ") ||
               line.starts_with("UPDATE ") || line.starts_with("DELETE ") ||
               line.starts_with("CREATE ") || line.starts_with("DROP ") {
                dependencies.push(line.to_string());
            }

            // Test annotations
            if line.contains("-- @test") || line.contains("-- @validate") {
                test_annotations.push(line.to_string());
            }
        }
    }

    /// Calculate confidence in classification
    fn calculate_classification_confidence(
        &self,
        dependencies: &[String],
        frameworks: &[String],
        test_annotations: &[String],
        test_keywords: &[String],
    ) -> f64 {
        let mut confidence = 0.0;

        // High confidence indicators
        if !test_annotations.is_empty() {
            confidence += 0.4;
        }

        if !frameworks.is_empty() {
            confidence += 0.3;
        }

        if test_keywords.len() > 2 {
            confidence += 0.2;
        }

        // Medium confidence indicators
        if !dependencies.is_empty() {
            confidence += 0.1;
        }

        confidence.min(1.0)
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

    /// Create a TestCase from file analysis
    async fn create_test_case_from_analysis(&self, analysis: &FileAnalysis) -> Result<TestCase> {
        let test_case_id = format!("tc-{}",
            chrono::Utc::now().timestamp_nanos()
        );

        let test_case_name = analysis.file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let current_path = analysis.file_path.clone();
        let target_path = self.generate_target_path(&analysis).await?;

        // Determine category based on test type and language
        let category = self.determine_category(&analysis);

        let metadata = TestCaseMetadata {
            file_size: analysis.file_size,
            line_count: analysis.content_analysis.line_count,
            created_at: None,
            modified_at: analysis.last_modified
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)),
            author: None,
            version: None,
            framework: analysis.content_analysis.frameworks.first().cloned(),
            environment_requirements: Vec::new(),
            estimated_execution_time: None,
            priority: TestPriority::Normal,
            custom_properties: HashMap::new(),
        };

        let test_case = TestCase::new(
            test_case_id,
            test_case_name,
            analysis.test_type.clone(),
            current_path,
            target_path,
        )
        .with_languages(vec![analysis.detected_language.clone()])
        .with_complexity(analysis.complexity.clone())
        .with_category(category)
        .with_dependencies(analysis.relationships.clone())
        .with_tags(analysis.content_analysis.test_keywords.clone())
        .with_description(format!("Test case for {}", test_case_name));

        Ok(test_case)
    }

    /// Generate target path for a test case in the new structure
    async fn generate_target_path(&self, analysis: &FileAnalysis) -> Result<PathBuf> {
        let language = &analysis.detected_language;
        let test_type_str = format!("{:?}", analysis.test_type).to_lowercase();
        let complexity_str = format!("{:?}", analysis.complexity).to_lowercase();

        // Get language configuration
        let lang_config = self.config.language_mapping.get_language_config(language)
            .ok_or_else(|| {
                // Create default config
                LanguageConfig {
                    language: language.clone(),
                    directory_name: language.clone(),
                    extensions: vec![],
                    common_test_types: vec![TestType::RuleValidation],
                    frameworks: vec![],
                    default_category: TestCategory::LanguageSpecific,
                    test_file_patterns: vec![],
                }
            });

        // Build path: newtest/testcases/{language}/{test-type}/
        let test_type_dir = match analysis.test_type {
            TestType::PatternMatching => "pattern-matching",
            TestType::RuleValidation => "rule-validation",
            TestType::Parsing => "parsing",
            TestType::Integration => "integration",
            TestType::Performance => "performance",
            TestType::Security => "security",
            TestType::Compatibility => "compatibility",
            TestType::Custom => "custom",
        };

        let mut target_path = self.config.root_directory
            .join("newtest")
            .join("testcases")
            .join(&lang_config.directory_name)
            .join(test_type_dir);

        // Add filename based on original file
        if let Some(filename) = analysis.file_path.file_stem() {
            target_path = target_path.join(filename);
        } else {
            // Generate a default filename
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            target_path = target_path.join(format!("test_{}.java", timestamp));
        }

        // Ensure file extension matches the language
        let target_ext = lang_config.extensions.first()
            .unwrap_or("txt");
        target_path = target_path.with_extension(target_ext);

        Ok(target_path)
    }

    /// Determine the category for a test case
    fn determine_category(&self, analysis: &FileAnalysis) -> Option<TestCategory> {
        match analysis.test_type {
            TestType::Basic => Some(TestCategory::Basic),
            TestType::PatternMatching => Some(TestCategory::Framework),
            TestType::RuleValidation => Some(TestCategory::Basic),
            TestType::Parsing => Some(TestCategory::Framework),
            TestType::Integration => Some(TestCategory::Integration),
            TestType::Performance => Some(TestCategory::Performance),
            TestType::Security => Some(TestCategory::Security),
            TestType::Compatibility => Some(TestCategory::Compatibility),
            TestType::Custom => Some(TestCategory::Other("Custom".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_language_discovery_config_default() {
        let config = LanguageDiscoveryConfig::default();
        assert_eq!(config.root_directory, PathBuf::from("."));
        assert!(config.recursive_search);
        assert_eq!(config.max_depth, 0);
        assert!(config.analyze_content);
        assert!(config.calculate_checksums);
    }

    #[test]
    fn test_language_mapping_creation() {
        let mapping = LanguageMapping::new();
        assert!(mapping.extension_to_language.contains_key("java"));
        assert!(mapping.extension_to_language.get("java").unwrap() == "java");
        assert!(mapping.language_configs.contains_key("java"));
        assert!(mapping.default_language == "unknown");
    }

    #[tokio::test]
    async fn test_file_analysis() {
        let config = LanguageDiscoveryConfig::default();
        let discovery = LanguageDiscovery::new(config).unwrap();

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
        let discovery = LanguageDiscovery::new(config).unwrap();

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

        assert!(discovery.should_exclude_file(&PathBuf::from("target/test.java")));
        assert!(discovery.should_exclude_file(&PathBuf::from("node_modules/test.py")));
        assert!(discovery.should_exclude_file(&PathBuf::from("test.tmp"))));
    }

    #[test]
    fn test_classification_confidence() {
        let discovery = LanguageDiscovery::new(LanguageDiscoveryConfig::default()).unwrap();

        // High confidence test
        let high_confidence = discovery.calculate_classification_confidence(
            &vec!["import os".to_string()],
            &vec!["JUnit".to_string()],
            &vec!["@Test".to_string()],
            &vec!["assert".to_string(), "test".to_string(), "should".to_string()],
        );
        assert!(high_confidence > 0.8);

        // Low confidence test
        let low_confidence = discovery.calculate_classification_confidence(
            &Vec::new(),
            &Vec::new(),
            &Vec::new(),
            &vec!["code".to_string()],
        );
        assert!(low_confidence < 0.3);
    }

    #[test]
    fn test_target_path_generation() async {
        let config = LanguageDiscoveryConfig {
            root_directory: PathBuf::from("/project"),
            language_mapping: LanguageMapping::new(),
            ..Default::default()
        };
        let discovery = LanguageDiscovery::new(config).unwrap();

        let analysis = FileAnalysis {
            file_path: PathBuf::from("/tests/SecurityTest.java"),
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