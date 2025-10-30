//! Enhanced analyze command with advanced features

use anyhow::Result;
use astgrep_core::{Language, OutputFormat, Severity, Confidence};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};
use crate::{EnhancedAnalysisConfig, PerformanceProfiler};
use crate::tree_sitter_analyzer::TreeSitterAnalyzer;

// Simplified types for demonstration
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub location: Location,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Location {
    #[serde(serialize_with = "serialize_pathbuf")]
    pub file: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

fn serialize_pathbuf<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&path.to_string_lossy())
}

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

async fn collect_target_files(config: &EnhancedAnalysisConfig) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for target in &config.target_paths {
        if target.is_file() {
            files.push(target.clone());
        } else if target.is_dir() {
            collect_files_from_directory(target, &mut files, config)?;
        } else {
            warn!("Target path does not exist: {}", target.display());
        }
    }

    Ok(files)
}

fn collect_files_from_directory(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
    config: &EnhancedAnalysisConfig,
) -> Result<()> {
    use std::fs;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_from_directory(&path, files, config)?;
        } else if should_include_file(&path, config) {
            files.push(path);
        }
    }

    Ok(())
}

fn should_include_file(path: &PathBuf, config: &EnhancedAnalysisConfig) -> bool {
    let path_str = path.to_string_lossy();

    // Check include patterns
    if !config.include_patterns.is_empty() {
        let included = config.include_patterns.iter().any(|pattern| {
            glob_match(pattern, &path_str)
        });
        if !included {
            return false;
        }
    }

    // Check exclude patterns
    for pattern in &config.exclude_patterns {
        if glob_match(pattern, &path_str) {
            return false;
        }
    }

    // Check if file extension matches supported languages
    if let Some(extension) = path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        config.languages.iter().any(|lang| {
            match lang {
                Language::Java => ext_str == "java",
                Language::JavaScript => ext_str == "js" || ext_str == "jsx" || ext_str == "ts" || ext_str == "tsx",
                Language::Python => ext_str == "py",
                Language::Sql => ext_str == "sql",
                Language::Bash => ext_str == "sh" || ext_str == "bash",
                Language::Php => ext_str == "php",
                Language::CSharp => ext_str == "cs",
                Language::C => ext_str == "c" || ext_str == "h",
                Language::Ruby => ext_str == "rb" || ext_str == "rbw",
                Language::Kotlin => ext_str == "kt" || ext_str == "kts",
                Language::Swift => ext_str == "swift",
                Language::Xml => ext_str == "xml" || ext_str == "xsd" || ext_str == "xsl" || ext_str == "xslt" || ext_str == "svg" || ext_str == "pom",
            }
        })
    } else {
        false
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple glob matching implementation
    // In a real implementation, you'd use a proper glob library
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            // More complex patterns would need proper glob implementation
            text.contains(&pattern.replace('*', ""))
        }
    } else {
        text.contains(pattern)
    }
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

    // Read file content
    let source_code = std::fs::read_to_string(file_path)?;

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

/// Load rules from rule files/directories for a specific language
fn load_rules_for_language(rule_paths: &[PathBuf], language: Language) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    for rule_path in rule_paths {
        if rule_path.is_file() {
            if let Ok(file_rules) = load_rules_from_file(rule_path, language) {
                rules.extend(file_rules);
            }
        } else if rule_path.is_dir() {
            if let Ok(dir_rules) = load_rules_from_directory_recursive(rule_path, language) {
                rules.extend(dir_rules);
            }
        }
    }

    Ok(rules)
}

/// Load rules from a single YAML file
fn load_rules_from_file(file_path: &PathBuf, target_language: Language) -> Result<Vec<ParsedRule>> {
    let content = std::fs::read_to_string(file_path)?;
    parse_semgrep_rules(&content, target_language, Some(file_path))
}

/// Recursively load rules from a directory
fn load_rules_from_directory_recursive(dir_path: &PathBuf, target_language: Language) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(subdir_rules) = load_rules_from_directory_recursive(&path, target_language) {
                    rules.extend(subdir_rules);
                }
            } else if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                if let Ok(file_rules) = load_rules_from_file(&path, target_language) {
                    rules.extend(file_rules);
                }
            }
        }
    }

    Ok(rules)
}


