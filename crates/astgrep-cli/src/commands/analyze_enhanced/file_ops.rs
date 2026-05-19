//! File collection and filtering operations

use super::types::glob_match;
use crate::EnhancedAnalysisConfig;
use anyhow::Result;
use std::path::PathBuf;
use tracing::warn;

/// Collect target files for analysis
pub async fn collect_target_files(config: &EnhancedAnalysisConfig) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for target in &config.target_paths {
        if target.is_file() {
            files.push(target.clone());
        } else if target.is_dir() {
            collect_files_from_directory(target, &mut files, config)?;
        } else {
            warn!("Target path does not exist: {}", target.display());
        }
    }

    Ok(files)
}

fn collect_files_from_directory(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
    config: &EnhancedAnalysisConfig,
) -> Result<()> {
    use std::fs;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_from_directory(&path, files, config)?;
        } else if should_include_file(&path, config) {
            files.push(path);
        }
    }

    Ok(())
}

fn should_include_file(path: &PathBuf, config: &EnhancedAnalysisConfig) -> bool {
    let path_str = path.to_string_lossy();

    // Check include patterns
    if !config.include_patterns.is_empty() {
        let included = config
            .include_patterns
            .iter()
            .any(|pattern| glob_match(pattern, &path_str));
        if !included {
            return false;
        }
    }

    // Check exclude patterns
    for pattern in &config.exclude_patterns {
        if glob_match(pattern, &path_str) {
            return false;
        }
    }

    // Check if file extension matches supported languages (including extra preprocess source languages)
    if let Some(extension) = path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        config.languages.iter().any(|lang| match lang {
            astgrep_core::Language::Java => ext_str == "java",
            astgrep_core::Language::JavaScript => {
                ext_str == "js" || ext_str == "jsx" || ext_str == "ts" || ext_str == "tsx"
            }
            astgrep_core::Language::Python => ext_str == "py",
            astgrep_core::Language::Sql => ext_str == "sql",
            astgrep_core::Language::Bash => ext_str == "sh" || ext_str == "bash",
            astgrep_core::Language::Xml => {
                ext_str == "xml"
                    || ext_str == "xsd"
                    || ext_str == "xsl"
                    || ext_str == "xslt"
                    || ext_str == "svg"
                    || ext_str == "pom"
            }
        })
    } else {
        false
    }
}
