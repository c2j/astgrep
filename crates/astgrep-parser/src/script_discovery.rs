//! Script discovery module for finding and analyzing test scripts

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, error, info, warn};
use walkdir::{DirEntry, WalkDir};

use astgrep_core::{models::test_asset::ScriptType, Language};

/// Configuration for script discovery operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Base directories to search for scripts
    pub search_paths: Vec<PathBuf>,
    /// File patterns to include (glob patterns)
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Maximum recursion depth
    pub max_depth: Option<usize>,
    /// Include hidden files
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Minimum file size in bytes
    pub min_file_size: u64,
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Platform filters to apply
    pub platform_filters: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("tests"),
                PathBuf::from("test"),
                PathBuf::from("scripts"),
                PathBuf::from("tools"),
            ],
            include_patterns: vec![
                "*.sh".to_string(),
                "*.bash".to_string(),
                "*.zsh".to_string(),
                "*.py".to_string(),
                "*.rb".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
                "*.java".to_string(),
                "*.php".to_string(),
                "*.bat".to_string(),
                "*.cmd".to_string(),
                "*.ps1".to_string(),
                "validate".to_string(),
                "test".to_string(),
                "run".to_string(),
                "check".to_string(),
            ],
            exclude_patterns: vec![
                ".*".to_string(),
                "*.tmp".to_string(),
                "*.bak".to_string(),
                "*.orig".to_string(),
                "*~".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".git".to_string(),
            ],
            max_depth: Some(10),
            include_hidden: false,
            follow_symlinks: false,
            min_file_size: 0,
            max_file_size: 10 * 1024 * 1024, // 10MB
            platform_filters: Vec::new(),
        }
    }
}

/// Discovered script information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredScript {
    pub path: PathBuf,
    pub name: String,
    pub script_type: ScriptType,
    pub language: Option<Language>,
    pub platforms: Vec<String>,
    pub file_size: u64,
    pub executable: bool,
    pub shebang: Option<String>,
    pub metadata: ScriptMetadata,
}

/// Additional metadata for discovered scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    pub created_at: Option<SystemTime>,
    pub modified_at: Option<SystemTime>,
    pub accessed_at: Option<SystemTime>,
    pub permissions: Option<u32>,
    pub file_hash: Option<String>,
    pub dependencies: Vec<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
}

/// Script discovery results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResults {
    pub total_scripts_found: usize,
    pub scripts_by_type: HashMap<ScriptType, Vec<DiscoveredScript>>,
    pub scripts_by_language: HashMap<String, Vec<DiscoveredScript>>,
    pub scripts_by_platform: HashMap<String, Vec<DiscoveredScript>>,
    pub discovery_errors: Vec<String>,
    pub config: DiscoveryConfig,
    pub discovery_time: SystemTime,
}

/// Script discovery engine
pub struct ScriptDiscovery {
    config: DiscoveryConfig,
}