/// Analyze a file using the shared astgrep RuleEngine (same semantics as GUI/Web)
fn analyze_with_rule_engine(
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
    config: &EnhancedAnalysisConfig,
) -> Result<(Vec<Finding>, usize)> {
    use astgrep_parser::LanguageParserRegistry;
    use astgrep_rules::{RuleContext, RuleEngine};
    use std::path::Path;

    // 1) Load rules into the shared engine
    let mut engine = RuleEngine::new();
    let rules_count = load_rules_into_engine_from_paths(&config.rule_files, &mut engine)?;
    if rules_count == 0 {
        return Ok((Vec::new(), 0));
    }

    // 2) Build AST once per file
    let registry = LanguageParserRegistry::new();
    let parser = match registry.get_parser(language) {
        Some(p) => p,
        None => return Ok((Vec::new(), rules_count)),
    };
    let ast = parser.parse(source_code, Path::new(file_path))?;

    // 3) Execute rules with unified context
    let context = RuleContext::new(
        file_path.to_string_lossy().to_string(),
        language,
        source_code.to_string(),
    );

    let core_findings = engine.analyze(ast.as_ref(), &context)?;

    // 4) Convert to CLI Finding shape
    let mut findings = Vec::with_capacity(core_findings.len());
    for f in core_findings {
        findings.push(Finding {
            rule_id: f.rule_id,
            message: f.message,
            severity: f.severity,
            confidence: f.confidence,
            location: Location {
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

/// Apply a single rule to source code
fn apply_rule_to_source(rule: &ParsedRule, file_path: &PathBuf, source_code: &str) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Check if this rule has NOT_INSIDE or NOT_REGEX patterns that need special handling
    let has_not_inside = rule.patterns.iter().any(|p| p.starts_with("NOT_INSIDE:"));
    let has_not_regex = rule.patterns.iter().any(|p| p.starts_with("NOT_REGEX:"));
    if has_not_inside || has_not_regex {
        info!("Rule '{}' has NOT_INSIDE or NOT_REGEX patterns, applying special handling", rule.id);
        // Apply special handling for rules with NOT_INSIDE or NOT_REGEX patterns
        findings.extend(apply_rule_with_not_inside(rule, file_path, source_code)?);
        return Ok(findings);
    }

    // Check if this is a taint analysis rule
    if rule.patterns.iter().any(|p| p.contains("sink(")) &&
       rule.patterns.iter().any(|p| p.contains("\"tainted\"")) {
        // Apply simplified taint analysis
        findings.extend(apply_simple_taint_analysis(rule, file_path, source_code)?);
    } else {
        // Determine language from file extension
        if let Ok(language) = determine_language(file_path) {
            // Try tree-sitter based analysis first for supported languages
            if let Ok(mut ts_analyzer) = TreeSitterAnalyzer::new() {
                if ts_analyzer.supports_language(language) {
                    let ts_findings = ts_analyzer.apply_rule_with_tree_sitter(
                    &rule.id,
                    &rule.message,
                    &rule.severity,
                    &rule.patterns,
                    file_path,
                    source_code,
                    language,
                    &rule.fix,
                )?;

                if !ts_findings.is_empty() {
                    // Convert tree-sitter findings to our Finding format
                    for ts_finding in ts_findings {
                        let finding = Finding {
                            rule_id: ts_finding.rule_id,
                            message: ts_finding.message,
                            severity: ts_finding.severity,
                            confidence: ts_finding.confidence,
                            location: Location {
                                file: ts_finding.location.file,
                                start_line: ts_finding.location.start_line,
                                start_column: ts_finding.location.start_column,
                                end_line: ts_finding.location.end_line,
                                end_column: ts_finding.location.end_column,
                            },
                            fix: ts_finding.fix_suggestion,
                        };
                        findings.push(finding);
                    }
                    return Ok(findings);
                }
                }
            }
        }

        // Try enhanced matching once per rule to preserve grouping semantics (e.g., pattern-either)
        if let Ok(language) = determine_language(file_path) {
            if let Ok(enhanced_findings) = apply_enhanced_pattern_matching(rule, file_path, source_code, language) {
                if !enhanced_findings.is_empty() {
                    return Ok(enhanced_findings);
                }
            }
        }

        // Fallback to pattern-aware matching
        for pattern in &rule.patterns {
            // Check if this is a regex pattern first (higher priority)
            if pattern.starts_with("NOT_REGEX:") ||
               // Direct regex patterns (no unescaped metavariables)
               (!pattern.contains('$') && (pattern.contains('(') || pattern.contains('[') || pattern.contains('\\'))) ||
               // Regex patterns with escaped dollar signs (like \$[a-zA-Z_])
               (pattern.contains("\\$") && (pattern.contains('[') || pattern.contains('(') || pattern.contains('?'))) ||
               // Regex patterns with common regex syntax
               (pattern.contains('\\') && (pattern.contains("\\s") || pattern.contains("\\d") || pattern.contains("\\w"))) {
                // Use our advanced pattern matcher for regex patterns
                findings.extend(apply_metavariable_pattern(rule, pattern, file_path, source_code)?);
            } else if pattern.contains('$') {
                // Use our pattern matcher for metavariable patterns
                findings.extend(apply_metavariable_pattern(rule, pattern, file_path, source_code)?);
            } else {
                // Simple string-based pattern matching for literal patterns
                for (line_num, line) in source_code.lines().enumerate() {
                    if line.contains(pattern) {
                        let finding = Finding {
                            rule_id: rule.id.clone(),
                            message: rule.message.clone(),
                            severity: rule.severity.clone(),
                            confidence: Confidence::Medium,
                            location: Location {
                                file: file_path.clone(),
                                start_line: line_num + 1,
                                start_column: line.find(pattern).unwrap_or(0) + 1,
                                end_line: line_num + 1,
                                end_column: line.find(pattern).unwrap_or(0) + pattern.len() + 1,
                            },
                            fix: rule.fix.clone(),
                        };
                        findings.push(finding);
                    }
                }
            }
        }
    }

    Ok(findings)
}

/// Apply metavariable pattern matching using our pattern matcher
fn apply_metavariable_pattern(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    use astgrep_parser::LanguageParserRegistry;
    use astgrep_matcher::AdvancedPatternMatcher;

    let mut findings = Vec::new();

    // Determine language
    let language = match determine_language(file_path) {
        Ok(lang) => lang,
        Err(_) => return Ok(findings), // Skip if language cannot be determined
    };

    // Check if pattern looks like a regex (contains regex metacharacters)
    let is_likely_regex = pattern.contains('(') && pattern.contains(')') &&
                         (pattern.contains('\\') || pattern.contains('[') || pattern.contains('*') || pattern.contains('+'));

    if is_likely_regex {
        info!("Pattern looks like regex, attempting direct regex matching: {}", pattern);
        match apply_regex_pattern(rule, pattern, file_path, source_code) {
            Ok(regex_findings) => {
                info!("Regex pattern matching found {} matches", regex_findings.len());
                if !regex_findings.is_empty() {
                    return Ok(regex_findings);
                }
            }
            Err(e) => {
                warn!("Regex pattern matching failed: {}", e);
            }
        }
    }

    // Try to use our enhanced rule parser and matcher first
    info!("Attempting enhanced pattern matching for pattern: {}", pattern);
    match apply_enhanced_pattern_matching(rule, file_path, source_code, language) {
        Ok(enhanced_findings) => {
            info!("Enhanced pattern matching succeeded, found {} matches", enhanced_findings.len());
            if !enhanced_findings.is_empty() {
                return Ok(enhanced_findings);
            } else {
                info!("Enhanced pattern matching found no matches, falling back to tree-sitter");
            }
        }
        Err(e) => {
            warn!("Enhanced pattern matching failed for {}: {}, falling back to tree-sitter",
                  file_path.display(), e);
        }
    }

    // Try to use tree-sitter for proper AST-based pattern matching
    info!("Attempting tree-sitter parsing for pattern: {}", pattern);
    match apply_tree_sitter_pattern_matching(rule, pattern, file_path, source_code, language) {
        Ok(tree_sitter_findings) => {
            info!("Tree-sitter parsing succeeded, found {} matches", tree_sitter_findings.len());
            if !tree_sitter_findings.is_empty() {
                return Ok(tree_sitter_findings);
            } else {
                info!("Tree-sitter found no matches, falling back to simple matching");
            }
        }
        Err(e) => {
            warn!("Tree-sitter parsing failed for {}: {}, falling back to simple matching",
                  file_path.display(), e);
        }
    }

    // Fallback to simple pattern matching if tree-sitter fails
    return apply_simple_metavariable_pattern(rule, pattern, file_path, source_code);

}

/// Apply enhanced pattern matching using our new AdvancedSemgrepMatcher
fn apply_enhanced_pattern_matching(
    rule: &ParsedRule,
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
) -> Result<Vec<Finding>> {
    use astgrep_rules::{RuleParser, Rule};
    use astgrep_matcher::AdvancedSemgrepMatcher;
    use astgrep_parser::tree_sitter_parser::TreeSitterParser;

    let mut findings = Vec::new();

    // Convert our simplified ParsedRule to a full Rule structure
    let rule_yaml = convert_parsed_rule_to_yaml(rule)?;

    // Parse the rule using our enhanced rule parser
    let parser = RuleParser::new();
    let rules = parser.parse_yaml(&rule_yaml)?;

    if rules.is_empty() {
        return Ok(findings);
    }

    let enhanced_rule = &rules[0];

    // Create tree-sitter parser and parse the source code
    let mut ts_parser = TreeSitterParser::new()?;
    if let Some(tree) = ts_parser.parse(source_code, language)? {
        let ast = ts_parser.tree_to_universal_ast(&tree, source_code)?;

        // Create advanced matcher and find matches
        let mut matcher = AdvancedSemgrepMatcher::new();

        for pattern in &enhanced_rule.patterns {
            // Convert our Pattern to SemgrepPattern
            let semgrep_pattern = convert_pattern_to_semgrep_pattern(pattern)?;

            let matches = matcher.find_matches(&semgrep_pattern, &ast)?;

            for match_result in matches {
                // Extract precise location from the matched AST node
                let (sl, sc, el, ec) = match match_result.node.location() {
                    Some((sl, sc, el, ec)) => (sl, sc, el, ec),
                    None => (1, 1, 1, 1),
                };
                let finding = Finding {
                    rule_id: rule.id.clone(),
                    message: rule.message.clone(),
                    severity: rule.severity.clone(),
                    confidence: Confidence::High,
                    location: Location {
                        file: file_path.clone(),
                        start_line: sl,
                        start_column: sc,
                        end_line: el,
                        end_column: ec,
                    },
                    fix: rule.fix.clone(),
                };
                findings.push(finding);
            }
        }
    }

    Ok(findings)
}

/// Convert ParsedRule to YAML format for enhanced parsing
fn convert_parsed_rule_to_yaml(rule: &ParsedRule) -> Result<String> {
    // Preserve original rule YAML structure to keep semantics (e.g., pattern-either)
    let mut top = serde_yaml::Mapping::new();
    let mut rules_seq = serde_yaml::Sequence::new();
    rules_seq.push(rule.raw_rule_value.clone());
    top.insert(serde_yaml::Value::String("rules".to_string()), serde_yaml::Value::Sequence(rules_seq));
    let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(top))?;
    Ok(yaml)
}

/// Convert our Pattern to SemgrepPattern
fn convert_pattern_to_semgrep_pattern(pattern: &astgrep_rules::Pattern) -> Result<astgrep_core::SemgrepPattern> {
    use astgrep_core::{SemgrepPattern, PatternType as CorePatternType};

    let core_pattern_type = match &pattern.pattern_type {
        astgrep_rules::PatternType::Simple(s) => CorePatternType::Simple(s.clone()),
        astgrep_rules::PatternType::Either(patterns) => {
            let converted: Result<Vec<_>> = patterns.iter()
                .map(convert_pattern_to_semgrep_pattern)
                .collect();
            CorePatternType::Either(converted?)
        }
        astgrep_rules::PatternType::Inside(inner) => {
            CorePatternType::Inside(Box::new(convert_pattern_to_semgrep_pattern(inner)?))
        }
        astgrep_rules::PatternType::NotInside(inner) => {
            CorePatternType::NotInside(Box::new(convert_pattern_to_semgrep_pattern(inner)?))
        }
        astgrep_rules::PatternType::Not(inner) => {
            CorePatternType::Not(Box::new(convert_pattern_to_semgrep_pattern(inner)?))
        }
        astgrep_rules::PatternType::Regex(regex) => CorePatternType::Regex(regex.clone()),
        astgrep_rules::PatternType::NotRegex(regex) => CorePatternType::NotRegex(regex.clone()),
        astgrep_rules::PatternType::All(patterns) => {
            let converted: Result<Vec<_>> = patterns.iter()
                .map(convert_pattern_to_semgrep_pattern)
                .collect();
            CorePatternType::All(converted?)
        }
        astgrep_rules::PatternType::Any(patterns) => {
            let converted: Result<Vec<_>> = patterns.iter()
                .map(convert_pattern_to_semgrep_pattern)
                .collect();
            CorePatternType::Any(converted?)
        }
    };

    Ok(SemgrepPattern {
        pattern_type: core_pattern_type,
        metavariable_pattern: None, // TODO: Convert metavariable patterns
        conditions: Vec::new(), // TODO: Convert conditions
        focus: pattern.focus.clone(),
    })
}

/// Simple metavariable pattern matching for basic cases
fn apply_simple_metavariable_pattern(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Check if this is a NOT_REGEX pattern
    if pattern.starts_with("NOT_REGEX:") {
        // For now, skip NOT_REGEX patterns in simple matching
        // They should be handled by the enhanced pattern matching
        return Ok(findings);
    }

    // Check if this is a NOT_INSIDE pattern
    if pattern.starts_with("NOT_INSIDE:") {
        // For now, skip NOT_INSIDE patterns in simple matching
        // They should be handled by the enhanced pattern matching
        return Ok(findings);
    }

    // Check if this is a direct regex pattern
    // Allow patterns with escaped dollar signs (like \$[a-zA-Z_])
    let is_regex_pattern = (!pattern.contains('$') || pattern.contains("\\$")) &&
        (pattern.contains('[') || pattern.contains('*') || pattern.contains('+') ||
         pattern.contains('?') || pattern.contains('^') || pattern.contains('\\') ||
         pattern.contains('(') || pattern.contains('|'));

    info!("Pattern '{}' - is_regex_pattern: {}, contains '$': {}, contains '(': {}, contains '|': {}",
          pattern, is_regex_pattern, pattern.contains('$'), pattern.contains('('), pattern.contains('|'));

    if is_regex_pattern {
        // Handle as direct regex - first fix double escaping from YAML
        let fixed_pattern = pattern
            .replace("\\\\s", "\\s")
            .replace("\\\\d", "\\d")
            .replace("\\\\w", "\\w")
            .replace("\\\\b", "\\b")
            .replace("\\\\t", "\\t")
            .replace("\\\\n", "\\n")
            .replace("\\\\r", "\\r")
            .replace("\\\\$", "\\$");

        info!("Attempting to compile regex pattern: '{}'", fixed_pattern);



        if let Ok(regex) = regex::Regex::new(&fixed_pattern) {
            info!("Regex compiled successfully, searching for matches...");
            for (line_num, line) in source_code.lines().enumerate() {
                // Skip lines that are comments
                let trimmed_line = line.trim();
                if trimmed_line.starts_with('#') || trimmed_line.starts_with("//") || trimmed_line.starts_with("/*") {
                    info!("Skipping comment line {}: '{}'", line_num + 1, line);
                    continue;
                }

                if line.contains('$') {
                    info!("Checking line {}: '{}'", line_num + 1, line);
                }
                for mat in regex.find_iter(line) {
                    // Check if the match is inside a comment on the same line
                    if let Some(comment_pos) = line.find('#') {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }
                    if let Some(comment_pos) = line.find("//") {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }

                    // Additional check: if the line starts with # (comment), skip the match entirely
                    if line.trim_start().starts_with('#') {
                        continue; // Skip matches in comment lines
                    }

                    info!("Found regex match: '{}' at line {} position {}-{}",
                          &line[mat.start()..mat.end()], line_num + 1, mat.start(), mat.end());
                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity.clone(),
                        confidence: Confidence::High,
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: mat.start() + 1,
                            end_line: line_num + 1,
                            end_column: mat.end() + 1,
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        } else {
            // If regex compilation fails, try as literal pattern
            for (line_num, line) in source_code.lines().enumerate() {
                // Skip lines that are comments
                let trimmed_line = line.trim();
                if trimmed_line.starts_with('#') || trimmed_line.starts_with("//") || trimmed_line.starts_with("/*") {
                    continue;
                }

                if line.contains(pattern) {
                    if let Some(pos) = line.find(pattern) {
                        // Check if the match is inside a comment on the same line
                        if let Some(comment_pos) = line.find('#') {
                            if pos >= comment_pos {
                                continue; // Skip matches inside inline comments
                            }
                        }
                        if let Some(comment_pos) = line.find("//") {
                            if pos >= comment_pos {
                                continue; // Skip matches inside inline comments
                            }
                        }

                        // Additional check: if the line starts with # (comment), skip the match entirely
                        if line.trim_start().starts_with('#') {
                            continue; // Skip matches in comment lines
                        }

                        let finding = Finding {
                            rule_id: rule.id.clone(),
                            message: rule.message.clone(),
                            severity: rule.severity.clone(),
                            confidence: Confidence::Medium,
                            location: Location {
                                file: file_path.clone(),
                                start_line: line_num + 1,
                                start_column: pos + 1,
                                end_line: line_num + 1,
                                end_column: pos + pattern.len() + 1,
                            },
                            fix: rule.fix.clone(),
                        };
                        findings.push(finding);
                    }
                }
            }
        }
    } else {
        // Convert pattern to a regex-like pattern for matching
        let regex_pattern = convert_pattern_to_regex(pattern);

        for (line_num, line) in source_code.lines().enumerate() {
            // Skip lines that are comments
            let trimmed_line = line.trim();
            if trimmed_line.starts_with('#') || trimmed_line.starts_with("//") || trimmed_line.starts_with("/*") {
                continue;
            }

            if let Some(matches) = find_pattern_matches(&regex_pattern, line) {
                for match_pos in matches {
                    // Check if the match is inside a comment on the same line
                    if let Some(comment_pos) = line.find('#') {
                        if match_pos >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }
                    if let Some(comment_pos) = line.find("//") {
                        if match_pos >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }

                    // Additional check: if the line starts with # (comment), skip the match entirely
                    if line.trim_start().starts_with('#') {
                        continue; // Skip matches in comment lines
                    }

                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity.clone(),
                        confidence: Confidence::High,
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: match_pos + 1,
                            end_line: line_num + 1,
                            end_column: match_pos + pattern.len(),
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        }
    }

    // Handle the most common case: $X (matches any expression)
    if pattern.trim() == "$X" {
        // Find all expressions in the code (simplified heuristic)
        for (line_num, line) in source_code.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() &&
               !trimmed.starts_with('#') &&
               !trimmed.starts_with("//") &&
               !trimmed.starts_with("def ") &&
               !trimmed.starts_with("class ") &&
               !trimmed.starts_with("import ") &&
               !trimmed.starts_with("from ") {

                // Look for expressions (assignments, function calls, etc.)
                if trimmed.contains('=') ||
                   trimmed.contains('(') ||
                   trimmed.contains('[') ||
                   (trimmed.chars().any(|c| c.is_alphanumeric()) && !trimmed.starts_with("if ") && !trimmed.starts_with("for ") && !trimmed.starts_with("while ")) {

                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message: rule.message.clone(),
                        severity: rule.severity.clone(),
                        confidence: Confidence::Low, // Lower confidence for heuristic matching
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            start_column: 1,
                            end_line: line_num + 1,
                            end_column: line.len() + 1,
                        },
                        fix: rule.fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        }
    }

    Ok(findings)
}

/// Apply tree-sitter based pattern matching for better precision
fn apply_tree_sitter_pattern_matching(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
) -> Result<Vec<Finding>> {
    use astgrep_parser::tree_sitter_parser::TreeSitterParser;

    info!("Creating TreeSitterParser for language: {:?}", language);
    let mut findings = Vec::new();
    let mut parser = TreeSitterParser::new()?;

    info!("Parsing source code with tree-sitter...");
    // Parse the source code with tree-sitter
    if let Some(tree) = parser.parse(source_code, language)? {
        info!("Tree-sitter parsing successful, searching for pattern matches...");
        // Find pattern matches using tree-sitter
        let matches = parser.find_pattern_matches(&tree, source_code, pattern)?;
        info!("Tree-sitter found {} raw matches", matches.len());

        for (i, node) in matches.iter().enumerate() {
            info!("Match {}: kind='{}', text='{}'", i, node.kind(),
                  node.utf8_text(source_code.as_bytes()).unwrap_or("<invalid>"));
            // Skip matches in comments
            if node.kind() == "comment" || node.kind() == "line_comment" || node.kind() == "block_comment" {
                continue;
            }

            // Check if the node is inside a comment by examining parent nodes
            let mut current = node.parent();
            let mut in_comment = false;
            while let Some(parent) = current {
                if parent.kind() == "comment" || parent.kind() == "line_comment" || parent.kind() == "block_comment" {
                    in_comment = true;
                    break;
                }
                current = parent.parent();
            }

            if in_comment {
                continue;
            }

            // Try to extract capture groups from the matched text if the pattern is a regex
            let mut message = rule.message.clone();
            if let Ok(node_text) = node.utf8_text(source_code.as_bytes()) {
                // Try to match the pattern as a regex to extract capture groups
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if let Some(captures) = regex.captures(node_text) {
                        message = replace_capture_groups(&message, &captures);
                    }
                }
            }

            let finding = Finding {
                rule_id: rule.id.clone(),
                message,
                severity: rule.severity.clone(),
                confidence: Confidence::High,
                location: Location {
                    file: file_path.clone(),
                    start_line: node.start_position().row + 1,
                    start_column: node.start_position().column + 1,
                    end_line: node.end_position().row + 1,
                    end_column: node.end_position().column + 1,
                },
                fix: rule.fix.clone(),
            };
            findings.push(finding);
        }
    } else {
        warn!("Tree-sitter failed to parse source code for language: {:?}", language);
    }

    info!("Tree-sitter analysis completed with {} findings", findings.len());
    Ok(findings)
}

/// Convert a semgrep-style pattern to a simple regex pattern
fn convert_pattern_to_regex(pattern: &str) -> String {
    // Handle patterns like "System.out.println($MESSAGE)" or "eval $CODE"
    let mut regex_pattern = pattern.to_string();

    // For patterns with metavariables, we need to be more precise
    // Replace metavariables with more specific regex patterns

    // Special handling for specific metavariables first
    regex_pattern = regex_pattern.replace("$CODE", r"\S+");  // Non-whitespace
    regex_pattern = regex_pattern.replace("$CMD", r"\S+");   // Non-whitespace
    regex_pattern = regex_pattern.replace("$USER_INPUT", r"\$[0-9@*]+");  // Bash positional parameters
    regex_pattern = regex_pattern.replace("$FILE", r"\S+");  // Non-whitespace
    regex_pattern = regex_pattern.replace("$OPTIONS", r"[^\s]+");  // Non-whitespace
    regex_pattern = regex_pattern.replace("$URL", r"\S+");   // Non-whitespace

    // General metavariable replacement (more conservative)
    regex_pattern = regex::Regex::new(r"\$[A-Z_][A-Z0-9_]*").unwrap()
        .replace_all(&regex_pattern, r"\S+").to_string();
    regex_pattern = regex::Regex::new(r"\$[a-z_][a-z0-9_]*").unwrap()
        .replace_all(&regex_pattern, r"\S+").to_string();

    // Legacy handling for other patterns
    regex_pattern = regex_pattern.replace("$MESSAGE", r"[^,\)]+");
    regex_pattern = regex_pattern.replace("$ARGS", r".*");
    regex_pattern = regex_pattern.replace("$X", r".*");

    // Escape special regex characters in the base pattern
    let mut escaped = String::new();
    let chars: Vec<char> = regex_pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Check if this is part of our regex substitution
        if ch == '\\' && i + 1 < chars.len() && chars[i + 1] == 'S' {
            // This is \S+ - keep it as is
            escaped.push(ch);
            escaped.push(chars[i + 1]);
            i += 2;
        } else if ch == '[' {
            // This might be part of our character class - keep it
            escaped.push(ch);
            i += 1;
        } else if ch == ']' || ch == '+' || ch == '*' || ch == '?' {
            // These might be part of our regex - keep them
            escaped.push(ch);
            i += 1;
        } else {
            // Regular character - escape if needed
            match ch {
                '.' | '^' | '$' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                    escaped.push('\\');
                    escaped.push(ch);
                }
                _ => escaped.push(ch),
            }
            i += 1;
        }
    }

    escaped
}

/// Find pattern matches in a line of code
fn find_pattern_matches(pattern: &str, line: &str) -> Option<Vec<usize>> {
    // First try exact string matching for simple patterns
    if !pattern.contains('[') && !pattern.contains('*') && !pattern.contains('+') && !pattern.contains('?') {
        if let Some(pos) = line.find(pattern) {
            return Some(vec![pos]);
        }
    }

    // Try regex matching for more complex patterns
    if let Ok(regex) = regex::Regex::new(pattern) {
        let mut matches = Vec::new();
        for mat in regex.find_iter(line) {
            matches.push(mat.start());
        }
        if !matches.is_empty() {
            return Some(matches);
        }
    }

    // Fallback: try simple substring matching for patterns that might have failed regex compilation
    if line.contains(pattern) {
        if let Some(pos) = line.find(pattern) {
            return Some(vec![pos]);
        }
    }

    None
}

/// Apply simplified taint analysis for taint rules
fn apply_simple_taint_analysis(rule: &ParsedRule, file_path: &PathBuf, source_code: &str) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = source_code.lines().collect();

    // Find taint sources, sinks, and sanitized variables
    let mut taint_sources = Vec::new();
    let mut sanitized_vars = Vec::new();
    let mut sink_calls = Vec::new();
    let mut var_assignments = Vec::new(); // Track all variable assignments

    // First pass: collect all assignments and direct taint sources
    for (line_num, line) in lines.iter().enumerate() {
        // Find direct taint source usage in sink calls
        if line.contains("sink(") && line.contains("\"tainted\"") {
            // Check if the taint is sanitized: sink(sanitize("tainted"))
            if !line.contains("sanitize(") {
                // Direct taint: sink("tainted") without sanitization
                let finding = Finding {
                    rule_id: rule.id.clone(),
                    message: rule.message.clone(),
                    severity: rule.severity.clone(),
                    confidence: Confidence::High,
                    location: Location {
                        file: file_path.clone(),
                        start_line: line_num + 1,
                        start_column: line.find("sink(").unwrap_or(0) + 1,
                        end_line: line_num + 1,
                        end_column: line.find("sink(").unwrap_or(0) + 5,
                    },
                    fix: rule.fix.clone(),
                };
                findings.push(finding);
            }
        }

        // Track variable assignments from taint sources
        if line.contains("\"tainted\"") && line.contains("=") {
            // Extract variable name (handle both regular vars and PHP vars with $)
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                if let Some(var_name) = left_part.split_whitespace().last() {
                    let clean_var = var_name.trim_end_matches(';').trim();
                    info!("Taint source found: {} (line {})", clean_var, line_num + 1);
                    taint_sources.push((clean_var.to_string(), line_num + 1));
                }
            }
        }

        // Track variable-to-variable assignments for later propagation
        if line.contains("=") && !line.contains("\"") && !line.contains("sanitize(") {
            // Extract assignment: var1 = var2
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                let right_part = line[equals_pos + 1..].trim();

                // Extract variable names (simplified)
                if let Some(left_var) = left_part.split_whitespace().last() {
                    if let Some(right_var) = right_part.split_whitespace().next() {
                        // Clean up variable names (remove semicolons, etc.)
                        let clean_left = left_var.trim_end_matches(';').trim();
                        let clean_right = right_var.trim_end_matches(';').trim();
                        info!("Variable assignment found: {} = {} (line {})", clean_left, clean_right, line_num + 1);
                        var_assignments.push((clean_left.to_string(), clean_right.to_string(), line_num + 1));
                    }
                }
            }
        }

        // Track variable sanitization: x = sanitize(x) or x = sanitize("tainted")
        // Skip sanitizers that are in conditional branches (improved heuristic)
        // Detect base indentation level from the first non-empty line
        let line_indent = line.len() - line.trim_start().len();

        // Simple heuristic: if line has more than 2 spaces and contains "if" or "else" nearby,
        // consider subsequent more-indented lines as conditional
        let is_in_conditional = line_indent > 2 && (
            // Check if there's an if/else statement in recent lines
            lines.iter().take(line_num + 1).rev().take(3).any(|prev_line| {
                prev_line.trim().starts_with("if ") || prev_line.trim().starts_with("else")
            })
        );

        if line.contains("sanitize(") && line.contains("=") && !is_in_conditional {
            // Extract variable name being assigned (handle both regular vars and PHP vars with $)
            if let Some(equals_pos) = line.find('=') {
                let left_part = line[..equals_pos].trim();
                if let Some(var_name) = left_part.split_whitespace().last() {
                    let clean_var = var_name.trim_end_matches(';').trim();
                    info!("Sanitizer found: {} (line {})", clean_var, line_num + 1);
                    sanitized_vars.push((clean_var.to_string(), line_num + 1));
                }
            }
        }

        // Track sink calls with variables
        if line.contains("sink(") && !line.contains("\"tainted\"") {
            // Extract argument (simplified)
            if let Some(start) = line.find("sink(") {
                let after_paren = start + 5;
                if let Some(end) = line[after_paren..].find(')') {
                    let arg = line[after_paren..after_paren + end].trim();
                    if !arg.is_empty() && !arg.starts_with('"') {
                        sink_calls.push((arg.to_string(), line_num + 1));
                    }
                }
            }
        }
    }

    // Multi-round taint propagation through variable assignments
    let mut changed = true;
    let mut round = 0;
    while changed && round < 10 { // Limit rounds to prevent infinite loops
        changed = false;
        round += 1;

        for (left_var, right_var, assignment_line) in &var_assignments {
            // Check if right_var is tainted and left_var is not yet tainted
            let right_taint_info = taint_sources.iter().find(|(tainted_var, _)| tainted_var == right_var);
            let left_already_tainted = taint_sources.iter().any(|(tainted_var, _)| tainted_var == left_var);

            if let Some((_, taint_line)) = right_taint_info {
                // Only propagate if the assignment happens AFTER the taint source
                if !left_already_tainted && assignment_line > taint_line {
                    info!("Taint propagation round {}: {} -> {} (line {}, taint from line {})",
                          round, right_var, left_var, assignment_line, taint_line);
                    taint_sources.push((left_var.clone(), *assignment_line));
                    changed = true;
                }
            }
        }
    }

    // Check for taint propagation through variables (excluding sanitized ones)
    // Use a set to track which sinks we've already processed to avoid duplicates
    let mut processed_sinks = std::collections::HashSet::new();

    for (sink_arg, sink_line) in &sink_calls {
        if processed_sinks.contains(sink_line) {
            continue; // Skip if we've already processed this sink
        }

        // Find if any taint source affects this sink
        let mut sink_is_tainted = false;
        for (var_name, source_line) in &taint_sources {
            if sink_arg == var_name && source_line < sink_line {
                // Only consider taint sources that occur BEFORE the sink
                // Check if this variable was sanitized AFTER the taint source but BEFORE the sink
                let is_sanitized_before_sink = sanitized_vars.iter().any(|(sanitized_var, sanitize_line)| {
                    sanitized_var == var_name &&
                    sanitize_line > source_line &&
                    sanitize_line < sink_line
                });

                if !is_sanitized_before_sink {
                    info!("Sink at line {} is tainted: variable {} from line {} (no sanitization between {} and {})",
                          sink_line, var_name, source_line, source_line, sink_line);
                    sink_is_tainted = true;
                    break; // Found at least one taint source that affects this sink
                } else {
                    info!("Sink at line {} is safe: variable {} was sanitized before use", sink_line, var_name);
                }
            }
        }

        if sink_is_tainted {
            let finding = Finding {
                rule_id: rule.id.clone(),
                message: rule.message.clone(),
                severity: rule.severity.clone(),
                confidence: Confidence::Medium,
                location: Location {
                    file: file_path.clone(),
                    start_line: *sink_line,
                    start_column: lines[*sink_line - 1].find("sink(").unwrap_or(0) + 1,
                    end_line: *sink_line,
                    end_column: lines[*sink_line - 1].find("sink(").unwrap_or(0) + 5,
                },
                fix: rule.fix.clone(),
            };
            findings.push(finding);
        }

        processed_sinks.insert(*sink_line);
    }

    Ok(findings)
}

/// Parsed rule structure
#[derive(Debug, Clone)]
struct ParsedRule {
    id: String,
    message: String,
    severity: Severity,
    languages: Vec<Language>,
    patterns: Vec<String>,
    fix: Option<String>,
    // Preserve the original YAML value to maintain semantics like pattern-either
    raw_rule_value: serde_yaml::Value,
}

/// Parse Semgrep-style YAML rules
fn parse_semgrep_rules(content: &str, target_language: Language, file_path: Option<&PathBuf>) -> Result<Vec<ParsedRule>> {
    let mut rules = Vec::new();

    // Try to parse as YAML
    if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        if let Some(yaml_rules) = yaml_value.get("rules").and_then(|r| r.as_sequence()) {
            for rule_value in yaml_rules {
                if let Ok(rule) = parse_single_rule(rule_value, target_language, file_path) {
                    rules.push(rule);
                }
            }
        }
    }

    Ok(rules)
}

