// Pattern matching functionality for enhanced analysis

pub mod core;
pub mod not_patterns;
pub mod regex;
pub mod taint;
pub mod utils;

// Re-export key functions from submodules for convenience
pub use self::{
    core::{apply_enhanced_pattern_matching, apply_metavariable_pattern},
    not_patterns::apply_rule_with_not_inside,
    regex::apply_regex_pattern,
    taint::apply_simple_taint_analysis,
    utils::apply_simple_metavariable_pattern,
};

use super::types::{determine_language, BasicPattern, ParsedRule};
use crate::output::analysis::{Confidence, Finding, Location};
use crate::tree_sitter_analyzer::TreeSitterAnalyzer;
use anyhow::Result;
use astgrep_core::Language;
use std::path::PathBuf;
use tracing::info;

/// Apply a single rule to source code
pub fn apply_rule_to_source(
    rule: &ParsedRule,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Check if this rule has NOT_INSIDE or NOT_REGEX patterns that need special handling
    let has_not_inside = rule.patterns.iter().any(|p| p.starts_with("NOT_INSIDE:"));
    let has_not_regex = rule.patterns.iter().any(|p| p.starts_with("NOT_REGEX:"));
    if has_not_inside || has_not_regex {
        info!(
            "Rule '{}' has NOT_INSIDE or NOT_REGEX patterns, applying special handling",
            rule.id
        );
        // Apply special handling for rules with NOT_INSIDE or NOT_REGEX patterns
        findings.extend(apply_rule_with_not_inside(rule, file_path, source_code)?);
        return Ok(findings);
    }

    // Check if this is a taint analysis rule
    if rule.patterns.iter().any(|p| p.contains("sink("))
        && rule.patterns.iter().any(|p| p.contains("\"tainted\""))
    {
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
            if let Ok(enhanced_findings) =
                apply_enhanced_pattern_matching(rule, file_path, source_code, language)
            {
                if !enhanced_findings.is_empty() {
                    return Ok(enhanced_findings);
                }
            }
        }

        // Fallback to pattern-aware matching
        for pattern in &rule.patterns {
            // Check if this is a regex pattern first (higher priority)
            if pattern.starts_with("NOT_REGEX:")
                || // Direct regex patterns (no unescaped metavariables)
                (!pattern.contains('$') && (pattern.contains('(') || pattern.contains('[') || pattern.contains('\\')))
                || // Regex patterns with escaped dollar signs (like \\$[a-zA-Z_])
                (pattern.contains("\\$") && (pattern.contains('[') || pattern.contains('(') || pattern.contains('?')))
                || // Regex patterns with common regex syntax
                (pattern.contains('\\') && (pattern.contains("\\s") || pattern.contains("\\d") || pattern.contains("\\w")))
            {
                // Use our advanced pattern matcher for regex patterns
                findings.extend(apply_metavariable_pattern(
                    rule,
                    pattern,
                    file_path,
                    source_code,
                )?);
            } else if pattern.contains('$') {
                // Use our pattern matcher for metavariable patterns
                findings.extend(apply_metavariable_pattern(
                    rule,
                    pattern,
                    file_path,
                    source_code,
                )?);
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

/// Get basic security patterns for a language
pub fn get_basic_security_patterns(language: Language) -> Vec<BasicPattern> {
    match language {
        Language::Java => vec![
            BasicPattern {
                rule_id: "java-sql-injection".to_string(),
                pattern: ".executeQuery(".to_string(),
                message: "Potential SQL injection vulnerability".to_string(),
                severity: crate::output::analysis::Severity::Critical,
                confidence: Confidence::Medium,
                fix: Some("Use PreparedStatement instead of Statement".to_string()),
            },
            BasicPattern {
                rule_id: "java-hardcoded-password".to_string(),
                pattern: "password".to_string(),
                message: "Potential hardcoded password".to_string(),
                severity: crate::output::analysis::Severity::Warning,
                confidence: Confidence::Low,
                fix: Some("Use environment variables or secure configuration".to_string()),
            },
            BasicPattern {
                rule_id: "java-weak-hash-sha1".to_string(),
                pattern: "\"SHA1\"".to_string(),
                message: "Use of SHA1 hash algorithm which is considered insecure".to_string(),
                severity: crate::output::analysis::Severity::Error,
                confidence: Confidence::High,
                fix: Some("Use SHA-256, SHA-384, or SHA-512 instead of SHA1".to_string()),
            },
            BasicPattern {
                rule_id: "java-weak-hash-md5".to_string(),
                pattern: "\"MD5\"".to_string(),
                message: "Use of MD5 hash algorithm which is considered insecure".to_string(),
                severity: crate::output::analysis::Severity::Error,
                confidence: Confidence::High,
                fix: Some("Use SHA-256, SHA-384, or SHA-512 instead of MD5".to_string()),
            },
        ],
        Language::JavaScript => vec![
            BasicPattern {
                rule_id: "js-eval-usage".to_string(),
                pattern: "eval(".to_string(),
                message: "Use of eval() can lead to code injection".to_string(),
                severity: crate::output::analysis::Severity::Critical,
                confidence: Confidence::High,
                fix: Some("Avoid using eval(), use safer alternatives".to_string()),
            },
            BasicPattern {
                rule_id: "js-innerhtml".to_string(),
                pattern: "innerHTML".to_string(),
                message: "Potential XSS vulnerability with innerHTML".to_string(),
                severity: crate::output::analysis::Severity::Warning,
                confidence: Confidence::Medium,
                fix: Some("Use textContent or sanitize input".to_string()),
            },
        ],
        Language::Python => vec![
            BasicPattern {
                rule_id: "python-exec-usage".to_string(),
                pattern: "exec(".to_string(),
                message: "Use of exec() can lead to code injection".to_string(),
                severity: crate::output::analysis::Severity::Critical,
                confidence: Confidence::High,
                fix: Some("Avoid using exec(), use safer alternatives".to_string()),
            },
            BasicPattern {
                rule_id: "python-sql-format".to_string(),
                pattern: ".format(".to_string(),
                message: "Potential SQL injection with string formatting".to_string(),
                severity: crate::output::analysis::Severity::Warning,
                confidence: Confidence::Low,
                fix: Some("Use parameterized queries".to_string()),
            },
        ],
        Language::Sql => vec![BasicPattern {
            rule_id: "sql-union-injection".to_string(),
            pattern: "UNION".to_string(),
            message: "Potential SQL injection with UNION".to_string(),
            severity: crate::output::analysis::Severity::Critical,
            confidence: Confidence::Medium,
            fix: Some("Use parameterized queries".to_string()),
        }],
        Language::Bash => vec![BasicPattern {
            rule_id: "bash-command-injection".to_string(),
            pattern: "$((".to_string(),
            message: "Potential command injection".to_string(),
            severity: crate::output::analysis::Severity::Critical,
            confidence: Confidence::Medium,
            fix: Some("Validate and sanitize input".to_string()),
        }],
        Language::Xml => vec![],
    }
}
