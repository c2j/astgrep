//! File extension mapping and language configuration
//!
//! This module handles file extension detection and language mapping
//! for test file classification.

use astgrep_core::models::LanguageMapping;
use std::path::Path;

/// Detect language from file path using language mapping
pub fn detect_language(
    file_path: &Path,
    language_mapping: &LanguageMapping,
) -> String {
    language_mapping.detect_language(file_path, None)
}

/// Check if file extension matches a test file pattern
pub fn is_test_extension(file_path: &Path) -> bool {
    if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
        match extension {
            "test" | "spec" | "it" | "re" => return true,
            _ => {}
        }
    }
    false
}

/// Get file extension as string
pub fn get_extension(file_path: &Path) -> Option<String> {
    file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string())
}

/// Check if file name contains test indicators
pub fn has_test_indicator(file_path: &Path) -> bool {
    if let Some(filename) = file_path.file_stem().and_then(|s| s.to_str()) {
        let filename_lower = filename.to_lowercase();
        return filename_lower.contains("test") ||
               filename_lower.contains("spec") ||
               filename_lower.contains("validate") ||
               filename_lower.contains("check");
    }
    false
}
