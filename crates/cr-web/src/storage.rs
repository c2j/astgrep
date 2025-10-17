//! Storage abstraction for jobs and results

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    models::{Job, AnalysisResults},
    WebError, WebResult,
};

/// Storage trait for job and result persistence
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Store a job
    async fn store_job(&self, job: &Job) -> WebResult<()>;
    
    /// Get a job by ID
    async fn get_job(&self, job_id: Uuid) -> WebResult<Option<Job>>;
    
    /// Update a job
    async fn update_job(&self, job: &Job) -> WebResult<()>;
    
    /// List jobs with optional filtering
    async fn list_jobs(&self, filter: &JobFilter) -> WebResult<Vec<Job>>;
    
    /// Delete a job
    async fn delete_job(&self, job_id: Uuid) -> WebResult<()>;
    
    /// Store analysis results
    async fn store_results(&self, job_id: Uuid, results: &AnalysisResults) -> WebResult<()>;
    
    /// Get analysis results
    async fn get_results(&self, job_id: Uuid) -> WebResult<Option<AnalysisResults>>;
    
    /// Delete old jobs
    async fn cleanup_old_jobs(&self, cutoff_time: chrono::DateTime<chrono::Utc>) -> WebResult<usize>;
}

/// Job filter for listing operations
#[derive(Debug, Default)]
pub struct JobFilter {
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// In-memory storage implementation
pub struct MemoryStorage {
    jobs: Arc<RwLock<HashMap<Uuid, Job>>>,
    results: Arc<RwLock<HashMap<Uuid, AnalysisResults>>>,
}

impl MemoryStorage {
    /// Create a new memory storage instance
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn store_job(&self, job: &Job) -> WebResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id, job.clone());
        Ok(())
    }
    
    async fn get_job(&self, job_id: Uuid) -> WebResult<Option<Job>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(&job_id).cloned())
    }
    
    async fn update_job(&self, job: &Job) -> WebResult<()> {
        let mut jobs = self.jobs.write().await;
        if jobs.contains_key(&job.id) {
            jobs.insert(job.id, job.clone());
            Ok(())
        } else {
            Err(WebError::not_found(format!("Job not found: {}", job.id)))
        }
    }
    
    async fn list_jobs(&self, filter: &JobFilter) -> WebResult<Vec<Job>> {
        let jobs = self.jobs.read().await;
        let mut filtered_jobs: Vec<Job> = jobs.values().cloned().collect();
        
        // Apply status filter
        if let Some(ref status) = filter.status {
            filtered_jobs.retain(|job| {
                format!("{:?}", job.status).to_lowercase() == status.to_lowercase()
            });
        }
        
        // Apply job type filter
        if let Some(ref job_type) = filter.job_type {
            filtered_jobs.retain(|job| job.job_type == *job_type);
        }
        
        // Sort by creation time (newest first)
        filtered_jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Apply pagination
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(100);
        
        let paginated_jobs = filtered_jobs
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        
        Ok(paginated_jobs)
    }
    
    async fn delete_job(&self, job_id: Uuid) -> WebResult<()> {
        let mut jobs = self.jobs.write().await;
        let mut results = self.results.write().await;
        
        jobs.remove(&job_id);
        results.remove(&job_id);
        
        Ok(())
    }
    
    async fn store_results(&self, job_id: Uuid, results: &AnalysisResults) -> WebResult<()> {
        let mut storage_results = self.results.write().await;
        storage_results.insert(job_id, results.clone());
        Ok(())
    }
    
    async fn get_results(&self, job_id: Uuid) -> WebResult<Option<AnalysisResults>> {
        let results = self.results.read().await;
        Ok(results.get(&job_id).cloned())
    }
    
    async fn cleanup_old_jobs(&self, cutoff_time: chrono::DateTime<chrono::Utc>) -> WebResult<usize> {
        let mut jobs = self.jobs.write().await;
        let mut results = self.results.write().await;
        
        let old_job_ids: Vec<Uuid> = jobs
            .values()
            .filter(|job| job.created_at < cutoff_time)
            .map(|job| job.id)
            .collect();
        
        let count = old_job_ids.len();
        
        for job_id in old_job_ids {
            jobs.remove(&job_id);
            results.remove(&job_id);
        }
        
        Ok(count)
    }
}

