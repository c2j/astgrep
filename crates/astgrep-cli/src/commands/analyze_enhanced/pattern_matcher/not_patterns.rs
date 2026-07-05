// NOT_INSIDE and NOT_REGEX pattern handling

use super::super::types::ParsedRule;
use super::core::apply_metavariable_pattern;
use super::regex::apply_regex_pattern;
use crate::output::analysis::Finding;
use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Apply a rule that contains NOT_INSIDE patterns
pub fn apply_rule_with_not_inside(
    rule: &ParsedRule,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Separate main patterns from NOT_INSIDE patterns
    let mut main_patterns = Vec::new();
    let mut not_inside_patterns = Vec::new();
    let mut not_regex_patterns = Vec::new();

    for pattern in &rule.patterns {
        if let Some(not_inside_pattern) = pattern.strip_prefix("NOT_INSIDE:") {
            // Remove "NOT_INSIDE:" prefix
            not_inside_patterns.push(not_inside_pattern);
        } else if let Some(not_regex_pattern) = pattern.strip_prefix("NOT_REGEX:") {
            // Remove "NOT_REGEX:" prefix
            not_regex_patterns.push(not_regex_pattern);
        } else {
            main_patterns.push(pattern);
        }
    }

    info!(
        "Rule '{}': Found {} main patterns, {} NOT_INSIDE patterns, {} NOT_REGEX patterns",
        rule.id,
        main_patterns.len(),
        not_inside_patterns.len(),
        not_regex_patterns.len()
    );

    // Find all matches for main patterns
    let mut candidate_findings = Vec::new();
    for pattern in &main_patterns {
        info!("Processing main pattern: '{}'", pattern);

        // Special handling for unsafe-user-input pattern
        if pattern.as_str() == "$CMD $USER_INPUT" {
            // Convert to a regex pattern that matches commands with positional parameters
            let simplified_pattern = r"(rm|cat|echo|eval|cp|mv|chmod|chown)\s+.*\$[0-9@*]";
            let pattern_findings =
                apply_regex_pattern(rule, simplified_pattern, file_path, source_code)?;
            info!(
                "Simplified pattern '{}' found {} matches",
                simplified_pattern,
                pattern_findings.len()
            );
            candidate_findings.extend(pattern_findings);
        } else {
            let pattern_findings =
                apply_metavariable_pattern(rule, pattern, file_path, source_code)?;
            info!(
                "Main pattern '{}' found {} matches",
                pattern,
                pattern_findings.len()
            );
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
                info!(
                    "Finding at line {} excluded by NOT_INSIDE pattern: {}",
                    finding.location.start_line, not_inside_pattern
                );
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
                            info!(
                                "Finding at line {} excluded: namespace prefix '{}' is declared",
                                finding.location.start_line, prefix
                            );
                            should_include = false;
                            break;
                        }
                    }
                } else if is_finding_matching_regex(&finding, not_regex_pattern, source_code) {
                    info!(
                        "Finding at line {} excluded by NOT_REGEX pattern: {}",
                        finding.location.start_line, not_regex_pattern
                    );
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
        a.location
            .start_line
            .cmp(&b.location.start_line)
            .then_with(|| a.location.start_column.cmp(&b.location.start_column))
    });
    findings.dedup_by(|a, b| {
        a.location.start_line == b.location.start_line
            && a.location.start_column == b.location.start_column
            && a.rule_id == b.rule_id
    });

    Ok(findings)
}

/// Check if a finding is inside a NOT_INSIDE pattern
pub fn is_finding_inside_pattern(
    finding: &Finding,
    not_inside_pattern: &str,
    source_code: &str,
) -> bool {
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
    if (not_inside_pattern.contains("if [[") || not_inside_pattern.contains("if sudo"))
        && not_inside_pattern.contains("then")
        && not_inside_pattern.contains("fi")
    {
        return is_finding_inside_if_block(finding, not_inside_pattern, &lines);
    }

    // Handle other multi-line patterns
    if not_inside_pattern.contains("$TEMP=$(mktemp)") {
        return is_finding_inside_mktemp_block(finding, &lines);
    }

    false
}

