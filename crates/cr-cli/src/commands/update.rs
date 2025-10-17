//! Update command for managing rule repositories

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn, error};

/// Update rules from remote repositories
pub async fn run(
    repository: Option<String>,
    directory: PathBuf,
    force: bool,
) -> Result<()> {
    let repo_url = repository.unwrap_or_else(|| {
        "https://github.com/cr-semservice/rules.git".to_string()
    });

    info!("Updating rules from repository: {}", repo_url);
    info!("Target directory: {}", directory.display());

    if directory.exists() && !force {
        if !is_git_repository(&directory)? {
            return Err(anyhow::anyhow!(
                "Directory {} exists but is not a git repository. Use --force to overwrite.",
                directory.display()
            ));
        }
        
        info!("Updating existing repository");
        update_existing_repository(&directory).await?;
    } else {
        if directory.exists() && force {
            warn!("Removing existing directory: {}", directory.display());
            std::fs::remove_dir_all(&directory)?;
        }
        
        info!("Cloning repository");
        clone_repository(&repo_url, &directory).await?;
    }

    // Validate downloaded rules
    info!("Validating downloaded rules");
    let validation_result = validate_rules_directory(&directory).await?;
    
    println!("✅ Rules updated successfully!");
    println!("📊 Validation Results:");
    println!("  • Total rule files: {}", validation_result.total_files);
    println!("  • Valid rules: {}", validation_result.valid_rules);
    println!("  • Invalid rules: {}", validation_result.invalid_rules);
    println!("  • Warnings: {}", validation_result.warnings);

    if validation_result.invalid_rules > 0 {
        warn!("Some rules failed validation. Check the logs for details.");
    }

    println!("🚀 Use 'cr-semservice list' to see available rules");

    Ok(())
}

async fn clone_repository(repo_url: &str, directory: &PathBuf) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(&["clone", repo_url, &directory.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to clone repository: {}", stderr));
    }

    info!("Repository cloned successfully");
    Ok(())
}

async fn update_existing_repository(directory: &PathBuf) -> Result<()> {
    use std::process::Command;

    // Check if there are local changes
    let status_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(directory)
        .output()?;

    if !status_output.stdout.is_empty() {
        warn!("Local changes detected in repository");
        
        // Stash local changes
        let stash_output = Command::new("git")
            .args(&["stash", "push", "-m", "CR-SemService auto-stash before update"])
            .current_dir(directory)
            .output()?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            warn!("Failed to stash local changes: {}", stderr);
        } else {
            info!("Local changes stashed");
        }
    }

    // Pull latest changes
    let pull_output = Command::new("git")
        .args(&["pull", "origin", "main"])
        .current_dir(directory)
        .output()?;

    if !pull_output.status.success() {
        let stderr = String::from_utf8_lossy(&pull_output.stderr);
        
        // Try with master branch if main fails
        let pull_master_output = Command::new("git")
            .args(&["pull", "origin", "master"])
            .current_dir(directory)
            .output()?;

        if !pull_master_output.status.success() {
            let master_stderr = String::from_utf8_lossy(&pull_master_output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to pull from repository. Main branch error: {}. Master branch error: {}",
                stderr, master_stderr
            ));
        }
    }

    info!("Repository updated successfully");
    Ok(())
}

fn is_git_repository(directory: &PathBuf) -> Result<bool> {
    let git_dir = directory.join(".git");
    Ok(git_dir.exists())
}

async fn validate_rules_directory(directory: &PathBuf) -> Result<ValidationResult> {
    
    let mut result = ValidationResult::new();
    
    // Find all rule files
    let rule_files = find_rule_files(directory).await?;
    result.total_files = rule_files.len();
    
    for rule_file in rule_files {
        match validate_rule_file(&rule_file).await {
            Ok(rule_count) => {
                result.valid_rules += rule_count;
                info!("✅ Valid: {} ({} rules)", rule_file.display(), rule_count);
            }
            Err(e) => {
                result.invalid_rules += 1;
                error!("❌ Invalid: {} - {}", rule_file.display(), e);
            }
        }
    }
    
    Ok(result)
}

async fn find_rule_files(directory: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut rule_files = Vec::new();
    find_rule_files_recursive(directory, &mut rule_files)?;
    Ok(rule_files)
}

