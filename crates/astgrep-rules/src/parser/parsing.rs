//! YAML rule parsing
//!
//! This module provides functionality to parse rules from YAML format.

use crate::types::*;
use astgrep_core::{AnalysisError, Confidence, Language, Result, Severity, ComparisonOperator};
use astgrep_core::{MetavariableAnalysis, EntropyAnalysis, TypeAnalysis, ComplexityAnalysis};
use serde_yaml::Value;
use std::collections::HashMap;

/// YAML rule parser
pub struct RuleParser {
    strict_mode: bool,
}

impl RuleParser {
    /// Create a new rule parser
    pub fn new() -> Self {
        Self {
            strict_mode: false,
        }
    }

    /// Create a parser in strict mode (fails on unknown fields)
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
        }
    }

    /// Parse rules from YAML content
    pub fn parse_yaml(&self, yaml_content: &str) -> Result<Vec<Rule>> {
        let yaml_value: Value = serde_yaml::from_str(yaml_content)
            .map_err(|e| AnalysisError::parse_error(format!("YAML syntax error: {}", e)))?;

        self.parse_rules_from_value(&yaml_value)
    }

    /// Parse rules from a YAML value
    fn parse_rules_from_value(&self, value: &Value) -> Result<Vec<Rule>> {
        let rules_array = value
            .get("rules")
            .ok_or_else(|| AnalysisError::parse_error("Missing 'rules' key in YAML"))?
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error("'rules' must be an array"))?;

        let mut rules = Vec::new();
        for (index, rule_value) in rules_array.iter().enumerate() {
            match self.parse_single_rule(rule_value, index) {
                Ok(rule) => rules.push(rule),
                Err(e) => {
                    if self.strict_mode {
                        return Err(e);
                    } else {
                        eprintln!("Warning: Skipping rule {}: {}", index, e);
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Parse a single rule from YAML value
    fn parse_single_rule(&self, value: &Value, index: usize) -> Result<Rule> {
        let rule_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} is not an object", index)))?;

        // Parse required fields
        let id = self.get_string_field(rule_obj, "id", index)?;
        let severity = self.parse_severity(rule_obj, index)?;
        let languages = self.parse_languages(rule_obj, index)?;

        // Parse message (required in semgrep format)
        let message = self.get_string_field(rule_obj, "message", index)?;

        // Use message as both name and description for semgrep compatibility
        let name = self.get_optional_string_field(rule_obj, "name").unwrap_or_else(|| id.clone());
        let description = self.get_optional_string_field(rule_obj, "description").unwrap_or_else(|| message.clone());

        // Parse optional fields
        let confidence = self.parse_confidence(rule_obj, index).unwrap_or(Confidence::Medium);
        let mode = self.parse_mode(rule_obj, index)?;
        
        // For taint mode, parse pattern-sources and pattern-sinks
        let (patterns, dataflow) = if mode == RuleMode::Taint {
            let sources = self.parse_pattern_sources(rule_obj, index)?;
            let sinks = self.parse_pattern_sinks(rule_obj, index)?;
            let sanitizers = self.parse_pattern_sanitizers(rule_obj, index).unwrap_or_default();
            let propagators = self.parse_pattern_propagators(rule_obj, index).unwrap_or_default();

            if !sources.is_empty() && !sinks.is_empty() {
                let mut dataflow = DataFlowSpec::new(sources, sinks)
                    .with_sanitizers(sanitizers);
                dataflow.propagators = propagators;
                
                // Parse taint options from the options field
                if let Some(options_obj) = rule_obj.get(&Value::String("options".to_string())).and_then(|v| v.as_mapping()) {
                    if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_booleans".to_string())) {
                        if let Some(b) = val.as_bool() {
                            dataflow.taint_assume_safe_booleans = Some(b);
                        }
                    }
                    if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_numbers".to_string())) {
                        if let Some(b) = val.as_bool() {
                            dataflow.taint_assume_safe_numbers = Some(b);
                        }
                    }
                    if let Some(val) = options_obj.get(&Value::String("taint_only_propagate_through_assignments".to_string())) {
                        if let Some(b) = val.as_bool() {
                            dataflow.taint_only_propagate_through_assignments = Some(b);
                        }
                    }
                }
                
                (Vec::new(), Some(dataflow))
            } else {
                (Vec::new(), None)
            }
        } else {
            let patterns = self.parse_patterns_or_pattern(rule_obj, index)?;
            let dataflow = self.parse_dataflow(rule_obj, index)?;
            (patterns, dataflow)
        };
        
        let fix = self.get_optional_string_field(rule_obj, "fix");
        let fix_regex = self.parse_fix_regex(rule_obj, index)?;
        let paths = self.parse_paths(rule_obj, index)?;
        let mut metadata = self.parse_metadata(rule_obj, index)?;
        // Parse optional options block and merge into metadata (as YAML values)
        if let Some(opts) = self.parse_options(rule_obj, index)? {
            for (k, v) in opts { metadata.insert(k, Value::String(v)); }
        }
        let enabled = self.get_optional_bool_field(rule_obj, "enabled").unwrap_or(true);

        let mut rule = Rule::new(id, name, description, severity, confidence, languages);
        rule.patterns = patterns;
        rule.dataflow = dataflow;
        rule.fix = fix;
        rule.fix_regex = fix_regex;
        rule.paths = paths;
        rule.metadata = metadata;
        rule.enabled = enabled;
        rule.mode = mode;

        Ok(rule)
    }

    /// Parse optional options block; currently recognizes sql_statement_boundary and symbolic_propagation
    fn parse_options(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Option<HashMap<String, String>>> {
        let options_value = obj.get(&Value::String("options".to_string()));
        if options_value.is_none() { return Ok(None); }
        let options_obj = options_value
            .unwrap()
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error("'options' must be an object".to_string()))?;
        let mut options = HashMap::new();
        
        // Parse sql_statement_boundary option
        if let Some(val) = options_obj.get(&Value::String("sql_statement_boundary".to_string())) {
            // Accept boolean or string "on"/"off" and stringify to "true"/"false"
            let str_val = if let Some(b) = val.as_bool() {
                b.to_string()
            } else if let Some(s) = val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" => "true".to_string(),
                    "off" | "false" | "0" | "no" => "false".to_string(),
                    _ => s.to_string(),
                }
            } else {
                // Unsupported type: ignore this option instead of forcing a string
                // so only boolean or string values are accepted
                return Ok(Some(options));
            };
            options.insert("sql_statement_boundary".to_string(), str_val);
        }

        // Parse symbolic_propagation option
        if let Some(val) = options_obj.get(&Value::String("symbolic_propagation".to_string())) {
            let str_val = if let Some(b) = val.as_bool() {
                b.to_string()
            } else if let Some(s) = val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" => "true".to_string(),
                    "off" | "false" | "0" | "no" => "false".to_string(),
                    _ => s.to_string(),
                }
            } else {
                return Ok(Some(options));
            };
            options.insert("symbolic_propagation".to_string(), str_val);
        }

        // Parse taint_assume_safe_booleans option
        if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_booleans".to_string())) {
            let str_val = if let Some(b) = val.as_bool() {
                b.to_string()
            } else if let Some(s) = val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" => "true".to_string(),
                    "off" | "false" | "0" | "no" => "false".to_string(),
                    _ => s.to_string(),
                }
            } else {
                return Ok(Some(options));
            };
            options.insert("taint_assume_safe_booleans".to_string(), str_val);
        }

        // Parse taint_assume_safe_numbers option
        if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_numbers".to_string())) {
            let str_val = if let Some(b) = val.as_bool() {
                b.to_string()
            } else if let Some(s) = val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" => "true".to_string(),
                    "off" | "false" | "0" | "no" => "false".to_string(),
                    _ => s.to_string(),
                }
            } else {
                return Ok(Some(options));
            };
            options.insert("taint_assume_safe_numbers".to_string(), str_val);
        }

        // Parse taint_only_propagate_through_assignments option
        if let Some(val) = options_obj.get(&Value::String("taint_only_propagate_through_assignments".to_string())) {
            let str_val = if let Some(b) = val.as_bool() {
                b.to_string()
            } else if let Some(s) = val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "on" | "true" | "1" | "yes" => "true".to_string(),
                    "off" | "false" | "0" | "no" => "false".to_string(),
                    _ => s.to_string(),
                }
            } else {
                return Ok(Some(options));
            };
            options.insert("taint_only_propagate_through_assignments".to_string(), str_val);
        }

        Ok(Some(options))
    }

    /// Get a required string field
    fn get_string_field(&self, obj: &serde_yaml::Mapping, field: &str, index: usize) -> Result<String> {
        obj.get(&Value::String(field.to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} missing required field: {}", index, field)))
    }

    /// Get an optional string field
    fn get_optional_string_field(&self, obj: &serde_yaml::Mapping, field: &str) -> Option<String> {
        obj.get(&Value::String(field.to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get an optional boolean field
    fn get_optional_bool_field(&self, obj: &serde_yaml::Mapping, field: &str) -> Option<bool> {
        obj.get(&Value::String(field.to_string()))
            .and_then(|v| v.as_bool())
    }

    /// Parse severity field
    fn parse_severity(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Severity> {
        let severity_str = self.get_string_field(obj, "severity", index)?;
        match severity_str.to_uppercase().as_str() {
            "INFO" => Ok(Severity::Info),
            "WARNING" => Ok(Severity::Warning),
            "ERROR" => Ok(Severity::Error),
            "CRITICAL" => Ok(Severity::Critical),
            _ => Err(AnalysisError::parse_error(format!(
                "Rule {} has invalid severity: {}",
                index, severity_str
            ))),
        }
    }

    /// Parse confidence field
    fn parse_confidence(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Confidence> {
        let confidence_str = self.get_optional_string_field(obj, "confidence")
            .unwrap_or_else(|| "MEDIUM".to_string());
        
        match confidence_str.to_uppercase().as_str() {
            "LOW" => Ok(Confidence::Low),
            "MEDIUM" => Ok(Confidence::Medium),
            "HIGH" => Ok(Confidence::High),
            _ => Err(AnalysisError::parse_error(format!(
                "Rule {} has invalid confidence: {}",
                index, confidence_str
            ))),
        }
    }

    /// Parse languages field
    fn parse_languages(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Vec<Language>> {
        let languages_value = obj
            .get(&Value::String("languages".to_string()))
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} missing 'languages' field", index)))?;

        let languages_array = languages_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'languages' must be an array", index)))?;

        let mut languages = Vec::new();
        for lang_value in languages_array {
            let lang_str = lang_value
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} language must be a string", index)))?;
            
            let language = Language::from_str(lang_str)
                .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} unknown language: {}", index, lang_str)))?;
            
            languages.push(language);
        }

        if languages.is_empty() {
            return Err(AnalysisError::parse_error(format!("Rule {} must specify at least one language", index)));
        }

        Ok(languages)
    }

    /// Parse patterns field or single pattern field (semgrep compatibility)
    fn parse_patterns_or_pattern(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Vec<Pattern>> {
        // Check for 'patterns' field first
        if let Some(patterns_value) = obj.get(&Value::String("patterns".to_string())) {
            return self.parse_patterns_array(patterns_value, index);
        }

        // Check for single 'pattern' field
        if let Some(pattern_value) = obj.get(&Value::String("pattern".to_string())) {
            let pattern = self.parse_single_pattern(pattern_value, index, 0)?;
            return Ok(vec![pattern]);
        }

        // Check for 'pattern-either' field
        if let Some(pattern_either_value) = obj.get(&Value::String("pattern-either".to_string())) {
            return self.parse_pattern_either(pattern_either_value, index);
        }

        // Check for 'pattern-inside' field
        if let Some(pattern_inside_value) = obj.get(&Value::String("pattern-inside".to_string())) {
            let pattern = self.parse_single_pattern(pattern_inside_value, index, 0)?;
            return Ok(vec![pattern]);
        }

        // No patterns found
        Ok(Vec::new())
    }

    /// Parse patterns array
    /// In Semgrep, items in `patterns` are combined with AND logic
    fn parse_patterns_array(&self, patterns_value: &Value, index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = patterns_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'patterns' must be an array", index)))?;

        // Collect all components
        let mut positive_patterns: Vec<Pattern> = Vec::new();
        let mut negative_patterns: Vec<Pattern> = Vec::new();
        let mut conditions: Vec<Condition> = Vec::new();

        for (pattern_index, pattern_value) in patterns_array.iter().enumerate() {
            // Check if this is a metavariable-comparison (not a pattern, but a condition)
            if let Some(mapping) = pattern_value.as_mapping() {
                if mapping.contains_key(&Value::String("metavariable-comparison".to_string())) {
                    if let Some(metavar_comp_value) = mapping.get(&Value::String("metavariable-comparison".to_string())) {
                        let metavar_comp = self.parse_metavariable_comparison(metavar_comp_value, index, pattern_index)?;
                        conditions.push(Condition::MetavariableComparison(metavar_comp));
                    }
                    continue;
                }
                
                // Check if this is a semgrep-internal-metavariable-name (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("semgrep-internal-metavariable-name".to_string())) {
                    if let Some(metavar_name_value) = mapping.get(&Value::String("semgrep-internal-metavariable-name".to_string())) {
                        let metavar_name = self.parse_internal_metavariable_name(metavar_name_value, index, pattern_index)?;
                        conditions.push(Condition::MetavariableName(metavar_name));
                    }
                    continue;
                }

                // Check if this is a metavariable-type (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-type".to_string())) {
                    if let Some(metavar_type_value) = mapping.get(&Value::String("metavariable-type".to_string())) {
                        let metavar_type = self.parse_metavariable_type(metavar_type_value, index, pattern_index)?;
                        conditions.push(Condition::MetavariableType(metavar_type));
                    }
                    continue;
                }

                // Check if this is a metavariable-regex (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-regex".to_string())) {
                    if let Some(metavar_regex_value) = mapping.get(&Value::String("metavariable-regex".to_string())) {
                        let metavar_regex = self.parse_metavariable_regex(metavar_regex_value, index, pattern_index)?;
                        conditions.push(Condition::MetavariableRegex(metavar_regex));
                    }
                    continue;
                }
            }

            let pattern = self.parse_single_pattern(pattern_value, index, pattern_index)?;
            
            // Separate positive and negative patterns
            match &pattern.pattern_type {
                PatternType::Not(_) | PatternType::NotRegex(_) | PatternType::NotInside(_) => {
                    negative_patterns.push(pattern);
                }
                _ => {
                    positive_patterns.push(pattern);
                }
            }
        }

        // Combine all components into a single Pattern::All (AND logic)
        if positive_patterns.is_empty() && negative_patterns.is_empty() && conditions.is_empty() {
            return Ok(Vec::new());
        }

        // Build the combined pattern
        let mut all_components: Vec<Pattern> = Vec::new();
        
        // Add positive patterns
        all_components.extend(positive_patterns);
        
        // Add negative patterns
        all_components.extend(negative_patterns);

        // Create the main pattern with all conditions
        let mut main_pattern = if all_components.len() == 1 {
            all_components.into_iter().next().unwrap()
        } else {
            Pattern::all(all_components)
        };
        
        // Add conditions to the main pattern
        main_pattern.conditions.extend(conditions);

        Ok(vec![main_pattern])
    }
    
    /// Parse metavariable comparison
    fn parse_metavariable_comparison(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableComparison> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-comparison must be an object",
                rule_index, pattern_index
            )))?;
        
        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let comparison = self.get_string_field(metavar_obj, "comparison", rule_index)?;
        
        // Remove $ prefix from metavariable name if present (bindings don't include $)
        let metavariable = if metavariable.starts_with('$') {
            metavariable[1..].to_string()
        } else {
            metavariable
        };
        
        // Parse the comparison expression and create appropriate operator
        // For now, store the full expression as a PythonExpression
        let operator = ComparisonOperator::PythonExpression(comparison);
        
        // The value field is not used when we have a PythonExpression, but we need to provide something
        let value = String::new();
        
        Ok(MetavariableComparison::new(metavariable, operator, value))
    }

    /// Parse metavariable type constraint
    fn parse_metavariable_type(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableType> {
        let type_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-type must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(type_obj, "metavariable", rule_index)?;
        let var_type = self.get_string_field(type_obj, "type", rule_index)?;

        // Remove $ prefix from metavariable name if present
        let metavariable = if metavariable.starts_with('$') {
            metavariable[1..].to_string()
        } else {
            metavariable
        };

        Ok(MetavariableType::new(metavariable, var_type))
    }

    /// Parse pattern-either (OR logic)
    fn parse_pattern_either(&self, pattern_either_value: &Value, index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = pattern_either_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'pattern-either' must be an array", index)))?;

        let mut sub_patterns = Vec::new();
        for (pattern_index, pattern_value) in patterns_array.iter().enumerate() {
            let pattern = self.parse_single_pattern(pattern_value, index, pattern_index)?;
            sub_patterns.push(pattern);
        }

        // Return a single pattern with Either type
        Ok(vec![Pattern::either(sub_patterns)])
    }

    /// Parse a single pattern
    fn parse_single_pattern(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<Pattern> {
        if let Some(pattern_str) = value.as_str() {
            // Simple string pattern
            return Ok(Pattern::simple(pattern_str.to_string()));
        }

        let pattern_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} must be a string or object",
                rule_index, pattern_index
            )))?;

        // Parse different pattern types
        let mut pattern = if let Some(pattern_str) = self.get_optional_string_field(pattern_obj, "pattern") {
            Pattern::simple(pattern_str)
        } else if let Some(pattern_inside) = self.get_optional_string_field(pattern_obj, "pattern-inside") {
            Pattern::inside(Pattern::simple(pattern_inside))
        } else if let Some(pattern_not_inside) = self.get_optional_string_field(pattern_obj, "pattern-not-inside") {
            Pattern::not_inside(Pattern::simple(pattern_not_inside))
        } else if let Some(pattern_not) = self.get_optional_string_field(pattern_obj, "pattern-not") {
            Pattern::not(Pattern::simple(pattern_not))
        } else if let Some(pattern_regex) = self.get_optional_string_field(pattern_obj, "pattern-regex") {
            Pattern::regex(pattern_regex)
        } else if let Some(pattern_not_regex) = self.get_optional_string_field(pattern_obj, "pattern-not-regex") {
            Pattern::not_regex(pattern_not_regex)
        } else if let Some(pattern_either_value) = pattern_obj.get(&Value::String("pattern-either".to_string())) {
            // Handle nested pattern-either
            let either_patterns = self.parse_pattern_either(pattern_either_value, rule_index)?;
            if either_patterns.len() == 1 {
                either_patterns.into_iter().next().unwrap()
            } else {
                Pattern::either(either_patterns)
            }
        } else if let Some(pattern_all_value) = pattern_obj.get(&Value::String("pattern-all".to_string())) {
            // Handle pattern-all
            let all_patterns = self.parse_pattern_all(pattern_all_value, rule_index)?;
            if all_patterns.len() == 1 {
                all_patterns.into_iter().next().unwrap()
            } else {
                Pattern::all(all_patterns)
            }
        } else if let Some(pattern_any_value) = pattern_obj.get(&Value::String("pattern-any".to_string())) {
            // Handle pattern-any
            let any_patterns = self.parse_pattern_any(pattern_any_value, rule_index)?;
            if any_patterns.len() == 1 {
                any_patterns.into_iter().next().unwrap()
            } else {
                Pattern::any(any_patterns)
            }
        } else if let Some(patterns_value) = pattern_obj.get(&Value::String("patterns".to_string())) {
            // Handle nested patterns (AND logic)
            let patterns = self.parse_patterns_array(patterns_value, rule_index)?;
            if patterns.len() == 1 {
                patterns.into_iter().next().unwrap()
            } else {
                Pattern::all(patterns)
            }
        } else if let Some(metavar_value) = pattern_obj.get(&Value::String("metavariable-pattern".to_string())) {
            // Handle standalone metavariable-pattern (no main pattern, just the constraint)
            // This creates a pattern that matches anything but applies the metavariable-pattern constraint
            let mut pattern = Pattern::simple("...".to_string());
            let metavar_pattern = self.parse_metavariable_pattern(metavar_value, rule_index, pattern_index)?;
            pattern.metavariable_pattern = Some(metavar_pattern);
            pattern
        } else {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} pattern {} must have a pattern field",
                rule_index, pattern_index
            )));
        };

        // Parse optional metavariable pattern
        if let Some(metavar_value) = pattern_obj.get(&Value::String("metavariable-pattern".to_string())) {
            let metavar_pattern = self.parse_metavariable_pattern(metavar_value, rule_index, pattern_index)?;
            pattern.metavariable_pattern = Some(metavar_pattern);
        }

        // Parse optional metavariable regex
        if let Some(metavar_regex_value) = pattern_obj.get(&Value::String("metavariable-regex".to_string())) {
            let metavar_regex = self.parse_metavariable_regex(metavar_regex_value, rule_index, pattern_index)?;
            pattern.conditions.push(Condition::MetavariableRegex(metavar_regex));
        }

        // Parse optional metavariable-name
        if let Some(metavar_name_value) = pattern_obj.get(&Value::String("metavariable-name".to_string())) {
            let metavar_name = self.parse_metavariable_name(metavar_name_value, rule_index, pattern_index)?;
            pattern.conditions.push(Condition::MetavariableName(metavar_name));
        }

        // Parse optional metavariable-analysis
        if let Some(metavar_analysis_value) = pattern_obj.get(&Value::String("metavariable-analysis".to_string())) {
            let metavar_analysis = self.parse_metavariable_analysis(metavar_analysis_value, rule_index, pattern_index)?;
            pattern.conditions.push(Condition::MetavariableAnalysis(metavar_analysis));
        }

        // Parse optional focus (single metavariable)
        if let Some(focus) = self.get_optional_string_field(pattern_obj, "focus") {
            pattern.focus = Some(vec![focus]);
        }

        // Parse optional focus-metavariable (single or array)
        if let Some(focus_metavar_value) = pattern_obj.get(&Value::String("focus-metavariable".to_string())) {
            if let Some(focus_str) = focus_metavar_value.as_str() {
                // Single focus metavariable
                pattern.focus = Some(vec![focus_str.to_string()]);
            } else if let Some(focus_array) = focus_metavar_value.as_sequence() {
                // Array of focus metavariables
                let mut focus_vars = Vec::new();
                for focus_value in focus_array {
                    if let Some(focus_str) = focus_value.as_str() {
                        focus_vars.push(focus_str.to_string());
                    }
                }
                if !focus_vars.is_empty() {
                    pattern.focus = Some(focus_vars);
                }
            }
        }

        Ok(pattern)
    }

    /// Parse metavariable pattern
    fn parse_metavariable_pattern(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariablePattern> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        
        // Support multiple ways to specify patterns:
        // 1. `patterns` - array of pattern strings
        // 2. `pattern` - single pattern string
        // 3. `pattern-either` - array of pattern objects with alternatives
        let mut patterns = Vec::new();
        
        // Check for `patterns` field (array of strings)
        if let Some(patterns_value) = metavar_obj.get(&Value::String("patterns".to_string())) {
            let patterns_array = patterns_value
                .as_sequence()
                .ok_or_else(|| AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'patterns' must be an array",
                    rule_index, pattern_index
                )))?;

            for pattern_value in patterns_array {
                let pattern_str = pattern_value
                    .as_str()
                    .ok_or_else(|| AnalysisError::parse_error(format!(
                        "Rule {} pattern {} metavariable pattern must be a string",
                        rule_index, pattern_index
                    )))?;
                patterns.push(pattern_str.to_string());
            }
        }
        // Check for `pattern` field (single string)
        else if let Some(pattern_value) = metavar_obj.get(&Value::String("pattern".to_string())) {
            let pattern_str = pattern_value
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'pattern' must be a string",
                    rule_index, pattern_index
                )))?;
            patterns.push(pattern_str.to_string());
        }
        // Check for `pattern-either` field (array of pattern objects)
        else if let Some(pattern_either_value) = metavar_obj.get(&Value::String("pattern-either".to_string())) {
            let either_array = pattern_either_value
                .as_sequence()
                .ok_or_else(|| AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'pattern-either' must be an array",
                    rule_index, pattern_index
                )))?;
            
            for pattern_obj in either_array {
                // Each element should be an object with a `pattern` field
                if let Some(obj) = pattern_obj.as_mapping() {
                    if let Some(pattern_value) = obj.get(&Value::String("pattern".to_string())) {
                        if let Some(pattern_str) = pattern_value.as_str() {
                            patterns.push(pattern_str.to_string());
                        }
                    }
                }
            }
        }
        else {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern must have 'patterns', 'pattern', or 'pattern-either' field",
                rule_index, pattern_index
            )));
        }

        let mut metavar_pattern = MetavariablePattern::with_patterns(metavariable, patterns);

        // Parse optional regex
        if let Some(regex) = self.get_optional_string_field(metavar_obj, "regex") {
            metavar_pattern.regex = Some(regex);
        }

        // Parse optional type constraint
        if let Some(type_constraint) = self.get_optional_string_field(metavar_obj, "type") {
            metavar_pattern.type_constraint = Some(type_constraint);
        }

        // Parse optional name constraint
        if let Some(name_constraint) = self.get_optional_string_field(metavar_obj, "name") {
            metavar_pattern.name_constraint = Some(name_constraint);
        }

        // Parse optional analysis
        if let Some(analysis_value) = metavar_obj.get(&Value::String("analysis".to_string())) {
            let analysis = self.parse_metavariable_analysis_config(analysis_value, rule_index, pattern_index)?;
            metavar_pattern.analysis = Some(analysis);
        }

        Ok(metavar_pattern)
    }

    /// Parse metavariable regex
    fn parse_metavariable_regex(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableRegex> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-regex must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let regex = self.get_string_field(metavar_obj, "regex", rule_index)?;

        Ok(MetavariableRegex::new(metavariable, regex))
    }

    /// Parse metavariable name constraint
    fn parse_metavariable_name(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableName> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-name must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let name_pattern = self.get_string_field(metavar_obj, "name", rule_index)?;

        Ok(MetavariableName::new(metavariable, name_pattern))
    }

    /// Parse semgrep-internal-metavariable-name constraint
    fn parse_internal_metavariable_name(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableName> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} semgrep-internal-metavariable-name must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let fqn = self.get_string_field(metavar_obj, "fqn", rule_index)?;

        Ok(MetavariableName::with_fqn(metavariable, fqn))
    }

    /// Parse metavariable analysis
    fn parse_metavariable_analysis(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableAnalysisCondition> {
        let metavar_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-analysis must be an object",
                rule_index, pattern_index
            )))?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let analysis = self.parse_metavariable_analysis_config(value, rule_index, pattern_index)?;

        Ok(MetavariableAnalysisCondition::new(metavariable, analysis))
    }

    /// Parse metavariable analysis configuration
    fn parse_metavariable_analysis_config(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<MetavariableAnalysis> {
        let analysis_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable analysis must be an object",
                rule_index, pattern_index
            )))?;

        let mut analysis = MetavariableAnalysis {
            entropy: None,
            type_analysis: None,
            complexity: None,
        };

        // Parse entropy analysis
        if let Some(entropy_value) = analysis_obj.get(&Value::String("entropy".to_string())) {
            analysis.entropy = Some(self.parse_entropy_analysis(entropy_value, rule_index, pattern_index)?);
        }

        // Parse type analysis
        if let Some(type_value) = analysis_obj.get(&Value::String("type".to_string())) {
            analysis.type_analysis = Some(self.parse_type_analysis(type_value, rule_index, pattern_index)?);
        }

        // Parse complexity analysis
        if let Some(complexity_value) = analysis_obj.get(&Value::String("complexity".to_string())) {
            analysis.complexity = Some(self.parse_complexity_analysis(complexity_value, rule_index, pattern_index)?);
        }

        Ok(analysis)
    }

    /// Parse entropy analysis
    fn parse_entropy_analysis(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<EntropyAnalysis> {
        let entropy_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} entropy analysis must be an object",
                rule_index, pattern_index
            )))?;

        let min_entropy = entropy_obj
            .get(&Value::String("min".to_string()))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let max_entropy = entropy_obj
            .get(&Value::String("max".to_string()))
            .and_then(|v| v.as_f64());

        let charset = self.get_optional_string_field(entropy_obj, "charset");

        Ok(EntropyAnalysis {
            min_entropy,
            max_entropy,
            charset,
        })
    }

    /// Parse type analysis
    fn parse_type_analysis(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<TypeAnalysis> {
        let type_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} type analysis must be an object",
                rule_index, pattern_index
            )))?;

        let expected_types = self.parse_string_array(type_obj, "expected")?;
        let forbidden_types = self.parse_string_array(type_obj, "forbidden")?;
        let nullable = type_obj
            .get(&Value::String("nullable".to_string()))
            .and_then(|v| v.as_bool());

        Ok(TypeAnalysis {
            expected_types,
            forbidden_types,
            nullable,
        })
    }

    /// Parse complexity analysis
    fn parse_complexity_analysis(&self, value: &Value, rule_index: usize, pattern_index: usize) -> Result<ComplexityAnalysis> {
        let complexity_obj = value
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} complexity analysis must be an object",
                rule_index, pattern_index
            )))?;

        let max_cyclomatic = complexity_obj
            .get(&Value::String("max_cyclomatic".to_string()))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let max_nesting_depth = complexity_obj
            .get(&Value::String("max_nesting_depth".to_string()))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let max_lines = complexity_obj
            .get(&Value::String("max_lines".to_string()))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        Ok(ComplexityAnalysis {
            max_cyclomatic,
            max_nesting_depth,
            max_lines,
        })
    }

    /// Parse dataflow field
    fn parse_dataflow(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Option<DataFlowSpec>> {
        let dataflow_value = obj.get(&Value::String("dataflow".to_string()));
        
        if dataflow_value.is_none() {
            return Ok(None);
        }

        let dataflow_obj = dataflow_value
            .unwrap()
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error("'dataflow' must be an object".to_string()))?;

        let sources = self.parse_string_array(dataflow_obj, "sources")?;
        let sinks = self.parse_string_array(dataflow_obj, "sinks")?;
        let sanitizers = self.parse_string_array(dataflow_obj, "sanitizers").unwrap_or_default();

        let mut dataflow = DataFlowSpec::from_strings(sources, sinks).with_sanitizers(sanitizers);

        if let Some(must_flow) = self.get_optional_bool_field(dataflow_obj, "must_flow") {
            dataflow.must_flow = must_flow;
        }

        if let Some(max_depth_value) = dataflow_obj.get(&Value::String("max_depth".to_string())) {
            if let Some(max_depth) = max_depth_value.as_u64() {
                dataflow.max_depth = Some(max_depth as usize);
            }
        }

        if let Some(taint_assume_safe_booleans_value) = dataflow_obj.get(&Value::String("taint_assume_safe_booleans".to_string())) {
            if let Some(b) = taint_assume_safe_booleans_value.as_bool() {
                dataflow.taint_assume_safe_booleans = Some(b);
            }
        }

        if let Some(taint_assume_safe_numbers_value) = dataflow_obj.get(&Value::String("taint_assume_safe_numbers".to_string())) {
            if let Some(b) = taint_assume_safe_numbers_value.as_bool() {
                dataflow.taint_assume_safe_numbers = Some(b);
            }
        }

        if let Some(taint_only_propagate_through_assignments_value) = dataflow_obj.get(&Value::String("taint_only_propagate_through_assignments".to_string())) {
            if let Some(b) = taint_only_propagate_through_assignments_value.as_bool() {
                dataflow.taint_only_propagate_through_assignments = Some(b);
            }
        }

        Ok(Some(dataflow))
    }

    /// Parse string array field
    fn parse_string_array(&self, obj: &serde_yaml::Mapping, field: &str) -> Result<Vec<String>> {
        let array_value = obj
            .get(&Value::String(field.to_string()))
            .ok_or_else(|| AnalysisError::parse_error(format!("Missing '{}' field", field)))?;

        let array = array_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("'{}' must be an array", field)))?;

        let mut result = Vec::new();
        for item in array {
            let item_str = item
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error(format!("'{}' items must be strings", field)))?;
            result.push(item_str.to_string());
        }

        Ok(result)
    }

    /// Parse pattern-all
    fn parse_pattern_all(&self, value: &Value, rule_index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern-all must be an array",
                rule_index
            )))?;

        let mut patterns = Vec::new();
        for (index, pattern_value) in patterns_array.iter().enumerate() {
            patterns.push(self.parse_single_pattern(pattern_value, rule_index, index)?);
        }

        Ok(patterns)
    }

    /// Parse pattern-any
    fn parse_pattern_any(&self, value: &Value, rule_index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern-any must be an array",
                rule_index
            )))?;

        let mut patterns = Vec::new();
        for (index, pattern_value) in patterns_array.iter().enumerate() {
            patterns.push(self.parse_single_pattern(pattern_value, rule_index, index)?);
        }

        Ok(patterns)
    }

    /// Parse fix-regex field
    fn parse_fix_regex(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Option<FixRegex>> {
        let fix_regex_value = obj.get(&Value::String("fix-regex".to_string()));

        if fix_regex_value.is_none() {
            return Ok(None);
        }

        let fix_regex_obj = fix_regex_value
            .unwrap()
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error("'fix-regex' must be an object".to_string()))?;

        let regex = self.get_string_field(fix_regex_obj, "regex", 0)?;
        let replacement = self.get_string_field(fix_regex_obj, "replacement", 0)?;

        Ok(Some(FixRegex { regex, replacement }))
    }

    /// Parse paths field
    fn parse_paths(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Option<PathsFilter>> {
        let paths_value = obj.get(&Value::String("paths".to_string()));

        if paths_value.is_none() {
            return Ok(None);
        }

        let paths_obj = paths_value
            .unwrap()
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error("'paths' must be an object".to_string()))?;

        let includes = self.parse_optional_string_array(paths_obj, "include")?;
        let excludes = self.parse_optional_string_array(paths_obj, "exclude")?;

        Ok(Some(PathsFilter { includes, excludes }))
    }

    /// Parse optional string array
    fn parse_optional_string_array(&self, obj: &serde_yaml::Mapping, field: &str) -> Result<Vec<String>> {
        let array_value = obj.get(&Value::String(field.to_string()));

        if array_value.is_none() {
            return Ok(Vec::new());
        }

        let array = array_value
            .unwrap()
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("'{}' must be an array", field)))?;

        let mut result = Vec::new();
        for item in array {
            let item_str = item
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error(format!("'{}' items must be strings", field)))?;
            result.push(item_str.to_string());
        }

        Ok(result)
    }

    /// Parse metadata field
    fn parse_metadata(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<HashMap<String, Value>> {
        let metadata_value = obj.get(&Value::String("metadata".to_string()));
        
        if metadata_value.is_none() {
            return Ok(HashMap::new());
        }

        let metadata_obj = metadata_value
            .unwrap()
            .as_mapping()
            .ok_or_else(|| AnalysisError::parse_error("'metadata' must be an object".to_string()))?;

        let mut metadata = HashMap::new();
        for (key, value) in metadata_obj {
            let key_str = key
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error("metadata keys must be strings".to_string()))?;
            // Accept any YAML value type (string, array, object, etc.)
            metadata.insert(key_str.to_string(), value.clone());
        }

        Ok(metadata)
    }

    /// Parse mode field (search or taint)
    fn parse_mode(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<RuleMode> {
        let mode_value = obj.get(&Value::String("mode".to_string()));
        
        if let Some(value) = mode_value {
            if let Some(mode_str) = value.as_str() {
                match mode_str.to_lowercase().as_str() {
                    "taint" => Ok(RuleMode::Taint),
                    "search" => Ok(RuleMode::Search),
                    _ => Ok(RuleMode::Search), // Default to search for unknown modes
                }
            } else {
                Ok(RuleMode::Search)
            }
        } else {
            Ok(RuleMode::Search) // Default mode
        }
    }

    /// Parse pattern-sources field for taint analysis
    fn parse_pattern_sources(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Vec<SourcePattern>> {
        let sources_value = obj.get(&Value::String("pattern-sources".to_string()));
        
        if sources_value.is_none() {
            return Ok(Vec::new());
        }

        let sources_array = sources_value
            .unwrap()
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'pattern-sources' must be an array", index)))?;

        let mut sources = Vec::new();
        for (i, source) in sources_array.iter().enumerate() {
            // Try to parse as a SourcePattern object
            if let Ok(source_pattern) = self.parse_source_pattern(source, i) {
                sources.push(source_pattern);
            } else {
                // Fallback to simple pattern
                if let Some(pattern_str) = self.extract_pattern_from_taint_def(source) {
                    sources.push(SourcePattern {
                        pattern: Pattern::simple(pattern_str),
                        focus_metavariables: Vec::new(),
                        is_fallback: true,
                    });
                } else {
                    return Err(AnalysisError::parse_error(format!(
                        "Rule {} source at index {} must have a 'pattern' field",
                        index, i
                    )));
                }
            }
        }

        Ok(sources)
    }

    /// Parse pattern-sinks field for taint analysis
    fn parse_pattern_sinks(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Vec<SinkPattern>> {
        let sinks_value = obj.get(&Value::String("pattern-sinks".to_string()));
        
        if sinks_value.is_none() {
            return Ok(Vec::new());
        }

        let sinks_array = sinks_value
            .unwrap()
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'pattern-sinks' must be an array", index)))?;

        let mut sinks = Vec::new();
        for (i, sink) in sinks_array.iter().enumerate() {
            // Try to parse as a SinkPattern object
            if let Ok(sink_pattern) = self.parse_sink_pattern(sink, i) {
                sinks.push(sink_pattern);
            } else {
                // Fallback to simple pattern
                if let Some(pattern_str) = self.extract_pattern_from_taint_def(sink) {
                    sinks.push(SinkPattern {
                        pattern: Pattern::simple(pattern_str),
                        focus_metavariables: Vec::new(),
                        is_fallback: true,
                    });
                } else {
                    return Err(AnalysisError::parse_error(format!(
                        "Rule {} sink at index {} must have a 'pattern' field",
                        index, i
                    )));
                }
            }
        }

        Ok(sinks)
    }

    /// Parse pattern-sanitizers field for taint analysis
    fn parse_pattern_sanitizers(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Vec<String>> {
        let sanitizers_value = obj.get(&Value::String("pattern-sanitizers".to_string()));
        
        if sanitizers_value.is_none() {
            return Ok(Vec::new());
        }

        let sanitizers_array = sanitizers_value
            .unwrap()
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error("'pattern-sanitizers' must be an array".to_string()))?;

        let mut sanitizers = Vec::new();
        for sanitizer in sanitizers_array.iter() {
            // Extract pattern from sanitizer definition
            if let Some(pattern_str) = self.extract_pattern_from_taint_def(sanitizer) {
                sanitizers.push(pattern_str);
            }
        }

        Ok(sanitizers)
    }

    /// Parse pattern-propagators field for taint analysis
    fn parse_pattern_propagators(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<Vec<PropagatorPattern>> {
        use crate::types::PropagatorPattern;

        let propagators_value = obj.get(&Value::String("pattern-propagators".to_string()));

        if propagators_value.is_none() {
            return Ok(Vec::new());
        }

        let propagators_array = propagators_value
            .unwrap()
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error("'pattern-propagators' must be an array".to_string()))?;

        let mut propagators = Vec::new();
        for propagator in propagators_array.iter() {
            if let Some(mapping) = propagator.as_mapping() {
                // Extract pattern (for propagators, preserve original metavariables but remove type qualifiers)
                let pattern = if let Some(pattern_val) = mapping.get(&Value::String("pattern".to_string())) {
                    if let Some(s) = pattern_val.as_str() {
                        // For propagators, don't simplify metavariables - keep $X, $Y, etc.
                        // But do remove type qualifiers like "(Type $VAR)." -> "$VAR."
                        Pattern::simple(self.simplify_type_qualifiers(s))
                    } else {
                        continue;
                    }
                } else if let Some(patterns_val) = mapping.get(&Value::String("patterns".to_string())) {
                    // Handle patterns array
                    if let Some(arr) = patterns_val.as_sequence() {
                        if let Some(first) = arr.first() {
                            if let Some(mapping) = first.as_mapping() {
                                if let Some(pattern) = mapping.get(&Value::String("pattern".to_string())) {
                                    if let Some(s) = pattern.as_str() {
                                        Pattern::simple(self.simplify_type_qualifiers(s))
                                    } else {
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Extract from and to fields
                let from = mapping
                    .get(&Value::String("from".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let to = mapping
                    .get(&Value::String("to".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !from.is_empty() && !to.is_empty() {
                    propagators.push(PropagatorPattern {
                        pattern,
                        from,
                        to,
                        is_fallback: false,
                    });
                }
            }
        }

        Ok(propagators)
    }

    /// Extract pattern string from taint definition (source, sink, or sanitizer)
    fn extract_pattern_from_taint_def(&self, value: &Value) -> Option<String> {
        // If it's a simple string, return it
        if let Some(s) = value.as_str() {
            return Some(s.to_string());
        }

        // If it's an object, try to extract pattern
        if let Some(mapping) = value.as_mapping() {
            // Try "pattern" field first
            if let Some(pattern) = mapping.get(&Value::String("pattern".to_string())) {
                if let Some(s) = pattern.as_str() {
                    // Simplify the pattern by removing complex Semgrep syntax
                    return Some(self.simplify_semgrep_pattern(s));
                }
            }

            // Try "pattern-either" field
            if let Some(pattern_either) = mapping.get(&Value::String("pattern-either".to_string())) {
                if let Some(arr) = pattern_either.as_sequence() {
                    let patterns: Vec<String> = arr.iter()
                        .filter_map(|v| self.extract_pattern_from_taint_def(v))
                        .collect();
                    if !patterns.is_empty() {
                        return Some(patterns.join("|"));
                    }
                }
            }

            // Try "patterns" field (array of patterns) - extract just the pattern part
            if let Some(patterns) = mapping.get(&Value::String("patterns".to_string())) {
                if let Some(arr) = patterns.as_sequence() {
                    // For patterns array, look for the actual pattern (not pattern-inside)
                    for item in arr.iter() {
                        if let Some(item_map) = item.as_mapping() {
                            // Skip pattern-inside and other context patterns
                            if item_map.contains_key(&Value::String("pattern-inside".to_string())) {
                                continue;
                            }
                            if item_map.contains_key(&Value::String("metavariable-regex".to_string())) {
                                continue;
                            }
                        }
                        // Try to extract pattern from this item
                        if let Some(pattern_str) = self.extract_pattern_from_taint_def(item) {
                            return Some(pattern_str);
                        }
                    }
                }
            }

            // Try "pattern-inside" field - simplify it
            if let Some(pattern_inside) = mapping.get(&Value::String("pattern-inside".to_string())) {
                if let Some(s) = pattern_inside.as_str() {
                    // Extract just the variable part from complex pattern-inside
                    let simplified = self.simplify_semgrep_pattern(s);
                    if !simplified.is_empty() {
                        return Some(simplified);
                    }
                }
            }
        }

        None
    }

    /// Extract pattern string from taint definition without simplifying metavariables
    /// This preserves original metavariable names like $SQL, $EM, etc.
    /// Also removes type qualifiers like "(Type $VAR)." to enable matching without type information.
    fn extract_pattern_raw(&self, value: &Value) -> Option<String> {
        // If it's a simple string, return it
        if let Some(s) = value.as_str() {
            return Some(self.simplify_type_qualifiers(s));
        }

        // If it's an object, try to extract pattern
        if let Some(mapping) = value.as_mapping() {
            // Try "pattern-inside" field - return as-is without simplification but remove type qualifiers
            if let Some(pattern_inside) = mapping.get(&Value::String("pattern-inside".to_string())) {
                if let Some(s) = pattern_inside.as_str() {
                    return Some(self.simplify_type_qualifiers(s));
                }
            }

            // Try "pattern" field - return as-is without simplification but remove type qualifiers
            if let Some(pattern) = mapping.get(&Value::String("pattern".to_string())) {
                if let Some(s) = pattern.as_str() {
                    return Some(self.simplify_type_qualifiers(s));
                }
            }
        }

        None
    }

    /// Simplify type qualifiers in patterns
    /// Converts "(Type $VAR).method(...)" to "$VAR.method(...)"
    /// This allows matching without type information
    fn simplify_type_qualifiers(&self, pattern: &str) -> String {
        use regex::Regex;
        
        let mut result = pattern.to_string();
        
        // Pattern to match "(Type $VAR)." and replace with "$VAR."
        // Matches: (typename $VAR). or (typename.sub $VAR).
        // Example: (javax.persistence.EntityManager $EM).createQuery -> $EM.createQuery
        let type_qualifier_regex = Regex::new(r"\([\w.]+\s+(\$\w+)\)\s*\.").unwrap();
        result = type_qualifier_regex.replace_all(&result, "$1.").to_string();
        
        result
    }

    /// Parse a source pattern from YAML value
    fn parse_source_pattern(&self, value: &Value, _index: usize) -> Result<SourcePattern> {
        // If it's a simple string, create a basic SourcePattern
        if let Some(s) = value.as_str() {
            return Ok(SourcePattern {
                pattern: Pattern::simple(s.to_string()),
                focus_metavariables: Vec::new(),
                is_fallback: false,
            });
        }

        // If it's an object, parse fields
        if let Some(mapping) = value.as_mapping() {
            // Extract pattern and focus-metavariables - check for "patterns" array (Semgrep format)
            let (pattern_str, focus_metavariables) = if let Some(patterns_value) = mapping.get(&Value::String("patterns".to_string())) {
                // Semgrep uses "patterns" array where:
                // - Elements with "pattern" field define the pattern to match
                // - Elements with "focus-metavariable" field specify which variable to track
                let patterns_array = patterns_value.as_sequence()
                    .ok_or_else(|| AnalysisError::parse_error("'patterns' must be an array".to_string()))?;

                if patterns_array.is_empty() {
                    return Err(AnalysisError::parse_error("'patterns' array must not be empty".to_string()));
                }

                // Extract pattern from first element with "pattern" field
                let mut pattern_str = None;
                let mut focus_vars = Vec::new();

                for pattern_elem in patterns_array {
                    if let Some(elem_map) = pattern_elem.as_mapping() {
                        // Look for pattern field
                        if let Some(p_val) = elem_map.get(&Value::String("pattern".to_string())) {
                            if let Some(p_str) = p_val.as_str() {
                                pattern_str = Some(p_str.to_string());
                            }
                        }
                        // Look for focus-metavariable field
                        if let Some(f_val) = elem_map.get(&Value::String("focus-metavariable".to_string())) {
                            if let Some(f_str) = f_val.as_str() {
                                focus_vars.push(f_str.to_string());
                            }
                        }
                    } else if let Some(p_str) = pattern_elem.as_str() {
                        // Simple string pattern
                        pattern_str = Some(p_str.to_string());
                    }
                }

                let pattern_str = pattern_str.ok_or_else(|| AnalysisError::parse_error("No pattern found in 'patterns' array".to_string()))?;
                (pattern_str, focus_vars)
            } else if let Some(pattern_value) = mapping.get(&Value::String("pattern".to_string())) {
                // Standard "pattern" field
                let pattern_str = pattern_value.as_str()
                    .ok_or_else(|| AnalysisError::parse_error("Source pattern must have a 'pattern' field".to_string()))?
                    .to_string();

                // Check for focus-metavariable at this level (alternate format)
                let focus_metavariables = mapping.get(&Value::String("focus-metavariable".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default();

                (pattern_str, focus_metavariables)
            } else {
                return Err(AnalysisError::parse_error("Source pattern must have 'pattern' or 'patterns' field".to_string()));
            };

            // Check if fallback flag is set (optional)
            let is_fallback = mapping.get(&Value::String("is_fallback".to_string()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            return Ok(SourcePattern {
                pattern: Pattern::simple(pattern_str),
                focus_metavariables,
                is_fallback,
            });
        }

        Err(AnalysisError::parse_error("Invalid source pattern format".to_string()))
    }

    /// Parse a sink pattern from YAML value
    fn parse_sink_pattern(&self, value: &Value, _index: usize) -> Result<SinkPattern> {
        // If it's a simple string, create a basic SinkPattern
        if let Some(s) = value.as_str() {
            return Ok(SinkPattern {
                pattern: Pattern::simple(s.to_string()),
                focus_metavariables: Vec::new(),
                is_fallback: false,
            });
        }

        // If it's an object, parse fields
        if let Some(mapping) = value.as_mapping() {
            // Check for "patterns" array (Semgrep format with focus-metavariable)
            let (pattern_str, focus_metavariables) = if let Some(patterns_value) = mapping.get(&Value::String("patterns".to_string())) {
                let patterns_array = patterns_value.as_sequence()
                    .ok_or_else(|| AnalysisError::parse_error("'patterns' must be an array".to_string()))?;

                if patterns_array.is_empty() {
                    return Err(AnalysisError::parse_error("'patterns' array must not be empty".to_string()));
                }

                // Extract pattern from first element with "pattern" or "pattern-either" field
                let mut pattern_str = None;
                let mut focus_vars = Vec::new();

                for pattern_elem in patterns_array {
                    if let Some(elem_map) = pattern_elem.as_mapping() {
                        // Look for focus-metavariable field
                        if let Some(f_val) = elem_map.get(&Value::String("focus-metavariable".to_string())) {
                            if let Some(f_str) = f_val.as_str() {
                                focus_vars.push(f_str.to_string());
                            }
                        }
                        // Look for pattern-either field - extract without simplifying to preserve metavariables
                        if pattern_str.is_none() {
                            if let Some(pattern_either) = elem_map.get(&Value::String("pattern-either".to_string())) {
                                if let Some(arr) = pattern_either.as_sequence() {
                                    let patterns: Vec<String> = arr.iter()
                                        .filter_map(|v| self.extract_pattern_raw(v))
                                        .collect();
                                    if !patterns.is_empty() {
                                        pattern_str = Some(patterns.join("|"));
                                    }
                                }
                            }
                        }
                    }
                }

                // If no pattern found in pattern-either, try to extract from patterns array directly
                if pattern_str.is_none() {
                    for pattern_elem in patterns_array {
                        if let Some(pattern) = self.extract_pattern_from_taint_def(pattern_elem) {
                            pattern_str = Some(pattern);
                            break;
                        }
                    }
                }

                let pattern_str = pattern_str.ok_or_else(|| AnalysisError::parse_error("No pattern found in sink 'patterns' array".to_string()))?;
                (pattern_str, focus_vars)
            } else if let Some(pattern_value) = mapping.get(&Value::String("pattern".to_string())) {
                // Standard "pattern" field
                let pattern_str = pattern_value.as_str()
                    .ok_or_else(|| AnalysisError::parse_error("Sink pattern must have a 'pattern' field".to_string()))?
                    .to_string();

                // Check for focus-metavariable at this level (alternate format)
                let focus_metavariables = mapping.get(&Value::String("focus-metavariable".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default();

                (pattern_str, focus_metavariables)
            } else {
                return Err(AnalysisError::parse_error("Sink pattern must have 'pattern' or 'patterns' field".to_string()));
            };

            // Check if fallback flag is set (optional)
            let is_fallback = mapping.get(&Value::String("is_fallback".to_string()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            return Ok(SinkPattern {
                pattern: Pattern::simple(pattern_str),
                focus_metavariables,
                is_fallback,
            });
        }

        Err(AnalysisError::parse_error("Invalid sink pattern format".to_string()))
    }

    /// Simplify Semgrep pattern to basic pattern matcher format
    fn simplify_semgrep_pattern(&self, pattern: &str) -> String {
        let mut result = pattern.to_string();
        
        // Replace complex metavariable patterns with simple wildcards
        // $METHODNAME(...) -> $VAR(...)
        // Use $$VAR to escape the $ in the replacement string (otherwise it's interpreted as capture group reference)
        result = regex::Regex::new(r"\$[A-Z][A-Z_0-9]*")
            .map(|re| re.replace_all(&result, "$$VAR").to_string())
            .unwrap_or(result);
        
        // Replace @ annotations
        result = regex::Regex::new(r"@\$[A-Z]+")
            .map(|re| re.replace_all(&result, "").to_string())
            .unwrap_or(result);
        
        // Simplify "..." to "*" or keep as is depending on context
        // For now, keep "..." as it might be supported
        
        // Clean up extra whitespace
        result = result.split_whitespace().collect::<Vec<_>>().join(" ");
        
        result
    }
}

impl Default for RuleParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
