//! Language-specific content analysis
//!
//! This module provides content analysis for different programming languages
//! to extract dependencies, frameworks, and test annotations.

use super::detection::ContentAnalysis;
use std::path::Path;

/// Analyze Java file content
pub fn analyze_java_content(
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
pub fn analyze_python_content(
    lines: &[&str],
    dependencies: &mut Vec<String>,
    frameworks: &mut Vec<String>,
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
pub fn analyze_javascript_content(
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
pub fn analyze_sql_content(
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

/// Analyze file content for additional classification
pub fn analyze_content(
    file_path: &Path,
    content: &str,
) -> Result<ContentAnalysis, anyhow::Error> {
    let lines: Vec<&str> = content.lines().collect();
    let mut dependencies = Vec::new();
    let mut frameworks = Vec::new();
    let mut test_annotations = Vec::new();
    let mut test_keywords = Vec::new();

    // Java-specific analysis
    if file_path.extension().and_then(|e| e.to_str()) == Some("java") {
        analyze_java_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
    }

    // Python-specific analysis
    if file_path.extension().and_then(|e| e.to_str()) == Some("py") {
        analyze_python_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
    }

    // JavaScript/TypeScript analysis
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        if ext == "js" || ext == "ts" || ext == "jsx" || ext == "tsx" {
            analyze_javascript_content(&lines, &mut dependencies, &mut frameworks, &mut test_annotations);
        }
    }

    // SQL analysis
    if file_path.extension().and_then(|e| e.to_str()) == Some("sql") {
        analyze_sql_content(&lines, &mut dependencies, &mut test_annotations);
    }

    // Generic test keyword detection
    for line in &lines {
        let line_lower = line.to_lowercase();
        if line_lower.contains("test") || line_lower.contains("assert") {
            test_keywords.push(line.trim().to_string());
        }
    }

    let line_count = lines.len();
    let classification_confidence = super::detection::calculate_classification_confidence(
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
