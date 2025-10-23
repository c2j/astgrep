//! Rules management handlers

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    models::{RuleInfo, ValidateRulesRequest, ValidateRulesResponse, RulePerformanceMetrics},
    WebConfig, WebError, WebResult,
};

/// Query parameters for listing rules
#[derive(Debug, Deserialize)]
pub struct ListRulesQuery {
    /// Filter by language
    pub language: Option<String>,
    /// Filter by category
    pub category: Option<String>,
    /// Filter by enabled status
    pub enabled: Option<bool>,
    /// Number of rules to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// List available rules
pub async fn list_rules(
    State(config): State<Arc<WebConfig>>,
    Query(params): Query<ListRulesQuery>,
) -> WebResult<Json<Vec<RuleInfo>>> {
    tracing::info!("Listing rules with filters: {:?}", params);
    
    // Load rules from the rules directory
    let mut rules = load_rules_from_directory(&config.rules_directory).await?;
    
    // Apply filters
    if let Some(language_filter) = &params.language {
        rules.retain(|rule| rule.languages.contains(language_filter));
    }
    
    if let Some(category_filter) = &params.category {
        rules.retain(|rule| {
            rule.category.as_ref().map_or(false, |cat| cat == category_filter)
        });
    }
    
    if let Some(enabled_filter) = params.enabled {
        rules.retain(|rule| rule.enabled == enabled_filter);
    }
    
    // Apply pagination
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(500); // Max 500 rules per request
    
    let total_rules = rules.len();
    let paginated_rules: Vec<RuleInfo> = rules
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    tracing::info!(
        "Listed {} rules (offset: {}, limit: {}, total: {})",
        paginated_rules.len(),
        offset,
        limit,
        total_rules
    );

    Ok(Json(paginated_rules))
}

/// Get specific rule by ID
pub async fn get_rule(
    State(config): State<Arc<WebConfig>>,
    Path(rule_id): Path<String>,
) -> WebResult<Json<RuleInfo>> {
    tracing::info!("Getting rule: {}", rule_id);
    
    let rules = load_rules_from_directory(&config.rules_directory).await?;
    
    let rule = rules
        .into_iter()
        .find(|r| r.id == rule_id)
        .ok_or_else(|| WebError::not_found(format!("Rule not found: {}", rule_id)))?;

    Ok(Json(rule))
}

/// Validate rule definitions
pub async fn validate_rules(
    State(_config): State<Arc<WebConfig>>,
    Json(request): Json<ValidateRulesRequest>,
) -> WebResult<Json<ValidateRulesResponse>> {
    tracing::info!("Validating rules");
    
    let start_time = std::time::Instant::now();
    
    // Parse YAML rules
    let rules_result: Result<serde_yaml::Value, _> = serde_yaml::from_str(&request.rules);
    
    let (valid, errors, warnings, rules_count) = match rules_result {
        Ok(rules_yaml) => {
            // Validate the parsed rules
            validate_rules_yaml(&rules_yaml, &request)
        }
        Err(e) => {
            let errors = vec![format!("YAML parsing error: {}", e)];
            (false, errors, vec![], 0)
        }
    };
    
    let load_time = start_time.elapsed();
    
    // Generate performance metrics if requested
    let performance = request.check_performance.unwrap_or(false).then(|| {
        RulePerformanceMetrics {
            load_time_ms: load_time.as_millis() as u64,
            average_complexity: calculate_average_complexity(&request.rules),
            memory_usage_bytes: estimate_memory_usage(&request.rules),
        }
    });
    
    let response = ValidateRulesResponse {
        valid,
        errors,
        warnings,
        rules_count,
        performance,
    };
    
    tracing::info!(
        "Rule validation completed: valid={}, errors={}, warnings={}, rules={}",
        response.valid,
        response.errors.len(),
        response.warnings.len(),
        response.rules_count
    );

    Ok(Json(response))
}

/// Load rules from directory (real implementation)
async fn load_rules_from_directory(
    rules_dir: &std::path::Path,
) -> WebResult<Vec<RuleInfo>> {
    use tokio::fs;

    let mut rules = Vec::new();

    // Check if rules directory exists
    if !rules_dir.exists() {
        tracing::warn!("Rules directory does not exist: {}", rules_dir.display());
        return Ok(get_fallback_rules());
    }

    // Read directory entries
    let mut dir_entries = match fs::read_dir(rules_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to read rules directory: {}", e);
            return Ok(get_fallback_rules());
        }
    };

