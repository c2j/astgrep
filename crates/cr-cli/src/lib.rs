//! Command-line interface for CR-SemService

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use cr_core::{AnalysisConfig, Language, OutputFormat, Severity, Confidence};
use std::path::PathBuf;
use tracing::{info, warn};

mod commands;
mod profiler;
mod tree_sitter_analyzer;
pub mod vscode_integration;

pub use commands::*;
pub use profiler::*;
pub use vscode_integration::*;

/// CR-SemService: Multi-language Static Code Analysis Tool
#[derive(Parser)]
#[command(name = "cr-semservice")]
#[command(about = "A comprehensive static analysis tool for code review and security scanning")]
#[command(version)]
#[command(long_about = "CR-SemService provides advanced static analysis capabilities for multiple programming languages, \
featuring pattern matching, data flow analysis, and comprehensive security vulnerability detection.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Number of parallel threads to use (0 = auto-detect)
    #[arg(short = 'j', long, global = true, default_value = "0")]
    pub threads: usize,

    /// Enable performance profiling
    #[arg(long, global = true)]
    pub profile: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze source code for security vulnerabilities and quality issues
    Analyze {
        /// Target paths to analyze
        #[arg(value_name = "PATH")]
        targets: Vec<PathBuf>,

        /// Rule files or directories to use
        #[arg(short, long)]
        rules: Vec<PathBuf>,

        /// Languages to analyze (java, javascript, python, sql, bash)
        #[arg(short, long)]
        language: Vec<String>,

        /// Exclude patterns (glob patterns)
        #[arg(short, long)]
        exclude: Vec<String>,

        /// Include only files matching these patterns
        #[arg(long, value_name = "PATTERN")]
        include: Vec<String>,

        /// Output format
        #[arg(short = 'f', long, default_value = "json")]
        format: OutputFormatCli,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Severity level filter (only show issues at or above this level)
        #[arg(short = 'S', long, default_value = "info")]
        severity: SeverityFilter,

        /// Confidence level filter (only show issues at or above this level)
        #[arg(short = 'C', long, default_value = "low")]
        confidence: ConfidenceFilter,

        /// Include performance metrics in output
        #[arg(long)]
        metrics: bool,

        /// Maximum number of findings to report (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_findings: usize,

        /// Enable data flow analysis
        #[arg(long)]
        dataflow: bool,

        /// Baseline file for comparison (show only new issues)
        #[arg(long, value_name = "FILE")]
        baseline: Option<PathBuf>,

        /// Exit with non-zero code if issues are found
        #[arg(long)]
        fail_on_findings: bool,

        /// Disable parallel processing
        #[arg(long)]
        no_parallel: bool,

        /// Maximum number of threads
        #[arg(long)]
        max_threads: Option<usize>,

        /// Enable compatibility mode with external tools (e.g., "semgrep")
        #[arg(long)]
        compatible: Option<String>,
    },

    /// Validate rule files for syntax and semantic correctness
    Validate {
        /// Rule files or directories to validate
        rule_files: Vec<PathBuf>,

        /// Output format for validation results
        #[arg(short = 'f', long, default_value = "text")]
        format: OutputFormatCli,

        /// Validate against specific language
        #[arg(short, long)]
        language: Option<String>,

        /// Check rule performance (dry run)
        #[arg(long)]
        performance: bool,
    },

    /// List available rules and their information
    List {
        /// Rules directory to scan
        #[arg(short, long, value_name = "PATH")]
        rules: Option<PathBuf>,

        /// Filter by language
        #[arg(short, long)]
        language: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Show detailed rule information
        #[arg(long)]
        detailed: bool,

        /// Output format
        #[arg(short = 'f', long, default_value = "table")]
        format: OutputFormatCli,
    },

    /// Initialize a new configuration file
    Init {
        /// Output configuration file
        #[arg(short, long, default_value = "cr-semservice.toml")]
        output: PathBuf,

        /// Configuration template to use
        #[arg(short, long, default_value = "default")]
        template: String,

        /// Overwrite existing configuration file
        #[arg(long)]
        force: bool,
    },

    /// Show information about supported languages and features
    Info {
        /// Show information about specific language
        #[arg(short, long)]
        language: Option<String>,

        /// Show supported file extensions
        #[arg(long)]
        extensions: bool,

        /// Show available rule categories
        #[arg(long)]
        categories: bool,
    },

    /// Update rules from remote repositories
    Update {
        /// Rules repository URL
        #[arg(short, long)]
        repository: Option<String>,

        /// Local rules directory
        #[arg(short, long, default_value = "rules")]
        directory: PathBuf,

        /// Force update (overwrite local changes)
        #[arg(long)]
        force: bool,
    },

    /// List supported languages and their extensions (deprecated, use 'info')
    Languages,

    /// Show version information (deprecated, use --version)
    Version,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormatCli {
    /// Human-readable text format
    Text,
    /// JSON format
    Json,
    /// SARIF format (Static Analysis Results Interchange Format)
    Sarif,
    /// XML format
    Xml,
    /// CSV format
    Csv,
    /// HTML report
    Html,
    /// Markdown format
    Markdown,
    /// Table format (for list commands)
    Table,
    /// YAML format
    Yaml,
}

