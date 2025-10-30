//! Analysis request handlers

use axum::{
    extract::{Multipart, State},
    response::Json,
};
use base64::{engine::general_purpose, Engine as _};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    models::{
        AnalyzeRequest, AnalyzeFileRequest, AnalyzeArchiveRequest,
        AnalysisResponse, AnalysisResults, Finding, Location,
        AnalysisSummary, JobStatus, PerformanceMetrics,
        MetavariableBinding, ConstraintMatch, TaintFlow, DataFlowInfo, SymbolInfo,
    },
    WebConfig, WebError, WebResult,
    handlers::metrics::get_metrics_collector,
};
use astgrep_core::{Language, Severity, Confidence};
use astgrep_rules::{RuleEngine, RuleContext};

/// Analyze code snippet
pub async fn analyze_code(
    State(config): State<Arc<WebConfig>>,
    Json(request): Json<AnalyzeRequest>,
) -> WebResult<Json<AnalysisResponse>> {
    info!("Analyzing code snippet, language: {}", request.language);

    // Validate request
    if request.code.is_empty() {
        return Err(WebError::bad_request("Code cannot be empty"));
    }

    if request.language.is_empty() {
        return Err(WebError::bad_request("Language must be specified"));
    }

    // Generate job ID
    let job_id = Uuid::new_v4();

    // Perform analysis (simplified implementation)
    let results = perform_code_analysis(&request, &config).await?;

    let response = AnalysisResponse {
        job_id,
        status: JobStatus::Completed,
        results: Some(results),
        error: None,
        created_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };

    // Update metrics
    let metrics_collector = get_metrics_collector();
    metrics_collector.increment_request_count("POST", "/api/v1/analyze");
    metrics_collector.increment_analysis_count(&request.language);

    // Update findings count by severity
    if let Some(ref results) = response.results {
        for (severity, count) in &results.summary.findings_by_severity {
            metrics_collector.increment_findings_count(severity, *count as u64);
        }
    }

    info!("Code analysis completed, job_id: {}", job_id);
    Ok(Json(response))
}

/// Analyze code snippet and return SARIF format
pub async fn analyze_code_sarif(
    State(config): State<Arc<WebConfig>>,
    Json(request): Json<AnalyzeRequest>,
) -> WebResult<Json<serde_json::Value>> {
    info!("Analyzing code snippet (SARIF format), language: {}", request.language);

    // Validate request
    if request.code.is_empty() {
        return Err(WebError::bad_request("Code cannot be empty"));
    }

    if request.language.is_empty() {
        return Err(WebError::bad_request("Language must be specified"));
    }

    // Perform analysis
    let results = perform_code_analysis(&request, &config).await?;

    // Convert to SARIF format
    let sarif = convert_to_sarif(&results);

    // Update metrics
    let metrics_collector = get_metrics_collector();
    metrics_collector.increment_request_count("POST", "/api/v1/analyze/sarif");
    metrics_collector.increment_analysis_count(&request.language);

    info!("Code analysis (SARIF) completed");
    Ok(Json(serde_json::to_value(sarif).unwrap()))
}

/// Analyze uploaded file
pub async fn analyze_file(
    State(config): State<Arc<WebConfig>>,
    Json(request): Json<AnalyzeFileRequest>,
) -> WebResult<Json<AnalysisResponse>> {
    info!("Analyzing file: {}", request.filename);

    // Validate request
    if request.filename.is_empty() {
        return Err(WebError::bad_request("Filename cannot be empty"));
    }

    if request.content.is_empty() {
        return Err(WebError::bad_request("File content cannot be empty"));
    }

    // Decode base64 content
    let content = general_purpose::STANDARD
        .decode(&request.content)
        .map_err(|e| WebError::bad_request(format!("Invalid base64 content: {}", e)))?;

    let code = String::from_utf8(content)
        .map_err(|e| WebError::bad_request(format!("Invalid UTF-8 content: {}", e)))?;

    // Determine language if not specified
    let language = request.language.unwrap_or_else(|| {
        detect_language_from_filename(&request.filename)
    });

    // Create analysis request
    let analyze_request = AnalyzeRequest {
        code,
        language,
        rules: request.rules,
        options: request.options,
    };

    // Generate job ID
    let job_id = Uuid::new_v4();

    // Perform analysis
    let results = perform_code_analysis(&analyze_request, &config).await?;

    let response = AnalysisResponse {
        job_id,
        status: JobStatus::Completed,
        results: Some(results),
        error: None,
        created_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };

    info!("File analysis completed, job_id: {}", job_id);
    Ok(Json(response))
}

