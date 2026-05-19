//! Progress tracking and reporting infrastructure for migration operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::services::migration_orchestrator::MigrationOperation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressConfig {
    pub enabled: bool,
    pub show_percentage: bool,
    pub show_eta: bool,
    pub show_rate: bool,
    pub show_elapsed: bool,
    pub show_items_remaining: bool,
    pub update_interval_ms: u64,
    pub progress_style: ProgressStyleType,
    pub log_level: ProgressLogLevel,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_percentage: true,
            show_eta: true,
            show_rate: true,
            show_elapsed: true,
            show_items_remaining: true,
            update_interval_ms: 100,
            progress_style: ProgressStyleType::Default,
            log_level: ProgressLogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressStyleType {
    Default,
    Simple,
    Detailed,
    Compact,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    pub migration_id: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub current_operation: Option<CurrentOperation>,
    pub phases: Vec<PhaseProgress>,
    pub metrics: ProgressMetrics,
    pub status: MigrationProgressStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentOperation {
    pub operation_id: String,
    pub operation_type: String,
    pub description: String,
    pub started_at: DateTime<Utc>,
    pub progress_percentage: f64,
    pub bytes_processed: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProgress {
    pub phase_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_items: usize,
    pub completed_items: usize,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMetrics {
    pub total_bytes_processed: u64,
    pub total_bytes_migrated: u64,
    pub operations_per_second: f64,
    pub bytes_per_second: f64,
    pub average_operation_time_ms: f64,
    pub fastest_operation_time_ms: f64,
    pub slowest_operation_time_ms: f64,
    pub error_rate: f64,
    pub estimated_completion_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationProgressStatus {
    NotStarted,
    Initializing,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    MigrationStarted(String),
    MigrationCompleted(String),
    MigrationFailed(String, String),
    PhaseStarted(String, String),
    PhaseCompleted(String, String),
    OperationStarted(String, String),
    OperationProgress(String, f64, u64, u64),
    OperationCompleted(String, String),
    OperationFailed(String, String, String),
    MetricsUpdate(String, ProgressMetrics),
}

pub struct ProgressTracker {
    config: ProgressConfig,
    progress: Arc<Mutex<MigrationProgress>>,
    event_sender: mpsc::UnboundedSender<ProgressEvent>,
    progress_bar: Option<ProgressBar>,
    operation_times: Arc<Mutex<Vec<Duration>>>,
}

impl ProgressTracker {
    pub fn new(config: ProgressConfig) -> Self {
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();

        let progress = Arc::new(Mutex::new(MigrationProgress {
            migration_id: String::new(),
            started_at: Utc::now(),
            updated_at: Utc::now(),
            total_operations: 0,
            completed_operations: 0,
            failed_operations: 0,
            skipped_operations: 0,
            current_operation: None,
            phases: Vec::new(),
            metrics: ProgressMetrics {
                total_bytes_processed: 0,
                total_bytes_migrated: 0,
                operations_per_second: 0.0,
                bytes_per_second: 0.0,
                average_operation_time_ms: 0.0,
                fastest_operation_time_ms: 0.0,
                slowest_operation_time_ms: 0.0,
                error_rate: 0.0,
                estimated_completion_time: None,
            },
            status: MigrationProgressStatus::NotStarted,
        }));

        let operation_times = Arc::new(Mutex::new(Vec::new()));

        // Clone references for the event processing task
        let progress_clone = Arc::clone(&progress);
        let operation_times_clone = Arc::clone(&operation_times);
        let config_clone = config.clone();

        // Spawn event processing task
        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                Self::handle_progress_event(event, &progress_clone, &operation_times_clone, &config_clone).await;
            }
        });

        let tracker = Self {
            config,
            progress,
            event_sender,
            progress_bar: None,
            operation_times,
        };

        tracker
    }

    /// Initialize migration progress tracking
    pub fn initialize_migration(&mut self, migration_id: String, total_operations: usize) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut progress = self.progress.lock().unwrap();
        progress.migration_id = migration_id.clone();
        progress.total_operations = total_operations;
        progress.status = MigrationProgressStatus::Initializing;
        progress.started_at = Utc::now();

        // Create progress bar if enabled
        if total_operations > 0 {
            self.progress_bar = Some(self.create_progress_bar(total_operations as u64));
        }

        // Send migration started event
        let _ = self.event_sender.send(ProgressEvent::MigrationStarted(migration_id.clone()));

        info!("Progress tracking initialized for migration: {} ({} operations)",
              migration_id, total_operations);

        Ok(())
    }

    /// Start tracking a new phase
    pub fn start_phase(&self, phase_name: String, total_items: usize) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Add phase to progress tracking
        let mut progress = self.progress.lock().unwrap();
        let migration_id = progress.migration_id.clone();

        progress.phases.push(PhaseProgress {
            phase_name: phase_name.clone(),
            started_at: Utc::now(),
            completed_at: None,
            total_items,
            completed_items: 0,
            status: PhaseStatus::InProgress,
        });

        // Send phase started event
        let _ = self.event_sender.send(ProgressEvent::PhaseStarted(migration_id, phase_name));

        Ok(())
    }

    /// Complete a phase
    pub fn complete_phase(&self, phase_name: String) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Update phase status
        let mut progress = self.progress.lock().unwrap();
        let migration_id = progress.migration_id.clone();

        if let Some(phase) = progress.phases.iter_mut().find(|p| p.phase_name == phase_name) {
            phase.status = PhaseStatus::Completed;
            phase.completed_at = Some(Utc::now());
            phase.completed_items = phase.total_items;
        }

        // Send phase completed event
        let _ = self.event_sender.send(ProgressEvent::PhaseCompleted(migration_id, phase_name));

        Ok(())
    }

    /// Start tracking an operation
    pub fn start_operation(&self, operation: &MigrationOperation) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let migration_id = self.progress.lock().unwrap().migration_id.clone();

        // Update current operation
        let mut progress = self.progress.lock().unwrap();
        progress.current_operation = Some(CurrentOperation {
            operation_id: operation.id.clone(),
            operation_type: format!("{:?}", operation.operation_type),
            description: format!("{:?}", operation.source_path),
            started_at: Utc::now(),
            progress_percentage: 0.0,
            bytes_processed: 0,
            bytes_total: 0,
        });

        progress.status = MigrationProgressStatus::InProgress;

        // Send operation started event
        let _ = self.event_sender.send(ProgressEvent::OperationStarted(
            migration_id,
            operation.id.clone(),
        ));

        Ok(())
    }

    /// Update operation progress
    pub fn update_operation_progress(&self, operation_id: String, progress_percentage: f64, bytes_processed: u64, bytes_total: u64) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let _migration_id = self.progress.lock().unwrap().migration_id.clone();

        // Update current operation progress
        let mut progress = self.progress.lock().unwrap();
        if let Some(current_op) = &mut progress.current_operation {
            if current_op.operation_id == operation_id {
                current_op.progress_percentage = progress_percentage;
                current_op.bytes_processed = bytes_processed;
                current_op.bytes_total = bytes_total;
            }
        }

        // Update progress bar
        if let Some(ref bar) = self.progress_bar {
            let total_progress = (progress.completed_operations as f64 + progress_percentage) / progress.total_operations as f64 * 100.0;
            bar.set_position(total_progress as u64);
            bar.set_message(format!("Processing {} ({:.1}%)", operation_id, progress_percentage));
        }

        // Send operation progress event
        let _ = self.event_sender.send(ProgressEvent::OperationProgress(
            operation_id,
            progress_percentage,
            bytes_processed,
            bytes_total,
        ));

        Ok(())
    }

    /// Complete an operation successfully
    pub fn complete_operation(&self, operation: &MigrationOperation, duration: Duration) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let migration_id = self.progress.lock().unwrap().migration_id.clone();

        // Update operation statistics
        {
            let mut progress = self.progress.lock().unwrap();
            progress.completed_operations += 1;
            progress.metrics.total_bytes_migrated += operation.bytes_transferred;

            // Update operation times for metrics calculation
            let mut times = self.operation_times.lock().unwrap();
            times.push(duration);
        }

        // Update progress bar
        if let Some(ref bar) = self.progress_bar {
            let progress = self.progress.lock().unwrap();
            let percentage = progress.completed_operations as f64 / progress.total_operations as f64 * 100.0;
            bar.set_position(percentage as u64);
            bar.set_message(format!("Completed {} ({}/{})",
                                   operation.id,
                                   progress.completed_operations,
                                   progress.total_operations));
        }

        // Send operation completed event
        let _ = self.event_sender.send(ProgressEvent::OperationCompleted(
            migration_id,
            operation.id.clone(),
        ));

        debug!("Operation completed: {}", operation.id);
        Ok(())
    }

    /// Mark an operation as failed
    pub fn fail_operation(&self, operation: &MigrationOperation, error_message: &str, duration: Duration) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let migration_id = self.progress.lock().unwrap().migration_id.clone();

        // Update operation statistics
        {
            let mut progress = self.progress.lock().unwrap();
            progress.failed_operations += 1;

            // Update operation times for metrics calculation
            let mut times = self.operation_times.lock().unwrap();
            times.push(duration);
        }

        // Send operation failed event
        let _ = self.event_sender.send(ProgressEvent::OperationFailed(
            migration_id,
            operation.id.clone(),
            error_message.to_string(),
        ));

        warn!("Operation failed: {} - {}", operation.id, error_message);
        Ok(())
    }

    /// Complete the migration
    pub fn complete_migration(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let migration_id = self.progress.lock().unwrap().migration_id.clone();

        // Update migration status
        {
            let mut progress = self.progress.lock().unwrap();
            progress.status = MigrationProgressStatus::Completed;
            progress.updated_at = Utc::now();
        }

        // Finish progress bar
        if let Some(ref bar) = self.progress_bar {
            bar.finish_with_message("Migration completed");
        }

        // Send migration completed event
        let _ = self.event_sender.send(ProgressEvent::MigrationCompleted(migration_id));

        info!("Migration completed successfully");
        Ok(())
    }

    /// Get current progress snapshot
    pub fn get_progress(&self) -> MigrationProgress {
        self.progress.lock().unwrap().clone()
    }

    /// Reset progress tracking
    pub fn reset(&mut self) {
        let mut progress = self.progress.lock().unwrap();
        progress.migration_id.clear();
        progress.total_operations = 0;
        progress.completed_operations = 0;
        progress.failed_operations = 0;
        progress.skipped_operations = 0;
        progress.current_operation = None;
        progress.phases.clear();
        progress.metrics = ProgressMetrics {
            total_bytes_processed: 0,
            total_bytes_migrated: 0,
            operations_per_second: 0.0,
            bytes_per_second: 0.0,
            average_operation_time_ms: 0.0,
            fastest_operation_time_ms: 0.0,
            slowest_operation_time_ms: 0.0,
            error_rate: 0.0,
            estimated_completion_time: None,
        };
        progress.status = MigrationProgressStatus::NotStarted;

        if let Some(ref bar) = self.progress_bar {
            bar.finish_and_clear();
        }
        self.progress_bar = None;
    }

    // Private helper methods

    fn create_progress_bar(&self, total: u64) -> ProgressBar {
        let style = match self.config.progress_style {
            ProgressStyleType::Default => ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .expect("Invalid template")
                .progress_chars("#>-"),
            ProgressStyleType::Simple => ProgressStyle::default_bar()
                .template("{bar:40} {pos}/{len}")
                .expect("Invalid template")
                .progress_chars("=> "),
            ProgressStyleType::Detailed => ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg} | {bytes}/{total_bytes} ({bytes_per_sec})")
                .expect("Invalid template")
                .progress_chars("#>-"),
            ProgressStyleType::Compact => ProgressStyle::default_bar()
                .template("{percent}% [{bar:20}] {pos}/{len}")
                .expect("Invalid template")
                .progress_chars("=> "),
            ProgressStyleType::Custom(ref template) => ProgressStyle::default_bar()
                .template(template)
                .expect("Invalid template")
                .progress_chars("#>-"),
        };

        ProgressBar::new(total)
            .with_style(style)
            .with_message("Initializing...")
    }

    async fn handle_progress_event(
        event: ProgressEvent,
        progress: &Arc<Mutex<MigrationProgress>>,
        operation_times: &Arc<Mutex<Vec<Duration>>>,
        config: &ProgressConfig,
    ) {
        let mut progress = progress.lock().unwrap();

        match event {
            ProgressEvent::MigrationStarted(migration_id) => {
                progress.migration_id = migration_id;
                progress.status = MigrationProgressStatus::InProgress;
            }
            ProgressEvent::MigrationCompleted(_) => {
                progress.status = MigrationProgressStatus::Completed;
            }
            ProgressEvent::MigrationFailed(_, _) => {
                progress.status = MigrationProgressStatus::Failed;
            }
            ProgressEvent::PhaseStarted(phase_name, _) => {
                // Phase tracking is handled in the main thread
                debug!("Phase started: {}", phase_name);
            }
            ProgressEvent::PhaseCompleted(phase_name, _) => {
                // Phase tracking is handled in the main thread
                debug!("Phase completed: {}", phase_name);
            }
            ProgressEvent::OperationStarted(_, _) => {
                // Operation start tracking is handled in the main thread
            }
            ProgressEvent::OperationProgress(_, _, _, _) => {
                // Operation progress is handled in the main thread
            }
            ProgressEvent::OperationCompleted(_, _) => {
                // Operation completion tracking is handled in the main thread
            }
            ProgressEvent::OperationFailed(_, _, _) => {
                // Operation failure tracking is handled in the main thread
            }
            ProgressEvent::MetricsUpdate(_, metrics) => {
                progress.metrics = metrics;
            }
        }

        progress.updated_at = Utc::now();

        // Calculate derived metrics
        Self::update_derived_metrics(&mut progress, operation_times);

        // Log progress based on configured log level
        Self::log_progress(&progress, config);
    }

    fn update_derived_metrics(progress: &mut MigrationProgress, operation_times: &Arc<Mutex<Vec<Duration>>>) {
        let times = operation_times.lock().unwrap();

        if !times.is_empty() {
            let total_time: Duration = times.iter().sum();
            progress.metrics.average_operation_time_ms = total_time.as_millis() as f64 / times.len() as f64;
            progress.metrics.fastest_operation_time_ms = times.iter()
                .min()
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);
            progress.metrics.slowest_operation_time_ms = times.iter()
                .max()
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);
        }

        // Calculate operations per second
        let elapsed = progress.updated_at.signed_duration_since(progress.started_at);
        if elapsed.num_milliseconds() > 0 {
            progress.metrics.operations_per_second = progress.completed_operations as f64 / (elapsed.num_milliseconds() as f64 / 1000.0);
            progress.metrics.bytes_per_second = progress.metrics.total_bytes_migrated as f64 / (elapsed.num_milliseconds() as f64 / 1000.0);
        }

        // Calculate error rate
        let total_processed = progress.completed_operations + progress.failed_operations;
        if total_processed > 0 {
            progress.metrics.error_rate = progress.failed_operations as f64 / total_processed as f64;
        }

        // Estimate completion time
        if progress.completed_operations > 0 && progress.total_operations > progress.completed_operations {
            let remaining_operations = progress.total_operations - progress.completed_operations;
            let operations_per_second = progress.metrics.operations_per_second;
            if operations_per_second > 0.0 {
                let estimated_seconds = remaining_operations as f64 / operations_per_second;
                progress.metrics.estimated_completion_time = Some(
                    progress.updated_at + chrono::Duration::seconds(estimated_seconds as i64)
                );
            }
        }
    }

    fn log_progress(progress: &MigrationProgress, config: &ProgressConfig) {
        let log_level = match config.log_level {
            ProgressLogLevel::Debug => tracing::Level::DEBUG,
            ProgressLogLevel::Info => tracing::Level::INFO,
            ProgressLogLevel::Warn => tracing::Level::WARN,
            ProgressLogLevel::Error => tracing::Level::ERROR,
        };

        if tracing::level_enabled!(log_level) {
            let message = format!(
                "Migration {}: {}/{} operations completed ({} failed), {:.1} ops/sec, {} processed",
                progress.migration_id,
                progress.completed_operations,
                progress.total_operations,
                progress.failed_operations,
                progress.metrics.operations_per_second,
                Self::format_bytes(progress.metrics.total_bytes_migrated)
            );

            match log_level {
                tracing::Level::DEBUG => debug!("{}", message),
                tracing::Level::INFO => info!("{}", message),
                tracing::Level::WARN => warn!("{}", message),
                tracing::Level::ERROR => error!("{}", message),
                _ => {}
            }
        }
    }

    fn format_bytes(bytes: u64) -> String {
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

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if let Some(ref bar) = self.progress_bar {
            bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_progress_tracker_initialization() {
        let config = ProgressConfig::default();
        let mut tracker = ProgressTracker::new(config);

        let migration_id = "test-migration".to_string();
        tracker.initialize_migration(migration_id.clone(), 10).unwrap();

        let progress = tracker.get_progress();
        assert_eq!(progress.migration_id, migration_id);
        assert_eq!(progress.total_operations, 10);
        assert!(matches!(progress.status, MigrationProgressStatus::Initializing));
    }

    #[tokio::test]
    async fn test_phase_tracking() {
        let config = ProgressConfig::default();
        let mut tracker = ProgressTracker::new(config);

        tracker.initialize_migration("test".to_string(), 5).unwrap();
        tracker.start_phase("test-phase".to_string(), 10).unwrap();
        tracker.complete_phase("test-phase".to_string()).unwrap();

        let progress = tracker.get_progress();
        assert_eq!(progress.phases.len(), 1);
        assert!(matches!(progress.phases[0].status, PhaseStatus::Completed));
    }

    #[tokio::test]
    async fn test_operation_tracking() {
        let config = ProgressConfig::default();
        let mut tracker = ProgressTracker::new(config);

        tracker.initialize_migration("test".to_string(), 1).unwrap();

        let operation = MigrationOperation {
            id: "test-op-001".to_string(),
            source_path: "/source/test".into(),
            target_path: "/target/test".into(),
            operation_type: crate::services::migration_orchestrator::OperationType::Copy,
            status: crate::services::migration_orchestrator::OperationStatus::Pending,
            error_message: None,
            bytes_transferred: 0,
            checksum_before: None,
            checksum_after: None,
            timestamp: Utc::now(),
        };

        tracker.start_operation(&operation).unwrap();
        tracker.complete_operation(&operation, Duration::from_millis(100)).unwrap();

        let progress = tracker.get_progress();
        assert_eq!(progress.completed_operations, 1);
        assert_eq!(progress.failed_operations, 0);
    }

    #[test]
    fn test_progress_config_default() {
        let config = ProgressConfig::default();
        assert!(config.enabled);
        assert!(config.show_percentage);
        assert!(config.show_eta);
        assert!(config.show_rate);
        assert!(config.show_elapsed);
        assert!(config.show_items_remaining);
        assert_eq!(config.update_interval_ms, 100);
        assert!(matches!(config.progress_style, ProgressStyleType::Default));
        assert!(matches!(config.log_level, ProgressLogLevel::Info));
    }
}