impl ScriptDiscovery {
    /// Create a new script discovery engine with default configuration
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
        }
    }

    /// Create a script discovery engine with custom configuration
    pub fn with_config(config: DiscoveryConfig) -> Self {
        Self { config }
    }

    /// Discover all scripts according to the configuration
    pub async fn discover_scripts(&self) -> Result<DiscoveryResults> {
        info!(
            "Starting script discovery with {} search paths",
            self.config.search_paths.len()
        );

        let mut results = DiscoveryResults {
            total_scripts_found: 0,
            scripts_by_type: HashMap::new(),
            scripts_by_language: HashMap::new(),
            scripts_by_platform: HashMap::new(),
            discovery_errors: Vec::new(),
            config: self.config.clone(),
            discovery_time: SystemTime::now(),
        };

        // Process each search path
        for search_path in &self.config.search_paths {
            if !search_path.exists() {
                warn!("Search path does not exist: {:?}", search_path);
                continue;
            }

            debug!("Processing search path: {:?}", search_path);
            match self.discover_in_path(search_path).await {
                Ok(path_scripts) => {
                    results.total_scripts_found += path_scripts.len();

                    for script in path_scripts {
                        // Categorize by script type
                        results
                            .scripts_by_type
                            .entry(script.script_type.clone())
                            .or_default()
                            .push(script.clone());

                        // Categorize by language
                        if let Some(lang) = &script.language {
                            let lang_str = format!("{:?}", lang);
                            results
                                .scripts_by_language
                                .entry(lang_str)
                                .or_default()
                                .push(script.clone());
                        }

                        // Categorize by platform
                        for platform in &script.platforms {
                            results
                                .scripts_by_platform
                                .entry(platform.clone())
                                .or_default()
                                .push(script.clone());
                        }
                    }
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to discover scripts in {:?}: {}", search_path, e);
                    error!("{}", error_msg);
                    results.discovery_errors.push(error_msg);
                }
            }
        }

        info!(
            "Script discovery completed: {} scripts found",
            results.total_scripts_found
        );
        Ok(results)
    }

    /// Discover scripts in a specific directory path
    async fn discover_in_path(&self, search_path: &Path) -> Result<Vec<DiscoveredScript>> {
        let mut scripts = Vec::new();

        // Build walkdir iterator
        let walkdir = WalkDir::new(search_path)
            .max_depth(self.config.max_depth.unwrap_or(usize::MAX))
            .follow_links(self.config.follow_symlinks)
            .into_iter();

        for entry in walkdir {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    if let Err(e) = self.process_file_entry(&entry, &mut scripts).await {
                        debug!("Error processing file {:?}: {}", entry.path(), e);
                    }
                }
                Err(e) => {
                    debug!("Error walking directory: {}", e);
                }
                _ => {} // Skip directories and other entries
            }
        }

        Ok(scripts)
    }

    /// Process a single file entry to determine if it's a script
    async fn process_file_entry(
        &self,
        entry: &DirEntry,
        scripts: &mut Vec<DiscoveredScript>,
    ) -> Result<()> {
        let path = entry.path();

        // Check if file should be included based on patterns
        if !self.should_include_file(path) {
            return Ok(());
        }

        // Get file metadata
        let metadata = entry.metadata().context("Failed to get file metadata")?;
        let file_size = metadata.len();

        // Check file size limits
        if file_size < self.config.min_file_size || file_size > self.config.max_file_size {
            return Ok(());
        }

        // Check if file is executable or has a shebang
        let is_executable = self.is_executable(&metadata);
        let shebang = self.read_shebang(path).await?;

        // Must be either executable or have a shebang to be considered a script
        if !is_executable && shebang.is_none() {
            return Ok(());
        }

        // Analyze the script
        let discovered_script = self.analyze_script(path, is_executable, shebang).await?;
        scripts.push(discovered_script);

        Ok(())
    }

    /// Check if a file should be included based on include/exclude patterns
    fn should_include_file(&self, path: &Path) -> bool {
        // Check exclude patterns first
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(path, pattern) {
                return false;
            }
        }

        // Check include patterns
        for pattern in &self.config.include_patterns {
            if self.matches_pattern(path, pattern) {
                return true;
            }
        }

        // Default to exclude if no patterns match
        false
    }

    /// Check if a path matches a glob pattern
    fn matches_pattern(&self, path: &Path, pattern: &str) -> bool {
        let path_str = path.to_string_lossy();
        match glob::glob(pattern) {
            Ok(paths) => {
                for p in paths.flatten() {
                    let p_str = p.to_string_lossy();
                    if path.starts_with(&p) || path_str.contains(&*p_str) {
                        return true;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Check if file metadata indicates it's executable
    fn is_executable(&self, metadata: &std::fs::Metadata) -> bool {
        #[cfg(unix)]
        {
            #[cfg(unix)]
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            // On Windows, check file extension for executable types
            false
        }
    }

    /// Read shebang from file
    async fn read_shebang(&self, path: &Path) -> Result<Option<String>> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(path)
            .await
            .with_context(|| format!("Failed to open file: {:?}", path))?;

        let mut buffer = [0; 256];
        let bytes_read = file
            .read(&mut buffer)
            .await
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        if bytes_read >= 2 && buffer[0] == b'#' && buffer[1] == b'!' {
            let shebang_len = buffer
                .iter()
                .position(|&b| b == b'\n')
                .unwrap_or(bytes_read);

            Ok(Some(
                String::from_utf8_lossy(&buffer[2..shebang_len])
                    .trim()
                    .to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    /// Analyze a script file to extract detailed information
    async fn analyze_script(
        &self,
        path: &Path,
        is_executable: bool,
        shebang: Option<String>,
    ) -> Result<DiscoveredScript> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Determine script type
        let script_type = self.determine_script_type(&file_name, &shebang);

        // Detect language from shebang or file extension
        let language = self
            .detect_language_from_shebang(&shebang)
            .or_else(|| self.detect_language_from_extension(path));

        // Get file metadata
        let metadata =
            fs::metadata(path).with_context(|| format!("Failed to get metadata for {:?}", path))?;

        let file_size = metadata.len();
        let created_at = metadata.created().ok();
        let modified_at = metadata.modified().ok();
        let accessed_at = metadata.accessed().ok();

        #[cfg(unix)]
        let permissions = Some(metadata.permissions().mode());
        #[cfg(not(unix))]
        let permissions = None;

        // Determine supported platforms
        let platforms = self.determine_platforms(&shebang, &language);

        // Read file content to extract additional metadata
        let script_metadata = ScriptMetadata {
            created_at,
            modified_at,
            accessed_at,
            permissions,
            file_hash: self.calculate_file_hash(path).await.ok(),
            dependencies: self.extract_dependencies(path).await?,
            description: self.extract_description(path).await.ok().flatten(),
            version: self.extract_version(path).await.ok().flatten(),
            author: self.extract_author(path).await.ok().flatten(),
        };

        Ok(DiscoveredScript {
            path: path.to_path_buf(),
            name: file_name,
            script_type,
            language,
            platforms,
            file_size,
            executable: is_executable,
            shebang,
            metadata: script_metadata,
        })
    }

    /// Determine script type based on filename and shebang
    fn determine_script_type(&self, filename: &str, _shebang: &Option<String>) -> ScriptType {
        let filename_lower = filename.to_lowercase();

        // Check for validation scripts
        if filename_lower.contains("validate") || filename_lower.contains("check") {
            return ScriptType::Validator;
        }

        // Check for runner scripts
        if filename_lower.contains("run")
            || filename_lower.contains("execute")
            || filename_lower.contains("test")
        {
            return ScriptType::Runner;
        }

        // Check for CI integration scripts
        if filename_lower.contains("ci")
            || filename_lower.contains("build")
            || filename_lower.contains("deploy")
        {
            return ScriptType::CiIntegration;
        }

        // Default to utility for other scripts
        ScriptType::Utility
    }

    /// Detect language from shebang
    fn detect_language_from_shebang(&self, shebang: &Option<String>) -> Option<Language> {
        let shebang = shebang.as_ref()?;

        if shebang.contains("bash") || shebang.contains("sh") {
            Some(Language::Bash)
        } else if shebang.contains("python") || shebang.contains("python3") {
            Some(Language::Python)
        } else if shebang.contains("node") {
            Some(Language::JavaScript)
        } else {
            None
        }
    }

    /// Detect language from file extension
    fn detect_language_from_extension(&self, path: &Path) -> Option<Language> {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "py" => Some(Language::Python),
                "js" | "jsx" => Some(Language::JavaScript),
                "ts" | "tsx" => Some(Language::JavaScript),
                "sh" | "bash" | "zsh" => Some(Language::Bash),
                "java" => Some(Language::Java),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Determine supported platforms based on shebang and language
    fn determine_platforms(
        &self,
        shebang: &Option<String>,
        language: &Option<Language>,
    ) -> Vec<String> {
        let mut platforms = Vec::new();

        // Default platforms based on language
        match language {
            Some(Language::Bash)
            | Some(Language::Python)
            | Some(Language::Java)
            | Some(Language::JavaScript) => {
                platforms.push("Linux".to_string());
                platforms.push("macOS".to_string());

                // Windows support for certain languages
                match language {
                    Some(Language::Python) | Some(Language::Java) | Some(Language::JavaScript) => {
                        platforms.push("Windows".to_string());
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Check shebang for Windows-specific interpreters
        if let Some(shebang) = shebang {
            if shebang.contains("cmd.exe")
                || shebang.contains("powershell")
                || shebang.contains("pwsh")
            {
                platforms.retain(|p| p != "Linux" && p != "macOS");
                platforms.push("Windows".to_string());
            }
        }

        // Apply platform filters if configured
        if !self.config.platform_filters.is_empty() {
            platforms.retain(|p| self.config.platform_filters.contains(p));
        }

        if platforms.is_empty() {
            vec!["All".to_string()]
        } else {
            platforms
        }
    }

    /// Calculate file hash for integrity checking
    async fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Extract script dependencies from content
    async fn extract_dependencies(&self, path: &Path) -> Result<Vec<String>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut dependencies = Vec::new();

        // Look for common dependency patterns
        for line in content.lines() {
            let trimmed = line.trim();

            // Shell script source/include patterns
            if trimmed.starts_with("source ") || trimmed.starts_with(". ") {
                if let Some(dep) = trimmed.split_whitespace().nth(1) {
                    dependencies.push(dep.to_string());
                }
            }

            // Python import patterns
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                // Extract module names from imports
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(parts[1].to_string());
                }
            }

            // Node.js require patterns
            if trimmed.contains("require(") {
                // Simple regex-like extraction for require statements
                if let Some(start) = trimmed.find("require(") {
                    let rest = &trimmed[start + 8..];
                    if let Some(end) = rest.find(')') {
                        let dep = &rest[..end];
                        dependencies.push(dep.trim_matches(['"', '\'']).to_string());
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// Extract script description from comments
    async fn extract_description(&self, path: &Path) -> Result<Option<String>> {
        let content = tokio::fs::read_to_string(path).await?;

        for line in content.lines() {
            let trimmed = line.trim();

            // Look for common comment patterns that might contain descriptions
            if trimmed.starts_with("# ") || trimmed.starts_with("// ") || trimmed.starts_with("/* ")
            {
                let comment = trimmed
                    .split_whitespace()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" ");

                // Heuristic: if it looks like a description (contains keywords), return it
                if comment.len() > 10
                    && (comment.to_lowercase().contains("script")
                        || comment.to_lowercase().contains("test")
                        || comment.to_lowercase().contains("validate")
                        || comment.to_lowercase().contains("run"))
                {
                    return Ok(Some(comment));
                }
            }
        }

        Ok(None)
    }

    /// Extract version information from script
    async fn extract_version(&self, path: &Path) -> Result<Option<String>> {
        let content = tokio::fs::read_to_string(path).await?;

        for line in content.lines() {
            let trimmed = line.trim().to_lowercase();

            // Look for version patterns
            if trimmed.contains("version") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let version_part = &trimmed[eq_pos + 1..].trim();
                    if !version_part.is_empty() {
                        return Ok(Some(version_part.trim_matches(['"', '\'']).to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract author information from script
    async fn extract_author(&self, path: &Path) -> Result<Option<String>> {
        let content = tokio::fs::read_to_string(path).await?;

        for line in content.lines() {
            let trimmed = line.trim().to_lowercase();

            // Look for author patterns
            if trimmed.contains("author") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let author_part = &trimmed[eq_pos + 1..].trim();
                    if !author_part.is_empty() {
                        return Ok(Some(author_part.trim_matches(['"', '\'']).to_string()));
                    }
                }
            }
        }

        Ok(None)
    }
}

impl Default for ScriptDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_script_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert!(!config.search_paths.is_empty());
        assert!(!config.include_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert_eq!(config.max_depth, Some(10));
        assert!(!config.include_hidden);
        assert!(!config.follow_symlinks);
    }

    #[tokio::test]
    async fn test_determine_script_type() {
        let discovery = ScriptDiscovery::new();

        assert!(matches!(
            discovery.determine_script_type("validate_tests.sh", &None),
            ScriptType::Validator
        ));
        assert!(matches!(
            discovery.determine_script_type("run_all_tests.py", &None),
            ScriptType::Runner
        ));
        assert!(matches!(
            discovery.determine_script_type("ci_build.sh", &None),
            ScriptType::CiIntegration
        ));
        assert!(matches!(
            discovery.determine_script_type("helper.sh", &None),
            ScriptType::Utility
        ));
    }

    #[tokio::test]
    async fn test_detect_language_from_shebang() {
        let discovery = ScriptDiscovery::new();

        assert_eq!(
            discovery.detect_language_from_shebang(&Some("/bin/bash".to_string())),
            Some(Language::Bash)
        );
        assert_eq!(
            discovery.detect_language_from_shebang(&Some("/usr/bin/python3".to_string())),
            Some(Language::Python)
        );
        assert_eq!(
            discovery.detect_language_from_shebang(&Some("/usr/bin/node".to_string())),
            Some(Language::JavaScript)
        );
        assert_eq!(discovery.detect_language_from_shebang(&None), None);
    }

    #[tokio::test]
    async fn test_script_file_analysis() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test_script.sh");

        // Create a test script
        fs::write(
            &script_path,
            "#!/bin/bash\n# Test script for validation\necho 'Hello World'\n",
        )?;

        let discovery = ScriptDiscovery::new();
        let discovered = discovery
            .analyze_script(&script_path, true, Some("/bin/bash".to_string()))
            .await?;

        assert_eq!(discovered.name, "test_script.sh");
        assert!(discovered.executable);
        assert_eq!(discovered.shebang, Some("/bin/bash".to_string()));
        assert!(matches!(
            discovered.script_type,
            ScriptType::Runner
        ));

        Ok(())
    }
}
