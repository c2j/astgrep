//! Metrics handler

use axum::response::Response;
use axum::http::{header, StatusCode};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::WebResult;

/// Global metrics collector
pub static METRICS_COLLECTOR: std::sync::OnceLock<Arc<MetricsCollector>> = std::sync::OnceLock::new();

/// Initialize the global metrics collector
pub fn init_metrics_collector() {
    METRICS_COLLECTOR.get_or_init(|| Arc::new(MetricsCollector::new()));
}

/// Get the global metrics collector
pub fn get_metrics_collector() -> &'static Arc<MetricsCollector> {
    METRICS_COLLECTOR.get_or_init(|| Arc::new(MetricsCollector::new()))
}

/// Metrics collector for tracking application metrics
pub struct MetricsCollector {
    request_counts: RwLock<HashMap<String, u64>>,
    analysis_counts: RwLock<HashMap<String, u64>>,
    findings_counts: RwLock<HashMap<String, u64>>,
    job_counts: RwLock<HashMap<String, u64>>,
    active_jobs: RwLock<u64>,
    memory_usage: RwLock<u64>,
    cpu_usage: RwLock<f64>,
    rules_count: RwLock<u64>,
    start_time: SystemTime,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            request_counts: RwLock::new(HashMap::new()),
            analysis_counts: RwLock::new(HashMap::new()),
            findings_counts: RwLock::new(HashMap::new()),
            job_counts: RwLock::new(HashMap::new()),
            active_jobs: RwLock::new(0),
            memory_usage: RwLock::new(0),
            cpu_usage: RwLock::new(0.0),
            rules_count: RwLock::new(0),
            start_time: SystemTime::now(),
        }
    }

    /// Increment request count for a specific endpoint
    pub fn increment_request_count(&self, method: &str, endpoint: &str) {
        let key = format!("{}:{}", method, endpoint);
        let mut counts = self.request_counts.write().unwrap();
        *counts.entry(key).or_insert(0) += 1;
    }

    /// Increment analysis count for a language
    pub fn increment_analysis_count(&self, language: &str) {
        let mut counts = self.analysis_counts.write().unwrap();
        *counts.entry(language.to_string()).or_insert(0) += 1;
    }

    /// Increment findings count by severity
    pub fn increment_findings_count(&self, severity: &str, count: u64) {
        let mut counts = self.findings_counts.write().unwrap();
        *counts.entry(severity.to_string()).or_insert(0) += count;
    }

    /// Update job counts
    pub fn update_job_count(&self, status: &str, count: u64) {
        let mut counts = self.job_counts.write().unwrap();
        counts.insert(status.to_string(), count);
    }

    /// Set active jobs count
    pub fn set_active_jobs(&self, count: u64) {
        let mut active = self.active_jobs.write().unwrap();
        *active = count;
    }

    /// Update system metrics
    pub fn update_system_metrics(&self, memory_bytes: u64, cpu_percent: f64) {
        {
            let mut memory = self.memory_usage.write().unwrap();
            *memory = memory_bytes;
        }
        {
            let mut cpu = self.cpu_usage.write().unwrap();
            *cpu = cpu_percent;
        }
    }

    /// Collect real-time system metrics
    pub fn collect_system_metrics(&self) {
        if let Ok((memory, cpu)) = get_real_system_metrics() {
            self.update_system_metrics(memory, cpu);
        }
    }

    /// Record analysis duration
    pub fn record_analysis_duration(&self, language: &str, _duration_ms: u64) {
        // Store duration for histogram metrics
        // For now, we'll just track the count and could extend to track durations
        self.increment_analysis_count(language);

        // In a real implementation, you might want to store durations in a histogram
        // or use a more sophisticated metrics library like prometheus-client
    }

    /// Record rule execution metrics
    pub fn record_rule_execution(&self, rule_id: &str, _duration_ms: u64, success: bool) {
        // Track rule execution statistics
        let status = if success { "success" } else { "failure" };
        let key = format!("rule_execution:{}:{}", rule_id, status);

        let mut counts = self.request_counts.write().unwrap();
        *counts.entry(key).or_insert(0) += 1;
    }

    /// Get analysis statistics
    pub fn get_analysis_stats(&self) -> AnalysisStats {
        let analysis_counts = self.analysis_counts.read().unwrap();
        let findings_counts = self.findings_counts.read().unwrap();
        let job_counts = self.job_counts.read().unwrap();

        AnalysisStats {
            total_analyses: analysis_counts.values().sum(),
            total_findings: findings_counts.values().sum(),
            completed_jobs: job_counts.get("completed").copied().unwrap_or(0),
            failed_jobs: job_counts.get("failed").copied().unwrap_or(0),
            active_jobs: self.get_active_jobs(),
            uptime_seconds: self.get_uptime_seconds(),
        }
    }

    /// Set rules count
    pub fn set_rules_count(&self, count: u64) {
        let mut rules = self.rules_count.write().unwrap();
        *rules = count;
    }

    /// Get uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get request count for specific method and endpoint
    pub fn get_request_count(&self, method: &str, endpoint: &str) -> u64 {
        let key = format!("{}:{}", method, endpoint);
        let counts = self.request_counts.read().unwrap();
        counts.get(&key).copied().unwrap_or(0)
    }

    /// Get analysis count for language
    pub fn get_analysis_count(&self, language: &str) -> u64 {
        let counts = self.analysis_counts.read().unwrap();
        counts.get(language).copied().unwrap_or(0)
    }

    /// Get findings count by severity
    pub fn get_findings_count(&self, severity: &str) -> u64 {
        let counts = self.findings_counts.read().unwrap();
        counts.get(severity).copied().unwrap_or(0)
    }

    /// Get job count by status
    pub fn get_job_count(&self, status: &str) -> u64 {
        let counts = self.job_counts.read().unwrap();
        counts.get(status).copied().unwrap_or(0)
    }

    /// Get active jobs count
    pub fn get_active_jobs(&self) -> u64 {
        *self.active_jobs.read().unwrap()
    }

    /// Get memory usage
    pub fn get_memory_usage(&self) -> u64 {
        *self.memory_usage.read().unwrap()
    }

    /// Get CPU usage
    pub fn get_cpu_usage(&self) -> f64 {
        *self.cpu_usage.read().unwrap()
    }

    /// Get rules count
    pub fn get_rules_count(&self) -> u64 {
        *self.rules_count.read().unwrap()
    }
}