/// SQLite storage implementation (optional)
#[cfg(feature = "database")]
pub struct SqliteStorage {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "database")]
impl SqliteStorage {
    /// Create a new SQLite storage instance
    pub async fn new(database_url: &str) -> WebResult<Self> {
        let pool = sqlx::SqlitePool::connect(database_url).await
            .map_err(|e| WebError::internal_server_error(format!("Failed to connect to database: {}", e)))?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await
            .map_err(|e| WebError::internal_server_error(format!("Failed to run migrations: {}", e)))?;
        
        Ok(Self { pool })
    }
}

#[cfg(feature = "database")]
#[async_trait::async_trait]
impl Storage for SqliteStorage {
    async fn store_job(&self, job: &Job) -> WebResult<()> {
        let metadata_json = serde_json::to_string(&job.metadata)
            .map_err(|e| WebError::internal_server_error(format!("Failed to serialize metadata: {}", e)))?;
        
        sqlx::query!(
            r#"
            INSERT INTO jobs (id, status, job_type, created_at, started_at, completed_at, progress, error, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            job.id.to_string(),
            format!("{:?}", job.status),
            job.job_type,
            job.created_at,
            job.started_at,
            job.completed_at,
            job.progress as i32,
            job.error,
            metadata_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to store job: {}", e)))?;
        
        Ok(())
    }
    
    async fn get_job(&self, job_id: Uuid) -> WebResult<Option<Job>> {
        let row = sqlx::query!(
            "SELECT * FROM jobs WHERE id = ?1",
            job_id.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to get job: {}", e)))?;
        
        if let Some(row) = row {
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&row.metadata)
                .map_err(|e| WebError::internal_server_error(format!("Failed to deserialize metadata: {}", e)))?;
            
            let status = match row.status.as_str() {
                "Queued" => crate::models::JobStatus::Queued,
                "Running" => crate::models::JobStatus::Running,
                "Completed" => crate::models::JobStatus::Completed,
                "Failed" => crate::models::JobStatus::Failed,
                "Cancelled" => crate::models::JobStatus::Cancelled,
                _ => crate::models::JobStatus::Queued,
            };
            
            let job = Job {
                id: Uuid::parse_str(&row.id)
                    .map_err(|e| WebError::internal_server_error(format!("Invalid job ID: {}", e)))?,
                status,
                job_type: row.job_type,
                created_at: row.created_at,
                started_at: row.started_at,
                completed_at: row.completed_at,
                progress: row.progress as u8,
                error: row.error,
                metadata,
            };
            
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }
    
    async fn update_job(&self, job: &Job) -> WebResult<()> {
        let metadata_json = serde_json::to_string(&job.metadata)
            .map_err(|e| WebError::internal_server_error(format!("Failed to serialize metadata: {}", e)))?;
        
        let result = sqlx::query!(
            r#"
            UPDATE jobs 
            SET status = ?2, started_at = ?3, completed_at = ?4, progress = ?5, error = ?6, metadata = ?7
            WHERE id = ?1
            "#,
            job.id.to_string(),
            format!("{:?}", job.status),
            job.started_at,
            job.completed_at,
            job.progress as i32,
            job.error,
            metadata_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to update job: {}", e)))?;
        
        if result.rows_affected() == 0 {
            return Err(WebError::not_found(format!("Job not found: {}", job.id)));
        }
        
        Ok(())
    }
    
    async fn list_jobs(&self, filter: &JobFilter) -> WebResult<Vec<Job>> {
        // This is a simplified implementation
        // In a real application, you would build dynamic SQL queries based on filters
        
        let rows = sqlx::query!("SELECT * FROM jobs ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WebError::internal_server_error(format!("Failed to list jobs: {}", e)))?;
        
        let mut jobs = Vec::new();
        for row in rows {
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&row.metadata)
                .map_err(|e| WebError::internal_server_error(format!("Failed to deserialize metadata: {}", e)))?;
            
            let status = match row.status.as_str() {
                "Queued" => crate::models::JobStatus::Queued,
                "Running" => crate::models::JobStatus::Running,
                "Completed" => crate::models::JobStatus::Completed,
                "Failed" => crate::models::JobStatus::Failed,
                "Cancelled" => crate::models::JobStatus::Cancelled,
                _ => crate::models::JobStatus::Queued,
            };
            
            let job = Job {
                id: Uuid::parse_str(&row.id)
                    .map_err(|e| WebError::internal_server_error(format!("Invalid job ID: {}", e)))?,
                status,
                job_type: row.job_type,
                created_at: row.created_at,
                started_at: row.started_at,
                completed_at: row.completed_at,
                progress: row.progress as u8,
                error: row.error,
                metadata,
            };
            
            jobs.push(job);
        }
        
        // Apply filters (simplified)
        if let Some(ref status_filter) = filter.status {
            jobs.retain(|job| format!("{:?}", job.status).to_lowercase() == status_filter.to_lowercase());
        }
        
        // Apply pagination
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(100);
        
        let paginated_jobs = jobs
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        
        Ok(paginated_jobs)
    }
    
    async fn delete_job(&self, job_id: Uuid) -> WebResult<()> {
        sqlx::query!("DELETE FROM jobs WHERE id = ?1", job_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WebError::internal_server_error(format!("Failed to delete job: {}", e)))?;
        
        sqlx::query!("DELETE FROM analysis_results WHERE job_id = ?1", job_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WebError::internal_server_error(format!("Failed to delete results: {}", e)))?;
        
        Ok(())
    }
    
    async fn store_results(&self, job_id: Uuid, results: &AnalysisResults) -> WebResult<()> {
        let results_json = serde_json::to_string(results)
            .map_err(|e| WebError::internal_server_error(format!("Failed to serialize results: {}", e)))?;
        
        sqlx::query!(
            "INSERT OR REPLACE INTO analysis_results (job_id, results) VALUES (?1, ?2)",
            job_id.to_string(),
            results_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to store results: {}", e)))?;
        
        Ok(())
    }
    
    async fn get_results(&self, job_id: Uuid) -> WebResult<Option<AnalysisResults>> {
        let row = sqlx::query!(
            "SELECT results FROM analysis_results WHERE job_id = ?1",
            job_id.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to get results: {}", e)))?;
        
        if let Some(row) = row {
            let results: AnalysisResults = serde_json::from_str(&row.results)
                .map_err(|e| WebError::internal_server_error(format!("Failed to deserialize results: {}", e)))?;
            Ok(Some(results))
        } else {
            Ok(None)
        }
    }
    
    async fn cleanup_old_jobs(&self, cutoff_time: chrono::DateTime<chrono::Utc>) -> WebResult<usize> {
        let result = sqlx::query!(
            "DELETE FROM jobs WHERE created_at < ?1",
            cutoff_time
        )
        .execute(&self.pool)
        .await
        .map_err(|e| WebError::internal_server_error(format!("Failed to cleanup jobs: {}", e)))?;
        
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{JobStatus, AnalysisSummary};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_memory_storage_job_operations() {
        let storage = MemoryStorage::new();
        
        let job = Job {
            id: Uuid::new_v4(),
            status: JobStatus::Queued,
            job_type: "test".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            progress: 0,
            error: None,
            metadata: HashMap::new(),
        };
        
        // Store job
        storage.store_job(&job).await.unwrap();
        
        // Get job
        let retrieved_job = storage.get_job(job.id).await.unwrap();
        assert!(retrieved_job.is_some());
        assert_eq!(retrieved_job.unwrap().id, job.id);
        
        // Update job
        let mut updated_job = job.clone();
        updated_job.status = JobStatus::Running;
        updated_job.progress = 50;
        storage.update_job(&updated_job).await.unwrap();
        
        let retrieved_job = storage.get_job(job.id).await.unwrap().unwrap();
        assert_eq!(retrieved_job.status, JobStatus::Running);
        assert_eq!(retrieved_job.progress, 50);
        
        // List jobs
        let filter = JobFilter::default();
        let jobs = storage.list_jobs(&filter).await.unwrap();
        assert_eq!(jobs.len(), 1);
        
        // Delete job
        storage.delete_job(job.id).await.unwrap();
        let retrieved_job = storage.get_job(job.id).await.unwrap();
        assert!(retrieved_job.is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_results_operations() {
        let storage = MemoryStorage::new();
        let job_id = Uuid::new_v4();
        
        let results = AnalysisResults {
            findings: vec![],
            summary: AnalysisSummary {
                total_findings: 0,
                findings_by_severity: HashMap::new(),
                findings_by_confidence: HashMap::new(),
                files_analyzed: 1,
                rules_executed: 5,
                duration_ms: 100,
            },
            metrics: None,
        };
        
        // Store results
        storage.store_results(job_id, &results).await.unwrap();
        
        // Get results
        let retrieved_results = storage.get_results(job_id).await.unwrap();
        assert!(retrieved_results.is_some());
        assert_eq!(retrieved_results.unwrap().summary.files_analyzed, 1);
        
        // Delete (via delete_job)
        storage.delete_job(job_id).await.unwrap();
        let retrieved_results = storage.get_results(job_id).await.unwrap();
        assert!(retrieved_results.is_none());
    }
}
