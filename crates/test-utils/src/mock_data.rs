//! Mock data generators for testing web APIs and other components

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// Mock rule information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRuleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub languages: Vec<String>,
    pub severity: String,
    pub confidence: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub metadata: HashMap<String, String>,
}

/// Mock job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockJob {
    pub id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub job_type: String,
    pub progress: f64,
    pub result: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Mock finding information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockFinding {
    pub rule_id: String,
    pub message: String,
    pub severity: String,
    pub confidence: String,
    pub location: MockLocation,
    pub fix: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Mock location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockLocation {
    pub file: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub snippet: Option<String>,
}

/// Mock metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockMetricsData {
    pub findings_by_severity: HashMap<String, u64>,
    pub jobs_by_status: HashMap<String, u64>,
    pub active_jobs: u64,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub uptime_seconds: u64,
}

/// Mock data generators
pub struct MockRules;

impl MockRules {
    /// Generate mock rules for testing
    pub fn generate() -> Vec<MockRuleInfo> {
        vec![
            MockRuleInfo {
                id: "java-system-out".to_string(),
                name: "Avoid System.out usage".to_string(),
                description: "Detects usage of System.out.print* methods which should be replaced with proper logging".to_string(),
                languages: vec!["java".to_string()],
                severity: "warning".to_string(),
                confidence: "high".to_string(),
                category: Some("best-practice".to_string()),
                tags: vec!["logging".to_string(), "java".to_string()],
                enabled: true,
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("cwe".to_string(), "CWE-532".to_string());
                    map.insert("owasp".to_string(), "A09:2021".to_string());
                    map
                },
            },
            MockRuleInfo {
                id: "sql-injection".to_string(),
                name: "SQL Injection Detection".to_string(),
                description: "Detects potential SQL injection vulnerabilities".to_string(),
                languages: vec!["java".to_string(), "python".to_string(), "php".to_string()],
                severity: "error".to_string(),
                confidence: "high".to_string(),
                category: Some("security".to_string()),
                tags: vec!["sql".to_string(), "injection".to_string(), "security".to_string()],
                enabled: true,
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("cwe".to_string(), "CWE-89".to_string());
                    map.insert("owasp".to_string(), "A03:2021".to_string());
                    map
                },
            },
            MockRuleInfo {
                id: "hardcoded-password".to_string(),
                name: "Hardcoded Password".to_string(),
                description: "Detects hardcoded passwords in source code".to_string(),
                languages: vec!["java".to_string(), "javascript".to_string(), "python".to_string()],
                severity: "critical".to_string(),
                confidence: "medium".to_string(),
                category: Some("security".to_string()),
                tags: vec!["password".to_string(), "hardcoded".to_string(), "security".to_string()],
                enabled: true,
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("cwe".to_string(), "CWE-798".to_string());
                    map.insert("owasp".to_string(), "A07:2021".to_string());
                    map
                },
            },
        ]
    }

    /// Generate filtered mock rules
    pub fn generate_filtered(language: Option<&str>, category: Option<&str>, enabled: Option<bool>) -> Vec<MockRuleInfo> {
        let mut rules = Self::generate();
        
        if let Some(lang) = language {
            rules.retain(|rule| rule.languages.contains(&lang.to_string()));
        }
        
        if let Some(cat) = category {
            rules.retain(|rule| rule.category.as_ref().map_or(false, |c| c == cat));
        }
        
        if let Some(en) = enabled {
            rules.retain(|rule| rule.enabled == en);
        }
        
        rules
    }

    /// Find a rule by ID
    pub fn find_by_id(id: &str) -> Option<MockRuleInfo> {
        Self::generate().into_iter().find(|rule| rule.id == id)
    }
}

/// Mock job data generator
pub struct MockJobs;