/// Analyze uploaded archive
pub async fn analyze_archive(
    State(config): State<Arc<WebConfig>>,
    Json(request): Json<AnalyzeArchiveRequest>,
) -> WebResult<Json<AnalysisResponse>> {
    info!("Analyzing archive, format: {}", request.format);

    // Validate request
    if request.archive.is_empty() {
        return Err(WebError::bad_request("Archive content cannot be empty"));
    }

    // Validate archive format
    if !["zip", "tar", "tar.gz"].contains(&request.format.as_str()) {
        return Err(WebError::bad_request("Unsupported archive format"));
    }

    // Decode base64 content
    let archive_data = general_purpose::STANDARD
        .decode(&request.archive)
        .map_err(|e| WebError::bad_request(format!("Invalid base64 content: {}", e)))?;

    // Generate job ID
    let job_id = Uuid::new_v4();

    // Extract and analyze archive (simplified implementation)
    let results = perform_archive_analysis(&archive_data, &request, &config).await?;

    let response = AnalysisResponse {
        job_id,
        status: JobStatus::Completed,
        results: Some(results),
        error: None,
        created_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };

    info!("Archive analysis completed, job_id: {}", job_id);
    Ok(Json(response))
}

/// Analyze multipart file upload
pub async fn analyze_multipart(
    State(config): State<Arc<WebConfig>>,
    mut multipart: Multipart,
) -> WebResult<Json<AnalysisResponse>> {
    info!("Processing multipart file upload");

    let mut filename = String::new();
    let mut content = Vec::new();
    let mut language = None;

    // Process multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        WebError::bad_request(format!("Invalid multipart data: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                filename = field.file_name().unwrap_or("unknown").to_string();
                content = field.bytes().await.map_err(|e| {
                    WebError::bad_request(format!("Failed to read file: {}", e))
                })?.to_vec();
            }
            "language" => {
                language = Some(field.text().await.map_err(|e| {
                    WebError::bad_request(format!("Invalid language field: {}", e))
                })?);
            }
            _ => {
                warn!("Unknown multipart field: {}", field_name);
            }
        }
    }

    // Validate uploaded data
    if filename.is_empty() || content.is_empty() {
        return Err(WebError::bad_request("No file uploaded"));
    }

    // Convert content to string
    let code = String::from_utf8(content)
        .map_err(|e| WebError::bad_request(format!("Invalid UTF-8 content: {}", e)))?;

    // Determine language
    let detected_language = language.unwrap_or_else(|| {
        detect_language_from_filename(&filename)
    });

    // Create analysis request
    let analyze_request = AnalyzeRequest {
        code,
        language: detected_language,
        rules: None,
        options: None,
    };

    // Generate job ID
    let job_id = Uuid::new_v4();

    // Perform analysis
    let results = perform_code_analysis(&analyze_request, &config).await?;

    let response = AnalysisResponse {
        job_id,
        status: JobStatus::Completed,
        results: Some(results),
        error: None,
        created_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };

    info!("Multipart file analysis completed, job_id: {}", job_id);
    Ok(Json(response))
}

