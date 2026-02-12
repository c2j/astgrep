//! Script execution verification for ASTGreP
//!
//! This module provides runtime verification and execution capabilities
//! for test scripts, ensuring they run correctly in the new organized structure.

use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
    process::{Stdio, Command},
    sync::{Arc, Mutex},
    thread,
};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn, error, debug, instrument};
use anyhow::{Result, anyhow};

mod types;
pub use types::{
    ScriptExecutionResult,
    ScriptMetadata,
    ScriptRunnerConfig,
    VerificationCheck,
    VerificationCheckType,
    VerificationStatistics,
    VerificationSummary,
};

/// Script execution verification runner
pub struct ScriptRunner {
    config: ScriptRunnerConfig,
    execution_count: Arc<Mutex<usize>>,
    success_count: Arc<Mutex<usize>>,
    failure_count: Arc<Mutex<usize>>,
}

impl ScriptRunner {
    /// Create a new script runner
    pub fn new(config: ScriptRunnerConfig) -> Self {
        Self {
            config,
            execution_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            failure_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Verify execution of multiple scripts
    #[instrument(skip(self, scripts))]
    pub async fn verify_scripts_execution(
        &self,
        scripts: &[PathBuf],
    ) -> Result<VerificationSummary> {
        info!("Starting verification of {} scripts", scripts.len());

        let start_time = Instant::now();
        let mut execution_results = Vec::new();

        if self.config.parallel_execution {
            execution_results = self.verify_scripts_parallel(scripts).await?;
        } else {
            execution_results = self.verify_scripts_sequential(scripts).await?;
        }

        let total_time = start_time.elapsed();
        let statistics = self.calculate_statistics(&execution_results);

        let summary = VerificationSummary {
            total_scripts: scripts.len(),
            successful_verifications: execution_results.iter().filter(|r| r.success).count(),
            failed_verifications: execution_results.iter().filter(|r| !r.success).count(),
            total_verification_time: total_time,
            execution_results,
            statistics,
            verified_at: chrono::Utc::now(),
        };

        info!("Verification completed: {}/{} scripts passed verification",
              summary.successful_verifications, summary.total_scripts);

        Ok(summary)
    }

    /// Verify scripts in parallel
    async fn verify_scripts_parallel(
        &self,
        scripts: &[PathBuf],
    ) -> Result<Vec<ScriptExecutionResult>> {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_executions));
        let execution_count = self.execution_count.clone();
        let success_count = self.success_count.clone();
        let failure_count = self.failure_count.clone();

        let tasks: Vec<_> = scripts.iter()
            .map(|script| {
                let semaphore = std::sync::Arc::clone(&semaphore);
                let config = self.config.clone();
                let execution_count = execution_count.clone();
                let success_count = success_count.clone();
                let failure_count = failure_count.clone();

                async move {
                    let _permit = semaphore.acquire().await;
                    let runner = ScriptRunner::new(config);
                    runner.verify_single_script(script).await
                }
            })
            .collect();

        let results = futures::future::join_all(tasks).await;
        Ok(results.into_iter().collect::<Result<Vec<_>>>()?)
    }

