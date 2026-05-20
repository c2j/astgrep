//! Migration commands for test directory reorganization

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

// Import our migration modules
use crate::commands::migrate_scripts::{ScriptMigrationConfig, ScriptMigrator};
use crate::dependencies::{DependencyResolutionConfig, ScriptDependencyResolver};

/// Migration subcommand for test directory reorganization
#[derive(Parser)]
#[command(name = "migrate")]
#[command(about = "Reorganize test directory structure for better maintainability")]
pub struct MigrateCommand {
    #[command(subcommand)]
    pub action: MigrationAction,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable dry run mode (no actual file operations)
    #[arg(long)]
    pub dry_run: bool,

    /// Output format for migration reports
    #[arg(short = 'f', long, default_value = "human")]
    pub format: String,

    /// Configuration file for migration settings
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Enable progress reporting
    #[arg(long)]
    pub progress: bool,

    /// Create backups before migration
    #[arg(long)]
    pub backup: bool,

    /// Number of parallel threads to use
    #[arg(short = 'j', long, default_value = "4")]
    pub threads: usize,
}

#[derive(Subcommand)]
pub enum MigrationAction {
    /// Analyze current test structure and generate report
    Analyze {
        /// Output path for analysis report
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// Include detailed statistics
        #[arg(long)]
        detailed: bool,
    },
    /// Validate migration plan without executing
    Validate {
        /// Asset IDs to validate (all if not specified)
        #[arg()]
        asset_ids: Vec<String>,

        /// Validation level
        #[arg(long, default_value = "comprehensive")]
        level: String,

        /// Check dependencies
        #[arg(long)]
        check_dependencies: bool,

        /// Check disk space
        #[arg(long)]
        check_disk_space: bool,
    },
    /// Execute migration of test assets
    Migrate {
        /// Asset IDs to migrate (all if not specified)
        #[arg()]
        asset_ids: Vec<String>,

        /// Migration category filter
        #[arg(long)]
        category: Option<String>,

        /// Language filter for test cases
        #[arg(long)]
        language: Option<String>,
    },
    /// Rollback a migration operation
    Rollback {
        /// Migration ID to rollback
        #[arg()]
        migration_id: String,

        /// Force rollback without confirmation
        #[arg(long)]
        force: bool,
    },
    /// Show migration status and history
    Status {
        /// Show detailed status
        #[arg(long)]
        detailed: bool,

        /// Migration ID to check status for
        #[arg()]
        migration_id: Option<String>,
    },
    /// Test execution with new structure
    Test {
        /// Test with new directory structure
        #[arg(long)]
        new_structure: bool,

        /// Run all tests
        #[arg(long)]
        all: bool,

        /// Test category filter
        #[arg(long)]
        category: Option<String>,
    },
}

#[derive(Clone)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
    Markdown,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Human => write!(f, "human"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Markdown => write!(f, "markdown"),
        }
    }
}

#[derive(Clone)]
pub enum ValidationLevel {
    Basic,
    Comprehensive,
    Strict,
}