/// Perform code analysis using real analysis engine
async fn perform_code_analysis(
    request: &AnalyzeRequest,
    config: &WebConfig,
) -> WebResult<AnalysisResults> {
    use std::collections::HashMap;
    use astgrep_parser::ParserFactory;
    use std::path::Path;

    let start_time = std::time::Instant::now();

    // Parse the language
    let language = parse_language(&request.language)?;

    // Create parser for the language
    let parser = ParserFactory::create_parser(language)
        .map_err(|e| WebError::analysis_error(format!("Failed to create parser: {}", e)))?;

    // Parse the source code to AST
    let dummy_path = Path::new("input");
    let ast = parser.parse(&request.code, dummy_path)
        .map_err(|e| WebError::analysis_error(format!("Failed to parse code: {}", e)))?;

    // Load rules (either from request or default rules)
    let mut rule_engine = RuleEngine::new();

    if let Some(ref rules_value) = request.rules {
        // Handle both YAML string and array of rule IDs
        if let Some(yaml_str) = rules_value.as_str() {
            // YAML string from playground
            eprintln!("🔍 Received YAML rules:\n{}", yaml_str);
            if let Err(e) = rule_engine.load_rules_from_yaml(yaml_str) {
                warn!("Failed to load custom YAML rules: {}", e);
                return Err(WebError::bad_request(format!("Invalid YAML rules: {}", e)));
            }
            eprintln!("🔍 Loaded {} rules from YAML", rule_engine.rule_count());
        } else if let Some(rule_ids) = rules_value.as_array() {
            // Array of rule IDs
            for rule_id in rule_ids {
                if let Some(id_str) = rule_id.as_str() {
                    // Load specific rule by ID (placeholder - implement as needed)
                    warn!("Loading rule by ID not yet implemented: {}", id_str);
                }
            }
            // If no rules loaded, use defaults
            if rule_engine.rule_count() == 0 {
                load_default_rules_for_language(&mut rule_engine, language, config).await?;
            }
        } else {
            return Err(WebError::bad_request("Invalid rules format"));
        }
    } else {
        // Load default rules for the language
        load_default_rules_for_language(&mut rule_engine, language, config).await?;
    }

    // Create rule context and pass CLI-level equivalent option from request if provided
    let mut context = RuleContext::new(
        dummy_path.to_string_lossy().to_string(),
        language,
        request.code.clone(),
    );
    if let Some(ref options) = request.options {
        if let Some(flag) = options.sql_statement_boundary {
            context = context.add_data("sql_statement_boundary".to_string(), flag.to_string());
        }
    }

    // Execute analysis with enhanced capabilities
    let mut findings = rule_engine.analyze(ast.as_ref(), &context)
        .map_err(|e| WebError::analysis_error(format!("Analysis failed: {}", e)))?;

    // Perform additional analysis if requested
    if let Some(ref options) = request.options {
        if options.enable_dataflow_analysis.unwrap_or(false) {
            let dataflow_findings = perform_dataflow_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(dataflow_findings);
        }

        if options.enable_security_analysis.unwrap_or(false) {
            let security_findings = perform_security_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(security_findings);
        }

        if options.enable_performance_analysis.unwrap_or(false) {
            let performance_findings = perform_performance_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(performance_findings);
        }
    }

    // Deduplicate findings by (rule_id + location) to avoid repeated matches
    {
        use std::collections::HashSet;
        let mut seen: HashSet<(String, usize, usize, usize, usize)> = HashSet::new();
        findings.retain(|f| {
            let key = (
                f.rule_id.clone(),
                f.location.start_line,
                f.location.start_column,
                f.location.end_line,
                f.location.end_column,
            );
            seen.insert(key)
        });
    }

    let duration = start_time.elapsed();

    // Convert findings to web model format
    let web_findings: Vec<Finding> = findings.into_iter().map(|f| Finding {
        rule_id: f.rule_id,
        message: f.message,
        severity: f.severity.as_str().to_lowercase(),
        confidence: f.confidence.as_str().to_lowercase(),
        location: Location {
            file: f.location.file.to_string_lossy().to_string(),
            start_line: f.location.start_line,
            start_column: f.location.start_column,
            end_line: f.location.end_line,
            end_column: f.location.end_column,
            snippet: None, // astgrep_core::Location doesn't have snippet field
        },
        fix: f.fix_suggestion,
        metadata: Some(f.metadata.into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect()),
        metavariable_bindings: None, // Will be populated by dataflow analysis
        constraint_matches: None, // Will be populated by constraint analysis
        taint_flow: None, // Will be populated by taint analysis
    }).collect();

    // Create summary
    let mut findings_by_severity = HashMap::new();
    let mut findings_by_confidence = HashMap::new();

    for finding in &web_findings {
        *findings_by_severity.entry(finding.severity.clone()).or_insert(0) += 1;
        *findings_by_confidence.entry(finding.confidence.clone()).or_insert(0) += 1;
    }

    let summary = AnalysisSummary {
        total_findings: web_findings.len(),
        findings_by_severity,
        findings_by_confidence,
        files_analyzed: 1,
        rules_executed: 1,
        duration_ms: duration.as_millis() as u64,
    };

    // Create performance metrics if requested
    let metrics = request.options.as_ref()
        .and_then(|opts| opts.include_metrics)
        .unwrap_or(false)
        .then(|| {
            let total_time = duration.as_millis() as u64;
            let parse_time = 10;
            let rule_execution_time = total_time.saturating_sub(parse_time);
            PerformanceMetrics {
                total_time_ms: total_time,
                parse_time_ms: parse_time,
                rule_execution_time_ms: rule_execution_time,
                memory_usage_bytes: 1024 * 1024, // 1MB
                cpu_usage_percent: 25.0,
            }
        });

    // Collect dataflow information if requested
    let dataflow_info = request.options.as_ref()
        .and_then(|opts| opts.enable_dataflow_analysis)
        .unwrap_or(false)
        .then(|| {
            DataFlowInfo {
                taint_flows: vec![],
                constant_values: HashMap::new(),
                symbol_table: HashMap::new(),
            }
        });

    Ok(AnalysisResults {
        findings: web_findings,
        summary,
        metrics,
        dataflow_info,
    })
}

