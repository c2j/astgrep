//! Enhanced analyze command with advanced features
//! 
//! This module provides enhanced analysis capabilities with features like:
//! - Rule-based analysis with YAML rule files
//! - Pattern matching with tree-sitter integration
//! - Taint analysis
//! - Embedded SQL extraction and analysis
//! - Multiple output formats (JSON, SARIF, Text)

use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};
use astgrep_core::Language;

use crate::{EnhancedAnalysisConfig, PerformanceProfiler};
use crate::tree_sitter_analyzer::TreeSitterAnalyzer;
use crate::output::analysis::{Finding, AnalysisStatistics};

// Import submodules
mod file_ops;
mod rule_loader;
mod pattern_matcher;
mod output;
mod types;

// Re-export types that might be needed by other modules
pub use types::{ParsedRule, EmbeddedSqlSnippet, BasicPattern, determine_language, glob_match};
pub use file_ops::collect_target_files;
pub use rule_loader::load_rules_for_language;
pub use pattern_matcher::{apply_rule_to_source, get_basic_security_patterns};
pub use output::{generate_enhanced_output, apply_filters};

/// Run enhanced analysis with advanced features
pub async fn run_enhanced(config: EnhancedAnalysisConfig, output_file: Option<PathBuf>) -> Result<()> {
    let start_time = Instant::now();

    info!("Starting enhanced analysis");

    // Collect target files
    let target_files = collect_target_files(&config).await?;
    info!("Found {} files to analyze", target_files.len());

    if target_files.is_empty() {
        warn!("No files found to analyze");
        return Ok(());
    }

    // Run simplified analysis
    let mut all_findings = Vec::new();
    let mut analysis_stats = AnalysisStatistics::new();

    for file_path in target_files {
        info!("Analyzing file: {:?}", file_path);
        analyze_file_simple(&file_path, &config, &mut all_findings, &mut analysis_stats)?;
    }

    // Apply filters
    let filtered_findings = apply_filters(&all_findings, &config);

    // Apply max findings limit
    let limited_findings = if let Some(max) = config.max_findings {
        filtered_findings.into_iter().take(max).collect()
    } else {
        filtered_findings
    };

    // Generate output
    let total_time = start_time.elapsed();
    let output = generate_enhanced_output(
        &limited_findings,
        &analysis_stats,
        &config,
        total_time,
        None,
    )?;

    // Write output
    if let Some(output_path) = output_file {
        std::fs::write(&output_path, output)?;
        info!("Results written to: {}", output_path.display());
    } else {
        println!("{}", output);
    }

    // Exit with appropriate code
    if config.fail_on_findings && !limited_findings.is_empty() {
        info!("Found {} issues, exiting with error code", limited_findings.len());
        std::process::exit(1);
    }

    info!("Analysis completed in {:?}", total_time);
    Ok(())
}

fn analyze_file_simple(
    file_path: &PathBuf,
    config: &EnhancedAnalysisConfig,
    findings: &mut Vec<Finding>,
    stats: &mut AnalysisStatistics,
) -> Result<()> {
    stats.files_analyzed += 1;

    // Determine language from file extension
    let language = determine_language(file_path)?;

    // Skip if language is not in the configured languages
    if !config.languages.contains(&language) {
        return Ok(());
    }

    // Read file content with lossy UTF-8 conversion to handle invalid UTF-8 files
    let source_code = match std::fs::read(file_path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(e) => {
            warn!("Failed to read file {}: {}", file_path.display(), e);
            return Ok(());
        }
    };

    // Load rules if any are specified
    if !config.rule_files.is_empty() {
        // Use shared astgrep RuleEngine to ensure consistent behavior across CLI/GUI/Web
        let (file_findings, rules_count) = analyze_with_rule_engine(file_path, &source_code, language, config)?;
        findings.extend(file_findings);
        // Record executed rules count once
        if stats.rules_executed == 0 {
            stats.rules_executed = rules_count;
        }
    } else {
        // No rules specified - no findings
    }

    Ok(())
}

/// Real rule-based analysis using actual rule files
/// Returns (findings, rules_count)
fn analyze_with_basic_patterns(
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
    config: &EnhancedAnalysisConfig,
) -> Result<(Vec<Finding>, usize)> {
    let mut findings = Vec::new();

    // Load rules from the specified rule files/directories
    let rules = load_rules_for_language(&config.rule_files, language)?;

    if rules.is_empty() {
        info!("No rules found for language {:?}", language);
        return Ok((findings, 0));
    }

    let rules_count = rules.len();
    info!("Loaded {} rules for {:?}", rules_count, language);

    // Apply each rule to the source code
    for rule in &rules {
        let rule_findings = apply_rule_to_source(rule, file_path, source_code)?;
        findings.extend(rule_findings);
    }

    Ok((findings, rules_count))
}

