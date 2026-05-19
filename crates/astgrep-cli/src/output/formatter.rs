//! Output formatting for migration CLI commands

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use console::{style, Color};
use prettytable::{format::TableFormat, row, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

use crate::services::migration_orchestrator::MigrationOperation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Human,
    Json,
    PrettyJson,
    Table,
    Csv,
    Yaml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSummary {
    pub migration_id: String,
    pub status: MigrationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<String>,
    pub total_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub bytes_transferred: u64,
    pub files_processed: usize,
    pub directories_created: usize,
    pub symlinks_created: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

pub struct OutputFormatter {
    format: OutputFormat,
    use_colors: bool,
    verbose: bool,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat, use_colors: bool, verbose: bool) -> Self {
        Self {
            format,
            use_colors,
            verbose,
        }
    }

    /// Format migration summary for output
    pub fn format_migration_summary(&self, summary: &MigrationSummary) -> Result<String> {
        match self.format {
            OutputFormat::Human => self.format_human_summary(summary),
            OutputFormat::Json => self.format_json_summary(summary),
            OutputFormat::PrettyJson => self.format_pretty_json_summary(summary),
            OutputFormat::Table => self.format_table_summary(summary),
            OutputFormat::Csv => self.format_csv_summary(summary),
            OutputFormat::Yaml => self.format_yaml_summary(summary),
        }
    }

    /// Format list of operations for output
    pub fn format_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        match self.format {
            OutputFormat::Human => self.format_human_operations(operations),
            OutputFormat::Json => self.format_json_operations(operations),
            OutputFormat::PrettyJson => self.format_pretty_json_operations(operations),
            OutputFormat::Table => self.format_table_operations(operations),
            OutputFormat::Csv => self.format_csv_operations(operations),
            OutputFormat::Yaml => self.format_yaml_operations(operations),
        }
    }

    /// Format backup information
    pub fn format_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        match self.format {
            OutputFormat::Human => self.format_human_backup_info(backup_info),
            OutputFormat::Json => self.format_json_backup_info(backup_info),
            OutputFormat::PrettyJson => self.format_pretty_json_backup_info(backup_info),
            OutputFormat::Table => self.format_table_backup_info(backup_info),
            OutputFormat::Csv => self.format_csv_backup_info(backup_info),
            OutputFormat::Yaml => self.format_yaml_backup_info(backup_info),
        }
    }

    /// Format validation report
    pub fn format_validation_report(
        &self,
        validation_report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        match self.format {
            OutputFormat::Human => self.format_human_validation_report(validation_report),
            OutputFormat::Json => self.format_json_validation_report(validation_report),
            OutputFormat::PrettyJson => {
                self.format_pretty_json_validation_report(validation_report)
            }
            OutputFormat::Table => self.format_table_validation_report(validation_report),
            OutputFormat::Csv => self.format_csv_validation_report(validation_report),
            OutputFormat::Yaml => self.format_yaml_validation_report(validation_report),
        }
    }

    // Human-readable format methods

    fn format_human_summary(&self, summary: &MigrationSummary) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "{}\n{}\n\n",
            self.style_text("Migration Summary", Color::Cyan, true),
            "=".repeat(50)
        ));

        // Basic information
        output.push_str(&format!(
            "Migration ID: {}\n",
            self.style_text(&summary.migration_id, Color::White, false)
        ));

        output.push_str(&format!(
            "Status: {}\n",
            self.style_status(&format!("{:?}", summary.status))
        ));

        output.push_str(&format!(
            "Started: {}\n",
            summary.started_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(completed) = summary.completed_at {
            output.push_str(&format!(
                "Completed: {}\n",
                completed.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        if let Some(duration) = &summary.duration {
            output.push_str(&format!("Duration: {}\n", duration));
        }

        output.push('\n');

        // Operations summary
        output.push_str(&format!(
            "Operations: {} total, {} completed, {} failed, {} skipped\n",
            summary.total_operations,
            summary.completed_operations,
            summary.failed_operations,
            summary.skipped_operations
        ));

        // File processing summary
        output.push_str(&format!(
            "Files: {} processed, {} bytes transferred\n",
            summary.files_processed,
            self.format_bytes(summary.bytes_transferred)
        ));

        output.push_str(&format!(
            "Directories: {} created\n",
            summary.directories_created
        ));

        output.push_str(&format!("Symlinks: {} created\n", summary.symlinks_created));

        output.push('\n');

        // Errors and warnings
        if !summary.errors.is_empty() {
            output.push_str(&self.style_text("Errors:", Color::Red, true));
            output.push('\n');
            for error in &summary.errors {
                output.push_str(&format!("  • {}\n", error));
            }
            output.push('\n');
        }

        if !summary.warnings.is_empty() {
            output.push_str(&self.style_text("Warnings:", Color::Yellow, true));
            output.push('\n');
            for warning in &summary.warnings {
                output.push_str(&format!("  • {}\n", warning));
            }
        }

        Ok(output)
    }

    fn format_human_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format!(
            "{}\n{}\n\n",
            self.style_text("Migration Operations", Color::Cyan, true),
            "=".repeat(50)
        ));

        for (i, operation) in operations.iter().enumerate() {
            let status = self.style_status(&format!("{:?}", operation.status));

            output.push_str(&format!(
                "{:3}. {} -> {} ({})\n",
                i + 1,
                operation.source_path.display(),
                operation.target_path.display(),
                status
            ));

            if self.verbose {
                output.push_str(&format!(
                    "     Type: {:?}, Bytes: {}, Time: {}\n",
                    operation.operation_type,
                    self.format_bytes(operation.bytes_transferred),
                    operation.timestamp.format("%H:%M:%S")
                ));

                if let Some(error) = &operation.error_message {
                    output.push_str(&format!(
                        "     Error: {}\n",
                        self.style_text(error, Color::Red, false)
                    ));
                }
            }
        }

        Ok(output)
    }

    fn format_human_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format!(
            "{}\n{}\n\n",
            self.style_text("Backup Information", Color::Cyan, true),
            "=".repeat(50)
        ));

        for (key, value) in backup_info {
            output.push_str(&format!(
                "{}: {}\n",
                self.style_text(key, Color::Green, true),
                value
            ));
        }

        Ok(output)
    }

    fn format_human_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format!(
            "{}\n{}\n\n",
            self.style_text("Validation Report", Color::Cyan, true),
            "=".repeat(50)
        ));

        output.push_str(&format!("Migration ID: {}\n", report.migration_id));

        output.push_str(&format!(
            "Status: {}\n",
            self.style_validation_status(&report.overall_status)
        ));

        output.push_str(&format!(
            "Operations: {} total, {} valid, {} failed, {} skipped\n",
            report.total_operations,
            report.valid_operations,
            report.failed_operations,
            report.skipped_operations
        ));

        output.push_str(&format!(
            "Bytes validated: {}\n",
            self.format_bytes(report.summary_metrics.total_bytes_validated)
        ));

        output.push('\n');

        if !report.errors.is_empty() {
            output.push_str(&self.style_text("Validation Errors:", Color::Red, true));
            output.push('\n');
            for error in &report.errors {
                output.push_str(&format!(
                    "  • {} [{}]: {}\n",
                    error.error_type, error.severity, error.message
                ));
            }
        }

        Ok(output)
    }

    // JSON format methods

    fn format_json_summary(&self, summary: &MigrationSummary) -> Result<String> {
        serde_json::to_string(summary).with_context(|| "Failed to serialize summary to JSON")
    }

    fn format_pretty_json_summary(&self, summary: &MigrationSummary) -> Result<String> {
        serde_json::to_string_pretty(summary)
            .with_context(|| "Failed to serialize summary to pretty JSON")
    }

    fn format_json_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        serde_json::to_string(operations).with_context(|| "Failed to serialize operations to JSON")
    }

    fn format_pretty_json_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        serde_json::to_string_pretty(operations)
            .with_context(|| "Failed to serialize operations to pretty JSON")
    }

    fn format_json_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        serde_json::to_string(backup_info)
            .with_context(|| "Failed to serialize backup info to JSON")
    }

    fn format_pretty_json_backup_info(
        &self,
        backup_info: &HashMap<String, String>,
    ) -> Result<String> {
        serde_json::to_string_pretty(backup_info)
            .with_context(|| "Failed to serialize backup info to pretty JSON")
    }

    fn format_json_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        serde_json::to_string(report)
            .with_context(|| "Failed to serialize validation report to JSON")
    }

    fn format_pretty_json_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        serde_json::to_string_pretty(report)
            .with_context(|| "Failed to serialize validation report to pretty JSON")
    }

    // Table format methods

    fn format_table_summary(&self, summary: &MigrationSummary) -> Result<String> {
        let mut table = Table::new();
        table.set_format(TableFormat::new());

        table.add_row(row!["Migration Summary", ""]);
        table.add_empty_row();
        table.add_row(row!["Migration ID", &summary.migration_id]);
        table.add_row(row!["Status", format!("{:?}", summary.status)]);
        table.add_row(row![
            "Started",
            summary.started_at.format("%Y-%m-%d %H:%M:%S")
        ]);

        if let Some(completed) = summary.completed_at {
            table.add_row(row!["Completed", completed.format("%Y-%m-%d %H:%M:%S")]);
        }

        table.add_row(row![
            "Total Operations",
            summary.total_operations.to_string()
        ]);
        table.add_row(row!["Completed", summary.completed_operations.to_string()]);
        table.add_row(row!["Failed", summary.failed_operations.to_string()]);
        table.add_row(row!["Files Processed", summary.files_processed.to_string()]);
        table.add_row(row![
            "Bytes Transferred",
            self.format_bytes(summary.bytes_transferred)
        ]);

        Ok(table.to_string())
    }

    fn format_table_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        let mut table = Table::new();
        table.set_format(TableFormat::new());

        table.add_row(row![
            "#",
            "Operation",
            "Source",
            "Target",
            "Status",
            "Bytes"
        ]);
        table.add_empty_row();

        for (i, operation) in operations.iter().enumerate() {
            table.add_row(row![
                i + 1,
                format!("{:?}", operation.operation_type),
                operation.source_path.display(),
                operation.target_path.display(),
                format!("{:?}", operation.status),
                self.format_bytes(operation.bytes_transferred)
            ]);
        }

        Ok(table.to_string())
    }

    fn format_table_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        let mut table = Table::new();
        table.set_format(TableFormat::new());

        table.add_row(row!["Backup Information"]);
        table.add_empty_row();

        for (key, value) in backup_info {
            table.add_row(row![key, value]);
        }

        Ok(table.to_string())
    }

    fn format_table_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        let mut table = Table::new();
        table.set_format(TableFormat::new());

        table.add_row(row!["Validation Report"]);
        table.add_empty_row();
        table.add_row(row!["Migration ID", &report.migration_id]);
        table.add_row(row!["Status", format!("{:?}", report.overall_status)]);
        table.add_row(row![
            "Total Operations",
            report.total_operations.to_string()
        ]);
        table.add_row(row![
            "Valid Operations",
            report.valid_operations.to_string()
        ]);
        table.add_row(row![
            "Failed Operations",
            report.failed_operations.to_string()
        ]);
        table.add_row(row![
            "Bytes Validated",
            self.format_bytes(report.summary_metrics.total_bytes_validated)
        ]);

        Ok(table.to_string())
    }

    // CSV format methods

    fn format_csv_summary(&self, summary: &MigrationSummary) -> Result<String> {
        let mut output = Vec::new();

        // Header
        output.write_all(b"migration_id,status,started_at,completed_at,duration,total_operations,completed_operations,failed_operations,skipped_operations,bytes_transferred,files_processed\n")?;

        // Data row
        writeln!(
            &mut output,
            "{},{},{},{},{},{},{},{},{},{},{}",
            summary.migration_id,
            format!("{:?}", summary.status),
            summary.started_at.to_rfc3339(),
            summary
                .completed_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_default(),
            summary.duration.as_deref().unwrap_or(""),
            summary.total_operations,
            summary.completed_operations,
            summary.failed_operations,
            summary.skipped_operations,
            summary.bytes_transferred,
            summary.files_processed
        )?;

        Ok(String::from_utf8(output)?)
    }

    fn format_csv_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        let mut output = Vec::new();

        // Header
        output.write_all(b"id,operation_type,source_path,target_path,status,bytes_transferred,error_message,timestamp\n")?;

        // Data rows
        for operation in operations {
            writeln!(
                &mut output,
                "{},{},{},{},{},{},{},{}",
                operation.id,
                format!("{:?}", operation.operation_type),
                operation.source_path.display(),
                operation.target_path.display(),
                format!("{:?}", operation.status),
                operation.bytes_transferred,
                operation.error_message.as_deref().unwrap_or(""),
                operation.timestamp.to_rfc3339()
            )?;
        }

        Ok(String::from_utf8(output)?)
    }

    fn format_csv_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        let mut output = Vec::new();

        // Header
        output.write_all(b"key,value\n")?;

        // Data rows
        for (key, value) in backup_info {
            writeln!(&mut output, "{},{}", key, value)?;
        }

        Ok(String::from_utf8(output)?)
    }

    fn format_csv_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        let mut output = Vec::new();

        // Header
        output.write_all(b"migration_id,overall_status,total_operations,valid_operations,failed_operations,skipped_operations,total_bytes_validated,total_files_validated\n")?;

        // Data row
        writeln!(
            &mut output,
            "{},{},{},{},{},{},{},{}",
            report.migration_id,
            format!("{:?}", report.overall_status),
            report.total_operations,
            report.valid_operations,
            report.failed_operations,
            report.skipped_operations,
            report.summary_metrics.total_bytes_validated,
            report.summary_metrics.total_files_validated
        )?;

        Ok(String::from_utf8(output)?)
    }

    // YAML format methods

    fn format_yaml_summary(&self, summary: &MigrationSummary) -> Result<String> {
        serde_yaml::to_string(summary).with_context(|| "Failed to serialize summary to YAML")
    }

    fn format_yaml_operations(&self, operations: &[MigrationOperation]) -> Result<String> {
        serde_yaml::to_string(operations).with_context(|| "Failed to serialize operations to YAML")
    }

    fn format_yaml_backup_info(&self, backup_info: &HashMap<String, String>) -> Result<String> {
        serde_yaml::to_string(backup_info)
            .with_context(|| "Failed to serialize backup info to YAML")
    }

    fn format_yaml_validation_report(
        &self,
        report: &crate::validation::ValidationReport,
    ) -> Result<String> {
        serde_yaml::to_string(report)
            .with_context(|| "Failed to serialize validation report to YAML")
    }

    // Helper methods

    fn style_text(&self, text: &str, color: Color, bold: bool) -> String {
        if !self.use_colors {
            return text.to_string();
        }

        let mut style = style(text);
        style = style.fg(color);
        if bold {
            style = style.bold();
        }
        style.to_string()
    }

    fn style_status(&self, status: &str) -> String {
        let color = match status {
            "Completed" | "Passed" => Color::Green,
            "Failed" | "Error" => Color::Red,
            "InProgress" | "Pending" => Color::Yellow,
            _ => Color::White,
        };
        self.style_text(status, color, false)
    }

    fn style_validation_status(&self, status: &crate::validation::ValidationStatus) -> String {
        let status_str = match status {
            crate::validation::ValidationStatus::Passed => "Passed",
            crate::validation::ValidationStatus::Failed => "Failed",
            crate::validation::ValidationStatus::PartialSuccess => "Partial Success",
            crate::validation::ValidationStatus::Skipped => "Skipped",
        };
        self.style_status(status_str)
    }

    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_output_formatter_creation() {
        let formatter = OutputFormatter::new(OutputFormat::Human, true, false);
        assert!(formatter.use_colors);
        assert!(!formatter.verbose);
    }

    #[test]
    fn test_json_format_summary() {
        let formatter = OutputFormatter::new(OutputFormat::Json, false, false);
        let summary = MigrationSummary {
            migration_id: "test-001".to_string(),
            status: MigrationStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration: Some("00:01:30".to_string()),
            total_operations: 10,
            completed_operations: 10,
            failed_operations: 0,
            skipped_operations: 0,
            bytes_transferred: 1024,
            files_processed: 5,
            directories_created: 2,
            symlinks_created: 1,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let result = formatter.format_migration_summary(&summary).unwrap();
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn test_human_format_summary() {
        let formatter = OutputFormatter::new(OutputFormat::Human, true, true);
        let summary = MigrationSummary {
            migration_id: "test-001".to_string(),
            status: MigrationStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration: Some("00:01:30".to_string()),
            total_operations: 10,
            completed_operations: 10,
            failed_operations: 0,
            skipped_operations: 0,
            bytes_transferred: 1024,
            files_processed: 5,
            directories_created: 2,
            symlinks_created: 1,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let result = formatter.format_migration_summary(&summary).unwrap();
        assert!(result.contains("Migration Summary"));
        assert!(result.contains("test-001"));
        assert!(result.contains("Completed"));
    }

    #[test]
    fn test_bytes_formatting() {
        let formatter = OutputFormatter::new(OutputFormat::Human, false, false);

        assert_eq!(formatter.format_bytes(512), "512 B");
        assert_eq!(formatter.format_bytes(1024), "1.0 KB");
        assert_eq!(formatter.format_bytes(1536), "1.5 KB");
        assert_eq!(formatter.format_bytes(1048576), "1.0 MB");
        assert_eq!(formatter.format_bytes(1073741824), "1.0 GB");
    }
}