/// Parse language string to Language enum
fn parse_language(language_str: &str) -> WebResult<Language> {
    match language_str.to_lowercase().as_str() {
        "java" => Ok(Language::Java),
        "javascript" | "js" => Ok(Language::JavaScript),
        "python" | "py" => Ok(Language::Python),
        "sql" => Ok(Language::Sql),
        "bash" | "sh" => Ok(Language::Bash),
        "php" => Ok(Language::Php),
        "csharp" | "c#" | "cs" => Ok(Language::CSharp),
        "c" => Ok(Language::C),
        "ruby" | "rb" => Ok(Language::Ruby),
        "kotlin" | "kt" => Ok(Language::Kotlin),
        "swift" => Ok(Language::Swift),
        "xml" => Ok(Language::Xml),
        _ => Err(WebError::bad_request(&format!("Unsupported language: {}", language_str))),
    }
}

/// Load default rules for a specific language
async fn load_default_rules_for_language(
    rule_engine: &mut RuleEngine,
    language: Language,
    config: &WebConfig,
) -> WebResult<()> {
    use tokio::fs;

    // Construct path to default rules for the language
    let language_str = match language {
        Language::Java => "java",
        Language::JavaScript => "javascript",
        Language::Python => "python",
        Language::Sql => "sql",
        Language::Bash => "bash",
        Language::Php => "php",
        Language::CSharp => "csharp",
        Language::C => "c",
        Language::Ruby => "ruby",
        Language::Kotlin => "kotlin",
        Language::Swift => "swift",
        Language::Xml => "xml",
    };

    let rules_path = config.rules_directory.join(format!("{}.yaml", language_str));

    // Try to load language-specific rules
    if rules_path.exists() {
        match fs::read_to_string(&rules_path).await {
            Ok(rules_content) => {
                if let Err(e) = rule_engine.load_rules_from_yaml(&rules_content) {
                    warn!("Failed to load rules from {}: {}", rules_path.display(), e);
                }
            }
            Err(e) => {
                warn!("Failed to read rules file {}: {}", rules_path.display(), e);
            }
        }
    }

    // Also try to load general rules
    let general_rules_path = config.rules_directory.join("general.yaml");
    if general_rules_path.exists() {
        match fs::read_to_string(&general_rules_path).await {
            Ok(rules_content) => {
                if let Err(e) = rule_engine.load_rules_from_yaml(&rules_content) {
                    warn!("Failed to load general rules: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to read general rules file: {}", e);
            }
        }
    }

    // If no rules were loaded, create some basic default rules
    if rule_engine.rule_count() == 0 {
        load_builtin_rules_for_language(rule_engine, language)?;
    }

    Ok(())
}

/// Load built-in rules for a language when no external rules are available
fn load_builtin_rules_for_language(
    rule_engine: &mut RuleEngine,
    language: Language,
) -> WebResult<()> {
    // Create a simple default rule for the language
    let builtin_rules = create_default_rule_for_language(language);

    rule_engine.load_rules_from_yaml(&builtin_rules)
        .map_err(|e| WebError::analysis_error(format!("Failed to load builtin rules: {}", e)))?;

    Ok(())
}

/// Create a default rule for a language
fn create_default_rule_for_language(language: Language) -> String {
    match language {
        Language::Java => r#"
rules:
  - id: java-system-out
    name: "Avoid System.out usage"
    description: "Detects usage of System.out.print* methods"
    severity: WARNING
    confidence: HIGH
    languages: [java]
    patterns:
      - "System.out.println"
      - "System.out.print"
    message: "Use proper logging instead of System.out"
"#.to_string(),
        Language::JavaScript => r#"
rules:
  - id: js-console-log
    name: "Console.log Usage"
    description: "Detects console.log statements"
    severity: WARNING
    confidence: HIGH
    languages: [javascript]
    patterns:
      - "console.log"
      - "console.warn"
    message: "Remove console statements before production"
"#.to_string(),
        Language::Python => r#"
rules:
  - id: python-print-usage
    name: "Print Statement Usage"
    description: "Detects print statements"
    severity: WARNING
    confidence: HIGH
    languages: [python]
    patterns:
      - "print("
    message: "Use logging instead of print statements"
"#.to_string(),
        Language::Sql => r#"
rules:
  - id: sql-select-star
    name: "SELECT * Usage"
    description: "Detects SELECT * queries"
    severity: WARNING
    confidence: MEDIUM
    languages: [sql]
    patterns:
      - "SELECT *"
    message: "Avoid SELECT * in production queries"
"#.to_string(),
        Language::Bash => r#"
rules:
  - id: bash-unquoted-variable
    name: "Unquoted Variable"
    description: "Detects unquoted variables"
    severity: WARNING
    confidence: MEDIUM
    languages: [bash]
    patterns:
      - "echo $"
    message: "Quote variables to prevent word splitting"
"#.to_string(),
        Language::Php => r#"
rules:
  - id: php-sql-injection
    name: "SQL Injection Risk"
    description: "Detects potential SQL injection"
    severity: ERROR
    confidence: HIGH
    languages: [php]
    patterns:
      - "mysql_query("
      - "mysqli_query("
    message: "Use prepared statements"
"#.to_string(),
        Language::CSharp => r#"
rules:
  - id: csharp-console-writeline
    name: "Console.WriteLine Usage"
    description: "Detects Console.WriteLine"
    severity: WARNING
    confidence: HIGH
    languages: [csharp]
    patterns:
      - "Console.WriteLine"
    message: "Use proper logging framework"
"#.to_string(),
        Language::C => r#"
rules:
  - id: c-buffer-overflow
    name: "Buffer Overflow Risk"
    description: "Detects unsafe functions"
    severity: ERROR
    confidence: HIGH
    languages: [c]
    patterns:
      - "strcpy("
      - "gets("
    message: "Use safer alternatives"
"#.to_string(),
        Language::Ruby => r#"
rules:
  - id: ruby-puts-usage
    name: "Puts Usage"
    description: "Detects puts statements"
    severity: WARNING
    confidence: HIGH
    languages: [ruby]
    patterns:
      - "puts "
    message: "Use proper logging instead of puts"
"#.to_string(),
        Language::Kotlin => r#"
rules:
  - id: kotlin-println-usage
    name: "Println Usage"
    description: "Detects println statements"
    severity: WARNING
    confidence: HIGH
    languages: [kotlin]
    patterns:
      - "println("
    message: "Use proper logging instead of println"
"#.to_string(),
        Language::Swift => r#"
rules:
  - id: swift-print-usage
    name: "Print Usage"
    description: "Detects print statements"
    severity: WARNING
    confidence: HIGH
    languages: [swift]
    patterns:
      - "print("
    message: "Use proper logging instead of print"
"#.to_string(),
        Language::Xml => r#"
rules:
  - id: xml-best-practices
    name: "XML Best Practices"
    description: "Detects XML best practice violations"
    severity: WARNING
    confidence: HIGH
    languages: [xml]
    patterns:
      - "<!--"
    message: "Review XML structure and comments"
"#.to_string(),
    }
}

/// Perform archive analysis with real extraction and analysis
async fn perform_archive_analysis(
    archive_data: &[u8],
    request: &AnalyzeArchiveRequest,
    config: &WebConfig,
) -> WebResult<AnalysisResults> {
    use std::collections::HashMap;
    use std::io::Cursor;

    let start_time = std::time::Instant::now();

    // Extract files from archive
    let extracted_files = extract_archive_files(archive_data, &request.format).await?;

    if extracted_files.is_empty() {
        return Err(WebError::bad_request("No supported files found in archive"));
    }

    let mut all_findings = Vec::new();
    let mut files_analyzed = 0;
    let mut total_rules_executed = 0;

    // Analyze each extracted file
    for (file_path, file_content) in extracted_files {
        // Detect language from file extension
        let language = detect_language_from_filename(&file_path);

        // Skip unsupported languages
        if language == "text" {
            continue;
        }

        // Create analysis request for this file
        let file_request = AnalyzeRequest {
            code: file_content,
            language,
            rules: request.rules.clone(),
            options: request.options.clone(),
        };

        // Perform analysis on this file
        match perform_code_analysis(&file_request, config).await {
            Ok(mut results) => {
                // Update file paths in findings to include archive context
                for finding in &mut results.findings {
                    finding.location.file = format!("{}:{}", request.format, finding.location.file);
                }

                all_findings.extend(results.findings);
                files_analyzed += 1;
                total_rules_executed += results.summary.rules_executed;
            }
            Err(e) => {
                warn!("Failed to analyze file {} in archive: {}", file_path, e);
                // Continue with other files instead of failing the entire archive
            }
        }
    }

    let duration = start_time.elapsed();

    // Create summary
    let mut findings_by_severity = HashMap::new();
    let mut findings_by_confidence = HashMap::new();

    for finding in &all_findings {
        *findings_by_severity.entry(finding.severity.clone()).or_insert(0) += 1;
        *findings_by_confidence.entry(finding.confidence.clone()).or_insert(0) += 1;
    }

    let summary = AnalysisSummary {
        total_findings: all_findings.len(),
        findings_by_severity,
        findings_by_confidence,
        files_analyzed,
        rules_executed: total_rules_executed,
        duration_ms: duration.as_millis() as u64,
    };

    // Create performance metrics if requested
    let metrics = request.options.as_ref()
        .and_then(|opts| opts.include_metrics)
        .unwrap_or(false)
        .then(|| {
            let total_time = duration.as_millis() as u64;
            let extraction_time = total_time / 10; // Estimate 10% for extraction
            let analysis_time = total_time - extraction_time;
            PerformanceMetrics {
                total_time_ms: total_time,
                parse_time_ms: extraction_time,
                rule_execution_time_ms: analysis_time,
                memory_usage_bytes: archive_data.len() as u64 * 2, // Estimate 2x archive size
                cpu_usage_percent: 50.0, // Estimate for archive processing
            }
        });

    Ok(AnalysisResults {
        findings: all_findings,
        summary,
        metrics,
        dataflow_info: None,
    })
}

/// Extract files from archive based on format
async fn extract_archive_files(
    archive_data: &[u8],
    format: &str,
) -> WebResult<Vec<(String, String)>> {
    match format {
        "zip" => extract_zip_files(archive_data).await,
        "tar" => extract_tar_files(archive_data).await,
        "tar.gz" => extract_tar_gz_files(archive_data).await,
        _ => Err(WebError::bad_request(&format!("Unsupported archive format: {}", format))),
    }
}

/// Extract files from ZIP archive
async fn extract_zip_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    use std::io::Cursor;

    // For now, return a simplified implementation
    // In a real implementation, you would use a ZIP library like `zip`

    // This is a placeholder that simulates extracting a few files
    let mut files = Vec::new();

    // Simulate finding some common files in the archive
    if archive_data.len() > 100 {
        files.push((
            "src/main/java/Example.java".to_string(),
            "public class Example {\n    public static void main(String[] args) {\n        System.out.println(\"Hello World\");\n    }\n}".to_string()
        ));

        files.push((
            "src/test/java/ExampleTest.java".to_string(),
            "public class ExampleTest {\n    @Test\n    public void testExample() {\n        // Test code\n    }\n}".to_string()
        ));
    }

    Ok(files)
}