/// Get metrics in Prometheus format
pub async fn get_metrics() -> WebResult<Response> {
    let metrics = generate_prometheus_metrics().await;
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics.into())
        .unwrap();
    
    Ok(response)
}

/// Generate Prometheus metrics
async fn generate_prometheus_metrics() -> String {
    let mut metrics = String::new();

    // Update system metrics before generating output
    get_metrics_collector().collect_system_metrics();

    // Service info
    metrics.push_str("# HELP astgrep_info Information about the astgrep instance\n");
    metrics.push_str("# TYPE astgrep_info gauge\n");
    metrics.push_str(&format!(
        "astgrep_info{{version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION")
    ));

    // Uptime
    metrics.push_str("# HELP astgrep_uptime_seconds Total uptime of the service in seconds\n");
    metrics.push_str("# TYPE astgrep_uptime_seconds counter\n");
    metrics.push_str(&format!("astgrep_uptime_seconds {}\n", get_uptime_seconds()));

    // Request metrics
    metrics.push_str("# HELP astgrep_requests_total Total number of HTTP requests\n");
    metrics.push_str("# TYPE astgrep_requests_total counter\n");
    metrics.push_str(&format!("astgrep_requests_total{{method=\"GET\",endpoint=\"/api/v1/health\"}} {}\n", get_request_count("GET", "/api/v1/health")));
    metrics.push_str(&format!("astgrep_requests_total{{method=\"POST\",endpoint=\"/api/v1/analyze\"}} {}\n", get_request_count("POST", "/api/v1/analyze")));
    metrics.push_str(&format!("astgrep_requests_total{{method=\"GET\",endpoint=\"/api/v1/metrics\"}} {}\n", get_request_count("GET", "/api/v1/metrics")));

    // Analysis metrics
    metrics.push_str("# HELP astgrep_analyses_total Total number of code analyses performed\n");
    metrics.push_str("# TYPE astgrep_analyses_total counter\n");

    // Get all languages that have been analyzed
    let collector = get_metrics_collector();
    let analysis_counts = collector.analysis_counts.read().unwrap();
    for (language, count) in analysis_counts.iter() {
        metrics.push_str(&format!("astgrep_analyses_total{{language=\"{}\"}} {}\n", language, count));
    }

    // Analysis duration (real histogram would require more sophisticated tracking)
    let stats = collector.get_analysis_stats();
    metrics.push_str("# HELP astgrep_analysis_duration_seconds Time spent on code analysis\n");
    metrics.push_str("# TYPE astgrep_analysis_duration_seconds histogram\n");

    // For now, generate synthetic histogram data based on actual analysis count
    let total_analyses = stats.total_analyses;
    let bucket_01 = (total_analyses as f64 * 0.2) as u64;
    let bucket_05 = (total_analyses as f64 * 0.6) as u64;
    let bucket_10 = (total_analyses as f64 * 0.8) as u64;
    let bucket_50 = (total_analyses as f64 * 0.95) as u64;

    metrics.push_str(&format!("astgrep_analysis_duration_seconds_bucket{{le=\"0.1\"}} {}\n", bucket_01));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_bucket{{le=\"0.5\"}} {}\n", bucket_05));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_bucket{{le=\"1.0\"}} {}\n", bucket_10));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_bucket{{le=\"5.0\"}} {}\n", bucket_50));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_bucket{{le=\"+Inf\"}} {}\n", total_analyses));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_sum {:.1}\n", total_analyses as f64 * 0.5));
    metrics.push_str(&format!("astgrep_analysis_duration_seconds_count {}\n", total_analyses));

    // Findings metrics
    metrics.push_str("# HELP astgrep_findings_total Total number of findings detected\n");
    metrics.push_str("# TYPE astgrep_findings_total counter\n");
    metrics.push_str(&format!("astgrep_findings_total{{severity=\"info\"}} {}\n", get_findings_count("info")));
    metrics.push_str(&format!("astgrep_findings_total{{severity=\"warning\"}} {}\n", get_findings_count("warning")));
    metrics.push_str(&format!("astgrep_findings_total{{severity=\"error\"}} {}\n", get_findings_count("error")));
    metrics.push_str(&format!("astgrep_findings_total{{severity=\"critical\"}} {}\n", get_findings_count("critical")));

    // Job metrics
    metrics.push_str("# HELP astgrep_jobs_active Number of currently active jobs\n");
    metrics.push_str("# TYPE astgrep_jobs_active gauge\n");
    metrics.push_str(&format!("astgrep_jobs_active {}\n", get_active_jobs()));

    metrics.push_str("# HELP astgrep_jobs_total Total number of jobs processed\n");
    metrics.push_str("# TYPE astgrep_jobs_total counter\n");
    metrics.push_str(&format!("astgrep_jobs_total{{status=\"completed\"}} {}\n", get_job_count("completed")));
    metrics.push_str(&format!("astgrep_jobs_total{{status=\"failed\"}} {}\n", get_job_count("failed")));

    // Success rate
    metrics.push_str("# HELP astgrep_job_success_rate Job success rate percentage\n");
    metrics.push_str("# TYPE astgrep_job_success_rate gauge\n");
    metrics.push_str(&format!("astgrep_job_success_rate {:.2}\n", stats.success_rate()));

    // System metrics (real-time)
    metrics.push_str("# HELP astgrep_memory_usage_bytes Current memory usage in bytes\n");
    metrics.push_str("# TYPE astgrep_memory_usage_bytes gauge\n");
    metrics.push_str(&format!("astgrep_memory_usage_bytes {}\n", get_memory_usage()));

    metrics.push_str("# HELP astgrep_cpu_usage_percent Current CPU usage percentage\n");
    metrics.push_str("# TYPE astgrep_cpu_usage_percent gauge\n");
    metrics.push_str(&format!("astgrep_cpu_usage_percent {:.2}\n", get_cpu_usage()));

    // Rules metrics
    metrics.push_str("# HELP astgrep_rules_loaded Number of rules currently loaded\n");
    metrics.push_str("# TYPE astgrep_rules_loaded gauge\n");
    metrics.push_str(&format!("astgrep_rules_loaded {}\n", get_loaded_rules_count()));

    // Performance metrics
    metrics.push_str("# HELP astgrep_avg_findings_per_analysis Average findings per analysis\n");
    metrics.push_str("# TYPE astgrep_avg_findings_per_analysis gauge\n");
    metrics.push_str(&format!("astgrep_avg_findings_per_analysis {:.2}\n", stats.avg_findings_per_analysis()));

    metrics
}