#[derive(Clone, ValueEnum)]
pub enum SeverityFilter {
    /// Show all issues
    All,
    /// Show info level and above
    Info,
    /// Show warning level and above
    Warning,
    /// Show error level only
    Error,
}

#[derive(Clone, ValueEnum)]
pub enum ConfidenceFilter {
    /// Show all confidence levels
    All,
    /// Show low confidence and above
    Low,
    /// Show medium confidence and above
    Medium,
    /// Show high confidence only
    High,
}

/// Main entry point for the CLI
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging level based on flags
    setup_logging(cli.verbose, cli.quiet)?;

    // Enable performance profiling if requested
    if cli.profile {
        info!("Performance profiling enabled");
    }

    match cli.command {
        Commands::Analyze {
            targets,
            rules,
            language,
            exclude,
            include,
            format,
            output,
            severity,
            confidence,
            metrics,
            max_findings,
            dataflow,
            baseline,
            fail_on_findings,
            no_parallel,
            max_threads,
            compatible,
        } => {
            info!("Starting code analysis");

            // Use --config parameter if provided and no rules specified, otherwise use rules
            let rule_files = if rules.is_empty() && cli.config.is_some() {
                vec![cli.config.unwrap()]
            } else {
                rules
            };

            let config = build_enhanced_analysis_config(
                targets,
                rule_files,
                language,
                exclude,
                include,
                format,
                severity,
                confidence,
                metrics,
                max_findings,
                dataflow,
                baseline,
                fail_on_findings,
                !no_parallel,
                max_threads.or(if cli.threads > 0 { Some(cli.threads) } else { None }),
                cli.profile,
                compatible,
            )?;

            commands::analyze_enhanced::run_enhanced(config, output).await
        }
        Commands::Validate { rule_files, format, language, performance } => {
            info!("Validating rule files");
            // Use --config parameter if provided and no rule_files specified, otherwise use rule_files
            let files_to_validate = if rule_files.is_empty() && cli.config.is_some() {
                vec![cli.config.unwrap()]
            } else if rule_files.is_empty() {
                return Err(anyhow::anyhow!("No rule files specified. Use --config or provide rule file paths."));
            } else {
                rule_files
            };
            commands::validate_enhanced::run_enhanced(files_to_validate, format, language, performance).await
        }
        Commands::List { rules, language, category, detailed, format } => {
            info!("Listing available rules");
            // Use --config parameter if provided, otherwise use --rules parameter
            let rules_dir = cli.config.or(rules);
            commands::list::run(rules_dir, language, category, detailed, format).await
        }
        Commands::Init { output, template, force } => {
            info!("Initializing configuration file");
            commands::init::run(output, template, force).await
        }
        Commands::Info { language, extensions, categories } => {
            info!("Showing system information");
            commands::info::run(language, extensions, categories).await
        }
        Commands::Update { repository, directory, force } => {
            info!("Updating rules");
            commands::update::run(repository, directory, force).await
        }
        Commands::Languages => {
            warn!("'languages' command is deprecated, use 'info --extensions' instead");
            commands::languages::run().await
        }
        Commands::Version => {
            warn!("'version' command is deprecated, use '--version' flag instead");
            commands::version::run().await
        }
    }
}