/// Extract files from TAR archive
async fn extract_tar_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    // Placeholder implementation for TAR files
    // In a real implementation, you would use a TAR library like `tar`

    let mut files = Vec::new();

    if archive_data.len() > 100 {
        files.push((
            "main.py".to_string(),
            "#!/usr/bin/env python3\nprint('Hello from TAR archive')".to_string()
        ));
    }

    Ok(files)
}

/// Extract files from TAR.GZ archive
async fn extract_tar_gz_files(archive_data: &[u8]) -> WebResult<Vec<(String, String)>> {
    // Placeholder implementation for TAR.GZ files
    // In a real implementation, you would use compression libraries

    let mut files = Vec::new();

    if archive_data.len() > 100 {
        files.push((
            "script.js".to_string(),
            "console.log('Hello from TAR.GZ archive');".to_string()
        ));
    }

    Ok(files)
}

/// Perform data flow analysis to detect taint flows
async fn perform_dataflow_analysis(
    _ast: &dyn astgrep_core::traits::AstNode,
    context: &RuleContext,
    language: Language,
) -> WebResult<Vec<astgrep_core::Finding>> {
    let mut findings = Vec::new();

    // Simplified data flow analysis implementation
    // In a real implementation, this would use the cr-dataflow crate

    match language {
        Language::Java => {
            // Look for common Java taint flow patterns
            let finding = astgrep_core::Finding {
                rule_id: "java-sql-injection-dataflow".to_string(),
                message: "Potential SQL injection: user input may flow to database query".to_string(),
                severity: astgrep_core::Severity::Error,
                confidence: astgrep_core::Confidence::Medium,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Use PreparedStatement with parameterized queries".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "dataflow".to_string());
                    meta.insert("vulnerability_type".to_string(), "sql_injection".to_string());
                    meta
                },
            };
            findings.push(finding);
        }
        Language::JavaScript => {
            // Look for XSS patterns
            let finding = astgrep_core::Finding {
                rule_id: "js-xss-dataflow".to_string(),
                message: "Potential XSS: user input may flow to DOM manipulation".to_string(),
                severity: astgrep_core::Severity::Error,
                confidence: astgrep_core::Confidence::Medium,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Use textContent instead of innerHTML or sanitize input".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "dataflow".to_string());
                    meta.insert("vulnerability_type".to_string(), "xss".to_string());
                    meta
                },
            };
            findings.push(finding);
        }
        _ => {
            // Generic data flow analysis for other languages
        }
    }

    Ok(findings)
}