    /// Verify scripts sequentially
    async fn verify_scripts_sequential(
        &self,
        scripts: &[PathBuf],
    ) -> Result<Vec<ScriptExecutionResult>> {
        let mut results = Vec::new();

        for script in scripts {
            let result = self.verify_single_script(script).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Verify execution of a single script
    #[instrument(skip(self, script_path))]
    async fn verify_single_script(
        &self,
        script_path: &PathBuf,
    ) -> Result<ScriptExecutionResult> {
        debug!("Verifying script: {}", script_path.display());

        let start_time = Instant::now();
        let mut verification_checks = Vec::new();

        // Step 1: File existence check
        let existence_check = self.check_file_existence(script_path).await?;
        verification_checks.push(existence_check);

        // Step 2: File permissions check
        let permissions_check = self.check_file_permissions(script_path).await?;
        verification_checks.push(permissions_check);

        // Step 3: Syntax validation if enabled
        if self.config.validate_syntax {
            let syntax_check = self.validate_script_syntax(script_path).await?;
            verification_checks.push(syntax_check);
        }

        // Step 4: Dependency check if enabled
        if self.config.verify_dependencies {
            let dep_check = self.check_script_dependencies(script_path).await?;
            verification_checks.push(dep_check);
        }

        // Step 5: Collect script metadata
        let metadata = self.collect_script_metadata(script_path).await?;

        // Step 6: Execute script
        let execution_result = self.execute_script(script_path, &metadata).await?;
        let execution_time = start_time.elapsed();

        // Step 7: Performance check
        let performance_check = VerificationCheck {
            check_type: VerificationCheckType::PerformanceCheck,
            description: "Script execution time within acceptable limits".to_string(),
            passed: execution_time.as_millis() < self.config.execution_timeout.as_millis(),
            details: Some(format!("Execution time: {}ms", execution_time.as_millis())),
            check_time_ms: execution_time.as_millis() as u64,
        };
        verification_checks.push(performance_check);

        let result = ScriptExecutionResult {
            script_path: script_path.clone(),
            success: execution_result.success,
            exit_code: execution_result.exit_code,
            stdout: execution_result.stdout,
            stderr: execution_result.stderr,
            execution_time_ms: execution_time.as_millis() as u64,
            error_message: execution_result.error_message,
            verification_checks,
            metadata,
            executed_at: chrono::Utc::now(),
        };

        // Update counters
        {
            let mut count = self.execution_count.lock().unwrap();
            *count += 1;
        }

        {
            if result.success {
                let mut count = self.success_count.lock().unwrap();
                *count += 1;
            } else {
                let mut count = self.failure_count.lock().unwrap();
                *count += 1;
            }
        }

        debug!("Script verification completed: {:?} ({}ms)", result.success, result.execution_time_ms);
        Ok(result)
    }

    /// Check if script file exists
    async fn check_file_existence(&self, script_path: &PathBuf) -> Result<VerificationCheck> {
        let start_time = Instant::now();
        let exists = script_path.exists();

        Ok(VerificationCheck {
            check_type: VerificationCheckType::FileExistence,
            description: "Script file exists and is readable".to_string(),
            passed: exists,
            details: Some(format!("File: {}", script_path.display())),
            check_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Check script file permissions
    async fn check_file_permissions(&self, script_path: &PathBuf) -> Result<VerificationCheck> {
        let start_time = Instant::now();

        let metadata = std::fs::metadata(script_path)?;
        let readonly = metadata.permissions().readonly();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            let is_executable = mode & 0o111 != 0;

            Ok(VerificationCheck {
                check_type: VerificationCheckType::FilePermissions,
                description: "Script has appropriate permissions".to_string(),
                passed: is_executable,
                details: Some(format!("Permissions: {:o}, Executable: {}", mode, is_executable)),
                check_time_ms: start_time.elapsed().as_millis() as u64,
            })
        }

        #[cfg(not(unix))]
        {
            Ok(VerificationCheck {
                check_type: VerificationCheckType::FilePermissions,
                description: "Script file is accessible".to_string(),
                passed: !readonly,
                details: Some(format!("Read-only: {}", readonly)),
                check_time_ms: start_time.elapsed().as_millis() as u64,
            })
        }
    }

    /// Validate script syntax without executing
    async fn validate_script_syntax(&self, script_path: &PathBuf) -> Result<VerificationCheck> {
        let start_time = Instant::now();
        let extension = script_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let (passed, details) = match extension {
            "sh" | "bash" => self.validate_bash_syntax(script_path).await?,
            "py" => self.validate_python_syntax(script_path).await?,
            _ => (true, Some("Syntax validation not supported for this file type".to_string())),
        };

        Ok(VerificationCheck {
            check_type: VerificationCheckType::SyntaxValidation,
            description: "Script syntax is valid".to_string(),
            passed,
            details,
            check_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Validate bash script syntax
    async fn validate_bash_syntax(&self, script_path: &PathBuf) -> Result<(bool, Option<String>)> {
        let result = tokio::process::Command::new("bash")
            .arg("-n")
            .arg(script_path)
            .output()
            .await;

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok((true, Some("Bash syntax is valid".to_string())))
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    Ok((false, Some(format!("Bash syntax error: {}", error_msg))))
                }
            }
            Err(e) => Ok((true, Some(format!("Could not validate bash syntax: {}", e)))),
        }
    }

    /// Validate Python script syntax
    async fn validate_python_syntax(&self, script_path: &PathBuf) -> Result<(bool, Option<String>)> {
        let result = tokio::process::Command::new("python3")
            .arg("-m")
            .arg("py_compile")
            .arg(script_path)
            .output()
            .await;

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok((true, Some("Python syntax is valid".to_string())))
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    Ok((false, Some(format!("Python syntax error: {}", error_msg))))
                }
            }
            Err(e) => Ok((true, Some(format!("Could not validate Python syntax: {}", e)))),
        }
    }

    /// Check script dependencies
    async fn check_script_dependencies(&self, script_path: &PathBuf) -> Result<VerificationCheck> {
        let start_time = Instant::now();

        let content = std::fs::read_to_string(script_path)?;
        let mut missing_deps = Vec::new();
        let mut found_deps = Vec::new();

        // Check for common dependencies
        let dependency_checks = vec![
            ("bash", which::which("bash")),
            ("sh", which::which("sh")),
            ("python", which::which("python3").or_else(|_| which::which("python"))),
            ("python3", which::which("python3")),
            ("node", which::which("node")),
        ];

        for (dep_name, which_result) in dependency_checks {
            if content.contains(dep_name) {
                match which_result {
                    Ok(_) => found_deps.push(dep_name.to_string()),
                    Err(_) => missing_deps.push(dep_name.to_string()),
                }
            }
        }

        let passed = missing_deps.is_empty();
        let details = if passed {
            Some(format!("Found {} dependencies", found_deps.len()))
        } else {
            Some(format!("Missing dependencies: {:?}", missing_deps))
        };

        Ok(VerificationCheck {
            check_type: VerificationCheckType::DependencyCheck,
            description: "All required dependencies are available".to_string(),
            passed,
            details,
            check_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Collect script metadata
    async fn collect_script_metadata(&self, script_path: &PathBuf) -> Result<ScriptMetadata> {
        let metadata = std::fs::metadata(script_path)?;
        let content = std::fs::read_to_string(script_path)?;

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            Some(metadata.permissions().mode())
        };
        #[cfg(not(unix))]
        let permissions = None;

        let modified_at = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()));

        let shebang = content.lines().next()
            .filter(|line| line.starts_with("#!"))
            .map(|line| line.to_string());

        let script_type = self.detect_script_type(script_path, &content);
        let dependencies = self.extract_dependencies(&content);
        let checksum = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            Some(format!("{:x}", hasher.finalize()))
        };

        Ok(ScriptMetadata {
            file_size: metadata.len(),
            permissions,
            modified_at,
            script_type,
            shebang,
            dependencies,
            checksum,
        })
    }

