use std::path::Path;

use astgrep_core::Language;
use astgrep_rules::{RuleContext, RuleEngine};
use tracing::warn;

use super::analysis_extras::{
    perform_dataflow_analysis,
    perform_performance_analysis,
    perform_security_analysis,
};
use super::embedded_sql::extract_embedded_sql_snippets;
use super::rules::{load_default_rules_for_language, parse_language};
use crate::{
    models::{
        AnalysisResults, AnalysisSummary, DataFlowInfo, Finding, Location, PerformanceMetrics,
        AnalyzeRequest,
    },
    WebConfig, WebError, WebResult,
};

/// Perform code analysis using real analysis engine
pub async fn perform_code_analysis(
    request: &AnalyzeRequest,
    config: &WebConfig,
) -> WebResult<AnalysisResults> {
    use astgrep_parser::ParserFactory;
    use std::collections::HashMap;

    let start_time = std::time::Instant::now();

    // Parse the language
    let language = parse_language(&request.language)?;

    // Create parser for the language
    let parser = ParserFactory::create_parser(language)
        .map_err(|e| WebError::analysis_error(format!("Failed to create parser: {}", e)))?;

    // Parse the source code to AST
    let dummy_path = Path::new("input");
    let ast = parser
        .parse(&request.code, dummy_path)
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
    let mut findings = rule_engine
        .analyze(ast.as_ref(), &context)
        .map_err(|e| WebError::analysis_error(format!("Analysis failed: {}", e)))?;

    // Perform additional analysis if requested
    if let Some(ref options) = request.options {
        if options.enable_dataflow_analysis.unwrap_or(false) {
            let dataflow_findings =
                perform_dataflow_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(dataflow_findings);
        }

        if options.enable_security_analysis.unwrap_or(false) {
            let security_findings =
                perform_security_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(security_findings);
        }

        if options.enable_performance_analysis.unwrap_or(false) {
            let performance_findings =
                perform_performance_analysis(ast.as_ref(), &context, language).await?;
            findings.extend(performance_findings);
        }
    }
    // Embedded SQL preprocessing: apply SQL rules with metadata.preprocess=embedded-sql to Java/XML sources
    if matches!(language, Language::Java | Language::Xml) {
        use astgrep_parser::ParserFactory;
        // Collect eligible SQL rules that request embedded-sql preprocessing from this language
        let sql_rules: Vec<_> = rule_engine
            .rules()
            .iter()
            .filter(|r| r.languages.contains(&Language::Sql))
            .filter(|r| {
                r.metadata
                    .get("preprocess")
                    .map(|v| v.eq_ignore_ascii_case("embedded-sql"))
                    .unwrap_or(false)
            })
            .filter(|r| {
                if let Some(from) = r.metadata.get("preprocess.from") {
                    let from_l = from.to_ascii_lowercase();
                    (language == Language::Java && from_l.contains("java"))
                        || (language == Language::Xml && from_l.contains("xml"))
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        if !sql_rules.is_empty() {
            if let Ok(sql_parser) = ParserFactory::create_parser(Language::Sql) {
                let snippets = extract_embedded_sql_snippets(&request.code, language);
                for sn in &snippets {
                    if sn.sql.trim().is_empty() {
                        continue;
                    }
                    if let Ok(ast_sql) = sql_parser.parse(&sn.sql, dummy_path) {
                        let ctx_sql = RuleContext::new(
                            context.file_path.clone(),
                            Language::Sql,
                            sn.sql.clone(),
                        );
                        for rule in &sql_rules {
                            if let Ok(Some(result)) =
                                rule_engine.execute_rule(&rule.id, ast_sql.as_ref(), &ctx_sql)
                            {
                                for mut f in result.findings {
                                    // Map snippet-relative line numbers back to the original file
                                    let line_off = sn.start_line.saturating_sub(1);
                                    if f.location.start_line > 0 {
                                        f.location.start_line += line_off;
                                    }
                                    if f.location.end_line > 0 {
                                        f.location.end_line += line_off;
                                    }
                                    // Annotate metadata to indicate preprocessing origin
                                    f.metadata
                                        .insert("preprocess".to_string(), "embedded-sql".to_string());
                                    if let Some(ref c) = sn.context {
                                        f.metadata
                                            .insert("embedded_context".to_string(), c.clone());
                                    }
                                    findings.push(f);
                                }
                            }
                        }
                    }
                }
            }
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
    let web_findings: Vec<Finding> = findings
        .into_iter()
        .map(|f| Finding {
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
            metadata: Some(
                f.metadata
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
            ),
            metavariable_bindings: None, // Will be populated by dataflow analysis
            constraint_matches: None,    // Will be populated by constraint analysis
            taint_flow: None,            // Will be populated by taint analysis
        })
        .collect();

    // Create summary
    let mut findings_by_severity = HashMap::new();
    let mut findings_by_confidence = HashMap::new();

    for finding in &web_findings {
        *findings_by_severity
            .entry(finding.severity.clone())
            .or_insert(0) += 1;
        *findings_by_confidence
            .entry(finding.confidence.clone())
            .or_insert(0) += 1;
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
    let metrics = request
        .options
        .as_ref()
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
    let dataflow_info = request
        .options
        .as_ref()
        .and_then(|opts| opts.enable_dataflow_analysis)
        .unwrap_or(false)
        .then(|| DataFlowInfo {
            taint_flows: vec![],
            constant_values: HashMap::new(),
            symbol_table: HashMap::new(),
        });

    Ok(AnalysisResults {
        findings: web_findings,
        summary,
        metrics,
        dataflow_info,
    })
}