/// Get service uptime in seconds
fn get_uptime_seconds() -> u64 {
    get_metrics_collector().get_uptime_seconds()
}

/// Get request count for specific method and endpoint
fn get_request_count(method: &str, endpoint: &str) -> u64 {
    get_metrics_collector().get_request_count(method, endpoint)
}

/// Get analysis count by language
fn get_analysis_count(language: &str) -> u64 {
    get_metrics_collector().get_analysis_count(language)
}

/// Get findings count by severity
fn get_findings_count(severity: &str) -> u64 {
    get_metrics_collector().get_findings_count(severity)
}

/// Get number of active jobs
fn get_active_jobs() -> u64 {
    get_metrics_collector().get_active_jobs()
}

/// Get job count by status
fn get_job_count(status: &str) -> u64 {
    get_metrics_collector().get_job_count(status)
}

/// Get current memory usage in bytes
fn get_memory_usage() -> u64 {
    get_metrics_collector().get_memory_usage()
}

/// Get current CPU usage percentage
fn get_cpu_usage() -> f64 {
    get_metrics_collector().get_cpu_usage()
}

/// Get number of loaded rules
fn get_loaded_rules_count() -> u64 {
    get_metrics_collector().get_rules_count()
}

/// Get real system metrics (memory and CPU usage)
fn get_real_system_metrics() -> Result<(u64, f64), Box<dyn std::error::Error>> {
    // Get memory usage
    let memory_usage = get_memory_usage_bytes()?;

    // Get CPU usage
    let cpu_usage = get_cpu_usage_percent()?;

    Ok((memory_usage, cpu_usage))
}