    /// Detect script type from path and content
    fn detect_script_type(&self, script_path: &PathBuf, content: &str) -> String {
        // Check shebang first
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                if first_line.contains("bash") || first_line.contains("sh") {
                    return "bash".to_string();
                } else if first_line.contains("python") {
                    return "python".to_string();
                } else if first_line.contains("node") {
                    return "javascript".to_string();
                }
            }
        }

        // Fallback to file extension
        if let Some(extension) = script_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "sh" | "bash" => "bash".to_string(),
                "py" => "python".to_string(),
                "js" => "javascript".to_string(),
                "ts" => "typescript".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Extract dependencies from script content
    fn extract_dependencies(&self, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Bash dependencies
        for line in &lines {
            if line.trim_start().starts_with("source ") || line.trim_start().starts_with(". ") {
                if let Some(dep) = line.split_whitespace().nth(1) {
                    dependencies.push(dep.trim_matches('"').trim_start_matches("./").to_string());
                }
            }
        }

        // Python dependencies
        for line in &lines {
            if line.trim_start().starts_with("import ") || line.trim_start().starts_with("from ") {
                // Simple extraction - could be enhanced with proper parsing
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(parts[1].to_string());
                }
            }
        }

        dependencies
    }

    /// Execute script and capture results
    async fn execute_script(&self, script_path: &PathBuf, metadata: &ScriptMetadata) -> Result<ExecutionResult> {
        let result = tokio::time::timeout(
            self.config.execution_timeout,
            self.execute_script_with_timeout(script_path)
        ).await;

        match result {
            Ok(Ok(output)) => {
                Ok(ExecutionResult {
                    success: output.status.success(),
                    exit_code: output.status.code(),
                    stdout: if self.config.capture_output {
                        Some(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        None
                    },
                    stderr: if self.config.capture_output {
                        Some(String::from_utf8_lossy(&output.stderr).to_string())
                    } else {
                        None
                    },
                    error_message: None,
                })
            }
            Ok(Err(e)) => Ok(ExecutionResult {
                success: false,
                exit_code: None,
                stdout: None,
                stderr: None,
                error_message: Some(format!("Script execution failed: {}", e)),
            }),
            Err(_) => Ok(ExecutionResult {
                success: false,
                exit_code: None,
                stdout: None,
                stderr: None,
                error_message: Some("Script execution timed out".to_string()),
            }),
        }
    }

    /// Execute script with proper environment and working directory
    async fn execute_script_with_timeout(&self, script_path: &PathBuf) -> Result<std::process::Output> {
        let mut cmd = TokioCommand::new(script_path);

        // Set working directory
        if let Some(working_dir) = &self.config.working_directory {
            cmd.current_dir(working_dir);
        } else {
            cmd.current_dir(script_path.parent().unwrap_or(script_path));
        }

        // Set environment variables
        for (key, value) in &self.config.environment_variables {
            cmd.env(key, value);
        }

        // Configure I/O
        if self.config.capture_output {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        Ok(cmd.output().await?)
    }

    /// Calculate verification statistics
    fn calculate_statistics(&self, results: &[ScriptExecutionResult]) -> VerificationStatistics {
        let execution_times: Vec<u64> = results.iter().map(|r| r.execution_time_ms).collect();
        let successful_results: Vec<&ScriptExecutionResult> = results.iter().filter(|r| r.success).collect();

        let average_execution_time = if !execution_times.is_empty() {
            execution_times.iter().sum::<u64>() as f64 / execution_times.len() as f64
        } else {
            0.0
        };

        let fastest_time = execution_times.iter().min().copied().unwrap_or(0);
        let slowest_time = execution_times.iter().max().copied().unwrap_or(0);

        // Find most common failure reason
        let failure_reasons: HashMap<String, usize> = results.iter()
            .filter(|r| !r.success && r.error_message.is_some())
            .map(|r| r.error_message.as_ref().unwrap().clone())
            .fold(HashMap::new(), |mut map, reason| {
                *map.entry(reason).or_insert(0) += 1;
                map
            });

        let common_failure_reason = failure_reasons
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(reason, _)| reason.clone());

        let success_rate = if !results.is_empty() {
            successful_results.len() as f64 / results.len() as f64 * 100.0
        } else {
            0.0
        };

        let scripts_with_dependencies = results.iter()
            .filter(|r| !r.metadata.dependencies.is_empty())
            .count();

        // Find most common script type
        let script_types: HashMap<String, usize> = results.iter()
            .map(|r| r.metadata.script_type.clone())
            .fold(HashMap::new(), |mut map, script_type| {
                *map.entry(script_type).or_insert(0) += 1;
                map
            });

        let most_common_script_type = script_types
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(script_type, _)| script_type.clone());

        VerificationStatistics {
            average_execution_time_ms: average_execution_time,
            fastest_execution_time_ms: fastest_time,
            slowest_execution_time_ms: slowest_time,
            common_failure_reason,
            success_rate,
            scripts_with_dependencies,
            most_common_script_type,
        }
    }

    /// Generate verification report
    pub fn generate_verification_report(&self, summary: &VerificationSummary) -> String {
        let mut report = String::new();

        report.push_str("# Script Execution Verification Report\n\n");
        report.push_str(&format!("**Verification Date**: {}\n", summary.verified_at.format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("**Total Scripts**: {}\n", summary.total_scripts));
        report.push_str(&format!("**Successful**: {} ({:.1}%)\n",
            summary.successful_verifications, summary.statistics.success_rate));
        report.push_str(&format!("**Failed**: {}\n", summary.failed_verifications));
        report.push_str(&format!("**Total Time**: {:.2}s\n\n", summary.total_verification_time.as_secs_f64()));

        report.push_str("## Execution Statistics\n\n");
        report.push_str(&format!("- **Average Execution Time**: {:.2}ms\n", summary.statistics.average_execution_time_ms));
        report.push_str(&format!("- **Fastest Execution**: {}ms\n", summary.statistics.fastest_execution_time_ms));
        report.push_str(&format!("- **Slowest Execution**: {}ms\n", summary.statistics.slowest_execution_time_ms));
        report.push_str(&format!("- **Scripts with Dependencies**: {}\n", summary.statistics.scripts_with_dependencies));

        if let Some(script_type) = &summary.statistics.most_common_script_type {
            report.push_str(&format!("- **Most Common Script Type**: {}\n", script_type));
        }

        if let Some(failure_reason) = &summary.statistics.common_failure_reason {
            report.push_str(&format!("- **Common Failure Reason**: {}\n", failure_reason));
        }

        if !summary.execution_results.is_empty() {
            report.push_str("\n## Individual Script Results\n\n");

            for result in &summary.execution_results {
                let status = if result.success { "✅ PASS" } else { "❌ FAIL" };
                report.push_str(&format!("### {} - {}\n\n",
                    result.script_path.file_name().unwrap_or_default().to_string_lossy(),
                    status));

                report.push_str(&format!("- **Type**: {}\n", result.metadata.script_type));
                report.push_str(&format!("- **Execution Time**: {}ms\n", result.execution_time_ms));
                report.push_str(&format!("- **Exit Code**: {:?}\n", result.exit_code));

                if let Some(shebang) = &result.metadata.shebang {
                    report.push_str(&format!("- **Shebang**: {}\n", shebang));
                }

                if !result.metadata.dependencies.is_empty() {
                    report.push_str(&format!("- **Dependencies**: {:?}\n", result.metadata.dependencies));
                }

                report.push_str("- **Verification Checks**:\n");
                for check in &result.verification_checks {
                    let check_status = if check.passed { "✅" } else { "❌" };
                    report.push_str(&format!("  - {} {}: {}ms\n",
                        check_status, check.description, check.check_time_ms));
                }

                if let Some(error) = &result.error_message {
                    report.push_str(&format!("- **Error**: {}\n", error));
                }

                if let Some(stderr) = &result.stderr {
                    if !stderr.trim().is_empty() {
                        report.push_str(&format!("- **Stderr**: `{}`\n", stderr.trim()));
                    }
                }

                report.push_str("\n");
            }
        }

        report
    }
}

/// Result of script execution
#[derive(Debug)]
struct ExecutionResult {
    success: bool,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    error_message: Option<String>,
}

#[cfg(test)]
mod tests;
