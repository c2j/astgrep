//! Cross-platform path handling utilities

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use tracing::debug;

/// Cross-platform path normalizer and validator
#[derive(Clone)]
pub struct PathHandler {
    current_platform: Platform,
    preserve_case: bool,
    normalize_slashes: bool,
    max_path_length: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Unix,
    Unknown,
}

impl PathHandler {
    pub fn new() -> Self {
        let current_platform = detect_platform();
        let (preserve_case, normalize_slashes, max_path_length) = platform_defaults(&current_platform);

        Self {
            current_platform,
            preserve_case,
            normalize_slashes,
            max_path_length,
        }
    }

    /// Create a path handler with custom configuration
    pub fn with_config(preserve_case: bool, normalize_slashes: bool, max_path_length: usize) -> Self {
        Self {
            current_platform: detect_platform(),
            preserve_case,
            normalize_slashes,
            max_path_length,
        }
    }

    /// Normalize a path for the current platform
    pub fn normalize_path(&self, path: &Path) -> Result<PathBuf> {
        let mut normalized = path.to_path_buf();

        // Convert slashes if needed
        if self.normalize_slashes {
            normalized = self.convert_slashes(&normalized);
        }

        // Handle case sensitivity
        if !self.preserve_case && (self.current_platform == Platform::Windows) {
            normalized = self.normalize_case(&normalized);
        }

        // Resolve parent directory references
        normalized = self.resolve_dots(&normalized);

        // Validate path length
        self.validate_path_length(&normalized)?;

        // Clean up path components
        normalized = self.clean_path(&normalized);

        debug!("Normalized path: {:?} -> {:?}", path, normalized);
        Ok(normalized)
    }

    /// Convert path separators to platform-specific format
    fn convert_slashes(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let separator = std::path::MAIN_SEPARATOR.to_string();

        // Replace both forward and backward slashes with platform separator
        let normalized = path_str
            .replace('/', &separator)
            .replace('\\', &separator);

        PathBuf::from(normalized)
    }

