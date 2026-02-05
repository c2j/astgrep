//! Thread pool management for parallel migration operations

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use super::migration_orchestrator::MigrationOperation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfig {
    pub max_threads: usize,
    pub queue_size: usize,
    pub keep_alive_timeout_secs: u64,
    pub max_idle_threads: usize,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            max_threads: num_cpus::get(),
            queue_size: 1000,
            keep_alive_timeout_secs: 30,
            max_idle_threads: 4,
        }
    }
}

pub struct ThreadPool {
    config: ThreadPoolConfig,
    semaphore: Arc<Semaphore>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(config: ThreadPoolConfig) -> Result<Self> {
        let semaphore = Arc::new(Semaphore::new(config.max_threads));

        Ok(Self {
            config,
            semaphore,
            workers: Vec::new(),
        })
    }

    /// Execute a function with thread pool management
    pub async fn execute<F, R>(&self, f: F, num_threads: usize) -> Result<Vec<R>>
    where
        F: Fn() -> R + Send + 'static,
        R: Send + 'static,
    {
        if num_threads == 0 {
            return Ok(vec![f()]);
        }

        if num_threads > self.config.max_threads {
            warn!("Requested threads ({}) exceeds max threads ({}), using max",
                  num_threads, self.config.max_threads);
            return self.execute(f, self.config.max_threads).await;
        }

        info!("Starting parallel execution with {} threads", num_threads);

        let semaphore = Arc::clone(&self.semaphore);
        let mut tasks = Vec::new();

        // Create tasks
        for _ in 0..num_threads {
            let sem_clone = Arc::clone(&semaphore);
            let task = tokio::spawn(async move {
                let _permit = sem_clone.acquire().await;
                let result = f();
                drop(_permit); // Release permit back to semaphore
                result
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.map_err(|e| {
                error!("Thread execution error: {}", e);
                anyhow::anyhow!("Thread execution failed: {}", e)
            })?);
        }

        info!("Completed parallel execution with {} threads", num_threads);
        Ok(results)
    }

    /// Process migration operations in parallel batches
    pub async fn process_operations_parallel<F, R>(
        &self,
        operations: Vec<MigrationOperation>,
        processor: F,
        batch_size: usize,
    ) -> Result<Vec<R>>
    where
        F: Fn(Vec<MigrationOperation>) -> R + Send + Sync + 'static,
        R: Send + 'static,
    {
        if operations.is_empty() {
            return Ok(Vec::new());
        }

        let num_threads = std::cmp::min(
            self.config.max_threads,
            (operations.len() + batch_size - 1) / batch_size,
        );

        info!("Processing {} operations in {} threads with batch size {}",
              operations.len(), num_threads, batch_size);

        let semaphore = Arc::clone(&self.semaphore);
        let mut handles = Vec::new();

        // Split operations into batches
        for chunk in operations.chunks(batch_size) {
            let chunk = chunk.to_vec();
            let sem_clone = Arc::clone(&semaphore);
            let processor_ref = &processor;

            let handle = tokio::spawn(async move {
                let _permit = sem_clone.acquire().await;
                let result = processor_ref(chunk);
                drop(_permit);
                result
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.map_err(|e| {
                error!("Batch processing error: {}", e);
                anyhow::anyhow!("Batch processing failed: {}", e)
            })?);
        }

        info!("Completed processing {} batches", results.len());
        Ok(results)
    }

    /// Execute operations with automatic batching based on thread availability
    pub async fn execute_operations_parallel<F>(
        &self,
        operations: Vec<MigrationOperation>,
        processor: F,
    ) -> Result<Vec<MigrationOperation>>
    where
        F: Fn(Vec<MigrationOperation>) -> Vec<MigrationOperation> + Send + Sync + 'static,
    {
        if operations.is_empty() {
            return Ok(Vec::new());
        }

        let optimal_batch_size = (operations.len() + self.config.max_threads - 1) / self.config.max_threads;
        self.process_operations_parallel(operations, processor, optimal_batch_size)
            .await
            .and_then(|batch_results| {
                // Flatten results from all batches
                Ok(batch_results.into_iter().flatten().collect())
            })
    }

    /// Get current pool statistics
    pub fn stats(&self) -> ThreadPoolStats {
        ThreadPoolStats {
            max_threads: self.config.max_threads,
            available_permits: self.semaphore.available_permits(),
            active_threads: self.config.max_threads - self.semaphore.available_permits(),
        }
    }

    /// Wait for all operations to complete (blocks until thread pool is idle)
    pub fn wait_for_completion(&self) {
        debug!("Waiting for thread pool to become idle...");

        // Wait until semaphore permits are back to maximum
        while self.semaphore.available_permits() < self.config.max_threads {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        debug!("Thread pool is now idle");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        info!("Shutting down thread pool");

        // Wait for all workers to complete
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadPoolStats {
    pub max_threads: usize,
    pub available_permits: usize,
    pub active_threads: usize,
}

/// Helper to get the number of available CPU cores
pub fn get_optimal_thread_count(base_operations: usize) -> usize {
    let num_cpus = num_cpus::get();

    // Rule of thumb: use min(num_cpus, base_operations) but with a reasonable minimum
    let min_threads = 1;
    let max_threads = 8; // Reasonable upper limit for most migration operations

    let optimal = std::cmp::max(
        min_threads,
        std::cmp::min(num_cpus, std::cmp::min(base_operations, max_threads)),
    );

    debug!("Optimal thread count for {} operations: {}", base_operations, optimal);
    optimal
}

/// Create a thread pool optimized for migration operations
pub fn create_migration_thread_pool(num_operations: usize) -> Result<ThreadPool> {
    let optimal_threads = get_optimal_thread_count(num_operations);

    let config = ThreadPoolConfig {
        max_threads: optimal_threads,
        queue_size: std::cmp::max(100, num_operations),
        keep_alive_timeout_secs: 60, // Longer timeout for migration operations
        max_idle_threads: std::cmp::max(2, optimal_threads / 2),
    };

    ThreadPool::new(config)
}

/// Execute a function in parallel with automatic thread pool management
pub async fn execute_parallel<F, R>(f: F, num_threads: usize) -> Result<Vec<R>>
where
    F: Fn() -> R + Send + 'static,
    R: Send + 'static,
{
    let pool = create_migration_thread_pool(num_threads)?;
    pool.execute(f, num_threads).await
}

/// Process a collection of items in parallel using rayon
pub async fn process_items_parallel<T, R, F>(
    items: Vec<T>,
    processor: F,
    num_threads: usize,
) -> Vec<R>
where
    T: Send + Sync + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
{
    if items.is_empty() {
        return Vec::new();
    }

    let effective_threads = std::cmp::min(num_threads, items.len());
    let pool = create_migration_thread_pool(effective_threads)?;

    let items_arc = Arc::new(items);
    let chunk_size = (items.len() + effective_threads - 1) / effective_threads;

    let results: Vec<Vec<R>> = pool.process_operations_parallel(
        items_arc.as_ref().to_vec(),
        move |chunk| {
            chunk.into_par_iter().map(&processor).collect()
        },
        chunk_size,
    ).await?;

    results.into_iter().flatten().collect()
}

/// Utility for progress reporting during parallel operations
pub struct ParallelProgressReporter {
    total_items: usize,
    completed_items: std::sync::atomic::AtomicUsize,
    start_time: std::time::Instant,
}

impl ParallelProgressReporter {
    pub fn new(total_items: usize) -> Self {
        Self {
            total_items,
            completed_items: std::sync::atomic::AtomicUsize::new(0),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn increment(&self) -> usize {
        let completed = self.completed_items.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let elapsed = self.start_time.elapsed().as_secs_f64();
        if completed > 0 && elapsed > 1.0 {
            let rate = completed as f64 / elapsed;
            let remaining = self.total_items - completed;
            let eta = if rate > 0.0 { remaining as f64 / rate } else { 0.0 };

            info!("Progress: {}/{} ({:.1}%) - Rate: {:.1}/sec - ETA: {:.1}s",
                 completed,
                 self.total_items,
                 (completed as f64 / self.total_items as f64) * 100.0,
                 rate,
                 eta);
        }

        completed
    }

    pub fn is_complete(&self) -> bool {
        self.completed_items.load(std::sync::atomic::Ordering::Relaxed) >= self.total_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_thread_pool_creation() {
        let pool = ThreadPool::new(ThreadPoolConfig::default()).unwrap();
        let stats = pool.stats();
        assert_eq!(stats.max_threads, num_cpus::get());
        assert_eq!(stats.available_permits, num_cpus::get());
        assert_eq!(stats.active_threads, 0);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let pool = ThreadPool::new(ThreadPoolConfig::default()).unwrap();

        let results = pool.execute(|| {
            thread::sleep(Duration::from_millis(100));
            42
        }, 3).await.unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|&r| r == &42));
    }

    #[test]
    fn test_optimal_thread_count() {
        assert_eq!(get_optimal_thread_count(10), 8); // min_threads=1, max_threads=8
        assert_eq!(get_optimal_thread_count(2), 2);
        assert_eq!(get_optimal_thread_count(0), 1);
    }

    #[tokio::test]
    async fn test_parallel_item_processing() {
        let items: Vec<i32> = (1..=100).collect();

        let results = process_items_parallel(
            items,
            |item| item * 2,
            4,
        ).await;

        assert_eq!(results.len(), 100);
        assert_eq!(results[0], 2);
        assert_eq!(results[99], 200);
    }

    #[tokio::test]
    async fn test_progress_reporter() {
        let reporter = ParallelProgressReporter::new(100);

        assert_eq!(reporter.increment(), 1);
        assert_eq!(reporter.increment(), 2);

        assert!(!reporter.is_complete());

        // Simulate completion
        for _ in 0..98 {
            reporter.increment();
        }

        assert!(reporter.is_complete());
        assert_eq!(reporter.increment(), 100);
    }
}