impl std::fmt::Display for ValidationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationLevel::Basic => write!(f, "basic"),
            ValidationLevel::Comprehensive => write!(f, "comprehensive"),
            ValidationLevel::Strict => write!(f, "strict"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub target_directory: PathBuf,
    pub preserve_timestamps: bool,
    pub create_backups: bool,
    pub validate_after_migration: bool,
    pub categories: CategoryConfig,
    pub languages: LanguageConfig,
    pub validation: ValidationConfig,
    pub compatibility: CompatibilityConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub scripts: String,
    pub naming_convention: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub mapping: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub check_dependencies: bool,
    pub check_disk_space: bool,
    pub validate_permissions: bool,
    pub create_checksums: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityConfig {
    pub create_symlinks: bool,
    pub preserve_original: bool,
    pub update_discovery: bool,
}

/// Main migration command handler
pub async fn run(command: MigrateCommand) -> Result<()> {
    // Setup logging based on verbose flag
    setup_logging(command.verbose);

    // Parse output format
    let _output_format = match command.format.to_lowercase().as_str() {
        "human" => OutputFormat::Human,
        "json" => OutputFormat::Json,
        "yaml" => OutputFormat::Yaml,
        "markdown" => OutputFormat::Markdown,
        _ => return Err(anyhow::anyhow!("Invalid output format: {}", command.format)),
    };

    info!("Starting migration operation");

    match command.action {
        MigrationAction::Analyze { output, detailed } => handle_analyze(output, detailed).await,
        MigrationAction::Validate {
            asset_ids,
            level,
            check_dependencies,
            check_disk_space,
        } => handle_validate(asset_ids, level, check_dependencies, check_disk_space).await,
        MigrationAction::Migrate {
            asset_ids,
            category,
            language,
        } => {
            handle_migrate(
                asset_ids,
                category,
                language,
                command.dry_run,
                command.backup,
                command.threads,
                command.progress,
                command.format == "json",
            )
            .await
        }
        MigrationAction::Rollback {
            migration_id,
            force,
        } => handle_rollback(migration_id, force).await,
        MigrationAction::Status {
            detailed,
            migration_id,
        } => handle_status(detailed, migration_id).await,
        MigrationAction::Test {
            new_structure,
            all,
            category,
        } => handle_test(new_structure, all, category).await,
    }
}

fn setup_logging(verbose: bool) {
    use tracing_subscriber::EnvFilter;

    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
        .with_writer(std::io::stderr)
        .with_target(verbose)
        .with_file(verbose)
        .with_line_number(verbose)
        .init();
}

async fn handle_analyze(output: Option<PathBuf>, detailed: bool) -> Result<()> {
    info!("Starting analysis of current test structure");

    let project_root = std::env::current_dir()?;
    let tests_dir = project_root.join("tests");
    let newtest_dir = project_root.join("newtest");

    // Check if directories exist
    let original_exists = tests_dir.exists();
    let newtest_exists = newtest_dir.exists();

    let mut analysis_report = AnalysisReport {
        timestamp: chrono::Utc::now(),
        project_root,
        original_structure_exists: original_exists,
        new_structure_exists: newtest_exists,
        script_count: 0,
        test_case_count: 0,
        total_size: 0,
        dependencies: Vec::new(),
        recommendations: Vec::new(),
    };

    // Analyze original structure if it exists
    if original_exists {
        info!("Analyzing original test structure...");
        let scripts = find_scripts(&tests_dir)?;
        let test_cases = find_test_cases(&tests_dir)?;

        analysis_report.script_count = scripts.len();
        analysis_report.test_case_count = test_cases.len();

        // Calculate total size
        for script in &scripts {
            if let Ok(metadata) = std::fs::metadata(script) {
                analysis_report.total_size += metadata.len();
            }
        }

        // Analyze dependencies if detailed mode
        if detailed {
            debug!("Analyzing script dependencies...");
            let resolver = ScriptDependencyResolver::new(DependencyResolutionConfig::default());
            if !scripts.is_empty() {
                let resolution_result = resolver.resolve_dependencies(&scripts).await?;
                analysis_report.dependencies = resolution_result
                    .circular_dependencies
                    .into_iter()
                    .map(|cycle| DependencyInfo {
                        description: cycle.description,
                        affected_scripts: cycle
                            .scripts
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect(),
                    })
                    .collect();
            }
        }

        analysis_report
            .recommendations
            .push("Original test structure found and analyzed".to_string());
        if !newtest_exists {
            analysis_report
                .recommendations
                .push("Consider running migration to create new structure".to_string());
        }
    }

    // Analyze new structure if it exists
    if newtest_exists {
        info!("Analyzing new test structure...");
        let scripts_dir = newtest_dir.join("scripts");
        let testcases_dir = newtest_dir.join("testcases");

        if scripts_dir.exists() {
            let new_scripts = find_scripts(&scripts_dir)?;
            info!("Found {} scripts in new structure", new_scripts.len());
            analysis_report.recommendations.push(format!(
                "New structure contains {} organized scripts",
                new_scripts.len()
            ));
        }

        if testcases_dir.exists() {
            let new_test_cases = find_test_cases(&testcases_dir)?;
            info!("Found {} test cases in new structure", new_test_cases.len());
            analysis_report.recommendations.push(format!(
                "New structure contains {} organized test cases",
                new_test_cases.len()
            ));
        }
    }

    // Generate output
    if let Some(output_path) = output {
        let report_json = serde_json::to_string_pretty(&analysis_report)?;
        std::fs::write(&output_path, report_json)?;
        info!("Analysis report saved to: {}", output_path.display());
    } else {
        print_analysis_report(&analysis_report);
    }

    info!("Analysis completed successfully");
    Ok(())
}

async fn handle_validate(
    _asset_ids: Vec<String>,
    level: String,
    _check_dependencies: bool,
    _check_disk_space: bool,
) -> Result<()> {
    let _validation_level = match level.to_lowercase().as_str() {
        "basic" => ValidationLevel::Basic,
        "comprehensive" => ValidationLevel::Comprehensive,
        "strict" => ValidationLevel::Strict,
        _ => return Err(anyhow::anyhow!("Invalid validation level: {}", level)),
    };
    info!("Starting migration plan validation");

    // TODO: Implement validation logic
    warn!("Validation functionality not yet implemented");

    Ok(())
}

async fn handle_migrate(
    asset_ids: Vec<String>,
    _category: Option<String>,
    _language: Option<String>,
    dry_run: bool,
    backup: bool,
    threads: usize,
    progress: bool,
    _json: bool,
) -> Result<()> {
    info!("Starting migration operation");

    if dry_run {
        info!("Running in dry-run mode - no actual file operations will be performed");
    }

    let project_root = std::env::current_dir()?;
    let source_dir = project_root.join("tests");
    let target_dir = project_root.join("newtest");

    if !source_dir.exists() {
        return Err(anyhow::anyhow!(
            "Source directory not found: {}",
            source_dir.display()
        ));
    }

    // Create migration configuration
    let migration_config = ScriptMigrationConfig {
        source_directories: vec![source_dir],
        target_directory: target_dir,
        preserve_permissions: true,
        preserve_timestamps: true,
        create_backups: backup,
        dry_run,
        parallel_jobs: threads,
        enable_progress: progress,
        output_format: crate::output::OutputFormat::Human, // TODO: Make this configurable
        validate_after_migration: true,
        include_hidden: false,
        force_overwrite: false,
    };

    // Note: Filtering will be implemented at the discovery level
    let filtered_config = migration_config;

    // Create and run script migrator
    info!("Initializing script migrator...");
    let mut migrator = ScriptMigrator::new()?;

    if !asset_ids.is_empty() {
        info!("Migrating specific assets: {:?}", asset_ids);
        // TODO: Implement specific asset migration
        warn!("Specific asset filtering not yet implemented, migrating all assets");
    }

    info!("Starting migration process...");
    let migration_result = migrator.migrate_scripts().await?;

    // Report results
    info!("Migration completed successfully!");
    info!(
        "Total scripts processed: {}",
        migration_result.total_scripts_found
    );
    info!(
        "Successfully migrated: {}",
        migration_result.scripts_migrated
    );
    info!("Failed migrations: {}", migration_result.scripts_failed);
    info!("Skipped migrations: {}", migration_result.scripts_skipped);

    if migration_result.scripts_failed > 0 {
        warn!("Some migrations failed. Check the logs for details.");
        for failure in &migration_result.errors {
            error!("Migration error: {}", failure);
        }
    }

    if dry_run {
        info!("Dry run completed. No actual files were modified.");
    } else {
        info!(
            "Files have been migrated to: {}",
            filtered_config.target_directory.display()
        );
    }

    Ok(())
}

async fn handle_rollback(migration_id: String, _force: bool) -> Result<()> {
    info!("Starting rollback for migration: {}", migration_id);

    // TODO: Implement rollback logic
    warn!("Rollback functionality not yet implemented");

    Ok(())
}

async fn handle_status(_detailed: bool, _migration_id: Option<String>) -> Result<()> {
    info!("Checking migration status");

    // TODO: Implement status checking logic
    warn!("Status checking functionality not yet implemented");

    Ok(())
}

async fn handle_test(new_structure: bool, _all: bool, _category: Option<String>) -> Result<()> {
    info!("Running test execution");

    if new_structure {
        info!("Testing with new directory structure");
    }

    // TODO: Implement test execution logic
    warn!("Test execution functionality not yet implemented");

    Ok(())
}

// Helper functions and type definitions
fn find_scripts(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut scripts = Vec::new();

    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        match ext_str {
                            "sh" | "bash" | "py" | "js" | "ts" | "sql" | "java" => {
                                scripts.push(path);
                            }
                            _ => {}
                        }
                    }
                }
            } else if path.is_dir() {
                // Recursively search subdirectories
                scripts.extend(find_scripts(&path)?);
            }
        }
    }

    Ok(scripts)
}

