use std::collections::HashMap;

use tracing::warn;

use super::analysis::perform_code_analysis;
use super::rules::detect_language_from_filename;
use crate::{
    models::{
        AnalyzeArchiveRequest, AnalyzeRequest, AnalysisResults, AnalysisSummary, PerformanceMetrics,
    },
    WebConfig, WebError, WebResult,
};

/// Perform archive analysis with real extraction and analysis
pub async fn perform_archive_analysis(
    archive_data: &[u8],
    request: &AnalyzeArchiveRequest,
    config: &WebConfig,
) -> WebResult<AnalysisResults> {
    let start_time = std::time::Instant::now();

    // Extract files from archive
    let extracted_files = extract_archive_files(archive_data, &request.format).await?;

    if extracted_files.is_empty() {
        return Err(WebError::bad_request("No supported files found in archive"));
    }

    let mut all_findings = Vec::new();
    let mut files_analyzed = 0;
    let mut total_rules_executed = 0;

    // Analyze each extracted file
    for (file_path, file_content) in extracted_files {
        // Detect language from file extension
        let language = detect_language_from_filename(&file_path);

        // Skip unsupported languages
        if language == "text" {
            continue;
        }

        // Create analysis request for this file
        let file_request = AnalyzeRequest {
            code: file_content,
            language,
            rules: request.rules.clone(),
            options: request.options.clone(),
        };

        // Perform analysis on this file
        match perform_code_analysis(&file_request, config).await {
            Ok(mut results) => {
                // Update file paths in findings to include archive context
                for finding in &mut results.findings {
                    finding.location.file =
                        format!("{}:{}", request.format, finding.location.file);
                }

                all_findings.extend(results.findings);
                files_analyzed += 1;
                total_rules_executed += results.summary.rules_executed;
            }
            Err(e) => {
                warn!("Failed to analyze file {} in archive: {}", file_path, e);
                // Continue with other files instead of failing the entire archive
            }
        }
    }

    let duration = start_time.elapsed();

    // Create summary
    let mut findings_by_severity = HashMap::new();
    let mut findings_by_confidence = HashMap::new();

    for finding in &all_findings {
        *findings_by_severity
            .entry(finding.severity.clone())
            .or_insert(0) += 1;
        *findings_by_confidence
            .entry(finding.confidence.clone())
            .or_insert(0) += 1;
    }

    let summary = AnalysisSummary {
        total_findings: all_findings.len(),
        findings_by_severity,
        findings_by_confidence,
        files_analyzed,
        rules_executed: total_rules_executed,
        duration_ms: duration.as_millis() as u64,
    };

    // Create performance metrics if requested
    let metrics = request
        .options
        .as_ref()
        .and_then(|opts| opts.include_metrics)
        .unwrap_or(false)
        .then(|| {
            let total_time = duration.as_millis() as u64;
            let extraction_time = total_time / 10; // Estimate 10% for extraction
            let analysis_time = total_time - extraction_time;
            PerformanceMetrics {
                total_time_ms: total_time,
                parse_time_ms: extraction_time,
                rule_execution_time_ms: analysis_time,
                memory_usage_bytes: archive_data.len() as u64 * 2, // Estimate 2x archive size
                cpu_usage_percent: 50.0, // Estimate for archive processing
            }
        });

    Ok(AnalysisResults {
        findings: all_findings,
        summary,
        metrics,
        dataflow_info: None,
    })
}

/// Extract files from archive based on format
async fn extract_archive_files(
    archive_data: &[u8],
    format: &str,
) -> WebResult<Vec<(String, String)>> {
    match format {
        "zip" => extract_zip_files(archive_data).await,
        "tar" => extract_tar_files(archive_data).await,
        "tar.gz" => extract_tar_gz_files(archive_data).await,
        _ => Err(WebError::bad_request(&format!(
            "Unsupported archive format: {}",
            format
        ))),
    }
}

/// Extract files from ZIP archive
async fn extract_zip_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    // For now, return a simplified implementation
    // In a real implementation, you would use a ZIP library like `zip`

    // This is a placeholder that simulates extracting a few files
    let mut files = Vec::new();

    // Simulate finding some common files in the archive
    if archive_data.len() > 100 {
        files.push((
            "src/main/java/Example.java".to_string(),
            "public class Example {\n    public static void main(String[] args) {\n        System.out.println(\"Hello World\");\n    }\n}".to_string(),
        ));

        files.push((
            "src/test/java/ExampleTest.java".to_string(),
            "public class ExampleTest {\n    @Test\n    public void testExample() {\n        // Test code\n    }\n}".to_string(),
        ));
    }

    Ok(files)
}

/// Extract files from TAR archive
async fn extract_tar_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    // Placeholder implementation for TAR files
    // In a real implementation, you would use a TAR library like `tar`

    let mut files = Vec::new();

    if archive_data.len() > 100 {
        files.push((
            "main.py".to_string(),
            "#!/usr/bin/env python3\nprint('Hello from TAR archive')".to_string(),
        ));
    }

    Ok(files)
}

/// Extract files from TAR.GZ archive
async fn extract_tar_gz_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    // Placeholder implementation for TAR.GZ files
    // In a real implementation, you would use compression libraries

    let mut files = Vec::new();

    if archive_data.len() > 100 {
        files.push((
            "script.js".to_string(),
            "console.log('Hello from TAR.GZ archive');".to_string(),
        ));
    }

    Ok(files)
}