/// Analyze a file using the shared astgrep RuleEngine (same semantics as GUI/Web)
fn analyze_with_rule_engine(
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
    config: &EnhancedAnalysisConfig,
) -> Result<(Vec<Finding>, usize)> {
    eprintln!("[DEBUG] entered analyze_with_rule_engine for {}", file_path.display());
    use astgrep_parser::LanguageParserRegistry;
    use astgrep_rules::{RuleContext, RuleEngine};
    use std::path::Path;

    // 1) Load rules into the shared engine
    let mut engine = RuleEngine::new();
    let rules_count = load_rules_into_engine_from_paths(&config.rule_files, &mut engine)?;
    if rules_count == 0 {
        return Ok((Vec::new(), 0));
    }

    // 2) Build AST once per file (if a parser exists). If not (e.g., Xml not yet wired), still allow preprocess path.
    let registry = LanguageParserRegistry::new();
    let parser_opt = registry.get_parser(language);
    let mut all_findings_core: Vec<astgrep_core::Finding> = Vec::new();

    if let Some(parser) = parser_opt {
        let ast = parser.parse(source_code, Path::new(file_path))?;

        let ast = try_tree_sitter_ast(source_code, language).unwrap_or(ast);

        // 3) Execute rules with unified context
        let mut context = RuleContext::new(
            file_path.to_string_lossy().to_string(),
            language,
            source_code.to_string(),
        );
        // Pass CLI level sql_statement_boundary (if provided) into context; per-rule YAML can override in engine
        if let Some(flag) = config.sql_statement_boundary {
            context = context.add_data("sql_statement_boundary".to_string(), flag.to_string());
        }

        // Perform constant propagation analysis if enabled
        // Use tree-sitter parser for better AST quality if available
        let constant_values = if config.enable_constant_propagation {
            use astgrep_dataflow::ConstantPropagator;
            use astgrep_parser::tree_sitter_parser::TreeSitterParser;
            
            let mut propagator = ConstantPropagator::new();
            
            // Try to use tree-sitter for better AST
            let constants_result = if let Ok(mut ts_parser) = TreeSitterParser::new() {
                if let Ok(Some(tree)) = ts_parser.parse(source_code, language) {
                    if let Ok(ts_ast) = ts_parser.tree_to_universal_ast(&tree, source_code) {
                        propagator.analyze_ast(&ts_ast)
                    } else {
                        propagator.analyze_ast(ast.as_ref())
                    }
                } else {
                    propagator.analyze_ast(ast.as_ref())
                }
            } else {
                propagator.analyze_ast(ast.as_ref())
            };
            
            match constants_result {
                Ok(constants) => {
                    if !constants.is_empty() {
                        tracing::info!("Constant propagation found {} constants", constants.len());
                        // Set constants in the engine's executor
                        engine.configure_executor().set_constant_values(constants.clone());
                        constants
                    } else {
                        std::collections::HashMap::new()
                    }
                }
                Err(e) => {
                    tracing::warn!("Constant propagation analysis failed: {}", e);
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        all_findings_core = engine.analyze(ast.as_ref(), &context)?;
    } else {
        tracing::warn!("No parser registered for {:?}; skipping direct analysis but will attempt preprocess path if configured", language);
    }

    // 3.b) YAML-configured preprocessors: allow SQL rules to apply to Java/XML via embedded SQL extraction
    // Convention: in rule YAML, set
    //   metadata:
    //     preprocess: "embedded-sql"
    //     preprocess.from: "java,xml"
    // When present on a SQL rule, we will extract SQL snippets from Java/XML sources and run the SQL rule on those snippets.
    {
        let lang_name = match language {
            Language::Java => "Java",
            Language::Xml => "Xml",
            Language::Sql => "Sql",
            _ => "Other",
        };
        eprintln!("[DEBUG-PREPROC] language for preprocessing check = {}", lang_name);
        tracing::info!("enhanced: language for preprocessing check = {}", lang_name);
    }
    if matches!(language, Language::Java | Language::Xml) {
        use astgrep_parser::LanguageParserRegistry;
        let registry2 = LanguageParserRegistry::new();
        if let Some(sql_parser) = registry2.get_parser(Language::Sql) {
            // Collect eligible SQL rules with preprocessing metadata
            tracing::info!("embedded-sql: total loaded rules = {}", engine.rules().len());
            let sql_rules: Vec<_> = engine
                .rules()
                .iter()
                .inspect(|r| {
                    tracing::debug!("rule id='{}', langs={:?}, metadata={:?}", r.id, r.languages, r.metadata);
                })
                .filter(|r| r.languages.contains(&Language::Sql))
                .filter(|r| {
                    if let Some(pp) = r.get_metadata_string("preprocess") {
                        pp == "embedded-sql"
                    } else { false }
                })
                .filter(|r| {
                    if let Some(from) = r.get_metadata_string("preprocess.from") {
                        let from_l = from.to_ascii_lowercase();
                        (language == Language::Java && from_l.contains("java")) ||
                        (language == Language::Xml && from_l.contains("xml"))
                    } else { false }
                })
                .cloned()
                .collect();
            tracing::info!("embedded-sql: eligible SQL rules after filter = {}", sql_rules.len());

            if !sql_rules.is_empty() {
                // Extract embedded SQL snippets
                let snippets = extract_embedded_sql_snippets(&source_code, language);
                tracing::info!("embedded-sql: {} eligible SQL rules; {} snippets extracted", sql_rules.len(), snippets.len());
                for (idx, sn) in snippets.iter().enumerate() {
                    if sn.sql.trim().is_empty() { continue; }
                    tracing::debug!("embedded-sql snippet #{}, start_line={}, context={:?}, sql_preview=\"{}\"",
                        idx + 1,
                        sn.start_line,
                        sn.context,
                        &sn.sql.chars().take(120).collect::<String>()
                    );
                    if let Ok(ast_sql) = sql_parser.parse(&sn.sql, std::path::Path::new(file_path)) {
                        // Build SQL context using original file path but snippet content
                        let mut ctx_sql = RuleContext::new(
                            file_path.to_string_lossy().to_string(),
                            Language::Sql,
                            sn.sql.clone(),
                        );
                        if let Some(flag) = config.sql_statement_boundary {
                            ctx_sql = ctx_sql.add_data("sql_statement_boundary".to_string(), flag.to_string());
                        }

                        for rule in &sql_rules {
                            if let Ok(Some(result)) = engine.execute_rule(&rule.id, ast_sql.as_ref(), &ctx_sql) {
                                if result.is_success() {
                                    tracing::debug!("embedded-sql: rule '{}' produced {} findings on snippet #{}", rule.id, result.findings.len(), idx + 1);
                                    for mut f in result.findings {
                                        // Adjust location lines by snippet offset
                                        let mut loc = f.location;
                                        let line_off = sn.start_line.saturating_sub(1);
                                        loc.start_line += line_off;
                                        loc.end_line += line_off;
                                        // Rewrap into CLI Finding
                                        all_findings_core.push(astgrep_core::Finding {
                                            rule_id: f.rule_id,
                                            message: f.message,
                                            severity: f.severity,
                                            confidence: f.confidence,
                                            location: astgrep_core::Location::new(
                                                std::path::PathBuf::from(&file_path),
                                                loc.start_line, loc.start_column, loc.end_line, loc.end_column
                                            ),
                                            metadata: {
                                                let mut m = f.metadata;
                                                if let Some(ctx) = sn.context.as_ref() { m.insert("embedded_context".to_string(), serde_yaml::Value::String(ctx.clone())); }
                                                m
                                            },
                                            fix_suggestion: f.fix_suggestion,
                                        });
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::warn!("embedded-sql: failed to parse snippet #{} as SQL", idx + 1);
                    }
                }
            }
        }
    }

    // 4) Convert to CLI Finding shape
    let mut findings = Vec::with_capacity(all_findings_core.len());
    for f in all_findings_core {
        findings.push(Finding {
            rule_id: f.rule_id,
            message: f.message,
            severity: f.severity,
            confidence: f.confidence,
            location: crate::output::analysis::Location {
                file: f.location.file,
                start_line: f.location.start_line,
                start_column: f.location.start_column,
                end_line: f.location.end_line,
                end_column: f.location.end_column,
            },
            fix: f.fix_suggestion,
        });
    }

    Ok((findings, rules_count))
}

/// Recursively load all YAML rules into the shared RuleEngine
fn load_rules_into_engine_from_paths(
    rule_paths: &[PathBuf],
    engine: &mut astgrep_rules::RuleEngine,
) -> Result<usize> {
    use std::fs;

    fn is_yaml(path: &std::path::Path) -> bool {
        path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml")
    }

    fn load_from_dir(dir: &std::path::Path, engine: &mut astgrep_rules::RuleEngine) -> anyhow::Result<usize> {
        let mut loaded = 0usize;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                loaded += load_from_dir(&path, engine)?;
            } else if is_yaml(&path) {
                if let Ok(content) = fs::read_to_string(&path) {
                    match engine.load_rules_from_yaml(&content) {
                        Ok(n) => { loaded += n; },
                        Err(e) => {
                            tracing::warn!("Failed to load rules from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        Ok(loaded)
    }

    let mut total = 0usize;
    for rule_path in rule_paths {
        if rule_path.is_file() {
            if is_yaml(rule_path) {
                if let Ok(content) = std::fs::read_to_string(rule_path) {
                    match engine.load_rules_from_yaml(&content) {
                        Ok(n) => { total += n; },
                        Err(e) => tracing::warn!("Failed to load rules from {:?}: {}", rule_path, e),
                    }
                }
            }
        } else if rule_path.is_dir() {
            total += load_from_dir(rule_path, engine)?;
        }
    }

    Ok(total)
}

/// Very lightweight extractor for SQL embedded in Java annotations/methods and MyBatis XML
fn extract_embedded_sql_snippets(source_code: &str, language: Language) -> Vec<EmbeddedSqlSnippet> {
    let mut out = Vec::new();
    match language {
        Language::Java => {
            use regex::Regex;
            // @Select("...") or @Query("...")
            if let Ok(re) = Regex::new(r#"(?s)@(?:[A-Za-z0-9_]+\.)*(Select|Query)\s*\(\s*"((?:\\.|[^"\\])*)"\s*\)"#) {
                for cap in re.captures_iter(source_code) {
                    if let (Some(m), Some(inner)) = (cap.get(0), cap.get(2)) {
                        let start_byte = m.start();
                        let start_line = 1 + byte_offset_to_line(source_code, start_byte);
                        let raw = inner.as_str();
                        let sql = normalize_sql(&unescape_java_string(raw));
                        out.push(EmbeddedSqlSnippet { sql, start_line, context: Some("@Select/@Query".to_string()) });
                    }
                }
            }
            // Common JDBC/native query methods with a single string literal argument
            if let Ok(re) = Regex::new(r#"(?s)\b(prepareStatement|executeQuery|createNativeQuery)\s*\(\s*"((?:\\.|[^"\\])*)""#) {
                for cap in re.captures_iter(source_code) {
                    if let (Some(m), Some(inner)) = (cap.get(0), cap.get(2)) {
                        let start_byte = m.start();
                        let start_line = 1 + byte_offset_to_line(source_code, start_byte);
                        let raw = inner.as_str();
                        let sql = normalize_sql(&unescape_java_string(raw));
                        out.push(EmbeddedSqlSnippet { sql, start_line, context: Some("JDBC".to_string()) });
                    }
                }
            }
        }
        Language::Xml => {
            use regex::Regex;
            // Extract inner text from <select>...</select>
            if let Ok(re) = Regex::new(r"(?is)<\s*select\b[^>]*>(.*?)</\s*select\s*>") {
                for cap in re.captures_iter(source_code) {
                    if let (Some(m0), Some(inner)) = (cap.get(0), cap.get(1)) {
                        let start_byte = m0.start();
                        let start_line = 1 + byte_offset_to_line(source_code, start_byte);
                        let raw = inner.as_str();
                        let sql = normalize_sql(raw);
                        out.push(EmbeddedSqlSnippet { sql, start_line, context: Some("<select>".to_string()) });
                    }
                }
            }
        }
        _ => {}
    }
    out
}

fn byte_offset_to_line(source: &str, byte_idx: usize) -> usize {
    // Returns 0-based line number corresponding to the byte offset
    let mut count = 0usize;
    for (i, b) in source.as_bytes().iter().enumerate() {
        if i >= byte_idx { break; }
        if *b == b'\n' { count += 1; }
    }
    count
}

fn unescape_java_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('\'') => out.push('\''),
                Some('u') => {
                    // rudimentary \uXXXX handling
                    let mut hex = String::new();
                    for _ in 0..4 { if let Some(h) = chars.next() { hex.push(h); } }
                    if let Ok(cp) = u16::from_str_radix(&hex, 16) {
                        if let Some(ch) = std::char::from_u32(cp as u32) { out.push(ch); }
                    }
                }
                Some(other) => { out.push(other); }
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn normalize_sql(raw: &str) -> String {
    use regex::Regex;
    let mut s = raw.to_string();
    // MyBatis placeholders
    if let Ok(re_hash) = Regex::new(r"(?is)#\{[^}]+\}") { s = re_hash.replace_all(&s, "1").into_owned(); }
    if let Ok(re_dollar) = Regex::new(r"(?is)\$\{[^}]+\}") { s = re_dollar.replace_all(&s, "T0").into_owned(); }
    // Collapse whitespace
    if let Ok(re_ws) = Regex::new(r"(?s)\s+") { s = re_ws.replace_all(&s, " ").into_owned(); }
    let s = s.trim().to_string();
    if s.ends_with(';') { s } else { format!("{};", s) }
}

fn try_tree_sitter_ast(
    source_code: &str,
    language: Language,
) -> Option<Box<dyn astgrep_core::AstNode>> {
    use astgrep_parser::tree_sitter_parser::TreeSitterParser;
    let mut ts_parser = TreeSitterParser::new().ok()?;
    let tree = ts_parser.parse(source_code, language).ok()??;
    let ts_ast = ts_parser.tree_to_universal_ast(&tree, source_code).ok()?;
    Some(Box::new(ts_ast))
}