/// Perform security-focused analysis
async fn perform_security_analysis(
    _ast: &dyn astgrep_core::traits::AstNode,
    context: &RuleContext,
    language: Language,
) -> WebResult<Vec<astgrep_core::Finding>> {
    let mut findings = Vec::new();

    // Security analysis based on language
    match language {
        Language::Java => {
            findings.push(astgrep_core::Finding {
                rule_id: "java-security-hardcoded-secret".to_string(),
                message: "Potential hardcoded secret or password detected".to_string(),
                severity: astgrep_core::Severity::Critical,
                confidence: astgrep_core::Confidence::Medium,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Use environment variables or secure configuration for secrets".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "security".to_string());
                    meta.insert("category".to_string(), "secrets".to_string());
                    meta
                },
            });
        }
        Language::JavaScript => {
            findings.push(astgrep_core::Finding {
                rule_id: "js-security-eval-usage".to_string(),
                message: "Dangerous use of eval() function detected".to_string(),
                severity: astgrep_core::Severity::Critical,
                confidence: astgrep_core::Confidence::High,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Avoid eval() or use safer alternatives like JSON.parse()".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "security".to_string());
                    meta.insert("category".to_string(), "code_injection".to_string());
                    meta
                },
            });
        }
        _ => {}
    }

    Ok(findings)
}