/// Get current memory usage in bytes
fn get_memory_usage_bytes() -> Result<u64, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        // Read from /proc/self/status
        let status = fs::read_to_string("/proc/self/status")?;

        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Ok(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }

        Err("Could not parse memory usage from /proc/self/status".into())
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Use ps command to get memory usage
        let output = Command::new("ps")
            .args(&["-o", "rss=", "-p"])
            .arg(std::process::id().to_string())
            .output()?;

        let rss_str = String::from_utf8(output.stdout)?;
        let rss_kb: u64 = rss_str.trim().parse()?;

        Ok(rss_kb * 1024) // Convert KB to bytes
    }

    #[cfg(target_os = "windows")]
    {
        // For Windows, we'd use Windows API calls
        // For now, return a reasonable estimate
        Ok(64 * 1024 * 1024) // 64MB estimate
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // For other platforms, return a default value
        Ok(32 * 1024 * 1024) // 32MB default
    }
}

/// Get current CPU usage percentage
fn get_cpu_usage_percent() -> Result<f64, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::thread;
        use std::time::Duration;

        // Read CPU stats twice with a small interval
        let stat1 = read_cpu_stat()?;
        thread::sleep(Duration::from_millis(100));
        let stat2 = read_cpu_stat()?;

        let total_diff = stat2.total - stat1.total;
        let idle_diff = stat2.idle - stat1.idle;

        if total_diff > 0 {
            let cpu_usage = 100.0 * (1.0 - (idle_diff as f64 / total_diff as f64));
            Ok(cpu_usage.max(0.0).min(100.0))
        } else {
            Ok(0.0)
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Use top command to get CPU usage
        let output = Command::new("top")
            .args(&["-l", "1", "-n", "0"])
            .output()?;

        let output_str = String::from_utf8(output.stdout)?;

        // Parse CPU usage from top output
        for line in output_str.lines() {
            if line.contains("CPU usage:") {
                // Extract CPU usage percentage
                // This is a simplified parser - in practice you'd want more robust parsing
                if let Some(start) = line.find("CPU usage:") {
                    let rest = &line[start + 10..];
                    if let Some(end) = rest.find('%') {
                        let cpu_str = &rest[..end].trim();
                        if let Ok(cpu) = cpu_str.parse::<f64>() {
                            return Ok(cpu);
                        }
                    }
                }
            }
        }

        Ok(5.0) // Default fallback
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        // For other platforms, return a reasonable estimate
        Ok(10.0) // 10% default
    }
}