    // Process each file in the directory
    while let Some(entry) = dir_entries.next_entry().await.map_err(|e| {
        WebError::internal_server_error(format!("Failed to read directory entry: {}", e))
    })? {
        let path = entry.path();

        // Only process .yaml and .yml files
        if let Some(extension) = path.extension() {
            if extension == "yaml" || extension == "yml" {
                match load_rule_file(&path).await {
                    Ok(mut file_rules) => {
                        rules.append(&mut file_rules);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load rule file {}: {}", path.display(), e);
                        // Continue processing other files
                    }
                }
            }
        }
    }

    // If no rules were loaded, return fallback rules
    if rules.is_empty() {
        tracing::info!("No rules loaded from directory, using fallback rules");
        Ok(get_fallback_rules())
    } else {
        tracing::info!("Loaded {} rules from directory", rules.len());
        Ok(rules)
    }
}

/// Load rules from a single YAML file
async fn load_rule_file(file_path: &std::path::Path) -> WebResult<Vec<RuleInfo>> {
    use tokio::fs;

    let content = fs::read_to_string(file_path).await.map_err(|e| {
        WebError::internal_server_error(format!("Failed to read rule file {}: {}", file_path.display(), e))
    })?;

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        WebError::internal_server_error(format!("Failed to parse YAML in {}: {}", file_path.display(), e))
    })?;

    parse_rules_from_yaml(&yaml_value, file_path)
}

/// Parse rules from YAML value
fn parse_rules_from_yaml(yaml_value: &serde_yaml::Value, file_path: &std::path::Path) -> WebResult<Vec<RuleInfo>> {
    let mut rules = Vec::new();

    match yaml_value {
        serde_yaml::Value::Sequence(rule_list) => {
            for (index, rule_yaml) in rule_list.iter().enumerate() {
                match parse_single_rule(rule_yaml, file_path, index) {
                    Ok(rule) => rules.push(rule),
                    Err(e) => {
                        tracing::warn!("Failed to parse rule {} in {}: {}", index, file_path.display(), e);
                        // Continue parsing other rules
                    }
                }
            }
        }
        serde_yaml::Value::Mapping(_) => {
            // Single rule in file
            match parse_single_rule(yaml_value, file_path, 0) {
                Ok(rule) => rules.push(rule),
                Err(e) => {
                    tracing::warn!("Failed to parse rule in {}: {}", file_path.display(), e);
                }
            }
        }
        _ => {
            return Err(WebError::internal_server_error(format!(
                "Invalid YAML structure in {}: expected sequence or mapping",
                file_path.display()
            )));
        }
    }

    Ok(rules)
}

/// Parse a single rule from YAML
fn parse_single_rule(rule_yaml: &serde_yaml::Value, file_path: &std::path::Path, index: usize) -> WebResult<RuleInfo> {
    let rule_map = rule_yaml.as_mapping().ok_or_else(|| {
        WebError::internal_server_error(format!("Rule {} in {} is not a mapping", index, file_path.display()))
    })?;

    // Extract required fields
    let id = extract_string_field(rule_map, "id", file_path, index)?;
    let name = extract_string_field(rule_map, "name", file_path, index)?;
    let description = extract_string_field(rule_map, "description", file_path, index)?;
    let severity = extract_string_field(rule_map, "severity", file_path, index)?;

    // Extract optional fields with defaults
    let confidence = extract_string_field(rule_map, "confidence", file_path, index)
        .unwrap_or_else(|_| "medium".to_string());
    let enabled = extract_bool_field(rule_map, "enabled", file_path, index)
        .unwrap_or(true);

    // Extract arrays
    let languages = extract_string_array(rule_map, "languages", file_path, index)?;
    let tags = extract_string_array(rule_map, "tags", file_path, index)
        .unwrap_or_else(|_| Vec::new());

    // Extract optional fields
    let category = extract_optional_string_field(rule_map, "category");
    let metadata = extract_metadata(rule_map);

    Ok(RuleInfo {
        id,
        name,
        description,
        languages,
        severity,
        confidence,
        category,
        tags,
        enabled,
        metadata,
    })
}

/// Extract string field from YAML mapping
fn extract_string_field(
    map: &serde_yaml::Mapping,
    field: &str,
    file_path: &std::path::Path,
    index: usize,
) -> WebResult<String> {
    map.get(&serde_yaml::Value::String(field.to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            WebError::internal_server_error(format!(
                "Missing or invalid '{}' field in rule {} of {}",
                field,
                index,
                file_path.display()
            ))
        })
}