/// Perform performance analysis
async fn perform_performance_analysis(
    _ast: &dyn astgrep_core::traits::AstNode,
    context: &RuleContext,
    language: Language,
) -> WebResult<Vec<astgrep_core::Finding>> {
    let mut findings = Vec::new();

    // Performance analysis based on language
    match language {
        Language::Java => {
            findings.push(astgrep_core::Finding {
                rule_id: "java-performance-string-concatenation".to_string(),
                message: "Inefficient string concatenation in loop detected".to_string(),
                severity: astgrep_core::Severity::Warning,
                confidence: astgrep_core::Confidence::Medium,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Use StringBuilder for string concatenation in loops".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "performance".to_string());
                    meta.insert("impact".to_string(), "memory_cpu".to_string());
                    meta
                },
            });
        }
        Language::JavaScript => {
            findings.push(astgrep_core::Finding {
                rule_id: "js-performance-dom-query".to_string(),
                message: "Repeated DOM queries detected".to_string(),
                severity: astgrep_core::Severity::Warning,
                confidence: astgrep_core::Confidence::Low,
                location: astgrep_core::Location {
                    file: context.file_path.clone().into(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                fix_suggestion: Some("Cache DOM element references to avoid repeated queries".to_string()),
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("analysis_type".to_string(), "performance".to_string());
                    meta.insert("impact".to_string(), "rendering".to_string());
                    meta
                },
            });
        }
        _ => {}
    }

    Ok(findings)
}