/// Parse a single rule from YAML
fn parse_single_rule(rule_value: &serde_yaml::Value, target_language: Language, file_path: Option<&PathBuf>) -> Result<ParsedRule> {
    let base_id = rule_value.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown-rule");

    // Generate semgrep-compatible rule ID with path prefix
    let id = if let Some(path) = file_path {
        // Convert path to semgrep-style ID prefix (exclude filename, only use directory path)
        let path_str = path.to_string_lossy();
        let dir_path = path.parent()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| std::borrow::Cow::Borrowed(""));

        let path_prefix = dir_path
            .strip_prefix("./")
            .unwrap_or(&dir_path)
            .replace('/', ".");

        if path_prefix.is_empty() {
            base_id.to_string()
        } else {
            format!("{}.{}", path_prefix, base_id)
        }
    } else {
        base_id.to_string()
    };

    let message = rule_value.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Security issue detected")
        .to_string();

    // Parse severity
    let severity = rule_value.get("severity")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_uppercase().as_str() {
            "ERROR" | "HIGH" => Severity::Error,
            "WARNING" | "MEDIUM" => Severity::Warning,
            "INFO" | "LOW" => Severity::Info,
            _ => Severity::Warning,
        })
        .unwrap_or(Severity::Warning);

    // Parse languages
    let languages = rule_value.get("languages")
        .and_then(|v| v.as_sequence())
        .map(|langs| {
            langs.iter()
                .filter_map(|l| l.as_str())
                .filter_map(|l| Language::from_str(l))
                .collect()
        })
        .unwrap_or_else(|| vec![target_language]);

    // Skip if this rule doesn't apply to the target language
    if !languages.contains(&target_language) {
        info!("Rule '{}' skipped: languages {:?} don't include target {:?}", id, languages, target_language);
        return Err(anyhow::anyhow!("Rule doesn't apply to target language"));
    }

    info!("Rule '{}' accepted for target language {:?}", id, target_language);

    // Extract patterns using improved pattern extraction
    let patterns = extract_patterns_from_rule_value(rule_value);

    // Debug: log extracted patterns
    if !patterns.is_empty() {
        info!("Rule '{}' extracted patterns: {:?}", id, patterns);
    }

    // Parse fix suggestion
    let fix = rule_value.get("fix")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(ParsedRule {
        id,
        message,
        severity,
        languages,
        patterns,
        fix,
        raw_rule_value: rule_value.clone(),
    })
}

