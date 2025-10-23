//! Job management handlers

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use tokio::sync::RwLock;
use chrono::Utc;
use std::sync::OnceLock;

use crate::{
    models::{Job, JobStatus},
    api::PaginatedResponse,
    WebConfig, WebError, WebResult,
};

/// In-memory job storage
#[derive(Debug, Clone)]
pub struct JobStorage {
    jobs: Arc<RwLock<HashMap<Uuid, Job>>>,
}

impl JobStorage {
    /// Create a new job storage
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new job
    pub async fn create_job(&self, job: Job) -> WebResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id, job);
        Ok(())
    }

    /// Get a job by ID
    pub async fn get_job(&self, job_id: Uuid) -> Option<Job> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).cloned()
    }

    /// Update a job
    pub async fn update_job(&self, job: Job) -> WebResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id, job);
        Ok(())
    }

    /// Get all jobs
    pub async fn get_all_jobs(&self) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Get jobs with filtering
    pub async fn get_jobs_filtered(&self, status: Option<JobStatus>) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .filter(|job| {
                if let Some(ref filter_status) = status {
                    &job.status == filter_status
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Delete a job
    pub async fn delete_job(&self, job_id: Uuid) -> WebResult<bool> {
        let mut jobs = self.jobs.write().await;
        Ok(jobs.remove(&job_id).is_some())
    }

    /// Get job count
    pub async fn get_job_count(&self) -> usize {
        let jobs = self.jobs.read().await;
        jobs.len()
    }
}

/// Global job storage instance
static JOB_STORAGE: OnceLock<JobStorage> = OnceLock::new();

/// Get the global job storage instance
fn get_job_storage() -> &'static JobStorage {
    JOB_STORAGE.get_or_init(|| {
        let storage = JobStorage::new();

        // Initialize with some sample jobs for demonstration
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            let _ = initialize_sample_jobs(&storage_clone).await;
        });

        storage
    })
}

/// Initialize some sample jobs for demonstration
async fn initialize_sample_jobs(storage: &JobStorage) -> WebResult<()> {
    use chrono::Duration;

    let now = Utc::now();

    let sample_jobs = vec![
        Job {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            status: JobStatus::Completed,
            job_type: "code_analysis".to_string(),
            created_at: now - Duration::hours(2),
            started_at: Some(now - Duration::hours(2) + Duration::minutes(1)),
            completed_at: Some(now - Duration::hours(1)),
            progress: 100,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("language".to_string(), serde_json::Value::String("java".to_string()));
                map.insert("file_count".to_string(), serde_json::Value::String("15".to_string()));
                map
            },
        },
        Job {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            status: JobStatus::Running,
            job_type: "security_scan".to_string(),
            created_at: now - Duration::minutes(30),
            started_at: Some(now - Duration::minutes(25)),
            completed_at: None,
            progress: 65,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("scan_type".to_string(), serde_json::Value::String("vulnerability".to_string()));
                map.insert("target".to_string(), serde_json::Value::String("web_app".to_string()));
                map
            },
        },
        Job {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap(),
            status: JobStatus::Failed,
            job_type: "code_analysis".to_string(),
            created_at: now - Duration::hours(1),
            started_at: Some(now - Duration::hours(1) + Duration::minutes(2)),
            completed_at: Some(now - Duration::minutes(45)),
            progress: 30,
            error: Some("Parser error: Unsupported file format".to_string()),
            metadata: {
                let mut map = HashMap::new();
                map.insert("language".to_string(), serde_json::Value::String("unknown".to_string()));
                map.insert("error_code".to_string(), serde_json::Value::String("PARSE_001".to_string()));
                map
            },
        },
        Job {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440004").unwrap(),
            status: JobStatus::Pending,
            job_type: "dependency_check".to_string(),
            created_at: now - Duration::minutes(5),
            started_at: None,
            completed_at: None,
            progress: 0,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("package_manager".to_string(), serde_json::Value::String("npm".to_string()));
                map.insert("priority".to_string(), serde_json::Value::String("high".to_string()));
                map
            },
        },
    ];

    for job in sample_jobs {
        storage.create_job(job).await?;
    }

    Ok(())
}

/// Query parameters for listing jobs
#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Number of jobs to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Get job status by ID
pub async fn get_job_status(
    State(_config): State<Arc<WebConfig>>,
    Path(job_id): Path<Uuid>,
) -> WebResult<Json<Job>> {
    // This is a simplified implementation
    // In a real application, you would query the job storage
    
    let job = get_job_from_storage(job_id).await
        .ok_or_else(|| WebError::not_found(format!("Job not found: {}", job_id)))?;

    Ok(Json(job))
}

