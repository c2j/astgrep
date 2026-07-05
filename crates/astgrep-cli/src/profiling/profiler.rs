//! Performance profiling infrastructure for migration operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use crate::services::migration_orchestrator::MigrationOperation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    pub enabled: bool,
    pub output_file: Option<String>,
    pub include_memory_usage: bool,
    pub include_io_stats: bool,
    pub sample_interval_ms: u64,
    pub detailed_tracing: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_file: Some("migration_profile.json".to_string()),
            include_memory_usage: true,
            include_io_stats: true,
            sample_interval_ms: 100,
            detailed_tracing: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetrics {
    pub migration_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_duration: Option<Duration>,
    pub operations_completed: usize,
    pub operations_failed: usize,
    pub bytes_transferred: u64,
    pub files_processed: usize,
    pub directories_created: usize,
    pub peak_memory_usage_mb: f64,
    pub average_cpu_usage_percent: f64,
    pub io_operations: IoStats,
    pub thread_metrics: ThreadMetrics,
    pub operation_metrics: Vec<OperationMetric>,
    pub checkpoints: Vec<Checkpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub read_operations: u64,
    pub write_operations: u64,
    pub seek_operations: u64,
    pub sync_operations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMetrics {
    pub max_concurrent_threads: usize,
    pub average_thread_utilization: f64,
    pub thread_pool_efficiency: f64,
    pub total_wait_time: Duration,
    pub lock_contention_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetric {
    pub operation_id: String,
    pub operation_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub bytes_processed: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub memory_usage_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub elapsed_time: Duration,
    pub operations_completed: usize,
    pub bytes_transferred: u64,
    pub memory_usage_mb: f64,
    pub custom_metrics: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PerformanceProfiler {
    config: ProfilingConfig,
    metrics: Arc<Mutex<ProfileMetrics>>,
    start_time: Option<Instant>,
    active_operations: Arc<Mutex<HashMap<String, Instant>>>,
    memory_samples: Arc<Mutex<Vec<(DateTime<Utc>, f64)>>>,
    cpu_samples: Arc<Mutex<Vec<(DateTime<Utc>, f64)>>>,
}

impl PerformanceProfiler {
    pub fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(ProfileMetrics {
                migration_id: uuid::Uuid::new_v4().to_string(),
                start_time: Utc::now(),
                end_time: None,
                total_duration: None,
                operations_completed: 0,
                operations_failed: 0,
                bytes_transferred: 0,
                files_processed: 0,
                directories_created: 0,
                peak_memory_usage_mb: 0.0,
                average_cpu_usage_percent: 0.0,
                io_operations: IoStats {
                    bytes_read: 0,
                    bytes_written: 0,
                    read_operations: 0,
                    write_operations: 0,
                    seek_operations: 0,
                    sync_operations: 0,
                },
                thread_metrics: ThreadMetrics {
                    max_concurrent_threads: 0,
                    average_thread_utilization: 0.0,
                    thread_pool_efficiency: 0.0,
                    total_wait_time: Duration::ZERO,
                    lock_contention_time: Duration::ZERO,
                },
                operation_metrics: Vec::new(),
                checkpoints: Vec::new(),
            })),
            start_time: None,
            active_operations: Arc::new(Mutex::new(HashMap::new())),
            memory_samples: Arc::new(Mutex::new(Vec::new())),
            cpu_samples: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start profiling session
    pub fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.start_time = Some(Instant::now());
        info!("Performance profiling started");

        if self.config.include_memory_usage {
            self.start_memory_sampling()?;
        }

        if self.config.detailed_tracing {
            self.start_cpu_sampling()?;
        }

        Ok(())
    }

    /// Stop profiling session and generate report
    pub async fn stop(&self) -> Result<ProfileMetrics> {
        if !self.config.enabled {
            return Ok(self.metrics.lock().unwrap().clone());
        }

        let duration = self
            .start_time
            .ok_or_else(|| anyhow::anyhow!("Profiling was not started"))?
            .elapsed();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.end_time = Some(Utc::now());
            metrics.total_duration = Some(duration);
        }

        // Calculate final averages outside the lock
        let averages = self.calculate_averages().await;
        if let (Some(avg_memory), Some(avg_cpu)) = averages {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.peak_memory_usage_mb = avg_memory;
            metrics.average_cpu_usage_percent = avg_cpu;
        }

        info!("Performance profiling stopped: {:?}", duration);

        // Write profiling data to file if configured
        if let Some(output_file) = &self.config.output_file {
            let metrics = self.metrics.lock().unwrap().clone();
            self.write_profile_report(&metrics, output_file).await?;
        }

        Ok(self.metrics.lock().unwrap().clone())
    }

    /// Record the start of an operation
    pub fn start_operation(&self, operation_id: String, operation_type: &str) {
        if !self.config.enabled {
            return;
        }

        self.active_operations
            .lock()
            .unwrap()
            .insert(operation_id.clone(), Instant::now());

        if self.config.detailed_tracing {
            debug!(
                "Started profiling operation: {} ({})",
                operation_id, operation_type
            );
        }
    }

    /// Record the completion of an operation
    pub fn end_operation(
        &self,
        operation: &MigrationOperation,
        bytes_processed: u64,
        success: bool,
    ) {
        if !self.config.enabled {
            return;
        }

        let start_time = {
            let mut active = self.active_operations.lock().unwrap();
            active.remove(&operation.id).unwrap_or_else(Instant::now)
        };

        let duration = start_time.elapsed();
        let memory_usage = self.get_current_memory_usage();

        let metric = OperationMetric {
            operation_id: operation.id.clone(),
            operation_type: format!("{:?}", operation.operation_type),
            start_time: Utc::now() - duration,
            end_time: Utc::now(),
            duration,
            bytes_processed,
            success,
            error_message: operation.error_message.clone(),
            memory_usage_mb: memory_usage,
        };

        let mut metrics = self.metrics.lock().unwrap();
        metrics.operation_metrics.push(metric);

        if success {
            metrics.operations_completed += 1;
            metrics.bytes_transferred += bytes_processed;
        } else {
            metrics.operations_failed += 1;
        }

        match operation.operation_type {
            crate::services::migration_orchestrator::OperationType::Copy
            | crate::services::migration_orchestrator::OperationType::Move => {
                metrics.files_processed += 1;
            }
            crate::services::migration_orchestrator::OperationType::CreateDirectory => {
                metrics.directories_created += 1;
            }
            _ => {}
        }

        if self.config.detailed_tracing {
            debug!(
                "Completed profiling operation: {} in {:?}",
                operation.id, duration
            );
        }
    }

    /// Record a checkpoint in the profiling timeline
    pub fn record_checkpoint(&self, name: &str, custom_metrics: HashMap<String, String>) {
        if !self.config.enabled {
            return;
        }

        let elapsed = self.start_time.unwrap_or_else(Instant::now).elapsed();
        let memory_usage = self.get_current_memory_usage();
        let ops = self.metrics.lock().unwrap().operations_completed;
        let bytes = self.metrics.lock().unwrap().bytes_transferred;

        let checkpoint = Checkpoint {
            name: name.to_string(),
            timestamp: Utc::now(),
            elapsed_time: elapsed,
            operations_completed: ops,
            bytes_transferred: bytes,
            memory_usage_mb: memory_usage,
            custom_metrics,
        };

        self.metrics.lock().unwrap().checkpoints.push(checkpoint);

        if self.config.detailed_tracing {
            debug!("Recorded checkpoint: {} at {:?}", name, elapsed);
        }
    }

    /// Update I/O statistics
    pub fn update_io_stats(
        &self,
        bytes_read: u64,
        bytes_written: u64,
        read_ops: u64,
        write_ops: u64,
    ) {
        if !self.config.enabled || !self.config.include_io_stats {
            return;
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.io_operations.bytes_read += bytes_read;
        metrics.io_operations.bytes_written += bytes_written;
        metrics.io_operations.read_operations += read_ops;
        metrics.io_operations.write_operations += write_ops;
    }

    /// Update thread metrics
    pub fn update_thread_metrics(
        &self,
        concurrent_threads: usize,
        utilization: f64,
        efficiency: f64,
    ) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.thread_metrics.max_concurrent_threads = metrics
            .thread_metrics
            .max_concurrent_threads
            .max(concurrent_threads);
        metrics.thread_metrics.average_thread_utilization =
            (metrics.thread_metrics.average_thread_utilization + utilization) / 2.0;
        metrics.thread_metrics.thread_pool_efficiency =
            (metrics.thread_metrics.thread_pool_efficiency + efficiency) / 2.0;
    }

    /// Get current memory usage in MB
    fn get_current_memory_usage(&self) -> f64 {
        #[cfg(unix)]
        {
            use std::fs;
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb as f64 / 1024.0;
                            }
                        }
                    }
                }
            }
        }

        // Fallback for non-Unix systems or if /proc is unavailable
        0.0
    }

    /// Start memory sampling thread
    fn start_memory_sampling(&self) -> Result<()> {
        let interval_ms = self.config.sample_interval_ms;
        let samples = Arc::clone(&self.memory_samples);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));

            loop {
                interval.tick().await;

                let memory_usage = Self::get_static_memory_usage();
                let timestamp = Utc::now();

                samples.lock().unwrap().push((timestamp, memory_usage));

                // Keep only last 1000 samples to prevent memory leak
                let mut samples_guard = samples.lock().unwrap();
                if samples_guard.len() > 1000 {
                    samples_guard.drain(0..100);
                }
            }
        });

        Ok(())
    }

    /// Static version of memory usage for the sampling thread
    fn get_static_memory_usage() -> f64 {
        #[cfg(unix)]
        {
            use std::fs;
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb as f64 / 1024.0;
                            }
                        }
                    }
                }
            }
        }
        0.0
    }

    /// Start CPU sampling thread
    fn start_cpu_sampling(&self) -> Result<()> {
        let interval_ms = self.config.sample_interval_ms;
        let samples = Arc::clone(&self.cpu_samples);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));

            loop {
                interval.tick().await;

                // This is a simplified CPU usage measurement
                // In a real implementation, you'd use platform-specific APIs
                let cpu_usage = Self::get_static_cpu_usage();
                let timestamp = Utc::now();

                samples.lock().unwrap().push((timestamp, cpu_usage));

                // Keep only last 1000 samples
                let mut samples_guard = samples.lock().unwrap();
                if samples_guard.len() > 1000 {
                    samples_guard.drain(0..100);
                }
            }
        });

        Ok(())
    }

    /// Static version of CPU usage for the sampling thread
    fn get_static_cpu_usage() -> f64 {
        // Simplified CPU usage measurement
        // In a real implementation, you'd use platform-specific APIs
        use std::thread;
        use std::time::Instant;

        let start = Instant::now();
        thread::sleep(Duration::from_millis(1));
        let elapsed = start.elapsed();

        // This is a very rough approximation
        (elapsed.as_nanos() as f64 / Duration::from_millis(1).as_nanos() as f64) * 100.0
    }

    /// Calculate averages from sampled data
    async fn calculate_averages(&self) -> (Option<f64>, Option<f64>) {
        let memory_guard = self.memory_samples.lock().unwrap();
        let cpu_guard = self.cpu_samples.lock().unwrap();

        let avg_memory = if memory_guard.is_empty() {
            None
        } else {
            let sum: f64 = memory_guard.iter().map(|(_, usage)| *usage).sum();
            Some(sum / memory_guard.len() as f64)
        };

        let avg_cpu = if cpu_guard.is_empty() {
            None
        } else {
            let sum: f64 = cpu_guard.iter().map(|(_, usage)| *usage).sum();
            Some(sum / cpu_guard.len() as f64)
        };

        (avg_memory, avg_cpu)
    }

    /// Write profiling report to file
    async fn write_profile_report(
        &self,
        metrics: &ProfileMetrics,
        output_file: &str,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(metrics)
            .with_context(|| "Failed to serialize profiling metrics")?;

        let mut file = fs::File::create(output_file)
            .await
            .with_context(|| format!("Failed to create profiling report file: {}", output_file))?;

        file.write_all(json.as_bytes())
            .await
            .with_context(|| "Failed to write profiling report")?;

        info!("Profiling report written to: {}", output_file);
        Ok(())
    }

    /// Get current metrics snapshot
    pub fn get_metrics_snapshot(&self) -> ProfileMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_config_default() {
        let config = ProfilingConfig::default();
        assert!(!config.enabled);
        assert!(config.output_file.is_some());
        assert!(config.include_memory_usage);
        assert!(config.include_io_stats);
        assert_eq!(config.sample_interval_ms, 100);
    }

    #[test]
    fn test_profiler_lifecycle() {
        let config = ProfilingConfig {
            enabled: true,
            output_file: None,
            include_memory_usage: false,
            include_io_stats: false,
            sample_interval_ms: 50,
            detailed_tracing: false,
        };

        let mut profiler = PerformanceProfiler::new(config);
        profiler.start().unwrap();
        profiler.record_checkpoint("test_checkpoint", HashMap::new());

        let rt = tokio::runtime::Runtime::new().expect("runtime creation");
        let metrics = rt.block_on(profiler.stop()).expect("stop");
        assert!(metrics.end_time.is_some());
        assert!(metrics.total_duration.is_some());
        assert_eq!(metrics.checkpoints.len(), 1);
        assert_eq!(metrics.checkpoints[0].name, "test_checkpoint");
    }

    #[test]
    fn test_operation_tracking() {
        let config = ProfilingConfig {
            enabled: true,
            output_file: None,
            include_memory_usage: false,
            include_io_stats: false,
            sample_interval_ms: 50,
            detailed_tracing: false,
        };

        let profiler = PerformanceProfiler::new(config);

        let operation = crate::services::migration_orchestrator::MigrationOperation {
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

        profiler.start_operation("test-op-001".to_string(), "Copy");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_operation(&operation, 1024, true);

        let metrics = profiler.get_metrics_snapshot();
        assert_eq!(metrics.operations_completed, 1);
        assert_eq!(metrics.operations_failed, 0);
        assert_eq!(metrics.bytes_transferred, 1024);
        assert_eq!(metrics.operation_metrics.len(), 1);
    }
}
