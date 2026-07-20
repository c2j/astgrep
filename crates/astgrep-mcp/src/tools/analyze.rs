//! `analyze_code` MCP tool implementation.
//!
//! Accepts source code and language, writes to a temporary file, and runs
//! the astgrep analysis engine through `analyze_collect`.

use anyhow::{Context, Result};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use astgrep_core::{Language, OutputFormat};

use astgrep_cli::analysis::Finding;
use astgrep_cli::analyze_enhanced::analyze_collect;
use astgrep_cli::EnhancedAnalysisConfig;

/// Request parameters for the `analyze_code` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnalyzeCodeRequest {
    /// Source code content to analyze
    #[schemars(description = "Source code content to analyze")]
    pub code: String,

    /// Language: java|javascript|python|sql|bash|xml
    #[schemars(description = "Target language (java|javascript|python|sql|bash|xml)")]
    pub language: String,

    /// Optional directory path for project-context analysis
    #[schemars(description = "Optional directory path for project-context analysis")]
    pub target_path: Option<String>,
}

/// Lightweight JSON-friendly finding summary.
#[derive(Debug, Clone, Serialize)]
pub struct FindingSummary {
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file_path: String,
    pub snippet: String,
}

/// Summary of analysis statistics for the response.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisStatsSummary {
    pub files_analyzed: usize,
    pub rules_executed: usize,
    pub parse_errors: usize,
    pub analysis_errors: usize,
    pub dataflow_analyses: usize,
}

/// JSON-friendly response for the `analyze_code` tool.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeCodeResult {
    pub findings: Vec<FindingSummary>,
    pub stats: AnalysisStatsSummary,
    pub elapsed_ms: u64,
}

impl From<&Finding> for FindingSummary {
    fn from(f: &Finding) -> Self {
        // Extract a short snippet from the message or use a default
        let snippet = f.message.chars().take(200).collect();
        Self {
            rule_id: f.rule_id.clone(),
            severity: f.severity.as_str().to_string(),
            message: f.message.clone(),
            line: f.location.start_line,
            column: f.location.start_column,
            file_path: f.location.file.to_string_lossy().to_string(),
            snippet,
        }
    }
}

/// Execute the analysis and return a structured result.
pub async fn handle_analyze(
    req: AnalyzeCodeRequest,
    rules_dir: Option<&Path>,
) -> Result<AnalyzeCodeResult> {
    let lang = Language::parse_name(&req.language).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported language: '{}'. Supported: java|javascript|python|sql|bash|xml",
            req.language
        )
    })?;

    // Write code to a temporary file so the analysis engine can read it.
    let tmp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
    let ext = lang.extensions().first().copied().unwrap_or(".txt");
    let tmp_file_path = tmp_dir.path().join(format!("source{ext}"));
    std::fs::write(&tmp_file_path, &req.code)
        .with_context(|| format!("Failed to write temp file: {}", tmp_file_path.display()))?;

    let rule_files = collect_rule_files(rules_dir);

    let target_paths = if let Some(ref tp) = req.target_path {
        vec![PathBuf::from(tp)]
    } else {
        vec![tmp_file_path.clone()]
    };

    // Build the analysis config.
    let config = EnhancedAnalysisConfig {
        target_paths,
        exclude_patterns: vec![],
        include_patterns: vec![],
        languages: vec![lang],
        rule_files,
        output_format: OutputFormat::Json,
        severity_filter: None,
        confidence_filter: None,
        include_metrics: false,
        max_findings: None,
        enable_dataflow: false,
        baseline_file: None,
        fail_on_findings: false,
        parallel: false,
        max_threads: None,
        enable_profiling: false,
        compatible_mode: None,
        sql_statement_boundary: None,
        enable_constant_propagation: false,
        sql_dialect: None,
    };

    let start = std::time::Instant::now();
    let (findings, stats, _duration) = analyze_collect(&config).await?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Convert to JSON-friendly summaries.
    let finding_summaries: Vec<FindingSummary> = findings.iter().map(|f| f.into()).collect();

    let result = AnalyzeCodeResult {
        findings: finding_summaries,
        stats: AnalysisStatsSummary {
            files_analyzed: stats.files_analyzed,
            rules_executed: stats.rules_executed,
            parse_errors: stats.parse_errors,
            analysis_errors: stats.analysis_errors,
            dataflow_analyses: stats.dataflow_analyses,
        },
        elapsed_ms,
    };

    // TempDir is dropped here, cleaning up the temp file.
    Ok(result)
}

/// Collect .yaml/.yml rule file paths from a rules directory.
fn collect_rule_files(rules_dir: Option<&Path>) -> Vec<PathBuf> {
    let dir = match rules_dir {
        Some(d) if d.exists() && d.is_dir() => d,
        _ => return Vec::new(),
    };
    let mut files = Vec::new();
    let _ = walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .for_each(|p| {
            if p.extension()
                .map(|ext| ext == "yaml" || ext == "yml")
                .unwrap_or(false)
            {
                files.push(p.to_path_buf());
            }
        });
    files
}