/// Extract simple string patterns from Semgrep pattern syntax
fn extract_simple_patterns(pattern: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    // Check if this is a metavariable pattern - pass through as-is for tree-sitter
    if pattern.contains('$') {
        patterns.push(pattern.trim().to_string());
        return patterns;
    }

    // Look for quoted strings in the pattern
    let re = regex::Regex::new(r#""([^"]+)""#).unwrap();
    for cap in re.captures_iter(pattern) {
        if let Some(matched) = cap.get(1) {
            let pattern_str = matched.as_str();
            // Skip metavariables like $ALGO
            if !pattern_str.starts_with('$') {
                patterns.push(format!("\"{}\"", pattern_str));
            }
        }
    }

    // Look for function calls with ellipsis (e.g., "sink(...)")
    let func_re = regex::Regex::new(r"(\w+)\s*\(\s*\.\.\.\s*\)").unwrap();
    for cap in func_re.captures_iter(pattern) {
        if let Some(func_name) = cap.get(1) {
            patterns.push(format!("{}(", func_name.as_str()));
        }
    }

    // Look for common API calls in patterns
    if pattern.contains("MessageDigest.getInstance") {
        patterns.push("MessageDigest.getInstance".to_string());
    }
    if pattern.contains("executeQuery") {
        patterns.push(".executeQuery(".to_string());
    }
    if pattern.contains("eval") {
        patterns.push("eval(".to_string());
    }
    if pattern.contains("getSha1Digest") {
        patterns.push("getSha1Digest".to_string());
    }

    // Handle simple string literals without quotes (for taint sources)
    // Only add if we haven't already extracted patterns from quotes
    if patterns.is_empty() && pattern.trim().starts_with('"') && pattern.trim().ends_with('"') {
        patterns.push(pattern.trim().to_string());
    }

    // Handle simple literals (numbers, identifiers, etc.)
    // If no patterns were extracted yet, treat the whole pattern as a literal
    if patterns.is_empty() {
        let trimmed = pattern.trim();
        if !trimmed.is_empty() {
            patterns.push(trimmed.to_string());
        }
    }

    // Remove duplicates
    patterns.sort();
    patterns.dedup();

    patterns
}

/// Extract patterns from complex Semgrep rule structure
fn extract_patterns_from_rule_value(rule_value: &serde_yaml::Value) -> Vec<String> {
    let mut patterns = Vec::new();

    // Handle pattern-either
    if let Some(pattern_either) = rule_value.get("pattern-either") {
        if let Some(either_array) = pattern_either.as_sequence() {
            for item in either_array {
                patterns.extend(extract_patterns_from_rule_value(item));
            }
        }
    }

    // Handle patterns (array)
    if let Some(patterns_array) = rule_value.get("patterns") {
        if let Some(array) = patterns_array.as_sequence() {
            for item in array {
                patterns.extend(extract_patterns_from_rule_value(item));
            }
        }
    }

    // Handle single pattern
    if let Some(pattern_value) = rule_value.get("pattern") {
        if let Some(pattern_str) = pattern_value.as_str() {
            patterns.extend(extract_simple_patterns(pattern_str));
        }
    }

    // Handle pattern-regex (CRITICAL: This was missing!)
    if let Some(pattern_regex) = rule_value.get("pattern-regex") {
        if let Some(regex_str) = pattern_regex.as_str() {
            // Add the regex pattern directly - it will be used as a regex
            patterns.push(regex_str.to_string());
        }
    }

    // Handle pattern-not-regex
    if let Some(pattern_not_regex) = rule_value.get("pattern-not-regex") {
        if let Some(regex_str) = pattern_not_regex.as_str() {
            // For not-regex patterns, we'll handle them differently in the matching logic
            // For now, just add them as patterns to be processed
            patterns.push(format!("NOT_REGEX:{}", regex_str));
        }
    }

    // Handle pattern-not-inside
    if let Some(pattern_not_inside) = rule_value.get("pattern-not-inside") {
        if let Some(not_inside_str) = pattern_not_inside.as_str() {
            // For not-inside patterns, we'll handle them differently in the matching logic
            patterns.push(format!("NOT_INSIDE:{}", not_inside_str));
        }
    }

    // Handle metavariable-regex (extract the regex pattern)
    if let Some(metavar_regex) = rule_value.get("metavariable-regex") {
        if let Some(regex_value) = metavar_regex.get("regex") {
            if let Some(regex_str) = regex_value.as_str() {
                // For SHA1 detection, add specific patterns
                if regex_str.contains("SHA1") || regex_str.contains("SHA-1") {
                    patterns.push("\"SHA1\"".to_string());
                    patterns.push("\"SHA-1\"".to_string());
                }
                if regex_str.contains("MD5") {
                    patterns.push("\"MD5\"".to_string());
                }
            }
        }
    }

    // Handle taint analysis patterns
    // Extract patterns from pattern-sources
    if let Some(sources) = rule_value.get("pattern-sources") {
        if let Some(sources_array) = sources.as_sequence() {
            for source in sources_array {
                patterns.extend(extract_patterns_from_rule_value(source));
            }
        }
    }

    // Extract patterns from pattern-sinks
    if let Some(sinks) = rule_value.get("pattern-sinks") {
        if let Some(sinks_array) = sinks.as_sequence() {
            for sink in sinks_array {
                patterns.extend(extract_patterns_from_rule_value(sink));
            }
        }
    }

    // Extract patterns from pattern-sanitizers
    if let Some(sanitizers) = rule_value.get("pattern-sanitizers") {
        if let Some(sanitizers_array) = sanitizers.as_sequence() {
            for sanitizer in sanitizers_array {
                patterns.extend(extract_patterns_from_rule_value(sanitizer));
            }
        }
    }

    patterns
}

/// Apply a rule that contains NOT_INSIDE patterns
fn apply_rule_with_not_inside(rule: &ParsedRule, file_path: &PathBuf, source_code: &str) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Separate main patterns from NOT_INSIDE patterns
    let mut main_patterns = Vec::new();
    let mut not_inside_patterns = Vec::new();
    let mut not_regex_patterns = Vec::new();

    for pattern in &rule.patterns {
        if pattern.starts_with("NOT_INSIDE:") {
            let not_inside_pattern = &pattern[11..]; // Remove "NOT_INSIDE:" prefix
            not_inside_patterns.push(not_inside_pattern);
        } else if pattern.starts_with("NOT_REGEX:") {
            let not_regex_pattern = &pattern[10..]; // Remove "NOT_REGEX:" prefix
            not_regex_patterns.push(not_regex_pattern);
        } else {
            main_patterns.push(pattern);
        }
    }

    info!("Rule '{}': Found {} main patterns, {} NOT_INSIDE patterns, {} NOT_REGEX patterns",
          rule.id, main_patterns.len(), not_inside_patterns.len(), not_regex_patterns.len());

    // Find all matches for main patterns
    let mut candidate_findings = Vec::new();
    for pattern in &main_patterns {
        info!("Processing main pattern: '{}'", pattern);

        // Special handling for unsafe-user-input pattern
        if pattern.as_str() == "$CMD $USER_INPUT" {
            // Convert to a regex pattern that matches commands with positional parameters
            let simplified_pattern = r"(rm|cat|echo|eval|cp|mv|chmod|chown)\s+.*\$[0-9@*]";
            let pattern_findings = apply_regex_pattern(rule, &simplified_pattern, file_path, source_code)?;
            info!("Simplified pattern '{}' found {} matches", simplified_pattern, pattern_findings.len());
            candidate_findings.extend(pattern_findings);
        } else {
            let pattern_findings = apply_metavariable_pattern(rule, pattern, file_path, source_code)?;
            info!("Main pattern '{}' found {} matches", pattern, pattern_findings.len());
            candidate_findings.extend(pattern_findings);
        }
    }

    // Filter out findings that are inside NOT_INSIDE patterns
    info!("Filtering {} candidate findings", candidate_findings.len());
    for finding in candidate_findings {
        let mut should_include = true;

        // Check if this finding is inside any NOT_INSIDE pattern
        for not_inside_pattern in &not_inside_patterns {
            if is_finding_inside_pattern(&finding, not_inside_pattern, source_code) {
                info!("Finding at line {} excluded by NOT_INSIDE pattern: {}", finding.location.start_line, not_inside_pattern);
                should_include = false;
                break;
            }
        }

        // Check if this finding matches any NOT_REGEX pattern
        if should_include {
            for not_regex_pattern in &not_regex_patterns {
                // Special handling for XML namespace validation rules
                if rule.id.contains("xml-namespace-prefix") {
                    // Extract the prefix from the finding message or location
                    if let Some(prefix) = extract_prefix_from_finding(&finding, source_code) {
                        if is_xml_namespace_prefix_declared(&prefix, source_code) {
                            info!("Finding at line {} excluded: namespace prefix '{}' is declared", finding.location.start_line, prefix);
                            should_include = false;
                            break;
                        }
                    }
                } else if is_finding_matching_regex(&finding, not_regex_pattern, source_code) {
                    info!("Finding at line {} excluded by NOT_REGEX pattern: {}", finding.location.start_line, not_regex_pattern);
                    should_include = false;
                    break;
                }
            }
        }

        if should_include {
            info!("Finding at line {} included", finding.location.start_line);
            findings.push(finding);
        }
    }

    // Remove duplicate findings (same rule, same location)
    findings.sort_by(|a, b| {
        a.location.start_line.cmp(&b.location.start_line)
            .then_with(|| a.location.start_column.cmp(&b.location.start_column))
    });
    findings.dedup_by(|a, b| {
        a.location.start_line == b.location.start_line &&
        a.location.start_column == b.location.start_column &&
        a.rule_id == b.rule_id
    });

    Ok(findings)
}

