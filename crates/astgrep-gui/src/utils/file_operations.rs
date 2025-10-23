//! File operation utilities

use std::path::PathBuf;
use anyhow::Result;

/// File operations helper
pub struct FileOperations;

impl FileOperations {
    /// Open a file dialog and return the selected path
    pub fn open_file_dialog(title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        
        dialog.pick_file()
    }
    
    /// Save a file dialog and return the selected path
    pub fn save_file_dialog(title: &str, filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }
        
        dialog.save_file()
    }
    
    /// Read file contents as string
    pub fn read_file(path: &PathBuf) -> Result<String> {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
    }
    
    /// Write string contents to file
    pub fn write_file(path: &PathBuf, contents: &str) -> Result<()> {
        std::fs::write(path, contents)
            .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", path.display(), e))
    }
    
    /// Get file extension
    pub fn get_extension(path: &PathBuf) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }
    
    /// Detect language from file extension
    pub fn detect_language(path: &PathBuf) -> Option<String> {
        match Self::get_extension(path)?.as_str() {
            "java" => Some("java".to_string()),
            "js" | "jsx" | "mjs" => Some("javascript".to_string()),
            "py" | "pyw" => Some("python".to_string()),
            "go" => Some("go".to_string()),
            "rs" => Some("rust".to_string()),
            "php" => Some("php".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "cxx" | "cc" | "hpp" => Some("cpp".to_string()),
            "cs" => Some("csharp".to_string()),
            "rb" => Some("ruby".to_string()),
            "kt" => Some("kotlin".to_string()),
            "swift" => Some("swift".to_string()),
            "ts" | "tsx" => Some("typescript".to_string()),
            _ => None,
        }
    }
    
    /// Get common file filters for different file types
    pub fn get_source_file_filters() -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("All Source Files", &["java", "js", "jsx", "py", "go", "rs", "php", "c", "cpp", "h", "hpp"]),
            ("Java Files", &["java"]),
            ("JavaScript Files", &["js", "jsx", "mjs"]),
            ("Python Files", &["py", "pyw"]),
            ("Go Files", &["go"]),
            ("Rust Files", &["rs"]),
            ("PHP Files", &["php"]),
            ("C/C++ Files", &["c", "cpp", "cxx", "cc", "h", "hpp"]),
            ("All Files", &["*"]),
        ]
    }
    
    /// Get rule file filters
    pub fn get_rule_file_filters() -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("YAML Files", &["yml", "yaml"]),
            ("JSON Files", &["json"]),
            ("All Files", &["*"]),
        ]
    }
    
    /// Get export file filters
    pub fn get_export_file_filters() -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("JSON Files", &["json"]),
            ("YAML Files", &["yml", "yaml"]),
            ("SARIF Files", &["sarif"]),
            ("XML Files", &["xml"]),
            ("Text Files", &["txt"]),
            ("All Files", &["*"]),
        ]
    }
    
    /// Create a backup of a file
    pub fn create_backup(path: &PathBuf) -> Result<PathBuf> {
        let backup_path = path.with_extension(
            format!("{}.backup", 
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or(""))
        );
        
        std::fs::copy(path, &backup_path)
            .map_err(|e| anyhow::anyhow!("Failed to create backup: {}", e))?;
        
        Ok(backup_path)
    }
    
    /// Check if a file exists and is readable
    pub fn is_readable(path: &PathBuf) -> bool {
        path.exists() && path.is_file() && 
        std::fs::metadata(path)
            .map(|metadata| !metadata.permissions().readonly())
            .unwrap_or(false)
    }
    
    /// Get file size in bytes
    pub fn get_file_size(path: &PathBuf) -> Result<u64> {
        std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .map_err(|e| anyhow::anyhow!("Failed to get file size: {}", e))
    }
    
    /// Format file size for display
    pub fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = size as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}