    /// Normalize case for case-insensitive platforms
    fn normalize_case(&self, path: &Path) -> PathBuf {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::Normal(s) => {
                    let s = s.to_string_lossy().to_lowercase();
                    result.push(s);
                }
                other => result.push(other.as_os_str()),
            }
        }
        result
    }

    /// Resolve "." and ".." components
    fn resolve_dots(&self, path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    /// Clean up path components (remove redundant separators, empty components)
    fn clean_path(&self, path: &Path) -> PathBuf {
        let components: Vec<_> = path.components()
            .filter(|comp| {
                !matches!(comp, std::path::Component::CurDir) // Skip "."
            })
            .collect();

        let mut result = PathBuf::new();
        for comp in components {
            result.push(comp);
        }

        result
    }

    /// Validate that path doesn't exceed platform limits
    fn validate_path_length(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();

        if path_str.len() > self.max_path_length {
            return Err(anyhow::anyhow!(
                "Path length {} exceeds maximum allowed length {} for platform {:?}",
                path_str.len(),
                self.max_path_length,
                self.current_platform
            ));
        }

        // Additional Windows-specific checks
        if self.current_platform == Platform::Windows {
            self.validate_windows_path(path)?;
        }

        Ok(())
    }

    /// Windows-specific path validation
    fn validate_windows_path(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Check for reserved characters
        let reserved_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for (i, ch) in path_str.chars().enumerate() {
            if reserved_chars.contains(&ch) {
                // Allow colon in drive letter (e.g., "C:")
                if ch == ':' && i == 1 && path_str.chars().nth(0).map_or(false, |c| c.is_ascii_alphabetic()) {
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "Path contains reserved character '{}' at position {}: {}",
                    ch, i, path_str
                ));
            }
        }

        // Check for reserved device names
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
        ];

        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy();
            let name_without_ext = if let Some(dot_pos) = name.find('.') {
                &name[..dot_pos]
            } else {
                &name
            };

            if reserved_names.contains(&name_without_ext.to_uppercase().as_str()) {
                return Err(anyhow::anyhow!(
                    "Path contains reserved device name: {}",
                    name_without_ext
                ));
            }
        }

        Ok(())
    }

    /// Check if a path is valid for the current platform
    pub fn is_valid_path(&self, path: &Path) -> bool {
        self.normalize_path(path).is_ok()
    }

    /// Get relative path from base to target, cross-platform
    pub fn relative_path(&self, base: &Path, target: &Path) -> Result<PathBuf> {
        let normalized_base = self.normalize_path(base)?;
        let normalized_target = self.normalize_path(target)?;

        // Use pathdiff crate if available, otherwise implement basic relative path logic
        pathdiff::diff_paths(&normalized_target, &normalized_base)
            .ok_or_else(|| anyhow::anyhow!("Cannot compute relative path"))
    }

    /// Join path components with proper separator handling
    pub fn join_paths(&self, components: &[&str]) -> PathBuf {
        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }
        self.normalize_path(&result).unwrap_or(result)
    }

    /// Convert path to URI format for cross-platform representation
    pub fn path_to_uri(&self, path: &Path) -> Result<String> {
        let absolute = fs::canonicalize(path)
            .with_context(|| format!("Cannot canonicalize path: {:?}", path))?;

        let path_str = absolute.to_string_lossy();

        match self.current_platform {
            Platform::Windows => {
                if path_str.starts_with("\\\\") {
                    // UNC path
                    Ok(format!("file:{}", path_str.replace('\\', "/")))
                } else {
                    // Regular Windows path
                    Ok(format!("file:/{}", path_str.replace('\\', "/")))
                }
            }
            _ => {
                Ok(format!("file://{}", path_str))
            }
        }
    }

    /// Convert URI back to local path
    pub fn uri_to_path(&self, uri: &str) -> Result<PathBuf> {
        if !uri.starts_with("file:") {
            return Err(anyhow::anyhow!("Invalid URI scheme: {}", uri));
        }

        let path_part = uri.strip_prefix("file:").unwrap();

        let path_str = match self.current_platform {
            Platform::Windows => {
                if path_part.starts_with("//") {
                    // UNC path or absolute path with leading slash
                    path_part[2..].replace('/', "\\")
                } else if path_part.starts_with('/') && path_part.len() > 3 && path_part.chars().nth(1) == Some(':') {
                    // Convert /C:/path to C:\path
                    path_part[1..].replace('/', "\\")
                } else {
                    path_part.replace('/', "\\")
                }
            }
            _ => {
                path_part.to_string()
            }
        };

        Ok(PathBuf::from(path_str))
    }

    /// Get platform-specific temporary directory
    pub fn temp_directory(&self) -> Result<PathBuf> {
        match env::temp_dir().to_str() {
            Some(path) => Ok(self.normalize_path(Path::new(path))?),
            None => Err(anyhow::anyhow!("Cannot determine temporary directory")),
        }
    }

    /// Get platform-specific home directory
    pub fn home_directory(&self) -> Result<PathBuf> {
        match dirs::home_dir() {
            Some(path) => Ok(self.normalize_path(&path)?),
            None => Err(anyhow::anyhow!("Cannot determine home directory")),
        }
    }

    /// Check if path exists and is accessible
    pub fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// Check if path is a directory
    pub fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    /// Check if path is a file
    pub fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    /// Create directory with proper permissions
    pub fn create_directory(&self, path: &Path) -> Result<()> {
        let normalized_path = self.normalize_path(path)?;
        fs::create_dir_all(&normalized_path)
            .with_context(|| format!("Failed to create directory: {:?}", normalized_path))
    }

    /// Get file extension in platform-independent way
    pub fn get_extension(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
    }

    /// Get file name without extension
    pub fn get_stem(&self, path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
    }

    /// Check if file has executable extension for the current platform
    pub fn is_executable_file(&self, path: &Path) -> bool {
        if let Some(ext) = self.get_extension(path) {
            match self.current_platform {
                Platform::Windows => {
                    // Windows executable extensions
                    matches!(ext.as_str(), "exe" | "bat" | "cmd" | "ps1" | "com" | "scr")
                }
                _ => {
                    // On Unix-like systems, check file permissions instead
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        path.metadata()
                            .map(|m| m.permissions().mode() & 0o111 != 0)
                            .unwrap_or(false)
                    }
                    #[cfg(not(unix))]
                    {
                        false
                    }
                }
            }
        } else {
            false
        }
    }
}

/// Detect the current platform
fn detect_platform() -> Platform {
    match env::consts::OS {
        "windows" => Platform::Windows,
        "macos" => Platform::MacOS,
        "linux" => Platform::Linux,
        "freebsd" | "netbsd" | "openbsd" | "dragonfly" => Platform::Unix,
        _ => Platform::Unknown,
    }
}