impl MockJobs {
    /// Generate mock jobs for testing
    pub fn generate() -> Vec<MockJob> {
        let now = Utc::now();
        
        vec![
            MockJob {
                id: "job-001".to_string(),
                status: "completed".to_string(),
                created_at: now - Duration::hours(2),
                updated_at: now - Duration::minutes(30),
                completed_at: Some(now - Duration::minutes(30)),
                job_type: "code_analysis".to_string(),
                progress: 100.0,
                result: Some("Analysis completed successfully".to_string()),
                error: None,
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("files_analyzed".to_string(), "45".to_string());
                    map.insert("findings_count".to_string(), "12".to_string());
                    map
                },
            },
            MockJob {
                id: "job-002".to_string(),
                status: "running".to_string(),
                created_at: now - Duration::minutes(15),
                updated_at: now - Duration::minutes(1),
                completed_at: None,
                job_type: "rule_validation".to_string(),
                progress: 65.0,
                result: None,
                error: None,
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("rules_processed".to_string(), "13".to_string());
                    map.insert("total_rules".to_string(), "20".to_string());
                    map
                },
            },
            MockJob {
                id: "job-003".to_string(),
                status: "failed".to_string(),
                created_at: now - Duration::hours(1),
                updated_at: now - Duration::minutes(45),
                completed_at: Some(now - Duration::minutes(45)),
                job_type: "code_analysis".to_string(),
                progress: 25.0,
                result: None,
                error: Some("Failed to parse input file".to_string()),
                metadata: HashMap::new(),
            },
        ]
    }

    /// Find a job by ID
    pub fn find_by_id(id: &str) -> Option<MockJob> {
        Self::generate().into_iter().find(|job| job.id == id)
    }
}

/// Mock findings generator
pub struct MockFindings;

impl MockFindings {
    /// Generate mock findings for a given language and code
    pub fn generate_for_code(language: &str, code: &str) -> Vec<MockFinding> {
        vec![
            MockFinding {
                rule_id: format!("{}-demo-rule-001", language),
                message: format!("Demo finding in {} code", language),
                severity: "warning".to_string(),
                confidence: "medium".to_string(),
                location: MockLocation {
                    file: "input".to_string(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 10,
                    snippet: Some(code.lines().next().unwrap_or("").to_string()),
                },
                fix: Some("This is a demo fix suggestion".to_string()),
                metadata: None,
            }
        ]
    }
}

/// Mock metrics generator
pub struct MockMetrics;

impl MockMetrics {
    /// Generate mock system metrics
    pub fn generate() -> MockMetricsData {
        let mut findings_by_severity = HashMap::new();
        findings_by_severity.insert("info".to_string(), 450);
        findings_by_severity.insert("warning".to_string(), 280);
        findings_by_severity.insert("error".to_string(), 95);
        findings_by_severity.insert("critical".to_string(), 12);

        let mut jobs_by_status = HashMap::new();
        jobs_by_status.insert("completed".to_string(), 485);
        jobs_by_status.insert("running".to_string(), 2);
        jobs_by_status.insert("failed".to_string(), 23);

        MockMetricsData {
            findings_by_severity,
            jobs_by_status,
            active_jobs: 2,
            memory_usage: 128 * 1024 * 1024, // 128 MB
            cpu_usage: 15.5,
            uptime_seconds: 86400, // 1 day
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_rules_generation() {
        let rules = MockRules::generate();
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().any(|r| r.id == "java-system-out"));
        assert!(rules.iter().any(|r| r.id == "sql-injection"));
        assert!(rules.iter().any(|r| r.id == "hardcoded-password"));
    }

    #[test]
    fn test_mock_rules_filtering() {
        let java_rules = MockRules::generate_filtered(Some("java"), None, None);
        assert!(java_rules.iter().all(|r| r.languages.contains(&"java".to_string())));

        let security_rules = MockRules::generate_filtered(None, Some("security"), None);
        assert!(security_rules.iter().all(|r| r.category.as_ref().unwrap() == "security"));
    }

    #[test]
    fn test_mock_jobs_generation() {
        let jobs = MockJobs::generate();
        assert_eq!(jobs.len(), 3);
        assert!(jobs.iter().any(|j| j.status == "completed"));
        assert!(jobs.iter().any(|j| j.status == "running"));
        assert!(jobs.iter().any(|j| j.status == "failed"));
    }

    #[test]
    fn test_mock_findings_generation() {
        let findings = MockFindings::generate_for_code("java", "System.out.println(\"test\");");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "java-demo-rule-001");
    }

    #[test]
    fn test_mock_metrics_generation() {
        let metrics = MockMetrics::generate();
        assert!(metrics.findings_by_severity.contains_key("warning"));
        assert!(metrics.jobs_by_status.contains_key("completed"));
        assert!(metrics.active_jobs > 0);
    }
}