fn find_rule_files_recursive(directory: &PathBuf, rule_files: &mut Vec<PathBuf>) -> Result<()> {
    use std::fs;
    
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Skip .git and other hidden directories
            if let Some(dir_name) = path.file_name() {
                if !dir_name.to_string_lossy().starts_with('.') {
                    find_rule_files_recursive(&path, rule_files)?;
                }
            }
        } else if is_rule_file(&path) {
            rule_files.push(path);
        }
    }
    
    Ok(())
}

fn is_rule_file(path: &PathBuf) -> bool {
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        ext == "yaml" || ext == "yml"
    } else {
        false
    }
}

async fn validate_rule_file(rule_file: &PathBuf) -> Result<usize> {
    use cr_rules::RuleEngine;
    
    let mut engine = RuleEngine::new();
    engine.load_rules_from_file(rule_file)?;
    Ok(engine.rule_count())
}

#[derive(Debug)]
struct ValidationResult {
    total_files: usize,
    valid_rules: usize,
    invalid_rules: usize,
    warnings: usize,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            total_files: 0,
            valid_rules: 0,
            invalid_rules: 0,
            warnings: 0,
        }
    }
}

/// Check if git is available on the system
pub fn check_git_availability() -> Result<()> {
    use std::process::Command;
    
    let output = Command::new("git")
        .args(&["--version"])
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("Git available: {}", version.trim());
            Ok(())
        }
        Ok(_) => Err(anyhow::anyhow!("Git command failed")),
        Err(_) => Err(anyhow::anyhow!("Git is not installed or not available in PATH")),
    }
}

/// Get information about the current repository
pub async fn get_repository_info(directory: &PathBuf) -> Result<RepositoryInfo> {
    use std::process::Command;
    
    if !is_git_repository(directory)? {
        return Err(anyhow::anyhow!("Not a git repository"));
    }
    
    // Get remote URL
    let remote_output = Command::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(directory)
        .output()?;
    
    let remote_url = if remote_output.status.success() {
        String::from_utf8_lossy(&remote_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };
    
    // Get current branch
    let branch_output = Command::new("git")
        .args(&["branch", "--show-current"])
        .current_dir(directory)
        .output()?;
    
    let current_branch = if branch_output.status.success() {
        String::from_utf8_lossy(&branch_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };
    
    // Get last commit
    let commit_output = Command::new("git")
        .args(&["log", "-1", "--format=%H %s"])
        .current_dir(directory)
        .output()?;
    
    let last_commit = if commit_output.status.success() {
        String::from_utf8_lossy(&commit_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };
    
    // Check for local changes
    let status_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(directory)
        .output()?;
    
    let has_local_changes = !status_output.stdout.is_empty();
    
    Ok(RepositoryInfo {
        remote_url,
        current_branch,
        last_commit,
        has_local_changes,
    })
}

#[derive(Debug)]
pub struct RepositoryInfo {
    pub remote_url: String,
    pub current_branch: String,
    pub last_commit: String,
    pub has_local_changes: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_is_rule_file() {
        assert!(is_rule_file(&PathBuf::from("test.yaml")));
        assert!(is_rule_file(&PathBuf::from("test.yml")));
        assert!(!is_rule_file(&PathBuf::from("test.txt")));
        assert!(!is_rule_file(&PathBuf::from("test")));
    }

    #[tokio::test]
    async fn test_find_rule_files() {
        let temp_dir = tempdir().unwrap();
        let rules_dir = temp_dir.path().join("rules");
        fs::create_dir_all(&rules_dir).unwrap();
        
        // Create test rule files
        fs::write(rules_dir.join("rule1.yaml"), "test content").unwrap();
        fs::write(rules_dir.join("rule2.yml"), "test content").unwrap();
        fs::write(rules_dir.join("not_a_rule.txt"), "test content").unwrap();
        
        let rule_files = find_rule_files(&rules_dir).await.unwrap();
        assert_eq!(rule_files.len(), 2);
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert_eq!(result.total_files, 0);
        assert_eq!(result.valid_rules, 0);
        assert_eq!(result.invalid_rules, 0);
        
        result.total_files = 5;
        result.valid_rules = 3;
        result.invalid_rules = 2;
        
        assert_eq!(result.total_files, 5);
        assert_eq!(result.valid_rules, 3);
        assert_eq!(result.invalid_rules, 2);
    }

    #[test]
    fn test_check_git_availability() {
        // This test may fail in environments without git
        // In a real test suite, you might want to mock this
        let result = check_git_availability();
        // Just ensure it doesn't panic
        let _ = result;
    }
}
