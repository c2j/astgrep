//! Performance profiler for CR-SemService

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Performance profiler for tracking operation timings and metrics
#[derive(Debug, Clone)]
pub struct PerformanceProfiler {
    operations: HashMap<String, Vec<Duration>>,
    start_time: Instant,
    memory_snapshots: Vec<MemorySnapshot>,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
            start_time: Instant::now(),
            memory_snapshots: Vec::new(),
        }
    }

    /// Time an operation and record its duration
    pub fn time_operation<F, R>(&mut self, operation_name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        
        self.operations
            .entry(operation_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        result
    }

    /// Start timing an operation (for async operations)
    pub fn start_operation(&self, _operation_name: &str) -> OperationTimer {
        OperationTimer::new()
    }

    /// Record the completion of an operation
    pub fn end_operation(&mut self, operation_name: &str, timer: OperationTimer) {
        let duration = timer.elapsed();
        self.operations
            .entry(operation_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Take a memory snapshot
    pub fn snapshot_memory(&mut self, label: String) {
        let snapshot = MemorySnapshot {
            label,
            timestamp: self.start_time.elapsed(),
            memory_usage: get_memory_usage(),
        };
        self.memory_snapshots.push(snapshot);
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let mut operation_stats = HashMap::new();
        
        for (operation, durations) in &self.operations {
            let stats = calculate_duration_stats(durations);
            operation_stats.insert(operation.clone(), stats);
        }

        PerformanceMetrics {
            total_time: self.start_time.elapsed(),
            operation_stats,
            memory_snapshots: self.memory_snapshots.clone(),
            peak_memory: self.memory_snapshots
                .iter()
                .map(|s| s.memory_usage)
                .max()
                .unwrap_or(0),
        }
    }

    /// Reset all collected metrics
    pub fn reset(&mut self) {
        self.operations.clear();
        self.memory_snapshots.clear();
        self.start_time = Instant::now();
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for individual operations
#[derive(Debug)]
pub struct OperationTimer {
    start_time: Instant,
}

impl OperationTimer {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Memory snapshot at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub label: String,
    pub timestamp: Duration,
    pub memory_usage: u64, // in bytes
}

/// Statistics for operation durations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStats {
    pub count: usize,
    pub total: Duration,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
    pub median: Duration,
}

/// Complete performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_time: Duration,
    pub operation_stats: HashMap<String, DurationStats>,
    pub memory_snapshots: Vec<MemorySnapshot>,
    pub peak_memory: u64,
}

impl PerformanceMetrics {
    /// Generate a human-readable performance report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("Total execution time: {:?}\n", self.total_time));
        report.push_str(&format!("Peak memory usage: {} MB\n\n", self.peak_memory / 1024 / 1024));
        
        if !self.operation_stats.is_empty() {
            report.push_str("Operation Statistics:\n");
            report.push_str("┌─────────────────────────┬───────┬─────────────┬─────────────┬─────────────┬─────────────┐\n");
            report.push_str("│ Operation               │ Count │ Total       │ Average     │ Min         │ Max         │\n");
            report.push_str("├─────────────────────────┼───────┼─────────────┼─────────────┼─────────────┼─────────────┤\n");
            
            let mut operations: Vec<_> = self.operation_stats.iter().collect();
            operations.sort_by(|a, b| b.1.total.cmp(&a.1.total));
            
            for (operation, stats) in operations {
                report.push_str(&format!(
                    "│ {:<23} │ {:>5} │ {:>11?} │ {:>11?} │ {:>11?} │ {:>11?} │\n",
                    truncate_string(operation, 23),
                    stats.count,
                    stats.total,
                    stats.average,
                    stats.min,
                    stats.max
                ));
            }
            
            report.push_str("└─────────────────────────┴───────┴─────────────┴─────────────┴─────────────┴─────────────┘\n\n");
        }
        
        if !self.memory_snapshots.is_empty() {
            report.push_str("Memory Usage Timeline:\n");
            for snapshot in &self.memory_snapshots {
                report.push_str(&format!(
                    "  {:>8?}: {} - {} MB\n",
                    snapshot.timestamp,
                    snapshot.label,
                    snapshot.memory_usage / 1024 / 1024
                ));
            }
            report.push_str("\n");
        }
        
        report
    }

    /// Get the slowest operations
    pub fn get_slowest_operations(&self, limit: usize) -> Vec<(&String, &DurationStats)> {
        let mut operations: Vec<_> = self.operation_stats.iter().collect();
        operations.sort_by(|a, b| b.1.total.cmp(&a.1.total));
        operations.into_iter().take(limit).collect()
    }

    /// Get operations that were called most frequently
    pub fn get_most_frequent_operations(&self, limit: usize) -> Vec<(&String, &DurationStats)> {
        let mut operations: Vec<_> = self.operation_stats.iter().collect();
        operations.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        operations.into_iter().take(limit).collect()
    }

    /// Calculate total time spent in all operations
    pub fn total_operation_time(&self) -> Duration {
        self.operation_stats
            .values()
            .map(|stats| stats.total)
            .sum()
    }

    /// Calculate overhead (time not accounted for by tracked operations)
    pub fn calculate_overhead(&self) -> Duration {
        let operation_time = self.total_operation_time();
        if self.total_time > operation_time {
            self.total_time - operation_time
        } else {
            Duration::from_secs(0)
        }
    }
}

fn calculate_duration_stats(durations: &[Duration]) -> DurationStats {
    if durations.is_empty() {
        return DurationStats {
            count: 0,
            total: Duration::from_secs(0),
            average: Duration::from_secs(0),
            min: Duration::from_secs(0),
            max: Duration::from_secs(0),
            median: Duration::from_secs(0),
        };
    }

    let count = durations.len();
    let total: Duration = durations.iter().sum();
    let average = total / count as u32;
    let min = *durations.iter().min().unwrap();
    let max = *durations.iter().max().unwrap();
    
    // Calculate median
    let mut sorted_durations = durations.to_vec();
    sorted_durations.sort();
    let median = if count % 2 == 0 {
        let mid1 = sorted_durations[count / 2 - 1];
        let mid2 = sorted_durations[count / 2];
        (mid1 + mid2) / 2
    } else {
        sorted_durations[count / 2]
    };

    DurationStats {
        count,
        total,
        average,
        min,
        max,
        median,
    }
}

fn get_memory_usage() -> u64 {
    // This is a simplified implementation
    // In a real implementation, you might use system-specific APIs
    // or libraries like `sysinfo` to get actual memory usage
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return 0 if we can't determine memory usage
    0
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_basic_operation() {
        let mut profiler = PerformanceProfiler::new();
        
        let result = profiler.time_operation("test_op", || {
            thread::sleep(Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.operation_stats.len(), 1);
        assert!(metrics.operation_stats.contains_key("test_op"));
        
        let test_op_stats = &metrics.operation_stats["test_op"];
        assert_eq!(test_op_stats.count, 1);
        assert!(test_op_stats.total >= Duration::from_millis(10));
    }

    #[test]
    fn test_profiler_multiple_operations() {
        let mut profiler = PerformanceProfiler::new();
        
        // Run the same operation multiple times
        for _ in 0..3 {
            profiler.time_operation("repeated_op", || {
                thread::sleep(Duration::from_millis(5));
            });
        }
        
        // Run a different operation
        profiler.time_operation("single_op", || {
            thread::sleep(Duration::from_millis(15));
        });
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.operation_stats.len(), 2);
        
        let repeated_stats = &metrics.operation_stats["repeated_op"];
        assert_eq!(repeated_stats.count, 3);
        
        let single_stats = &metrics.operation_stats["single_op"];
        assert_eq!(single_stats.count, 1);
    }

    #[test]
    fn test_operation_timer() {
        let mut profiler = PerformanceProfiler::new();
        
        let timer = profiler.start_operation("async_op");
        thread::sleep(Duration::from_millis(10));
        profiler.end_operation("async_op", timer);
        
        let metrics = profiler.get_metrics();
        assert!(metrics.operation_stats.contains_key("async_op"));
        
        let stats = &metrics.operation_stats["async_op"];
        assert_eq!(stats.count, 1);
        assert!(stats.total >= Duration::from_millis(10));
    }

    #[test]
    fn test_memory_snapshots() {
        let mut profiler = PerformanceProfiler::new();
        
        profiler.snapshot_memory("start".to_string());
        thread::sleep(Duration::from_millis(10));
        profiler.snapshot_memory("middle".to_string());
        thread::sleep(Duration::from_millis(10));
        profiler.snapshot_memory("end".to_string());
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.memory_snapshots.len(), 3);
        
        // Check that timestamps are increasing
        for i in 1..metrics.memory_snapshots.len() {
            assert!(metrics.memory_snapshots[i].timestamp > metrics.memory_snapshots[i-1].timestamp);
        }
    }

    #[test]
    fn test_duration_stats() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
        ];
        
        let stats = calculate_duration_stats(&durations);
        
        assert_eq!(stats.count, 3);
        assert_eq!(stats.total, Duration::from_millis(60));
        assert_eq!(stats.average, Duration::from_millis(20));
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(30));
        assert_eq!(stats.median, Duration::from_millis(20));
    }

    #[test]
    fn test_profiler_reset() {
        let mut profiler = PerformanceProfiler::new();
        
        profiler.time_operation("test", || {});
        profiler.snapshot_memory("test".to_string());
        
        let metrics_before = profiler.get_metrics();
        assert!(!metrics_before.operation_stats.is_empty());
        assert!(!metrics_before.memory_snapshots.is_empty());
        
        profiler.reset();
        
        let metrics_after = profiler.get_metrics();
        assert!(metrics_after.operation_stats.is_empty());
        assert!(metrics_after.memory_snapshots.is_empty());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this_is_a_very_long_string", 10), "this_is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }
}