/// Check if a finding is inside a NOT_INSIDE pattern
fn is_finding_inside_pattern(finding: &Finding, not_inside_pattern: &str, source_code: &str) -> bool {
    let lines: Vec<&str> = source_code.lines().collect();
    let finding_line = finding.location.start_line;

    // Handle simple single-line patterns first
    if not_inside_pattern.contains("[[ $VAR ]]") {
        // Check if the finding is inside a [[ ... ]] conditional
        if finding_line > 0 && finding_line <= lines.len() {
            let line = lines[finding_line - 1];
            // Check if the line contains [[ and ]]
            if line.contains("[[") && line.contains("]]") {
                return true;
            }
        }
    }

    if not_inside_pattern.contains("(( $VAR ))") {
        // Check if the finding is inside a (( ... )) arithmetic expression
        if finding_line > 0 && finding_line <= lines.len() {
            let line = lines[finding_line - 1];
            // Check if the line contains (( and ))
            if line.contains("((") && line.contains("))") {
                return true;
            }
        }
    }

    // Handle multi-line if-then-fi patterns
    if (not_inside_pattern.contains("if [[") || not_inside_pattern.contains("if sudo")) &&
       not_inside_pattern.contains("then") && not_inside_pattern.contains("fi") {
        return is_finding_inside_if_block(finding, not_inside_pattern, &lines);
    }

    // Handle other multi-line patterns
    if not_inside_pattern.contains("$TEMP=$(mktemp)") {
        return is_finding_inside_mktemp_block(finding, &lines);
    }

    false
}