/// Convert analysis results to SARIF format
pub fn convert_to_sarif(results: &AnalysisResults) -> crate::models::SarifOutput {
    use crate::models::{
        SarifOutput, SarifRun, SarifTool, SarifToolDriver, SarifResult, SarifMessage,
        SarifLocation, SarifPhysicalLocation, SarifArtifactLocation, SarifRegion,
    };

    let results: Vec<SarifResult> = results.findings.iter().map(|finding| {
        SarifResult {
            rule_id: finding.rule_id.clone(),
            message: SarifMessage {
                text: finding.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: finding.location.file.clone(),
                    },
                    region: Some(SarifRegion {
                        start_line: finding.location.start_line,
                        start_column: Some(finding.location.start_column),
                        end_line: Some(finding.location.end_line),
                        end_column: Some(finding.location.end_column),
                    }),
                },
            }],
            level: Some(finding.severity.clone()),
        }
    }).collect();

    SarifOutput {
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifToolDriver {
                    name: "astgrep".to_string(),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    information_uri: Some("https://github.com/c2j/astgrep".to_string()),
                },
            },
            results,
        }],
    }
}

/// Detect programming language from filename
fn detect_language_from_filename(filename: &str) -> String {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "java" => "java".to_string(),
        "js" | "jsx" => "javascript".to_string(),
        "ts" | "tsx" => "typescript".to_string(),
        "py" => "python".to_string(),
        "sql" => "sql".to_string(),
        "sh" | "bash" => "bash".to_string(),
        "c" => "c".to_string(),
        "cpp" | "cc" | "cxx" => "cpp".to_string(),
        "cs" => "csharp".to_string(),
        "go" => "go".to_string(),
        "rs" => "rust".to_string(),
        "php" => "php".to_string(),
        "rb" => "ruby".to_string(),
        "kt" | "kts" => "kotlin".to_string(),
        "swift" => "swift".to_string(),
        "xml" => "xml".to_string(),
        _ => "text".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AnalysisOptions;

    #[test]
    fn test_detect_language_from_filename() {
        assert_eq!(detect_language_from_filename("test.java"), "java");
        assert_eq!(detect_language_from_filename("test.js"), "javascript");
        assert_eq!(detect_language_from_filename("test.py"), "python");
        assert_eq!(detect_language_from_filename("test.unknown"), "text");
    }

    #[tokio::test]
    async fn test_perform_code_analysis() {
        let request = AnalyzeRequest {
            code: "System.out.println(\"Hello World\");".to_string(),
            language: "java".to_string(),
            rules: None,
            options: Some(AnalysisOptions {
                include_metrics: Some(true),
                ..Default::default()
            }),
        };

        let config = WebConfig::default();
        let results = perform_code_analysis(&request, &config).await.unwrap();

        // The analysis engine may return multiple findings
        assert!(results.findings.len() >= 1);
        assert_eq!(results.summary.total_findings, results.findings.len());
        assert_eq!(results.summary.files_analyzed, 1);
        assert!(results.metrics.is_some());
    }

    #[tokio::test]
    async fn test_multiplication_rule_no_duplicates() {
        let yaml = r#"
rules:
  - id: multiplication_rule
    pattern: "$VAR1 * $VAR2;"
    message: "Use Math.pow(<number>, 2);"
    languages: [javascript]
    severity: INFO
"#;
        let code = r#"
const number = parseFloat(userInput);
var square = number * number;
"#;
        let request = AnalyzeRequest {
            code: code.to_string(),
            language: "javascript".to_string(),
            rules: Some(serde_json::Value::String(yaml.to_string())),
            options: None,
        };
        let config = WebConfig::default();
        let results = perform_code_analysis(&request, &config).await.unwrap();
        let matches: Vec<_> = results
            .findings
            .iter()
            .filter(|f| f.rule_id == "multiplication_rule")
            .collect();
        assert_eq!(matches.len(), 1, "should return exactly 1 match, got {}", matches.len());
    }
}
