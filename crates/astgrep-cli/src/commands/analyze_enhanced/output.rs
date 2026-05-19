//! Output generation for enhanced analysis

use crate::output::analysis::AnalysisStatistics;
use crate::output::analysis::{OutputFactory, OutputFormat};
use crate::EnhancedAnalysisConfig;
use anyhow::Result;

/// Generate enhanced output based on findings and configuration
pub fn generate_enhanced_output(
    findings: &[crate::output::analysis::Finding],
    stats: &AnalysisStatistics,
    config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
    _profiler: Option<&crate::PerformanceProfiler>,
) -> Result<String> {
    // Check for compatibility mode
    if let Some(ref compatible_mode) = config.compatible_mode {
        if let Some(formatter) = OutputFactory::create_compatible(compatible_mode) {
            return formatter.format(findings, stats, total_time);
        } else {
            tracing::warn!(
                "Unknown compatibility mode: {}, falling back to default output",
                compatible_mode
            );
        }
    }

    // Convert from astgrep_core::OutputFormat to crate::output::analysis::OutputFormat
    let format = match config.output_format {
        astgrep_core::OutputFormat::Json => OutputFormat::Json,
        astgrep_core::OutputFormat::Sarif => OutputFormat::Sarif,
        astgrep_core::OutputFormat::Text => OutputFormat::Text,
        // XML and YAML fall back to Text output
        astgrep_core::OutputFormat::Xml | astgrep_core::OutputFormat::Yaml => OutputFormat::Text,
    };

    let formatter = OutputFactory::create(format);
    formatter.format(findings, stats, total_time)
}

/// Apply filters to findings based on configuration
pub fn apply_filters(
    findings: &[crate::output::analysis::Finding],
    config: &EnhancedAnalysisConfig,
) -> Vec<crate::output::analysis::Finding> {
    findings
        .iter()
        .filter(|finding| {
            // Apply severity filter
            if let Some(min_severity) = config.severity_filter {
                if finding.severity < min_severity {
                    return false;
                }
            }

            // Apply confidence filter
            if let Some(min_confidence) = config.confidence_filter {
                if finding.confidence < min_confidence {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}