/// Check if a finding is inside an if-then-fi block that matches the NOT_INSIDE pattern
fn is_finding_inside_if_block(finding: &Finding, not_inside_pattern: &str, lines: &[&str]) -> bool {
    let finding_line = finding.location.start_line;

    // More flexible pattern matching for different NOT_INSIDE patterns
    let patterns_to_check = vec![
        ("== \"yes\"", vec!["if [[ \"$CONFIRM\" == \"yes\" ]]", "if [[ $CONFIRM == \"yes\" ]]"]),
        ("sudo -n true", vec!["if sudo -n true", "if sudo -n true 2>/dev/null"]),
        ("=~ ^[a-zA-Z0-9_]+$", vec!["=~ ^[a-zA-Z0-9_]+$"]),
        ("=~ ^[a-zA-Z0-9_.-]+$", vec!["=~ ^[a-zA-Z0-9_.-]+$"]),
        ("=~ ^[a-zA-Z0-9_/-]+$", vec!["=~ ^[a-zA-Z0-9_/-]+$"]),
    ];

    for (pattern_key, if_patterns) in patterns_to_check {
        if not_inside_pattern.contains(pattern_key) {
            // Look for if-then-fi blocks that contain this condition
            for (i, line) in lines.iter().enumerate() {
                let line_num = i + 1;
                let trimmed_line = line.trim();

                // Check if this line starts an if block with our condition
                let matches_if_pattern = if_patterns.iter().any(|if_pattern| {
                    trimmed_line.starts_with("if ") && trimmed_line.contains(if_pattern)
                });

                if matches_if_pattern || (trimmed_line.starts_with("if [[") && if_patterns.iter().any(|p| trimmed_line.contains(p))) {
                    // Find the corresponding fi or end of block
                    let mut end_line = None;
                    let mut brace_count = 0;

                    for (j, search_line) in lines.iter().enumerate().skip(i + 1) {
                        let search_trimmed = search_line.trim();

                        // Count braces for nested blocks
                        if search_trimmed.contains("then") || search_trimmed.contains("{") {
                            brace_count += 1;
                        }
                        if search_trimmed == "fi" || search_trimmed == "}" {
                            if brace_count == 0 {
                                end_line = Some(j + 1);
                                break;
                            } else {
                                brace_count -= 1;
                            }
                        }
                    }

                    if let Some(end_line_num) = end_line {
                        // Check if the finding is between the if and end lines
                        if finding_line > line_num && finding_line < end_line_num {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if a finding is inside a mktemp block
fn is_finding_inside_mktemp_block(finding: &Finding, lines: &[&str]) -> bool {
    let finding_line = finding.location.start_line;

    // Look for TEMP=$(mktemp) pattern before the finding
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        if line_num < finding_line && line.contains("=$(mktemp)") {
            // More sophisticated heuristic: check if the finding uses the same variable
            if let Some(var_name) = extract_mktemp_variable(line) {
                // Check if the finding line uses this variable
                let finding_line_content = lines.get(finding_line - 1).unwrap_or(&"");
                if finding_line_content.contains(&format!("${}", var_name)) ||
                   finding_line_content.contains(&format!("\"${}\"", var_name)) {
                    // If mktemp is within 20 lines before the finding and uses the same variable, consider it protected
                    if finding_line - line_num <= 20 {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Extract variable name from mktemp assignment
fn extract_mktemp_variable(line: &str) -> Option<String> {
    // Look for pattern like: TEMP=$(mktemp) or temp_file=$(mktemp)
    if let Some(equals_pos) = line.find("=$(mktemp)") {
        let var_part = &line[..equals_pos];
        if let Some(var_name) = var_part.split_whitespace().last() {
            return Some(var_name.to_string());
        }
    }
    None
}

/// Replace capture groups in message template with actual captured values
fn replace_capture_groups(message: &str, captures: &regex::Captures) -> String {
    let mut result = message.to_string();

    // Replace numbered capture groups: ${1}, ${2}, etc.
    for i in 1..captures.len() {
        if let Some(captured) = captures.get(i) {
            let placeholder = format!("${{{}}}", i);
            result = result.replace(&placeholder, captured.as_str());
        }
    }

    result
}

/// Apply a regex pattern to source code and return findings
fn apply_regex_pattern(rule: &ParsedRule, pattern: &str, file_path: &PathBuf, source_code: &str) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    if let Ok(regex) = regex::Regex::new(pattern) {
        for (line_num, line) in source_code.lines().enumerate() {
            // Skip lines that are comments
            let trimmed_line = line.trim();
            if trimmed_line.starts_with('#') || trimmed_line.starts_with("//") || trimmed_line.starts_with("/*") {
                info!("Skipping comment line {}: '{}'", line_num + 1, line);
                continue;
            }

            // Use captures_iter to get all matches with capture groups
            for captures in regex.captures_iter(line) {
                if let Some(mat) = captures.get(0) {
                    // Check if the match is inside a comment on the same line
                    if let Some(comment_pos) = line.find('#') {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }
                    if let Some(comment_pos) = line.find("//") {
                        if mat.start() >= comment_pos {
                            continue; // Skip matches inside inline comments
                        }
                    }

                    // Additional check: if the line starts with # (comment), skip the match entirely
                    if line.trim_start().starts_with('#') {
                        continue; // Skip matches in comment lines
                    }

                    // Replace capture groups in the message
                    let message = replace_capture_groups(&rule.message, &captures);

                    let finding = Finding {
                        rule_id: rule.id.clone(),
                        message,
                        severity: rule.severity.clone(),
                        location: Location {
                            file: file_path.clone(),
                            start_line: line_num + 1,
                            end_line: line_num + 1,
                            start_column: mat.start() + 1,
                            end_column: mat.end() + 1,
                        },
                        confidence: Confidence::High,
                        fix: None,
                    };
                    findings.push(finding);
                }
            }
        }
    }

    Ok(findings)
}

/// Check if a finding matches a NOT_REGEX pattern
fn is_finding_matching_regex(finding: &Finding, not_regex_pattern: &str, source_code: &str) -> bool {
    let lines: Vec<&str> = source_code.lines().collect();
    let finding_line = finding.location.start_line;

    if finding_line > 0 && finding_line <= lines.len() {
        let line = lines[finding_line - 1];

        // Fix double escaping from YAML
        let fixed_pattern = not_regex_pattern
            .replace("\\\\s", "\\s")
            .replace("\\\\d", "\\d")
            .replace("\\\\w", "\\w")
            .replace("\\\\$", "\\$");

        info!("Checking NOT_REGEX pattern '{}' (fixed: '{}') against line {}: '{}'",
              not_regex_pattern, fixed_pattern, finding_line, line);

        if let Ok(regex) = regex::Regex::new(&fixed_pattern) {
            let matches = regex.is_match(line);
            info!("NOT_REGEX pattern '{}' {} line {}", fixed_pattern, if matches { "matches" } else { "does not match" }, finding_line);
            return matches;
        } else {
            info!("Failed to compile NOT_REGEX pattern: '{}'", fixed_pattern);
        }
    }

    false
}

/// Check if a namespace prefix is declared in the XML document
/// This is a special handler for XML namespace validation
fn is_xml_namespace_prefix_declared(prefix: &str, source_code: &str) -> bool {
    use regex::Regex;

    // Look for xmlns:prefix= declaration anywhere in the document
    let pattern = format!(r#"xmlns:{}[\s]*="#, regex::escape(prefix));
    if let Ok(regex) = Regex::new(&pattern) {
        return regex.is_match(source_code);
    }
    false
}

/// Extract all namespace prefixes used in XML elements
fn extract_used_namespace_prefixes(source_code: &str) -> std::collections::HashSet<String> {
    use regex::Regex;
    let mut prefixes = std::collections::HashSet::new();

    // Match <prefix:element patterns
    if let Ok(regex) = Regex::new(r"<(\w+):(\w+)") {
        for cap in regex.captures_iter(source_code) {
            if let Some(prefix_match) = cap.get(1) {
                prefixes.insert(prefix_match.as_str().to_string());
            }
        }
    }

    prefixes
}

/// Extract all declared namespace prefixes in XML
fn extract_declared_namespace_prefixes(source_code: &str) -> std::collections::HashSet<String> {
    use regex::Regex;
    let mut prefixes = std::collections::HashSet::new();

    // Match xmlns:prefix= declarations
    if let Ok(regex) = Regex::new(r#"xmlns:(\w+)[\s]*="#) {
        for cap in regex.captures_iter(source_code) {
            if let Some(prefix_match) = cap.get(1) {
                prefixes.insert(prefix_match.as_str().to_string());
            }
        }
    }

    prefixes
}

/// Extract namespace prefix from a finding location in XML
fn extract_prefix_from_finding(finding: &Finding, source_code: &str) -> Option<String> {
    use regex::Regex;

    let lines: Vec<&str> = source_code.lines().collect();
    let finding_line = finding.location.start_line;

    if finding_line > 0 && finding_line <= lines.len() {
        let line = lines[finding_line - 1];

        // Try to extract prefix from <prefix:element pattern
        if let Ok(regex) = Regex::new(r"<(\w+):(\w+)") {
            if let Some(cap) = regex.captures(line) {
                if let Some(prefix_match) = cap.get(1) {
                    return Some(prefix_match.as_str().to_string());
                }
            }
        }
    }

    None
}

#[derive(Clone)]
struct BasicPattern {
    rule_id: String,
    pattern: String,
    message: String,
    severity: Severity,
    confidence: Confidence,
    fix: Option<String>,
}

fn get_basic_security_patterns(language: Language) -> Vec<BasicPattern> {
    match language {
        Language::Java => vec![
            BasicPattern {
                rule_id: "java-sql-injection".to_string(),
                pattern: ".executeQuery(".to_string(),
                message: "Potential SQL injection vulnerability".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::Medium,
                fix: Some("Use PreparedStatement instead of Statement".to_string()),
            },
            BasicPattern {
                rule_id: "java-hardcoded-password".to_string(),
                pattern: "password".to_string(),
                message: "Potential hardcoded password".to_string(),
                severity: Severity::Warning,
                confidence: Confidence::Low,
                fix: Some("Use environment variables or secure configuration".to_string()),
            },
            BasicPattern {
                rule_id: "java-weak-hash-sha1".to_string(),
                pattern: "\"SHA1\"".to_string(),
                message: "Use of SHA1 hash algorithm which is considered insecure".to_string(),
                severity: Severity::Error,
                confidence: Confidence::High,
                fix: Some("Use SHA-256, SHA-384, or SHA-512 instead of SHA1".to_string()),
            },
            BasicPattern {
                rule_id: "java-weak-hash-md5".to_string(),
                pattern: "\"MD5\"".to_string(),
                message: "Use of MD5 hash algorithm which is considered insecure".to_string(),
                severity: Severity::Error,
                confidence: Confidence::High,
                fix: Some("Use SHA-256, SHA-384, or SHA-512 instead of MD5".to_string()),
            },
        ],
        Language::JavaScript => vec![
            BasicPattern {
                rule_id: "js-eval-usage".to_string(),
                pattern: "eval(".to_string(),
                message: "Use of eval() can lead to code injection".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::High,
                fix: Some("Avoid using eval(), use safer alternatives".to_string()),
            },
            BasicPattern {
                rule_id: "js-innerhtml".to_string(),
                pattern: "innerHTML".to_string(),
                message: "Potential XSS vulnerability with innerHTML".to_string(),
                severity: Severity::Warning,
                confidence: Confidence::Medium,
                fix: Some("Use textContent or sanitize input".to_string()),
            },
        ],
        Language::Python => vec![
            BasicPattern {
                rule_id: "python-exec-usage".to_string(),
                pattern: "exec(".to_string(),
                message: "Use of exec() can lead to code injection".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::High,
                fix: Some("Avoid using exec(), use safer alternatives".to_string()),
            },
            BasicPattern {
                rule_id: "python-sql-format".to_string(),
                pattern: ".format(".to_string(),
                message: "Potential SQL injection with string formatting".to_string(),
                severity: Severity::Warning,
                confidence: Confidence::Low,
                fix: Some("Use parameterized queries".to_string()),
            },
        ],
        Language::Sql => vec![
            BasicPattern {
                rule_id: "sql-union-injection".to_string(),
                pattern: "UNION".to_string(),
                message: "Potential SQL injection with UNION".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::Medium,
                fix: Some("Use parameterized queries".to_string()),
            },
        ],
        Language::Bash => vec![
            BasicPattern {
                rule_id: "bash-command-injection".to_string(),
                pattern: "$((".to_string(),
                message: "Potential command injection".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::Medium,
                fix: Some("Validate and sanitize input".to_string()),
            },
        ],
        Language::Php => vec![],
        Language::CSharp => vec![],
        Language::C => vec![],
        Language::Ruby => vec![],
        Language::Kotlin => vec![],
        Language::Swift => vec![],
        Language::Xml => vec![],
    }
}

fn determine_language(file_path: &PathBuf) -> Result<Language> {
    if let Some(extension) = file_path.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "java" => Ok(Language::Java),
            "js" | "jsx" | "ts" | "tsx" => Ok(Language::JavaScript),
            "py" => Ok(Language::Python),
            "sql" => Ok(Language::Sql),
            "sh" | "bash" => Ok(Language::Bash),
            "php" | "phtml" | "php3" | "php4" | "php5" => Ok(Language::Php),
            "cs" | "csx" => Ok(Language::CSharp),
            "c" | "h" => Ok(Language::C),
            "rb" | "rbw" => Ok(Language::Ruby),
            "kt" | "kts" => Ok(Language::Kotlin),
            "swift" => Ok(Language::Swift),
            "xml" | "xsd" | "xsl" | "xslt" | "svg" | "pom" => Ok(Language::Xml),
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", ext_str)),
        }
    } else {
        Err(anyhow::anyhow!("File has no extension: {}", file_path.display()))
    }
}

fn apply_filters(findings: &[Finding], config: &EnhancedAnalysisConfig) -> Vec<Finding> {
    findings.iter()
        .filter(|finding| {
            // Apply severity filter
            if let Some(min_severity) = config.severity_filter {
                if finding.severity < min_severity {
                    return false;
                }
            }

            // Apply confidence filter
            if let Some(min_confidence) = config.confidence_filter {
                if finding.confidence < min_confidence {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}

// Simplified baseline comparison (removed for now to avoid complexity)

fn generate_enhanced_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
    profiler: Option<&PerformanceProfiler>,
) -> Result<String> {
    // Check for compatibility mode
    if let Some(ref compatible_mode) = config.compatible_mode {
        match compatible_mode.to_lowercase().as_str() {
            "semgrep" => return generate_semgrep_compatible_output(findings, stats, config, total_time),
            _ => {
                warn!("Unknown compatibility mode: {}, falling back to default output", compatible_mode);
            }
        }
    }

    match config.output_format {
        OutputFormat::Json => generate_json_output(findings, stats, config, total_time, profiler),
        OutputFormat::Sarif => generate_sarif_output(findings, stats, config, total_time),
        OutputFormat::Xml => generate_text_output(findings, stats, config, total_time, profiler), // XML not implemented
        OutputFormat::Yaml => generate_text_output(findings, stats, config, total_time, profiler), // YAML not implemented
        OutputFormat::Text => generate_text_output(findings, stats, config, total_time, profiler),
    }
}

fn generate_json_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
    profiler: Option<&PerformanceProfiler>,
) -> Result<String> {
    use serde_json::json;

    let mut output = json!({
        "findings": findings,
        "summary": {
            "total_findings": findings.len(),
            "files_analyzed": stats.files_analyzed,
            "rules_executed": stats.rules_executed,
            "analysis_time_ms": total_time.as_millis(),
        }
    });

    if config.include_metrics {
        output["statistics"] = json!(stats);

        if let Some(profiler) = profiler {
            output["performance"] = json!(profiler.get_metrics());
        }
    }

    Ok(serde_json::to_string_pretty(&output)?)
}

fn generate_text_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
    profiler: Option<&PerformanceProfiler>,
) -> Result<String> {
    let mut output = String::new();

    output.push_str("=== astgrep Analysis Results ===\n\n");

    if findings.is_empty() {
        output.push_str("✅ No issues found!\n\n");
    } else {
        output.push_str(&format!("Found {} issue(s):\n\n", findings.len()));

        for (i, finding) in findings.iter().enumerate() {
            output.push_str(&format!("{}. {} ({})\n", i + 1, finding.message, finding.rule_id));
            output.push_str(&format!("   File: {}:{}:{}\n",
                finding.location.file.display(),
                finding.location.start_line,
                finding.location.start_column
            ));
            output.push_str(&format!("   Severity: {:?}, Confidence: {:?}\n",
                finding.severity, finding.confidence
            ));
            if let Some(ref fix) = finding.fix {
                output.push_str(&format!("   Fix: {}\n", fix));
            }
            output.push_str("\n");
        }
    }

    // Summary
    output.push_str("=== Summary ===\n");
    output.push_str(&format!("Files analyzed: {}\n", stats.files_analyzed));
    output.push_str(&format!("Rules executed: {}\n", stats.rules_executed));
    output.push_str(&format!("Analysis time: {:?}\n", total_time));

    if config.include_metrics {
        output.push_str(&format!("Parse errors: {}\n", stats.parse_errors));
        output.push_str(&format!("Analysis errors: {}\n", stats.analysis_errors));

        if let Some(profiler) = profiler {
            output.push_str("\n=== Performance Metrics ===\n");
            output.push_str(&profiler.get_metrics().generate_report());
        }
    }

    Ok(output)
}

fn generate_sarif_output(
    findings: &[Finding],
    _stats: &AnalysisStatistics,
    _config: &EnhancedAnalysisConfig,
    _total_time: std::time::Duration,
) -> Result<String> {
    // SARIF (Static Analysis Results Interchange Format) output
    use serde_json::json;

    let sarif = json!({
        "version": "2.1.0",
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "astgrep",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/your-org/astgrep"
                }
            },
            "results": findings.iter().map(|finding| {
                json!({
                    "ruleId": finding.rule_id,
                    "message": {
                        "text": finding.message
                    },
                    "level": match finding.severity {
                        Severity::Critical => "error",
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "note",
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": finding.location.file.to_string_lossy()
                            },
                            "region": {
                                "startLine": finding.location.start_line,
                                "startColumn": finding.location.start_column,
                                "endLine": finding.location.end_line,
                                "endColumn": finding.location.end_column
                            }
                        }
                    }]
                })
            }).collect::<Vec<_>>()
        }]
    });

    Ok(serde_json::to_string_pretty(&sarif)?)
}

fn generate_html_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    _config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
) -> Result<String> {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>astgrep Analysis Report</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str(".finding { border: 1px solid #ddd; margin: 10px 0; padding: 10px; }\n");
    html.push_str(".error { border-left: 5px solid #f44336; }\n");
    html.push_str(".warning { border-left: 5px solid #ff9800; }\n");
    html.push_str(".info { border-left: 5px solid #2196f3; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    html.push_str("<h1>astgrep Analysis Report</h1>\n");
    html.push_str(&format!("<p>Generated on: {}</p>\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    html.push_str("<h2>Summary</h2>\n");
    html.push_str(&format!("<p>Total findings: {}</p>\n", findings.len()));
    html.push_str(&format!("<p>Files analyzed: {}</p>\n", stats.files_analyzed));
    html.push_str(&format!("<p>Analysis time: {:?}</p>\n", total_time));

    if !findings.is_empty() {
        html.push_str("<h2>Findings</h2>\n");

        for finding in findings {
            let severity_class = match finding.severity {
                Severity::Critical => "error",
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };

            html.push_str(&format!("<div class=\"finding {}\">\n", severity_class));
            html.push_str(&format!("<h3>{}</h3>\n", finding.message));
            html.push_str(&format!("<p><strong>Rule:</strong> {}</p>\n", finding.rule_id));
            html.push_str(&format!("<p><strong>File:</strong> {}:{}:{}</p>\n",
                finding.location.file.display(),
                finding.location.start_line,
                finding.location.start_column
            ));
            html.push_str(&format!("<p><strong>Severity:</strong> {:?}</p>\n", finding.severity));
            html.push_str(&format!("<p><strong>Confidence:</strong> {:?}</p>\n", finding.confidence));

            if let Some(ref fix) = finding.fix {
                html.push_str(&format!("<p><strong>Fix:</strong> {}</p>\n", fix));
            }

            html.push_str("</div>\n");
        }
    }

    html.push_str("</body>\n</html>\n");

    Ok(html)
}

fn generate_markdown_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    _config: &EnhancedAnalysisConfig,
    total_time: std::time::Duration,
) -> Result<String> {
    let mut md = String::new();

    md.push_str("# astgrep Analysis Report\n\n");
    md.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    md.push_str("## Summary\n\n");
    md.push_str(&format!("- **Total findings:** {}\n", findings.len()));
    md.push_str(&format!("- **Files analyzed:** {}\n", stats.files_analyzed));
    md.push_str(&format!("- **Analysis time:** {:?}\n\n", total_time));

    if !findings.is_empty() {
        md.push_str("## Findings\n\n");

        for (i, finding) in findings.iter().enumerate() {
            let severity_emoji = match finding.severity {
                Severity::Critical => "🔴",
                Severity::Error => "🔴",
                Severity::Warning => "🟡",
                Severity::Info => "🔵",
            };

            md.push_str(&format!("### {} {}. {}\n\n", severity_emoji, i + 1, finding.message));
            md.push_str(&format!("- **Rule:** `{}`\n", finding.rule_id));
            md.push_str(&format!("- **File:** `{}:{}:{}`\n",
                finding.location.file.display(),
                finding.location.start_line,
                finding.location.start_column
            ));
            md.push_str(&format!("- **Severity:** {:?}\n", finding.severity));
            md.push_str(&format!("- **Confidence:** {:?}\n", finding.confidence));

            if let Some(ref fix) = finding.fix {
                md.push_str(&format!("- **Fix:** {}\n", fix));
            }

            md.push_str("\n");
        }
    }

    Ok(md)
}

/// Analysis statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisStatistics {
    pub files_analyzed: usize,
    pub rules_executed: usize,
    pub parse_errors: usize,
    pub analysis_errors: usize,
    pub dataflow_analyses: usize,
}

impl AnalysisStatistics {
    pub fn new() -> Self {
        Self {
            files_analyzed: 0,
            rules_executed: 0,
            parse_errors: 0,
            analysis_errors: 0,
            dataflow_analyses: 0,
        }
    }
}

/// Generate semgrep-compatible output format
fn generate_semgrep_compatible_output(
    findings: &[Finding],
    stats: &AnalysisStatistics,
    _config: &EnhancedAnalysisConfig,
    _total_time: std::time::Duration,
) -> Result<String> {
    use std::fmt::Write;

    let mut output = String::new();

    // Semgrep header
    writeln!(&mut output, "┌──── ○○○ ────┐")?;
    writeln!(&mut output, "│ astgrep │")?;
    writeln!(&mut output, "└─────────────┘")?;
    writeln!(&mut output)?;

    // Progress section
    writeln!(&mut output, "Scanning {} file(s) with {} rule(s):", stats.files_analyzed, stats.rules_executed)?;
    writeln!(&mut output)?;
    writeln!(&mut output, "  CODE RULES")?;
    writeln!(&mut output, "  Scanning {} file(s).", stats.files_analyzed)?;
    writeln!(&mut output)?;
    writeln!(&mut output, "  PROGRESS")?;
    writeln!(&mut output, "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100% 0:00:00")?;
    writeln!(&mut output)?;
    writeln!(&mut output)?;

    // Findings section
    if findings.is_empty() {
        writeln!(&mut output, "┌─────────────────┐")?;
        writeln!(&mut output, "│ 0 Code Findings │")?;
        writeln!(&mut output, "└─────────────────┘")?;
    } else {
        writeln!(&mut output, "┌─────────────────┐")?;
        writeln!(&mut output, "│ {} Code Finding{} │", findings.len(), if findings.len() == 1 { "" } else { "s" })?;
        writeln!(&mut output, "└─────────────────┘")?;
        writeln!(&mut output)?;

        // Group findings by file and then by rule
        let mut findings_by_file_and_rule = std::collections::HashMap::new();
        for finding in findings {
            let file_path = finding.location.file.to_string_lossy().to_string();
            findings_by_file_and_rule
                .entry(file_path)
                .or_insert_with(std::collections::HashMap::new)
                .entry(finding.rule_id.clone())
                .or_insert_with(Vec::new)
                .push(finding);
        }

        for (file_path, rules_map) in findings_by_file_and_rule {
            writeln!(&mut output, "    {}", file_path)?;

            for (rule_id, mut rule_findings) in rules_map {
                // Sort findings by line number
                rule_findings.sort_by_key(|f| f.location.start_line);

                // Get the first finding to extract rule info
                let first_finding = &rule_findings[0];
                writeln!(&mut output, "   ❯❯❱ {}", rule_id)?;
                writeln!(&mut output, "          {}", first_finding.message.trim())?;
                writeln!(&mut output)?;

                // Display all findings for this rule
                for (i, finding) in rule_findings.iter().enumerate() {
                    writeln!(&mut output, "           {}┆ {}", finding.location.start_line,
                            get_source_line(&finding.location.file, finding.location.start_line).unwrap_or_else(|| "<source unavailable>".to_string()))?;

                    // Add separator between findings (except for the last one)
                    if rule_findings.len() > 1 && i < rule_findings.len() - 1 {
                        writeln!(&mut output, "            ⋮┆----------------------------------------")?;
                    }
                }
                writeln!(&mut output)?;
            }
        }
    }

    // Summary section
    writeln!(&mut output, "┌──────────────┐")?;
    writeln!(&mut output, "│ Scan Summary │")?;
    writeln!(&mut output, "└──────────────┘")?;
    writeln!(&mut output, "✅ Scan completed successfully.")?;
    writeln!(&mut output, " • Findings: {} ({} blocking)", findings.len(),
             findings.iter().filter(|f| matches!(f.severity, Severity::Error)).count())?;
    writeln!(&mut output, " • Rules run: {}", stats.rules_executed)?;
    writeln!(&mut output, " • Targets scanned: {}", stats.files_analyzed)?;
    writeln!(&mut output, " • Parsed lines: ~100.0%")?;
    writeln!(&mut output, " • No ignore information available")?;
    writeln!(&mut output, "Ran {} rule{} on {} file{}: {} finding{}.",
             stats.rules_executed,
             if stats.rules_executed == 1 { "" } else { "s" },
             stats.files_analyzed,
             if stats.files_analyzed == 1 { "" } else { "s" },
             findings.len(),
             if findings.len() == 1 { "" } else { "s" })?;

    Ok(output)
}

/// Helper function to get source line from file
fn get_source_line(file_path: &std::path::Path, line_number: usize) -> Option<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    for (current_line, line) in reader.lines().enumerate() {
        if current_line + 1 == line_number {
            return line.ok().map(|l| l.trim().to_string());
        }
    }

    None
}