/// Get platform-specific defaults
fn platform_defaults(platform: &Platform) -> (bool, bool, usize) {
    match platform {
        Platform::Windows => (
            false,  // Don't preserve case (case-insensitive)
            true,   // Normalize slashes
            260,    // Traditional Windows path limit (though extended paths support longer)
        ),
        Platform::MacOS => (
            true,   // Preserve case (though HFS+ is case-insensitive)
            true,   // Normalize slashes for consistency
            1024,   // macOS has higher path limits
        ),
        Platform::Linux => (
            true,   // Preserve case (case-sensitive)
            true,   // Normalize slashes for consistency
            4096,   // Linux typically supports 4096+
        ),
        Platform::Unix => (
            true,   // Preserve case (case-sensitive)
            true,   // Normalize slashes for consistency
            1024,   // Conservative estimate
        ),
        Platform::Unknown => (
            true,   // Default to preserving case
            true,   // Normalize slashes
            255,    // Conservative limit
        ),
    }
}

/// Create a path handler for a specific target platform
pub fn create_path_handler_for_platform(target_platform: Platform) -> PathHandler {
    let (preserve_case, normalize_slashes, max_path_length) = platform_defaults(&target_platform);

    PathHandler {
        current_platform: target_platform,
        preserve_case,
        normalize_slashes,
        max_path_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_platform_detection() {
        let platform = detect_platform();
        // Should not panic on current platform
        assert!(matches!(platform, Platform::Windows | Platform::MacOS | Platform::Linux | Platform::Unix));
    }

    #[test]
    fn test_path_normalization() {
        let handler = PathHandler::new();

        #[cfg(windows)]
        {
            let path = Path::new("C:/Users/test/../admin/./file.txt");
            let normalized = handler.normalize_path(path).unwrap();
            assert!(normalized.to_string_lossy().contains("admin"));
            assert!(!normalized.to_string_lossy().contains(".."));
            assert!(!normalized.to_string_lossy().contains("."));
        }

        #[cfg(unix)]
        {
            let path = Path::new("/home/user/../admin/./file.txt");
            let normalized = handler.normalize_path(path).unwrap();
            assert!(normalized.to_string_lossy().contains("admin"));
        }
    }

    #[test]
    fn test_path_validation() {
        let handler = PathHandler::new();

        // Valid path
        let valid_path = Path::new("test_file.txt");
        assert!(handler.is_valid_path(valid_path));

        #[cfg(windows)]
        {
            // Invalid path with reserved character
            let invalid_path = Path::new("test|file.txt");
            assert!(!handler.is_valid_path(invalid_path));
        }
    }

    #[test]
    fn test_path_operations() {
        let handler = PathHandler::new();

        let path = Path::new("/test/directory/file.txt");

        assert_eq!(handler.get_extension(path), Some("txt".to_string()));
        assert_eq!(handler.get_stem(path), Some("file".to_string()));

        let components = ["test", "directory", "file.txt"];
        let joined = handler.join_paths(&components);
        assert!(joined.to_string_lossy().contains("test"));
        assert!(joined.to_string_lossy().contains("directory"));
        assert!(joined.to_string_lossy().contains("file.txt"));
    }

    #[test]
    fn test_directory_creation() {
        let handler = PathHandler::new();
        let temp_dir = tempdir().unwrap();
        let test_path = temp_dir.path().join("test").join("subdirectory");

        let result = handler.create_directory(&test_path);
        assert!(result.is_ok());
        assert!(test_path.exists());
        assert!(test_path.is_dir());
    }

    #[test]
    fn test_uri_conversion() {
        let handler = PathHandler::new();

        #[cfg(windows)]
        {
            let path = Path::new("C:\\test\\file.txt");
            let uri = handler.path_to_uri(path).unwrap();
            assert!(uri.starts_with("file:"));

            let converted_back = handler.uri_to_path(&uri).unwrap();
            assert_eq!(converted_back, handler.normalize_path(path).unwrap());
        }

        #[cfg(unix)]
        {
            let path = Path::new("/test/file.txt");
            let uri = handler.path_to_uri(path).unwrap();
            assert!(uri.starts_with("file://"));

            let converted_back = handler.uri_to_path(&uri).unwrap();
            assert_eq!(converted_back, handler.normalize_path(path).unwrap());
        }
    }

    #[test]
    fn test_executable_detection() {
        let handler = PathHandler::new();

        #[cfg(windows)]
        {
            assert!(handler.is_executable_file(Path::new("test.exe")));
            assert!(handler.is_executable_file(Path::new("test.bat")));
            assert!(!handler.is_executable_file(Path::new("test.txt")));
        }

        #[cfg(unix)]
        {
            // This test depends on file permissions, so we just check that it doesn't panic
            let _ = handler.is_executable_file(Path::new("test.sh"));
            let _ = handler.is_executable_file(Path::new("test.txt"));
        }
    }
}