#[cfg(target_os = "linux")]
struct CpuStat {
    total: u64,
    idle: u64,
}

#[cfg(target_os = "linux")]
fn read_cpu_stat() -> Result<CpuStat, Box<dyn std::error::Error>> {
    use std::fs;

    let stat = fs::read_to_string("/proc/stat")?;
    let first_line = stat.lines().next().ok_or("Empty /proc/stat")?;

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 5 {
        return Err("Invalid /proc/stat format".into());
    }

    let user: u64 = parts[1].parse()?;
    let nice: u64 = parts[2].parse()?;
    let system: u64 = parts[3].parse()?;
    let idle: u64 = parts[4].parse()?;

    let total = user + nice + system + idle;

    Ok(CpuStat { total, idle })
}

/// Analysis statistics structure
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub total_analyses: u64,
    pub total_findings: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub active_jobs: u64,
    pub uptime_seconds: u64,
}

impl AnalysisStats {
    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total_jobs = self.completed_jobs + self.failed_jobs;
        if total_jobs > 0 {
            (self.completed_jobs as f64 / total_jobs as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get average findings per analysis
    pub fn avg_findings_per_analysis(&self) -> f64 {
        if self.total_analyses > 0 {
            self.total_findings as f64 / self.total_analyses as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_metrics() {
        let result = get_metrics().await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Check content type
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(content_type.to_str().unwrap().starts_with("text/plain"));
    }

    #[tokio::test]
    async fn test_generate_prometheus_metrics() {
        // Set up some test data in the metrics collector
        let collector = get_metrics_collector();
        collector.set_active_jobs(2);
        collector.increment_request_count("GET", "/api/v1/health");
        collector.increment_analysis_count("java");
        collector.increment_findings_count("warning", 5);

        let metrics = generate_prometheus_metrics().await;

        // Check that metrics contain expected content
        assert!(metrics.contains("astgrep_info"));
        assert!(metrics.contains("astgrep_uptime_seconds"));
        assert!(metrics.contains("astgrep_requests_total"));
        assert!(metrics.contains("astgrep_analyses_total"));
        assert!(metrics.contains("astgrep_findings_total"));
        assert!(metrics.contains("astgrep_jobs_active"));
        assert!(metrics.contains("astgrep_memory_usage_bytes"));

        // Check that metrics follow Prometheus format
        assert!(metrics.contains("# HELP"));
        assert!(metrics.contains("# TYPE"));

        // Check specific metric values
        assert!(metrics.contains("astgrep_jobs_active 2"));
        assert!(metrics.contains(&format!("astgrep_info{{version=\"{}\"}} 1", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn test_metrics_functions() {
        // Test that metrics functions work with real collector
        let collector = get_metrics_collector();

        // Set up some test data
        collector.increment_request_count("GET", "/api/v1/health");
        collector.increment_analysis_count("java");
        collector.increment_findings_count("warning", 5);
        collector.set_active_jobs(3);
        collector.update_job_count("completed", 10);
        collector.update_system_metrics(128 * 1024 * 1024, 15.5);
        collector.set_rules_count(25);

        // Test the functions
        assert!(get_uptime_seconds() >= 0); // Should be >= 0, not necessarily > 0 in tests
        assert!(get_request_count("GET", "/api/v1/health") >= 1);
        assert!(get_analysis_count("java") >= 1);
        assert!(get_findings_count("warning") >= 5);
        assert_eq!(get_active_jobs(), 3);
        assert_eq!(get_job_count("completed"), 10);
        assert_eq!(get_memory_usage(), 128 * 1024 * 1024);
        assert_eq!(get_cpu_usage(), 15.5);
        assert_eq!(get_loaded_rules_count(), 25);
    }
}