/// List jobs with optional filtering
pub async fn list_jobs(
    State(_config): State<Arc<WebConfig>>,
    Query(params): Query<ListJobsQuery>,
) -> WebResult<Json<PaginatedResponse<Job>>> {
    // Parse status filter
    let status_filter = if let Some(status_str) = params.status {
        match status_str.to_lowercase().as_str() {
            "pending" => Some(JobStatus::Pending),
            "queued" => Some(JobStatus::Queued),
            "running" => Some(JobStatus::Running),
            "completed" => Some(JobStatus::Completed),
            "failed" => Some(JobStatus::Failed),
            "cancelled" => Some(JobStatus::Cancelled),
            _ => None,
        }
    } else {
        None
    };

    let mut jobs = get_all_jobs_from_storage(status_filter).await;

    // Sort jobs by creation time (newest first)
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply pagination
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 jobs per request

    let total_jobs = jobs.len();
    let paginated_jobs: Vec<Job> = jobs
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let page = (offset / limit) + 1;
    let total_pages = (total_jobs as f64 / limit as f64).ceil() as u32;

    let pagination = crate::api::PaginationMeta {
        page: page as u32,
        per_page: limit as u32,
        total: total_jobs as u32,
        total_pages,
        has_next: offset + limit < total_jobs,
        has_prev: offset > 0,
    };

    let response = PaginatedResponse::new(
        paginated_jobs,
        pagination,
        None, // request_id
    );
    tracing::info!(
        "Listed {} jobs (offset: {}, limit: {}, total: {})",
        response.data.len(),
        offset,
        limit,
        total_jobs
    );

    Ok(Json(response))
}

/// Get job from storage
async fn get_job_from_storage(job_id: Uuid) -> Option<Job> {
    let storage = get_job_storage();
    storage.get_job(job_id).await
}

/// Get all jobs from storage with optional filtering
async fn get_all_jobs_from_storage(status: Option<JobStatus>) -> Vec<Job> {
    let storage = get_job_storage();
    if let Some(status) = status {
        storage.get_jobs_filtered(Some(status)).await
    } else {
        storage.get_all_jobs().await
    }
}

/// Create a new analysis job
pub async fn create_analysis_job(
    job_type: String,
    metadata: HashMap<String, serde_json::Value>,
) -> WebResult<Uuid> {
    let storage = get_job_storage();
    let job_id = Uuid::new_v4();

    let job = Job {
        id: job_id,
        status: JobStatus::Pending,
        job_type,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
        progress: 0,
        error: None,
        metadata,
    };

    storage.create_job(job).await?;

    tracing::info!("Created new job: {}", job_id);
    Ok(job_id)
}

/// Update job status and progress
pub async fn update_job_status(
    job_id: Uuid,
    status: JobStatus,
    progress: u8,
    error: Option<String>,
) -> WebResult<()> {
    let storage = get_job_storage();

    if let Some(mut job) = storage.get_job(job_id).await {
        job.status = status.clone();
        job.progress = progress;
        job.error = error;

        match status {
            JobStatus::Running if job.started_at.is_none() => {
                job.started_at = Some(Utc::now());
            }
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
                if job.completed_at.is_none() {
                    job.completed_at = Some(Utc::now());
                }
            }
            _ => {}
        }

        storage.update_job(job).await?;
        tracing::info!("Updated job {} status to {:?}", job_id, status);
    } else {
        return Err(WebError::not_found(format!("Job not found: {}", job_id)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_job_status_existing() {
        let config = Arc::new(WebConfig::default());
        let job_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        
        let result = get_job_status(State(config), Path(job_id)).await;
        assert!(result.is_ok());
        
        let job = result.unwrap().0;
        assert_eq!(job.id, job_id);
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_get_job_status_not_found() {
        let config = Arc::new(WebConfig::default());
        let job_id = Uuid::new_v4(); // Random UUID that doesn't exist
        
        let result = get_job_status(State(config), Path(job_id)).await;
        assert!(result.is_err());
        
        if let Err(WebError::NotFound { message }) = result {
            assert!(message.contains("Job not found"));
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[tokio::test]
    async fn test_list_jobs_no_filter() {
        let config = Arc::new(WebConfig::default());
        let query = ListJobsQuery {
            status: None,
            limit: None,
            offset: None,
        };
        
        let result = list_jobs(State(config), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.data.len(), 4); // All sample jobs
        assert_eq!(response.pagination.total, 4);
    }

    #[tokio::test]
    async fn test_list_jobs_with_status_filter() {
        let config = Arc::new(WebConfig::default());
        let query = ListJobsQuery {
            status: Some("completed".to_string()),
            limit: None,
            offset: None,
        };
        
        let result = list_jobs(State(config), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.data.len(), 1); // Only completed jobs
        assert_eq!(response.data[0].status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_list_jobs_with_pagination() {
        let config = Arc::new(WebConfig::default());
        let query = ListJobsQuery {
            status: None,
            limit: Some(2),
            offset: Some(1),
        };
        
        let result = list_jobs(State(config), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.data.len(), 2); // Limited to 2 jobs
    }

    #[tokio::test]
    async fn test_create_and_update_job() {
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), serde_json::Value::String("rust".to_string()));

        // Create a new job
        let job_id = create_analysis_job("test_analysis".to_string(), metadata).await.unwrap();

        // Verify job was created
        let job = get_job_from_storage(job_id).await.unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.progress, 0);

        // Update job status
        update_job_status(job_id, JobStatus::Running, 50, None).await.unwrap();

        // Verify job was updated
        let updated_job = get_job_from_storage(job_id).await.unwrap();
        assert_eq!(updated_job.status, JobStatus::Running);
        assert_eq!(updated_job.progress, 50);
        assert!(updated_job.started_at.is_some());

        // Complete the job
        update_job_status(job_id, JobStatus::Completed, 100, None).await.unwrap();

        // Verify job completion
        let completed_job = get_job_from_storage(job_id).await.unwrap();
        assert_eq!(completed_job.status, JobStatus::Completed);
        assert_eq!(completed_job.progress, 100);
        assert!(completed_job.completed_at.is_some());
    }
}
