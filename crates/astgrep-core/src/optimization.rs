//! Performance optimization utilities for astgrep
//!
//! This module provides tools and utilities for optimizing the performance
//! of static analysis operations.

use crate::{AstNode, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation_counts: HashMap<String, u64>,
    pub operation_times: HashMap<String, Duration>,
    pub memory_usage: HashMap<String, usize>,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new() -> Self {
        Self {
            operation_counts: HashMap::new(),
            operation_times: HashMap::new(),
            memory_usage: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Record an operation
    pub fn record_operation(&mut self, operation: &str, duration: Duration) {
        *self.operation_counts.entry(operation.to_string()).or_insert(0) += 1;
        *self.operation_times.entry(operation.to_string()).or_insert(Duration::ZERO) += duration;
    }

    /// Record memory usage
    pub fn record_memory_usage(&mut self, component: &str, bytes: usize) {
        self.memory_usage.insert(component.to_string(), bytes);
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Get average operation time
    pub fn average_operation_time(&self, operation: &str) -> Option<Duration> {
        let total_time = self.operation_times.get(operation)?;
        let count = self.operation_counts.get(operation)?;
        if *count == 0 {
            None
        } else {
            Some(*total_time / *count as u32)
        }
    }

    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Performance Report ===\n");

        // Operation statistics
        report.push_str("\nOperation Statistics:\n");
        for (operation, count) in &self.operation_counts {
            if let Some(avg_time) = self.average_operation_time(operation) {
                report.push_str(&format!(
                    "  {}: {} calls, avg time: {:?}\n",
                    operation, count, avg_time
                ));
            }
        }

        // Memory usage
        if !self.memory_usage.is_empty() {
            report.push_str("\nMemory Usage:\n");
            for (component, bytes) in &self.memory_usage {
                report.push_str(&format!("  {}: {} bytes\n", component, bytes));
            }
        }

        // Cache statistics
        report.push_str(&format!(
            "\nCache Statistics:\n  Hits: {}\n  Misses: {}\n  Hit Rate: {:.2}%\n",
            self.cache_hits,
            self.cache_misses,
            self.cache_hit_rate() * 100.0
        ));

        report
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance profiler for timing operations
pub struct PerformanceProfiler {
    metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new())),
        }
    }

    /// Time an operation
    pub fn time_operation<F, R>(&self, operation: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_operation(operation, duration);
        }

        result
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap_or_else(|poisoned| {
            poisoned.into_inner()
        }).clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            *metrics = PerformanceMetrics::new();
        }
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// AST traversal optimizer
pub struct AstTraversalOptimizer;

impl AstTraversalOptimizer {
    /// Optimized AST traversal with early termination
    pub fn find_first_matching_node<F>(
        root: &dyn AstNode,
        predicate: F,
    ) -> Option<&dyn AstNode>
    where
        F: Fn(&dyn AstNode) -> bool,
    {
        if predicate(root) {
            return Some(root);
        }

        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                if let Some(found) = Self::find_first_matching_node(child, &predicate) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Count nodes efficiently
    pub fn count_nodes(root: &dyn AstNode) -> usize {
        let mut count = 1;
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                count += Self::count_nodes(child);
            }
        }
        count
    }

    /// Get maximum depth efficiently
    pub fn max_depth(root: &dyn AstNode) -> usize {
        let mut max_child_depth = 0;
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                let child_depth = Self::max_depth(child);
                max_child_depth = max_child_depth.max(child_depth);
            }
        }
        max_child_depth + 1
    }

    /// Collect nodes by type efficiently
    pub fn collect_nodes_by_type(
        root: &dyn AstNode,
        node_type: &str,
    ) -> Vec<String> {
        let mut results = Vec::new();
        Self::collect_nodes_by_type_recursive(root, node_type, &mut results);
        results
    }

    /// Recursive helper for collecting nodes by type
    fn collect_nodes_by_type_recursive(
        root: &dyn AstNode,
        node_type: &str,
        results: &mut Vec<String>,
    ) {
        if root.node_type() == node_type {
            if let Some(text) = root.text() {
                results.push(text.to_string());
            } else {
                results.push(format!("<{}>", node_type));
            }
        }

        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                Self::collect_nodes_by_type_recursive(child, node_type, results);
            }
        }
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    allocations: HashMap<String, usize>,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    /// Track allocation
    pub fn track_allocation(&mut self, component: &str, size: usize) {
        *self.allocations.entry(component.to_string()).or_insert(0) += size;
    }

    /// Track deallocation
    pub fn track_deallocation(&mut self, component: &str, size: usize) {
        if let Some(current) = self.allocations.get_mut(component) {
            *current = current.saturating_sub(size);
        }
    }

    /// Get current allocation for component
    pub fn get_allocation(&self, component: &str) -> usize {
        self.allocations.get(component).copied().unwrap_or(0)
    }

    /// Get total allocation
    pub fn total_allocation(&self) -> usize {
        self.allocations.values().sum()
    }

    /// Generate memory report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Memory Usage Report ===\n");

        for (component, size) in &self.allocations {
            report.push_str(&format!("  {}: {} bytes\n", component, size));
        }

        report.push_str(&format!("\nTotal: {} bytes\n", self.total_allocation()));
        report
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache for expensive operations
pub struct OperationCache<K, V> {
    cache: HashMap<K, V>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

impl<K, V> OperationCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// Create a new operation cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
        }
    }

    /// Get value from cache or compute it
    pub fn get_or_compute<F>(&mut self, key: K, compute: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.cache.get(&key) {
            self.hits += 1;
            value.clone()
        } else {
            self.misses += 1;
            let value = compute();
            
            // Simple eviction: remove oldest if at capacity
            if self.cache.len() >= self.max_size {
                if let Some(first_key) = self.cache.keys().next().cloned() {
                    self.cache.remove(&first_key);
                }
            }
            
            self.cache.insert(key, value.clone());
            value
        }
    }

    /// Get cache statistics
    pub fn statistics(&self) -> (u64, u64, f64) {
        let total = self.hits + self.misses;
        let hit_rate = if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        };
        (self.hits, self.misses, hit_rate)
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        
        metrics.record_operation("test_op", Duration::from_millis(100));
        metrics.record_operation("test_op", Duration::from_millis(200));
        
        assert_eq!(metrics.operation_counts.get("test_op"), Some(&2));
        assert_eq!(
            metrics.average_operation_time("test_op"),
            Some(Duration::from_millis(150))
        );
    }

    #[test]
    fn test_performance_profiler() {
        let profiler = PerformanceProfiler::new();
        
        let result = profiler.time_operation("test", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.operation_counts.get("test"), Some(&1));
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        
        tracker.track_allocation("component1", 1000);
        tracker.track_allocation("component2", 2000);
        
        assert_eq!(tracker.get_allocation("component1"), 1000);
        assert_eq!(tracker.total_allocation(), 3000);
        
        tracker.track_deallocation("component1", 500);
        assert_eq!(tracker.get_allocation("component1"), 500);
        assert_eq!(tracker.total_allocation(), 2500);
    }

    #[test]
    fn test_operation_cache() {
        let mut cache = OperationCache::new(2);
        
        let value1 = cache.get_or_compute("key1", || "value1".to_string());
        assert_eq!(value1, "value1");
        
        let value1_cached = cache.get_or_compute("key1", || "different".to_string());
        assert_eq!(value1_cached, "value1"); // Should return cached value
        
        let (hits, misses, hit_rate) = cache.statistics();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(hit_rate, 0.5);
    }
}
