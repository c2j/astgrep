// Core pattern matching functions

use super::super::types::{determine_language, ParsedRule};
use super::{apply_regex_pattern, apply_simple_metavariable_pattern};
use crate::output::analysis::{Confidence, Finding, Location};
use anyhow::Result;
use astgrep_core::Language;
use std::path::PathBuf;
use tracing::{info, warn};

/// Apply metavariable pattern matching using our pattern matcher
pub fn apply_metavariable_pattern(
    rule: &ParsedRule,
    pattern: &str,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<Vec<Finding>> {
    
    

    let findings = Vec::new();

    // Determine language
    let language = match determine_language(file_path) {
        Ok(lang) => lang,
        Err(_) => return Ok(findings), // Skip if language cannot be determined
    };

    // Check if pattern looks like a regex (contains regex metacharacters)
    let is_likely_regex = pattern.contains('(')
        && pattern.contains(')')
        && (pattern.contains('\\')
            || pattern.contains('[')
            || pattern.contains('*')
            || pattern.contains('+'));

    if is_likely_regex {
        info!(
            "Pattern looks like regex, attempting direct regex matching: {}",
            pattern
        );
        match apply_regex_pattern(rule, pattern, file_path, source_code) {
            Ok(regex_findings) => {
                info!(
                    "Regex pattern matching found {} matches",
                    regex_findings.len()
                );
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
    info!(
        "Attempting enhanced pattern matching for pattern: {}",
        pattern
    );
    match apply_enhanced_pattern_matching(rule, file_path, source_code, language) {
        Ok(enhanced_findings) => {
            info!(
                "Enhanced pattern matching succeeded, found {} matches",
                enhanced_findings.len()
            );
            if !enhanced_findings.is_empty() {
                return Ok(enhanced_findings);
            } else {
                info!("Enhanced pattern matching found no matches, falling back to tree-sitter");
            }
        }
        Err(e) => {
            warn!(
                "Enhanced pattern matching failed for {}: {}, falling back to tree-sitter",
                file_path.display(),
                e
            );
        }
    }

    // Try to use tree-sitter for proper AST-based pattern matching
    info!("Attempting tree-sitter parsing for pattern: {}", pattern);
    match apply_tree_sitter_pattern_matching(rule, pattern, file_path, source_code, language) {
        Ok(tree_sitter_findings) => {
            info!(
                "Tree-sitter parsing succeeded, found {} matches",
                tree_sitter_findings.len()
            );
            if !tree_sitter_findings.is_empty() {
                return Ok(tree_sitter_findings);
            } else {
                info!("Tree-sitter found no matches, falling back to simple matching");
            }
        }
        Err(e) => {
            warn!(
                "Tree-sitter parsing failed for {}: {}, falling back to simple matching",
                file_path.display(),
                e
            );
        }
    }

    // Fallback to simple pattern matching if tree-sitter fails
    return apply_simple_metavariable_pattern(rule, pattern, file_path, source_code);
}

/// Apply enhanced pattern matching using our new AdvancedSemgrepMatcher
pub fn apply_enhanced_pattern_matching(
    rule: &ParsedRule,
    file_path: &PathBuf,
    source_code: &str,
    language: Language,
) -> Result<Vec<Finding>> {
    use astgrep_matcher::AdvancedSemgrepMatcher;
    use astgrep_parser::tree_sitter_parser::TreeSitterParser;
    use astgrep_rules::RuleParser;

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
pub fn convert_parsed_rule_to_yaml(rule: &ParsedRule) -> Result<String> {
    // Preserve original rule YAML structure to keep semantics (e.g., pattern-either)
    let mut top = serde_yaml::Mapping::new();
    let mut rules_seq = serde_yaml::Sequence::new();
    rules_seq.push(rule.raw_rule_value.clone());
    top.insert(
        serde_yaml::Value::String("rules".to_string()),
        serde_yaml::Value::Sequence(rules_seq),
    );
    let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(top))?;
    Ok(yaml)
}

/// Convert our Pattern to SemgrepPattern
pub fn convert_pattern_to_semgrep_pattern(
    pattern: &astgrep_rules::Pattern,
) -> Result<astgrep_core::SemgrepPattern> {
    use astgrep_core::{PatternType as CorePatternType, SemgrepPattern};

    let core_pattern_type = match &pattern.pattern_type {
        astgrep_rules::PatternType::Simple(s) => CorePatternType::Simple(s.clone()),
        astgrep_rules::PatternType::Either(patterns) => {
            let converted: Result<Vec<_>> = patterns
                .iter()
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
            let converted: Result<Vec<_>> = patterns
                .iter()
                .map(convert_pattern_to_semgrep_pattern)
                .collect();
            CorePatternType::All(converted?)
        }
        astgrep_rules::PatternType::Any(patterns) => {
            let converted: Result<Vec<_>> = patterns
                .iter()
                .map(convert_pattern_to_semgrep_pattern)
                .collect();
            CorePatternType::Any(converted?)
        }
    };

    Ok(SemgrepPattern {
        pattern_type: core_pattern_type,
        metavariable_pattern: None, // TODO: Convert metavariable patterns
        conditions: Vec::new(),     // TODO: Convert conditions
        focus: pattern.focus.clone(),
    })
}

/// Apply tree-sitter based pattern matching for better precision
pub fn apply_tree_sitter_pattern_matching(
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
            info!(
                "Match {}: kind='{}', text='{}'",
                i,
                node.kind(),
                node.utf8_text(source_code.as_bytes())
                    .unwrap_or("<invalid>")
            );
            // Skip matches in comments
            if node.kind() == "comment"
                || node.kind() == "line_comment"
                || node.kind() == "block_comment"
            {
                continue;
            }

            // Check if the node is inside a comment by examining parent nodes
            let mut current = node.parent();
            let mut in_comment = false;
            while let Some(parent) = current {
                if parent.kind() == "comment"
                    || parent.kind() == "line_comment"
                    || parent.kind() == "block_comment"
                {
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
        warn!(
            "Tree-sitter failed to parse source code for language: {:?}",
            language
        );
    }

    info!(
        "Tree-sitter analysis completed with {} findings",
        findings.len()
    );
    Ok(findings)
}

/// Convert a semgrep-style pattern to a simple regex pattern
pub fn convert_pattern_to_regex(pattern: &str) -> String {
    // Handle patterns like "System.out.println($MESSAGE)" or "eval $CODE"
    let mut regex_pattern = pattern.to_string();

    // For patterns with metavariables, we need to be more precise
    // Replace metavariables with more specific regex patterns

    // Special handling for specific metavariables first
    regex_pattern = regex_pattern.replace("$CODE", r"\S+"); // Non-whitespace
    regex_pattern = regex_pattern.replace("$CMD", r"\S+"); // Non-whitespace
    regex_pattern = regex_pattern.replace("$USER_INPUT", r"\$[0-9@*]+"); // Bash positional parameters
    regex_pattern = regex_pattern.replace("$FILE", r"\S+"); // Non-whitespace
    regex_pattern = regex_pattern.replace("$OPTIONS", r"[^\s]+"); // Non-whitespace
    regex_pattern = regex_pattern.replace("$URL", r"\S+"); // Non-whitespace

    // General metavariable replacement (more conservative)
    regex_pattern = regex::Regex::new(r"\$[A-Z_][A-Z0-9_]*")
        .unwrap()
        .replace_all(&regex_pattern, r"\S+")
        .to_string();
    regex_pattern = regex::Regex::new(r"\$[a-z_][a-z0-9_]*")
        .unwrap()
        .replace_all(&regex_pattern, r"\S+")
        .to_string();

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
pub fn find_pattern_matches(pattern: &str, line: &str) -> Option<Vec<usize>> {
    // First try exact string matching for simple patterns
    if !pattern.contains('[')
        && !pattern.contains('*')
        && !pattern.contains('+')
        && !pattern.contains('?')
    {
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

/// Replace capture groups in message template with actual captured values
pub fn replace_capture_groups(message: &str, captures: &regex::Captures) -> String {
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