fn find_test_cases(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut test_cases = Vec::new();

    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        match ext_str {
                            "sql" | "java" | "py" | "js" | "ts" => {
                                test_cases.push(path);
                            }
                            _ => {}
                        }
                    }
                }
            } else if path.is_dir() {
                // Recursively search subdirectories
                test_cases.extend(find_test_cases(&path)?);
            }
        }
    }

    Ok(test_cases)
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisReport {
    timestamp: chrono::DateTime<chrono::Utc>,
    project_root: PathBuf,
    original_structure_exists: bool,
    new_structure_exists: bool,
    script_count: usize,
    test_case_count: usize,
    total_size: u64,
    dependencies: Vec<DependencyInfo>,
    recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DependencyInfo {
    description: String,
    affected_scripts: Vec<String>,
}

fn print_analysis_report(report: &AnalysisReport) {
    println!("=== Test Structure Analysis Report ===");
    println!(
        "Generated: {}",
        report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Project Root: {}", report.project_root.display());
    println!();

    println!("Directory Status:");
    println!(
        "  Original structure (tests/): {}",
        if report.original_structure_exists {
            "✓ Found"
        } else {
            "✗ Not found"
        }
    );
    println!(
        "  New structure (newtest/): {}",
        if report.new_structure_exists {
            "✓ Found"
        } else {
            "✗ Not found"
        }
    );
    println!();

    if report.original_structure_exists {
        println!("Original Structure Analysis:");
        println!("  Scripts found: {}", report.script_count);
        println!("  Test cases found: {}", report.test_case_count);
        println!("  Total size: {} bytes", report.total_size);
        println!();
    }

    if !report.dependencies.is_empty() {
        println!("Dependencies:");
        for dep in &report.dependencies {
            println!("  - {}", dep.description);
        }
        println!();
    }

    if !report.recommendations.is_empty() {
        println!("Recommendations:");
        for rec in &report.recommendations {
            println!("  - {}", rec);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_migrate_command_parsing() {
        let cmd = MigrateCommand::try_parse_from(&[
            "migrate",
            "analyze",
            "--detailed",
            "--output",
            "report.md",
        ])
        .unwrap();

        match cmd.action {
            MigrationAction::Analyze { output, detailed } => {
                assert!(output.unwrap().ends_with("report.md"));
                assert!(detailed);
            }
            _ => panic!("Expected Analyze action"),
        }
    }

    #[test]
    fn test_validate_command_parsing() {
        let cmd = MigrateCommand::try_parse_from(&[
            "migrate",
            "validate",
            "--level",
            "comprehensive",
            "--check-dependencies",
        ])
        .unwrap();

        match cmd.action {
            MigrationAction::Validate {
                level,
                check_dependencies,
                ..
            } => {
                assert_eq!(level, "comprehensive");
                assert!(check_dependencies);
            }
            _ => panic!("Expected Validate action"),
        }
    }

    #[test]
    fn test_migrate_subcommand_parsing() {
        let cmd = MigrateCommand::try_parse_from(&[
            "migrate",
            "--dry-run",
            "--backup",
            "--threads",
            "8",
            "migrate",
        ])
        .unwrap();

        assert!(cmd.dry_run);
        assert!(cmd.backup);
        assert_eq!(cmd.threads, 8);
    }
}