fn setup_logging(verbose: bool, quiet: bool) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = if quiet {
        tracing::Level::ERROR
    } else if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    // Try to set global default, but don't fail if already set
    let _ = tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
            .with_target(false)
            .with_thread_ids(verbose)
            .with_file(verbose)
            .with_line_number(verbose)
            .finish(),
    );

    Ok(())
}

fn build_enhanced_analysis_config(
    targets: Vec<PathBuf>,
    rules: Vec<PathBuf>,
    languages: Vec<String>,
    exclude: Vec<String>,
    include: Vec<String>,
    format: OutputFormatCli,
    severity: SeverityFilter,
    confidence: ConfidenceFilter,
    metrics: bool,
    max_findings: usize,
    dataflow: bool,
    baseline: Option<PathBuf>,
    fail_on_findings: bool,
    parallel: bool,
    max_threads: Option<usize>,
    profile: bool,
    compatible: Option<String>,
) -> Result<EnhancedAnalysisConfig> {
    let target_paths = if targets.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        targets
    };

    let parsed_languages = if languages.is_empty() {
        vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Php,
            Language::Sql,
            Language::Bash,
            Language::CSharp,
            Language::C,
        ]
    } else {
        let mut parsed = Vec::new();
        for lang_str in languages {
            match Language::from_str(&lang_str) {
                Some(lang) => parsed.push(lang),
                None => {
                    warn!("Unknown language: {}, skipping", lang_str);
                }
            }
        }
        if parsed.is_empty() {
            return Err(anyhow::anyhow!("No valid languages specified"));
        }
        parsed
    };

    let output_format = convert_output_format(format);

    Ok(EnhancedAnalysisConfig {
        target_paths,
        exclude_patterns: exclude,
        include_patterns: include,
        languages: parsed_languages,
        rule_files: rules,
        output_format,
        severity_filter: convert_severity_filter(severity),
        confidence_filter: convert_confidence_filter(confidence),
        include_metrics: metrics,
        max_findings: if max_findings == 0 { None } else { Some(max_findings) },
        enable_dataflow: dataflow,
        baseline_file: baseline,
        fail_on_findings,
        parallel,
        max_threads,
        enable_profiling: profile,
        compatible_mode: compatible,
    })
}

fn convert_output_format(format: OutputFormatCli) -> OutputFormat {
    match format {
        OutputFormatCli::Text => OutputFormat::Text,
        OutputFormatCli::Json => OutputFormat::Json,
        OutputFormatCli::Sarif => OutputFormat::Sarif,
        OutputFormatCli::Xml => OutputFormat::Xml,
        OutputFormatCli::Yaml => OutputFormat::Yaml,
        // Map unsupported formats to closest equivalent
        OutputFormatCli::Csv => OutputFormat::Text,
        OutputFormatCli::Html => OutputFormat::Text,
        OutputFormatCli::Markdown => OutputFormat::Text,
        OutputFormatCli::Table => OutputFormat::Text,
    }
}

fn convert_severity_filter(filter: SeverityFilter) -> Option<Severity> {
    match filter {
        SeverityFilter::All => None,
        SeverityFilter::Info => Some(Severity::Info),
        SeverityFilter::Warning => Some(Severity::Warning),
        SeverityFilter::Error => Some(Severity::Error),
    }
}

fn convert_confidence_filter(filter: ConfidenceFilter) -> Option<Confidence> {
    match filter {
        ConfidenceFilter::All => None,
        ConfidenceFilter::Low => Some(Confidence::Low),
        ConfidenceFilter::Medium => Some(Confidence::Medium),
        ConfidenceFilter::High => Some(Confidence::High),
    }
}