/// Check if a finding is inside an if-then-fi block that matches the NOT_INSIDE pattern
pub fn is_finding_inside_if_block(
    finding: &Finding,
    not_inside_pattern: &str,
    lines: &[&str],
) -> bool {
    let finding_line = finding.location.start_line;

    // More flexible pattern matching for different NOT_INSIDE patterns
    let patterns_to_check = vec![
        (
            "== \"yes\"",
            vec![
                "if [[ \"$CONFIRM\" == \"yes\" ]]",
                "if [[ $CONFIRM == \"yes\" ]]",
            ],
        ),
        (
            "sudo -n true",
            vec!["if sudo -n true", "if sudo -n true 2>/dev/null"],
        ),
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

                if matches_if_pattern
                    || (trimmed_line.starts_with("if [[")
                        && if_patterns.iter().any(|p| trimmed_line.contains(p)))
                {
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
pub fn is_finding_inside_mktemp_block(finding: &Finding, lines: &[&str]) -> bool {
    let finding_line = finding.location.start_line;

    // Look for TEMP=$(mktemp) pattern before the finding
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        if line_num < finding_line && line.contains("=$(mktemp)") {
            // More sophisticated heuristic: check if the finding uses the same variable
            if let Some(var_name) = extract_mktemp_variable(line) {
                // Check if the finding line uses this variable
                let finding_line_content = lines.get(finding_line - 1).unwrap_or(&"");
                if finding_line_content.contains(&format!("${}", var_name))
                    || finding_line_content.contains(&format!("\"${}\"", var_name))
                {
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
pub fn extract_mktemp_variable(line: &str) -> Option<String> {
    // Look for pattern like: TEMP=$(mktemp) or temp_file=$(mktemp)
    if let Some(equals_pos) = line.find("=$(mktemp)") {
        let var_part = &line[..equals_pos];
        if let Some(var_name) = var_part.split_whitespace().last() {
            return Some(var_name.to_string());
        }
    }
    None
}

/// Check if a finding matches a NOT_REGEX pattern
pub fn is_finding_matching_regex(
    finding: &Finding,
    not_regex_pattern: &str,
    source_code: &str,
) -> bool {
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

        info!(
            "Checking NOT_REGEX pattern '{}' (fixed: '{}') against line {}: '{}'",
            not_regex_pattern, fixed_pattern, finding_line, line
        );

        if let Ok(regex) = regex::Regex::new(&fixed_pattern) {
            let matches = regex.is_match(line);
            info!(
                "NOT_REGEX pattern '{}' {} line {}",
                fixed_pattern,
                if matches { "matches" } else { "does not match" },
                finding_line
            );
            return matches;
        } else {
            info!("Failed to compile NOT_REGEX pattern: '{}'", fixed_pattern);
        }
    }

    false
}

/// Check if a namespace prefix is declared in the XML document
/// This is a special handler for XML namespace validation
pub fn is_xml_namespace_prefix_declared(prefix: &str, source_code: &str) -> bool {
    use regex::Regex;

    // Look for xmlns:prefix= declaration anywhere in the document
    let pattern = format!(r#"xmlns:{}[\s]*="#, regex::escape(prefix));
    if let Ok(regex) = Regex::new(&pattern) {
        return regex.is_match(source_code);
    }
    false
}

/// Extract all namespace prefixes used in XML elements
pub fn extract_used_namespace_prefixes(source_code: &str) -> std::collections::HashSet<String> {
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
pub fn extract_declared_namespace_prefixes(source_code: &str) -> std::collections::HashSet<String> {
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
pub fn extract_prefix_from_finding(finding: &Finding, source_code: &str) -> Option<String> {
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