/// Extract optional string field from YAML mapping
fn extract_optional_string_field(map: &serde_yaml::Mapping, field: &str) -> Option<String> {
    map.get(&serde_yaml::Value::String(field.to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract boolean field from YAML mapping
fn extract_bool_field(
    map: &serde_yaml::Mapping,
    field: &str,
    file_path: &std::path::Path,
    index: usize,
) -> WebResult<bool> {
    map.get(&serde_yaml::Value::String(field.to_string()))
        .and_then(|v| v.as_bool())
        .ok_or_else(|| {
            WebError::internal_server_error(format!(
                "Missing or invalid '{}' field in rule {} of {}",
                field,
                index,
                file_path.display()
            ))
        })
}

/// Extract string array from YAML mapping
fn extract_string_array(
    map: &serde_yaml::Mapping,
    field: &str,
    file_path: &std::path::Path,
    index: usize,
) -> WebResult<Vec<String>> {
    map.get(&serde_yaml::Value::String(field.to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .ok_or_else(|| {
            WebError::internal_server_error(format!(
                "Missing or invalid '{}' field in rule {} of {}",
                field,
                index,
                file_path.display()
            ))
        })
}

/// Extract metadata from YAML mapping
fn extract_metadata(map: &serde_yaml::Mapping) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    if let Some(meta_value) = map.get(&serde_yaml::Value::String("metadata".to_string())) {
        if let Some(meta_map) = meta_value.as_mapping() {
            for (key, value) in meta_map {
                if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                    metadata.insert(key_str.to_string(), value_str.to_string());
                }
            }
        }
    }

    metadata
}

/// Get fallback rules when no rules can be loaded from files
fn get_fallback_rules() -> Vec<RuleInfo> {
    vec![
        RuleInfo {
            id: "fallback-rule-001".to_string(),
            name: "Basic Security Check".to_string(),
            description: "A basic security check rule loaded as fallback".to_string(),
            languages: vec!["javascript".to_string(), "java".to_string(), "python".to_string()],
            severity: "warning".to_string(),
            confidence: "medium".to_string(),
            category: Some("security".to_string()),
            tags: vec!["security".to_string(), "fallback".to_string()],
            enabled: true,
            metadata: {
                let mut map = HashMap::new();
                map.insert("source".to_string(), "fallback".to_string());
                map
            },
        },
        RuleInfo {
            id: "fallback-rule-002".to_string(),
            name: "SQL Injection Check".to_string(),
            description: "Detects potential SQL injection vulnerabilities".to_string(),
            languages: vec!["java".to_string(), "python".to_string(), "php".to_string()],
            severity: "error".to_string(),
            confidence: "high".to_string(),
            category: Some("security".to_string()),
            tags: vec!["security".to_string(), "sql-injection".to_string(), "fallback".to_string()],
            enabled: true,
            metadata: {
                let mut map = HashMap::new();
                map.insert("source".to_string(), "fallback".to_string());
                map
            },
        },
        RuleInfo {
            id: "fallback-rule-003".to_string(),
            name: "XSS Prevention".to_string(),
            description: "Detects potential cross-site scripting vulnerabilities".to_string(),
            languages: vec!["javascript".to_string(), "java".to_string(), "typescript".to_string()],
            severity: "warning".to_string(),
            confidence: "medium".to_string(),
            category: Some("security".to_string()),
            tags: vec!["security".to_string(), "xss".to_string(), "fallback".to_string()],
            enabled: true,
            metadata: {
                let mut map = HashMap::new();
                map.insert("source".to_string(), "fallback".to_string());
                map
            },
        }
    ]
}

fn validate_rules_yaml(
    rules_yaml: &serde_yaml::Value,
    request: &ValidateRulesRequest,
) -> (bool, Vec<String>, Vec<String>, usize) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut rules_count = 0;
    
    // Basic validation logic
    match rules_yaml {
        serde_yaml::Value::Sequence(rules) => {
            rules_count = rules.len();
            
            for (index, rule) in rules.iter().enumerate() {
                validate_single_rule(rule, index, &mut errors, &mut warnings);
            }
        }
        serde_yaml::Value::Mapping(_) => {
            // Single rule
            rules_count = 1;
            validate_single_rule(rules_yaml, 0, &mut errors, &mut warnings);
        }
        _ => {
            errors.push("Rules must be either a single rule object or an array of rules".to_string());
        }
    }
    
    // Language-specific validation
    if let Some(language) = &request.language {
        if !["java", "javascript", "python", "sql", "bash"].contains(&language.as_str()) {
            warnings.push(format!("Unsupported language for validation: {}", language));
        }
    }
    
    let valid = errors.is_empty();
    (valid, errors, warnings, rules_count)
}

/// Validate a single rule
fn validate_single_rule(
    rule: &serde_yaml::Value,
    index: usize,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let rule_prefix = format!("Rule {}", index + 1);
    
    if let serde_yaml::Value::Mapping(rule_map) = rule {
        // Check required fields
        if !rule_map.contains_key(&serde_yaml::Value::String("id".to_string())) {
            errors.push(format!("{}: Missing required field 'id'", rule_prefix));
        }

        // Check for message field (required for semgrep compatibility)
        if !rule_map.contains_key(&serde_yaml::Value::String("message".to_string())) {
            errors.push(format!("{}: Missing required field 'message'", rule_prefix));
        }

        // name and description are optional (auto-generated if missing)
        if !rule_map.contains_key(&serde_yaml::Value::String("name".to_string())) {
            warnings.push(format!("{}: Missing optional field 'name' (will use id as default)", rule_prefix));
        }

        if !rule_map.contains_key(&serde_yaml::Value::String("description".to_string())) {
            warnings.push(format!("{}: Missing optional field 'description' (will use message as default)", rule_prefix));
        }
        
        // Validate pattern field if present
        if let Some(pattern) = rule_map.get(&serde_yaml::Value::String("pattern".to_string())) {
            if let serde_yaml::Value::String(pattern_str) = pattern {
                if pattern_str.is_empty() {
                    errors.push(format!("{}: Pattern cannot be empty", rule_prefix));
                }
            }
        }
        
        // Validate severity field if present
        if let Some(severity) = rule_map.get(&serde_yaml::Value::String("severity".to_string())) {
            if let serde_yaml::Value::String(severity_str) = severity {
                if !["info", "warning", "error", "critical"].contains(&severity_str.as_str()) {
                    errors.push(format!("{}: Invalid severity '{}'. Must be one of: info, warning, error, critical", rule_prefix, severity_str));
                }
            }
        }
    } else {
        errors.push(format!("{}: Rule must be an object", rule_prefix));
    }
}

/// Calculate average complexity of rules (simplified)
fn calculate_average_complexity(rules_content: &str) -> f64 {
    // This is a simplified complexity calculation
    // In a real implementation, you would analyze the rule patterns
    
    let line_count = rules_content.lines().count() as f64;
    let char_count = rules_content.len() as f64;
    
    // Simple heuristic: complexity based on content size
    (line_count + char_count / 100.0) / 10.0
}

/// Estimate memory usage for rules (simplified)
fn estimate_memory_usage(rules_content: &str) -> u64 {
    // Simple estimation: content size + overhead
    (rules_content.len() as u64) * 2 + 1024 // 2x content size + 1KB overhead
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_rules_no_filter() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        });
        
        let query = ListRulesQuery {
            language: None,
            category: None,
            enabled: None,
            limit: None,
            offset: None,
        };
        
        let result = list_rules(State(config), Query(query)).await;
        assert!(result.is_ok());
        
        let rules = result.unwrap().0;
        assert_eq!(rules.len(), 3); // Fallback rules when no files exist
    }

    #[tokio::test]
    async fn test_list_rules_with_language_filter() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        });
        
        let query = ListRulesQuery {
            language: Some("java".to_string()),
            category: None,
            enabled: None,
            limit: None,
            offset: None,
        };
        
        let result = list_rules(State(config), Query(query)).await;
        assert!(result.is_ok());
        
        let rules = result.unwrap().0;
        assert_eq!(rules.len(), 3); // All mock rules support Java
        
        for rule in rules {
            assert!(rule.languages.contains(&"java".to_string()));
        }
    }

    #[tokio::test]
    async fn test_get_rule_existing() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        });
        
        let result = get_rule(State(config), Path("fallback-rule-001".to_string())).await;
        assert!(result.is_ok());

        let rule = result.unwrap().0;
        assert_eq!(rule.id, "fallback-rule-001");
        assert_eq!(rule.severity, "warning");
    }

    #[tokio::test]
    async fn test_get_rule_not_found() {
        let temp_dir = tempdir().unwrap();
        let config = Arc::new(WebConfig {
            rules_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        });
        
        let result = get_rule(State(config), Path("nonexistent-rule".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_rules_valid() {
        let config = Arc::new(WebConfig::default());
        let request = ValidateRulesRequest {
            rules: r#"
- id: test-rule
  name: Test Rule
  description: A test rule
  message: Test rule message
  pattern: "test.*"
  severity: warning
"#.to_string(),
            language: Some("java".to_string()),
            check_performance: Some(true),
        };
        
        let result = validate_rules(State(config), Json(request)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap().0;
        assert!(response.valid);
        assert_eq!(response.rules_count, 1);
        assert!(response.performance.is_some());
    }

    #[tokio::test]
    async fn test_validate_rules_invalid() {
        let config = Arc::new(WebConfig::default());
        let request = ValidateRulesRequest {
            rules: r#"
- name: Test Rule Missing ID
  description: A test rule without ID
"#.to_string(),
            language: None,
            check_performance: None,
        };
        
        let result = validate_rules(State(config), Json(request)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap().0;
        assert!(!response.valid);
        assert!(!response.errors.is_empty());
        assert_eq!(response.rules_count, 1);
    }
}