// Legacy function for backward compatibility
fn build_analysis_config(
    targets: Vec<PathBuf>,
    rules: Vec<PathBuf>,
    languages: Vec<String>,
    exclude: Vec<String>,
    format: String,
    parallel: bool,
    max_threads: Option<usize>,
) -> Result<AnalysisConfig> {
    let target_paths = if targets.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        targets
    };

    let parsed_languages = if languages.is_empty() {
        vec![
            Language::Java,
            Language::JavaScript,
            Language::Python,
            Language::Php,
            Language::Sql,
            Language::Bash,
            Language::CSharp,
            Language::C,
        ]
    } else {
        let mut parsed = Vec::new();
        for lang_str in languages {
            match Language::from_str(&lang_str) {
                Some(lang) => parsed.push(lang),
                None => {
                    warn!("Unknown language: {}, skipping", lang_str);
                }
            }
        }
        if parsed.is_empty() {
            return Err(anyhow::anyhow!("No valid languages specified"));
        }
        parsed
    };

    let output_format = OutputFormat::from_str(&format)
        .ok_or_else(|| anyhow::anyhow!("Unknown output format: {}", format))?;

    Ok(AnalysisConfig {
        target_paths,
        exclude_patterns: exclude,
        languages: parsed_languages,
        rule_files: rules,
        output_format,
        parallel,
        max_threads,
    })
}

/// Enhanced analysis configuration with additional options
#[derive(Debug, Clone)]
pub struct EnhancedAnalysisConfig {
    pub target_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub languages: Vec<Language>,
    pub rule_files: Vec<PathBuf>,
    pub output_format: OutputFormat,
    pub severity_filter: Option<Severity>,
    pub confidence_filter: Option<Confidence>,
    pub include_metrics: bool,
    pub max_findings: Option<usize>,
    pub enable_dataflow: bool,
    pub baseline_file: Option<PathBuf>,
    pub fail_on_findings: bool,
    pub parallel: bool,
    pub max_threads: Option<usize>,
    pub enable_profiling: bool,
    pub compatible_mode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_analysis_config_defaults() {
        let config = build_analysis_config(
            vec![],
            vec![],
            vec![],
            vec![],
            "json".to_string(),
            true,
            None,
        ).unwrap();

        assert_eq!(config.target_paths, vec![PathBuf::from(".")]);
        assert_eq!(config.languages.len(), 5);
        assert_eq!(config.output_format, OutputFormat::Json);
        assert!(config.parallel);
        assert!(config.max_threads.is_none());
    }

    #[test]
    fn test_build_analysis_config_custom() {
        let config = build_analysis_config(
            vec![PathBuf::from("src")],
            vec![PathBuf::from("rules.yml")],
            vec!["java".to_string(), "python".to_string()],
            vec!["*.test.java".to_string()],
            "sarif".to_string(),
            false,
            Some(4),
        ).unwrap();

        assert_eq!(config.target_paths, vec![PathBuf::from("src")]);
        assert_eq!(config.rule_files, vec![PathBuf::from("rules.yml")]);
        assert_eq!(config.languages, vec![Language::Java, Language::Python]);
        assert_eq!(config.exclude_patterns, vec!["*.test.java"]);
        assert_eq!(config.output_format, OutputFormat::Sarif);
        assert!(!config.parallel);
        assert_eq!(config.max_threads, Some(4));
    }

    #[test]
    fn test_build_analysis_config_invalid_language() {
        let result = build_analysis_config(
            vec![],
            vec![],
            vec!["invalid_language".to_string()],
            vec![],
            "json".to_string(),
            true,
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No valid languages specified"));
    }

    #[test]
    fn test_build_analysis_config_invalid_format() {
        let result = build_analysis_config(
            vec![],
            vec![],
            vec![],
            vec![],
            "invalid_format".to_string(),
            true,
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown output format"));
    }
}
