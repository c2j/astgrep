//! Language detection and test classification
//!
//! This module provides language-specific pattern matching for test file detection
//! and classification of test types and complexity levels.

use regex::Regex;
use astgrep_core::models::{TestType, TestComplexity};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Pattern for identifying test files of a specific language
#[derive(Debug, Clone)]
pub struct LanguagePattern {
    /// Regex pattern to match file paths
    pub path_pattern: Regex,
    /// Language this pattern belongs to
    pub language: String,
    /// Test type inferred from pattern
    pub test_type: TestType,
    /// Complexity level inferred from pattern
    pub complexity: TestComplexity,
    /// Description of pattern
    pub description: String,
}

/// Initialize language-specific patterns for test file identification
pub fn initialize_language_patterns() -> Result<HashMap<String, Vec<LanguagePattern>>> {
    let mut patterns = HashMap::new();

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
            path_pattern: Regex::new(r".*IT.*\.java$")?,
            language: "java".to_string(),
            test_type: TestType::Integration,
            complexity: TestComplexity::Complex,
            description: "Integration test interface".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*[Ss]ecurity.*Test.*\.java$")?,
            language: "java".to_string(),
            test_type: TestType::Security,
            complexity: TestComplexity::Expert,
            description: "Security test class".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*[Pp]erformance.*\.java$")?,
            language: "java".to_string(),
            test_type: TestType::Performance,
            complexity: TestComplexity::Complex,
            description: "Performance test".to_string(),
        },
        // Maven test directory patterns
        LanguagePattern {
            path_pattern: Regex::new(r".*/src/test/java/.*\.java$")?,
            language: "java".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "Maven test directory".to_string(),
        },
    ];

    // Python test patterns
    let python_patterns = vec![
        LanguagePattern {
            path_pattern: Regex::new(r"test_.*\.py$")?,
            language: "python".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "pytest test file".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*_test\.py$")?,
            language: "python".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "unittest test file".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r"tests?/.*test.*\.py$")?,
            language: "python".to_string(),
            test_type: TestType::Integration,
            complexity: TestComplexity::Medium,
            description: "Python test directory".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*_integration.*\.py$")?,
            language: "python".to_string(),
            test_type: TestType::Integration,
            complexity: TestComplexity::Complex,
            description: "Integration test".to_string(),
        },
    ];

    // SQL test patterns
    let sql_patterns = vec![
        LanguagePattern {
            path_pattern: Regex::new(r".*_test.*\.sql$")?,
            language: "sql".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "SQL test file".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*_validate.*\.sql$")?,
            language: "sql".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "SQL validation file".to_string(),
        },
    ];

    // JavaScript/TypeScript test patterns
    let js_patterns = vec![
        LanguagePattern {
            path_pattern: Regex::new(r".*\.test\.js$")?,
            language: "javascript".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "JavaScript test file".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*\.spec\.js$")?,
            language: "javascript".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "Jasmine/JavaScript test spec".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*\.test\.ts$")?,
            language: "typescript".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "TypeScript test file".to_string(),
        },
        LanguagePattern {
            path_pattern: Regex::new(r".*\.spec\.ts$")?,
            language: "typescript".to_string(),
            test_type: TestType::RuleValidation,
            complexity: TestComplexity::Medium,
            description: "TypeScript test spec".to_string(),
        },
    ];

    patterns.insert("java".to_string(), java_patterns);
    patterns.insert("python".to_string(), python_patterns);
    patterns.insert("sql".to_string(), sql_patterns);
    patterns.insert("javascript".to_string(), js_patterns);
    patterns.insert("typescript".to_string(), js_patterns);

    Ok(patterns)
}

/// Check if a file is identified as a test file
pub fn is_test_file(
    file_path: &std::path::Path,
    detected_language: &str,
    language_patterns: &HashMap<String, Vec<LanguagePattern>>,
    content_analysis: &ContentAnalysis,
) -> bool {
    // Check if file extension suggests it's a test file
    if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
        match extension {
            "test" | "spec" | "it" | "re" => return true,
            _ => {}
        }
    }

    // Check if filename contains test indicators
    if let Some(filename) = file_path.file_stem().and_then(|s| s.to_str()) {
        let filename_lower = filename.to_lowercase();
        if filename_lower.contains("test") ||
           filename_lower.contains("spec") ||
           filename_lower.contains("validate") ||
           filename_lower.contains("check") {
            return true;
        }
    }

    // Check if language patterns classify it as a test file
    if let Some(patterns) = language_patterns.get(detected_language) {
        let path_str = file_path.to_string_lossy();
        for pattern in patterns {
            if pattern.path_pattern.is_match(&path_str) {
                return true;
            }
        }
    }

    // Check content analysis
    if content_analysis.classification_confidence > 0.7 {
        return true;
    }

    false
}

/// Classify a file as a test case
pub fn classify_test_file(
    file_path: &std::path::Path,
    language: &str,
    language_patterns: &HashMap<String, Vec<LanguagePattern>>,
) -> (TestType, TestComplexity) {
    let filename_lower = file_path.file_stem()
        .and_then(|s| s.to_string())
        .unwrap_or_default()
        .to_lowercase();

    // Use language-specific patterns first
    if let Some(patterns) = language_patterns.get(language) {
        let path_str = file_path.to_string_lossy();
        for pattern in patterns {
            if pattern.path_pattern.is_match(&path_str) {
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
        return (TestType::RuleValidation, TestComplexity::Medium);
    }

    // Default classification
    (TestType::RuleValidation, TestComplexity::Medium)
}

/// Analysis of file content for classification
#[derive(Debug, Clone, Default)]
pub struct ContentAnalysis {
    /// Lines of code
    pub line_count: usize,
    /// Dependencies detected (imports, includes, etc.)
    pub dependencies: Vec<String>,
    /// Test framework usage detected
    pub frameworks: Vec<String>,
    /// Test annotations or decorators
    pub test_annotations: Vec<String>,
    /// Keywords indicating test type
    pub test_keywords: Vec<String>,
    /// Confidence in classification (0.0-1.0)
    pub classification_confidence: f64,
}

/// Calculate confidence in classification
pub fn calculate_classification_confidence(
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
