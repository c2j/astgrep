//! YAML rule parsing
//!
//! This module provides functionality to parse rules from YAML format.

use crate::types::*;
use astgrep_core::{AnalysisError, ComparisonOperator, Confidence, Language, Result, Severity};
use astgrep_core::{ComplexityAnalysis, EntropyAnalysis, MetavariableAnalysis, TypeAnalysis};
use serde_yaml::Value;
use std::collections::HashMap;

/// YAML rule parser
pub struct RuleParser {
    strict_mode: bool,
}

impl RuleParser {
    /// Create a new rule parser
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    /// Create a parser in strict mode (fails on unknown fields)
    pub fn strict() -> Self {
        Self { strict_mode: true }
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
        let rule_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} is not an object", index))
        })?;

        // Parse required fields
        let id = self.get_string_field(rule_obj, "id", index)?;
        let severity = self.parse_severity(rule_obj, index)?;
        let languages = self.parse_languages(rule_obj, index)?;

        // Parse message (required in semgrep format)
        let message = self.get_string_field(rule_obj, "message", index)?;

        // Use message as both name and description for semgrep compatibility
        let name = self
            .get_optional_string_field(rule_obj, "name")
            .unwrap_or_else(|| id.clone());
        let description = self
            .get_optional_string_field(rule_obj, "description")
            .unwrap_or_else(|| message.clone());

        // Parse optional fields
        let confidence = self
            .parse_confidence(rule_obj, index)
            .unwrap_or(Confidence::Medium);
        let mode = self.parse_mode(rule_obj, index)?;

        // For taint mode, parse pattern-sources and pattern-sinks
        let (patterns, dataflow) = if mode == RuleMode::Taint {
            let sources = self.parse_pattern_sources(rule_obj, index)?;
            let sinks = self.parse_pattern_sinks(rule_obj, index)?;
            let sanitizers = self
                .parse_pattern_sanitizers(rule_obj, index)
                .unwrap_or_default();
            let propagators = self
                .parse_pattern_propagators(rule_obj, index)
                .unwrap_or_default();

            if !sources.is_empty() && !sinks.is_empty() {
                let mut dataflow = DataFlowSpec::new(sources, sinks).with_sanitizers(sanitizers);
                dataflow.propagators = propagators;

                // Detect label/requires usage by checking raw YAML for these keys
                let has_labels = self.check_dataflow_uses_labels(rule_obj);
                dataflow.uses_labels = has_labels;

                // Parse taint options from the options field
                if let Some(options_obj) = rule_obj
                    .get(&Value::String("options".to_string()))
                    .and_then(|v| v.as_mapping())
                {
                    if let Some(val) =
                        options_obj.get(&Value::String("taint_assume_safe_booleans".to_string()))
                    {
                        if let Some(b) = val.as_bool() {
                            dataflow.taint_assume_safe_booleans = Some(b);
                        }
                    }
                    if let Some(val) =
                        options_obj.get(&Value::String("taint_assume_safe_numbers".to_string()))
                    {
                        if let Some(b) = val.as_bool() {
                            dataflow.taint_assume_safe_numbers = Some(b);
                        }
                    }
                    if let Some(val) = options_obj.get(&Value::String(
                        "taint_only_propagate_through_assignments".to_string(),
                    )) {
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
            for (k, v) in opts {
                metadata.insert(k, Value::String(v));
            }
        }
        let enabled = self
            .get_optional_bool_field(rule_obj, "enabled")
            .unwrap_or(true);

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
    fn parse_options(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<Option<HashMap<String, String>>> {
        let options_value = obj.get(&Value::String("options".to_string()));
        if options_value.is_none() {
            return Ok(None);
        }
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

        // Parse constant_propagation option
        if let Some(val) = options_obj.get(&Value::String("constant_propagation".to_string())) {
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

        // Parse constant_propagation option
        if let Some(val) = options_obj.get(&Value::String("constant_propagation".to_string())) {
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
            options.insert("constant_propagation".to_string(), str_val);
        }

        // Parse taint_assume_safe_booleans option
        if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_booleans".to_string()))
        {
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
        if let Some(val) = options_obj.get(&Value::String("taint_assume_safe_numbers".to_string()))
        {
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
        if let Some(val) = options_obj.get(&Value::String(
            "taint_only_propagate_through_assignments".to_string(),
        )) {
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
            options.insert(
                "taint_only_propagate_through_assignments".to_string(),
                str_val,
            );
        }

        Ok(Some(options))
    }

    /// Get a required string field
    fn get_string_field(
        &self,
        obj: &serde_yaml::Mapping,
        field: &str,
        index: usize,
    ) -> Result<String> {
        obj.get(&Value::String(field.to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AnalysisError::parse_error(format!(
                    "Rule {} missing required field: {}",
                    index, field
                ))
            })
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
        let confidence_str = self
            .get_optional_string_field(obj, "confidence")
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
            .ok_or_else(|| {
                AnalysisError::parse_error(format!("Rule {} missing 'languages' field", index))
            })?;

        let languages_array = languages_value.as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} 'languages' must be an array", index))
        })?;

        let mut languages = Vec::new();
        for lang_value in languages_array {
            let lang_str = lang_value.as_str().ok_or_else(|| {
                AnalysisError::parse_error(format!("Rule {} language must be a string", index))
            })?;

            let language = Language::parse_name(lang_str).ok_or_else(|| {
                AnalysisError::parse_error(format!("Rule {} unknown language: {}", index, lang_str))
            })?;

            languages.push(language);
        }

        if languages.is_empty() {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} must specify at least one language",
                index
            )));
        }

        Ok(languages)
    }

    /// Parse patterns field or single pattern field (semgrep compatibility)
    fn parse_patterns_or_pattern(
        &self,
        obj: &serde_yaml::Mapping,
        index: usize,
    ) -> Result<Vec<Pattern>> {
        // Check for 'patterns' field first
        if let Some(patterns_value) = obj.get(&Value::String("patterns".to_string())) {
            return self.parse_patterns_array(patterns_value, index);
        }

        // Check for single 'pattern' field
        if let Some(pattern_value) = obj.get(&Value::String("pattern".to_string())) {
            let pattern = self.parse_single_pattern(pattern_value, index, 0)?;
            return Ok(vec![pattern]);
        }

        // Check for 'match' field (semgrep shorthand for a single pattern)
        if let Some(match_value) = obj.get(&Value::String("match".to_string())) {
            if let Some(match_str) = match_value.as_str() {
                let pattern = Pattern::simple(match_str.to_string());
                return Ok(vec![pattern]);
            } else if let Some(match_obj) = match_value.as_mapping() {
                let pattern = self.parse_match_object(match_obj, index)?;
                return Ok(vec![pattern]);
            }
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

        // Check for 'pattern-regex' field (regex match at top level)
        if let Some(pattern_regex_value) = obj.get(&Value::String("pattern-regex".to_string())) {
            if let Some(regex_str) = pattern_regex_value.as_str() {
                let pattern = Pattern::regex(regex_str.to_string());
                return Ok(vec![pattern]);
            }
        }

        // No patterns found
        Ok(Vec::new())
    }

    /// Parse a 'match:' object value (supports all:/where: sub-fields)
    fn parse_match_object(&self, obj: &serde_yaml::Mapping, index: usize) -> Result<Pattern> {
        let pattern = if let Some(pattern_str) = self.get_optional_string_field(obj, "pattern") {
            Pattern::simple(pattern_str)
        } else {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} 'match' object must contain 'pattern'",
                index
            )));
        };

        Ok(pattern)
    }

    /// Parse patterns array
    /// In Semgrep, items in `patterns` are combined with AND logic
    fn parse_patterns_array(&self, patterns_value: &Value, index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = patterns_value.as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} 'patterns' must be an array", index))
        })?;

        // Collect all components
        let mut positive_patterns: Vec<Pattern> = Vec::new();
        let mut negative_patterns: Vec<Pattern> = Vec::new();
        let mut conditions: Vec<Condition> = Vec::new();
        let mut focus_vars: Vec<String> = Vec::new();

        for (pattern_index, pattern_value) in patterns_array.iter().enumerate() {
            // Check if this is a focus-metavariable (not a pattern, but a modifier)
            if let Some(mapping) = pattern_value.as_mapping() {
                if mapping.contains_key(&Value::String("focus-metavariable".to_string())) {
                    if let Some(focus_value) =
                        mapping.get(&Value::String("focus-metavariable".to_string()))
                    {
                        if let Some(focus_str) = focus_value.as_str() {
                            focus_vars.push(focus_str.to_string());
                        } else if let Some(focus_array) = focus_value.as_sequence() {
                            for f in focus_array {
                                if let Some(focus_str) = f.as_str() {
                                    focus_vars.push(focus_str.to_string());
                                }
                            }
                        }
                    }
                    continue;
                }
            }

            // Check if this is a metavariable-comparison (not a pattern, but a condition)
            if let Some(mapping) = pattern_value.as_mapping() {
                if mapping.contains_key(&Value::String("metavariable-comparison".to_string())) {
                    if let Some(metavar_comp_value) =
                        mapping.get(&Value::String("metavariable-comparison".to_string()))
                    {
                        let metavar_comp = self.parse_metavariable_comparison(
                            metavar_comp_value,
                            index,
                            pattern_index,
                        )?;
                        conditions.push(Condition::MetavariableComparison(metavar_comp));
                    }
                    continue;
                }

                // Check if this is a semgrep-internal-metavariable-name (not a pattern, but a condition)
                if mapping.contains_key(&Value::String(
                    "semgrep-internal-metavariable-name".to_string(),
                )) {
                    if let Some(metavar_name_value) = mapping.get(&Value::String(
                        "semgrep-internal-metavariable-name".to_string(),
                    )) {
                        let metavar_name = self.parse_internal_metavariable_name(
                            metavar_name_value,
                            index,
                            pattern_index,
                        )?;
                        conditions.push(Condition::MetavariableName(metavar_name));
                    }
                    continue;
                }

                // Check if this is a metavariable-type (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-type".to_string())) {
                    if let Some(metavar_type_value) =
                        mapping.get(&Value::String("metavariable-type".to_string()))
                    {
                        let metavar_type =
                            self.parse_metavariable_type(metavar_type_value, index, pattern_index)?;
                        conditions.push(Condition::MetavariableType(metavar_type));
                    }
                    continue;
                }

                // Check if this is a metavariable-regex (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-regex".to_string())) {
                    if let Some(metavar_regex_value) =
                        mapping.get(&Value::String("metavariable-regex".to_string()))
                    {
                        let metavar_regex = self.parse_metavariable_regex(
                            metavar_regex_value,
                            index,
                            pattern_index,
                        )?;
                        conditions.push(Condition::MetavariableRegex(metavar_regex));
                    }
                    continue;
                }

                // Check if this is a metavariable-pattern (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-pattern".to_string())) {
                    if let Some(metavar_pattern_value) =
                        mapping.get(&Value::String("metavariable-pattern".to_string()))
                    {
                        let (metavar_pattern, nested) = self.parse_metavariable_pattern(
                            metavar_pattern_value,
                            index,
                            pattern_index,
                        )?;
                        conditions.push(Condition::MetavariablePattern(metavar_pattern));
                        conditions.extend(nested);
                    }
                    continue;
                }

                // Check if this is a metavariable-analysis (not a pattern, but a condition)
                if mapping.contains_key(&Value::String("metavariable-analysis".to_string())) {
                    if let Some(metavar_analysis_value) =
                        mapping.get(&Value::String("metavariable-analysis".to_string()))
                    {
                        let metavar_analysis = self.parse_metavariable_analysis(
                            metavar_analysis_value,
                            index,
                            pattern_index,
                        )?;
                        conditions.push(Condition::MetavariableAnalysis(metavar_analysis));
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
        if positive_patterns.is_empty()
            && negative_patterns.is_empty()
            && conditions.is_empty()
            && focus_vars.is_empty()
        {
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

        // Set focus metavariables on the main pattern
        if !focus_vars.is_empty() {
            main_pattern.focus = Some(focus_vars);
        }

        Ok(vec![main_pattern])
    }

    /// Parse metavariable comparison
    fn parse_metavariable_comparison(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableComparison> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-comparison must be an object",
                rule_index, pattern_index
            ))
        })?;

        let comparison = self.get_string_field(metavar_obj, "comparison", rule_index)?;

        let metavariable =
            if let Some(mv) = self.get_optional_string_field(metavar_obj, "metavariable") {
                if mv.starts_with('$') {
                    mv[1..].to_string()
                } else {
                    mv
                }
            } else {
                String::new()
            };

        let operator = ComparisonOperator::PythonExpression(comparison);
        let value = String::new();

        Ok(MetavariableComparison::new(metavariable, operator, value))
    }

    /// Parse metavariable type constraint
    fn parse_metavariable_type(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableType> {
        let type_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-type must be an object",
                rule_index, pattern_index
            ))
        })?;

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
    fn parse_pattern_either(
        &self,
        pattern_either_value: &Value,
        index: usize,
    ) -> Result<Vec<Pattern>> {
        let patterns_array = pattern_either_value.as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} 'pattern-either' must be an array", index))
        })?;

        let mut sub_patterns = Vec::new();
        for (pattern_index, pattern_value) in patterns_array.iter().enumerate() {
            let pattern = self.parse_single_pattern(pattern_value, index, pattern_index)?;
            sub_patterns.push(pattern);
        }

        // Return a single pattern with Either type
        Ok(vec![Pattern::either(sub_patterns)])
    }

    /// Parse a single pattern
    fn parse_single_pattern(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<Pattern> {
        if let Some(pattern_str) = value.as_str() {
            // Simple string pattern
            return Ok(Pattern::simple(pattern_str.to_string()));
        }

        let pattern_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} must be a string or object",
                rule_index, pattern_index
            ))
        })?;

        // Parse different pattern types
        let mut pattern = if let Some(pattern_str) =
            self.get_optional_string_field(pattern_obj, "pattern")
        {
            Pattern::simple(pattern_str)
        } else if let Some(pattern_inside) =
            self.get_optional_string_field(pattern_obj, "pattern-inside")
        {
            Pattern::inside(Pattern::simple(pattern_inside))
        } else if let Some(pattern_inside_value) =
            pattern_obj.get(&Value::String("pattern-inside".to_string()))
        {
            if let Some(pattern_inside_obj) = pattern_inside_value.as_mapping() {
                if let Some(patterns_value) =
                    pattern_inside_obj.get(&Value::String("patterns".to_string()))
                {
                    let patterns = self.parse_patterns_array(patterns_value, rule_index)?;
                    if patterns.len() == 1 {
                        Pattern::inside(patterns.into_iter().next().unwrap())
                    } else {
                        Pattern::inside(Pattern::all(patterns))
                    }
                } else {
                    return Err(AnalysisError::parse_error(format!(
                        "Rule {} pattern {}: pattern-inside object must contain 'patterns' field",
                        rule_index, pattern_index
                    )));
                }
            } else {
                return Err(AnalysisError::parse_error(format!(
                    "Rule {} pattern {}: pattern-inside must be a string or object",
                    rule_index, pattern_index
                )));
            }
        } else if let Some(pattern_not_inside) =
            self.get_optional_string_field(pattern_obj, "pattern-not-inside")
        {
            Pattern::not_inside(Pattern::simple(pattern_not_inside))
        } else if let Some(pattern_not_inside_value) =
            pattern_obj.get(&Value::String("pattern-not-inside".to_string()))
        {
            if let Some(pattern_not_inside_obj) = pattern_not_inside_value.as_mapping() {
                if let Some(patterns_value) =
                    pattern_not_inside_obj.get(&Value::String("patterns".to_string()))
                {
                    let patterns = self.parse_patterns_array(patterns_value, rule_index)?;
                    if patterns.len() == 1 {
                        Pattern::not_inside(patterns.into_iter().next().unwrap())
                    } else {
                        Pattern::not_inside(Pattern::all(patterns))
                    }
                } else {
                    return Err(AnalysisError::parse_error(format!(
                        "Rule {} pattern {}: pattern-not-inside object must contain 'patterns' field",
                        rule_index, pattern_index
                    )));
                }
            } else {
                return Err(AnalysisError::parse_error(format!(
                    "Rule {} pattern {}: pattern-not-inside must be a string or object",
                    rule_index, pattern_index
                )));
            }
        } else if let Some(pattern_not) = self.get_optional_string_field(pattern_obj, "pattern-not")
        {
            Pattern::pattern_not(Pattern::simple(pattern_not))
        } else if let Some(pattern_regex) =
            self.get_optional_string_field(pattern_obj, "pattern-regex")
        {
            Pattern::regex(pattern_regex)
        } else if let Some(pattern_not_regex) =
            self.get_optional_string_field(pattern_obj, "pattern-not-regex")
        {
            Pattern::not_regex(pattern_not_regex)
        } else if let Some(pattern_either_value) =
            pattern_obj.get(&Value::String("pattern-either".to_string()))
        {
            // Handle nested pattern-either
            let either_patterns = self.parse_pattern_either(pattern_either_value, rule_index)?;
            if either_patterns.len() == 1 {
                either_patterns.into_iter().next().unwrap()
            } else {
                Pattern::either(either_patterns)
            }
        } else if let Some(pattern_all_value) =
            pattern_obj.get(&Value::String("pattern-all".to_string()))
        {
            // Handle pattern-all
            let all_patterns = self.parse_pattern_all(pattern_all_value, rule_index)?;
            if all_patterns.len() == 1 {
                all_patterns.into_iter().next().unwrap()
            } else {
                Pattern::all(all_patterns)
            }
        } else if let Some(pattern_any_value) =
            pattern_obj.get(&Value::String("pattern-any".to_string()))
        {
            // Handle pattern-any
            let any_patterns = self.parse_pattern_any(pattern_any_value, rule_index)?;
            if any_patterns.len() == 1 {
                any_patterns.into_iter().next().unwrap()
            } else {
                Pattern::any(any_patterns)
            }
        } else if let Some(patterns_value) = pattern_obj.get(&Value::String("patterns".to_string()))
        {
            // Handle nested patterns (AND logic)
            let patterns = self.parse_patterns_array(patterns_value, rule_index)?;
            if patterns.len() == 1 {
                patterns.into_iter().next().unwrap()
            } else {
                Pattern::all(patterns)
            }
        } else if let Some(metavar_value) =
            pattern_obj.get(&Value::String("metavariable-pattern".to_string()))
        {
            let mut pattern = Pattern::simple("...".to_string());
            let (metavar_pattern, nested) =
                self.parse_metavariable_pattern(metavar_value, rule_index, pattern_index)?;
            pattern.metavariable_pattern = Some(metavar_pattern);
            pattern.conditions.extend(nested);
            pattern
        } else {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} pattern {} must have a pattern field",
                rule_index, pattern_index
            )));
        };

        // Parse optional metavariable pattern
        if let Some(metavar_value) =
            pattern_obj.get(&Value::String("metavariable-pattern".to_string()))
        {
            let (metavar_pattern, nested) =
                self.parse_metavariable_pattern(metavar_value, rule_index, pattern_index)?;
            pattern.metavariable_pattern = Some(metavar_pattern);
            pattern.conditions.extend(nested);
        }

        // Parse optional metavariable regex
        if let Some(metavar_regex_value) =
            pattern_obj.get(&Value::String("metavariable-regex".to_string()))
        {
            let metavar_regex =
                self.parse_metavariable_regex(metavar_regex_value, rule_index, pattern_index)?;
            pattern
                .conditions
                .push(Condition::MetavariableRegex(metavar_regex));
        }

        // Parse optional metavariable-name
        if let Some(metavar_name_value) =
            pattern_obj.get(&Value::String("metavariable-name".to_string()))
        {
            let metavar_name =
                self.parse_metavariable_name(metavar_name_value, rule_index, pattern_index)?;
            pattern
                .conditions
                .push(Condition::MetavariableName(metavar_name));
        }

        // Parse optional metavariable-analysis
        if let Some(metavar_analysis_value) =
            pattern_obj.get(&Value::String("metavariable-analysis".to_string()))
        {
            let metavar_analysis = self.parse_metavariable_analysis(
                metavar_analysis_value,
                rule_index,
                pattern_index,
            )?;
            pattern
                .conditions
                .push(Condition::MetavariableAnalysis(metavar_analysis));
        }

        // Parse optional focus (single metavariable)
        if let Some(focus) = self.get_optional_string_field(pattern_obj, "focus") {
            pattern.focus = Some(vec![focus]);
        }

        // Parse optional focus-metavariable (single or array)
        if let Some(focus_metavar_value) =
            pattern_obj.get(&Value::String("focus-metavariable".to_string()))
        {
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
    fn parse_metavariable_pattern(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<(MetavariablePattern, Vec<Condition>)> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern must be an object",
                rule_index, pattern_index
            ))
        })?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;

        let mut patterns = Vec::new();
        let mut nested_conditions: Vec<Condition> = Vec::new();

        if let Some(patterns_value) = metavar_obj.get(&Value::String("patterns".to_string())) {
            let patterns_array = patterns_value.as_sequence().ok_or_else(|| {
                AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'patterns' must be an array",
                    rule_index, pattern_index
                ))
            })?;

            for pv in patterns_array {
                if let Some(obj) = pv.as_mapping() {
                    if let Some(pattern_not_value) =
                        obj.get(&Value::String("pattern-not".to_string()))
                    {
                        if let Some(pattern_str) = pattern_not_value.as_str() {
                            patterns.push(format!("__NOT__:{}", pattern_str));
                            continue;
                        }
                    }
                    if let Some(pv2) = obj.get(&Value::String("pattern".to_string())) {
                        if let Some(pattern_str) = pv2.as_str() {
                            patterns.push(pattern_str.to_string());
                            continue;
                        }
                    }
                    if let Some(nested_mv) =
                        obj.get(&Value::String("metavariable-pattern".to_string()))
                    {
                        let (nested_pattern, deeper_nested) =
                            self.parse_metavariable_pattern(nested_mv, rule_index, pattern_index)?;
                        nested_conditions.push(Condition::MetavariablePattern(nested_pattern));
                        nested_conditions.extend(deeper_nested);
                        continue;
                    }
                    if let Some(nested_regex) =
                        obj.get(&Value::String("metavariable-regex".to_string()))
                    {
                        let metavar_regex =
                            self.parse_metavariable_regex(nested_regex, rule_index, pattern_index)?;
                        nested_conditions.push(Condition::MetavariableRegex(metavar_regex));
                        continue;
                    }
                    if let Some(pattern_not_regex) =
                        obj.get(&Value::String("pattern-not-regex".to_string()))
                    {
                        if let Some(regex_str) = pattern_not_regex.as_str() {
                            patterns.push(format!("__NOT_REGEX__:{}", regex_str));
                            continue;
                        }
                    }
                    if let Some(pattern_regex) =
                        obj.get(&Value::String("pattern-regex".to_string()))
                    {
                        if let Some(regex_str) = pattern_regex.as_str() {
                            patterns.push(format!("__REGEX__:{}", regex_str));
                            continue;
                        }
                    }
                }
                let pattern_str = pv.as_str().ok_or_else(|| {
                    AnalysisError::parse_error(format!(
                        "Rule {} pattern {} metavariable pattern must be a string or object",
                        rule_index, pattern_index
                    ))
                })?;
                patterns.push(pattern_str.to_string());
            }
        } else if let Some(pattern_value) = metavar_obj.get(&Value::String("pattern".to_string())) {
            let pattern_str = pattern_value.as_str().ok_or_else(|| {
                AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'pattern' must be a string",
                    rule_index, pattern_index
                ))
            })?;
            patterns.push(pattern_str.to_string());
        } else if let Some(pattern_either_value) =
            metavar_obj.get(&Value::String("pattern-either".to_string()))
        {
            let either_array = pattern_either_value.as_sequence().ok_or_else(|| {
                AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable_pattern 'pattern-either' must be an array",
                    rule_index, pattern_index
                ))
            })?;

            for pattern_obj in either_array {
                if let Some(obj) = pattern_obj.as_mapping() {
                    if let Some(pattern_value) = obj.get(&Value::String("pattern".to_string())) {
                        if let Some(pattern_str) = pattern_value.as_str() {
                            patterns.push(pattern_str.to_string());
                        }
                    }
                }
            }
        } else {
            return Err(AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern must have 'patterns', 'pattern', or 'pattern-either' field",
                rule_index, pattern_index
            )));
        }

        let mut metavar_pattern = MetavariablePattern::with_patterns(metavariable, patterns);
        metavar_pattern.nested_conditions = nested_conditions;

        if let Some(regex) = self.get_optional_string_field(metavar_obj, "regex") {
            metavar_pattern.regex = Some(regex);
        }

        if let Some(type_constraint) = self.get_optional_string_field(metavar_obj, "type") {
            metavar_pattern.type_constraint = Some(type_constraint);
        }

        if let Some(name_constraint) = self.get_optional_string_field(metavar_obj, "name") {
            metavar_pattern.name_constraint = Some(name_constraint);
        }

        if let Some(analysis_value) = metavar_obj.get(&Value::String("analysis".to_string())) {
            let analysis =
                self.parse_metavariable_analysis_config(analysis_value, rule_index, pattern_index)?;
            metavar_pattern.analysis = Some(analysis);
        }

        Ok((metavar_pattern, Vec::new()))
    }

    /// Parse metavariable regex
    fn parse_metavariable_regex(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableRegex> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-regex must be an object",
                rule_index, pattern_index
            ))
        })?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let regex = self.get_string_field(metavar_obj, "regex", rule_index)?;

        Ok(MetavariableRegex::new(metavariable, regex))
    }

    /// Parse metavariable name constraint
    fn parse_metavariable_name(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableName> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-name must be an object",
                rule_index, pattern_index
            ))
        })?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let name_pattern = self.get_string_field(metavar_obj, "name", rule_index)?;

        Ok(MetavariableName::new(metavariable, name_pattern))
    }

    /// Parse semgrep-internal-metavariable-name constraint
    fn parse_internal_metavariable_name(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableName> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} semgrep-internal-metavariable-name must be an object",
                rule_index, pattern_index
            ))
        })?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let fqn = self.get_string_field(metavar_obj, "fqn", rule_index)?;

        Ok(MetavariableName::with_fqn(metavariable, fqn))
    }

    /// Parse metavariable analysis
    fn parse_metavariable_analysis(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableAnalysisCondition> {
        let metavar_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable-analysis must be an object",
                rule_index, pattern_index
            ))
        })?;

        let metavariable = self.get_string_field(metavar_obj, "metavariable", rule_index)?;
        let analysis = self.parse_metavariable_analysis_config(value, rule_index, pattern_index)?;

        Ok(MetavariableAnalysisCondition::new(metavariable, analysis))
    }

    /// Parse metavariable analysis configuration
    fn parse_metavariable_analysis_config(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<MetavariableAnalysis> {
        let analysis_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable analysis must be an object",
                rule_index, pattern_index
            ))
        })?;

        let mut analysis = MetavariableAnalysis {
            entropy: None,
            type_analysis: None,
            complexity: None,
        };

        // Parse entropy analysis
        if let Some(entropy_value) = analysis_obj.get(&Value::String("entropy".to_string())) {
            analysis.entropy =
                Some(self.parse_entropy_analysis(entropy_value, rule_index, pattern_index)?);
        }

        // Parse type analysis
        if let Some(type_value) = analysis_obj.get(&Value::String("type".to_string())) {
            analysis.type_analysis =
                Some(self.parse_type_analysis(type_value, rule_index, pattern_index)?);
        }

        // Parse complexity analysis
        if let Some(complexity_value) = analysis_obj.get(&Value::String("complexity".to_string())) {
            analysis.complexity = Some(self.parse_complexity_analysis(
                complexity_value,
                rule_index,
                pattern_index,
            )?);
        }

        Ok(analysis)
    }

    /// Parse entropy analysis
    fn parse_entropy_analysis(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<EntropyAnalysis> {
        let entropy_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} entropy analysis must be an object",
                rule_index, pattern_index
            ))
        })?;

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
    fn parse_type_analysis(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<TypeAnalysis> {
        let type_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} type analysis must be an object",
                rule_index, pattern_index
            ))
        })?;

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
    fn parse_complexity_analysis(
        &self,
        value: &Value,
        rule_index: usize,
        pattern_index: usize,
    ) -> Result<ComplexityAnalysis> {
        let complexity_obj = value.as_mapping().ok_or_else(|| {
            AnalysisError::parse_error(format!(
                "Rule {} pattern {} complexity analysis must be an object",
                rule_index, pattern_index
            ))
        })?;

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
    fn parse_dataflow(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<Option<DataFlowSpec>> {
        let dataflow_value = obj.get(&Value::String("dataflow".to_string()));

        if dataflow_value.is_none() {
            return Ok(None);
        }

        let dataflow_obj = dataflow_value.unwrap().as_mapping().ok_or_else(|| {
            AnalysisError::parse_error("'dataflow' must be an object".to_string())
        })?;

        let sources = self.parse_string_array(dataflow_obj, "sources")?;
        let sinks = self.parse_string_array(dataflow_obj, "sinks")?;
        let sanitizers = self
            .parse_string_array(dataflow_obj, "sanitizers")
            .unwrap_or_default();

        let mut dataflow = DataFlowSpec::from_strings(sources, sinks).with_sanitizers(sanitizers);

        if let Some(must_flow) = self.get_optional_bool_field(dataflow_obj, "must_flow") {
            dataflow.must_flow = must_flow;
        }

        if let Some(max_depth_value) = dataflow_obj.get(&Value::String("max_depth".to_string())) {
            if let Some(max_depth) = max_depth_value.as_u64() {
                dataflow.max_depth = Some(max_depth as usize);
            }
        }

        if let Some(taint_assume_safe_booleans_value) =
            dataflow_obj.get(&Value::String("taint_assume_safe_booleans".to_string()))
        {
            if let Some(b) = taint_assume_safe_booleans_value.as_bool() {
                dataflow.taint_assume_safe_booleans = Some(b);
            }
        }

        if let Some(taint_assume_safe_numbers_value) =
            dataflow_obj.get(&Value::String("taint_assume_safe_numbers".to_string()))
        {
            if let Some(b) = taint_assume_safe_numbers_value.as_bool() {
                dataflow.taint_assume_safe_numbers = Some(b);
            }
        }

        if let Some(taint_only_propagate_through_assignments_value) = dataflow_obj.get(
            &Value::String("taint_only_propagate_through_assignments".to_string()),
        ) {
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
            let item_str = item.as_str().ok_or_else(|| {
                AnalysisError::parse_error(format!("'{}' items must be strings", field))
            })?;
            result.push(item_str.to_string());
        }

        Ok(result)
    }

    /// Parse pattern-all
    fn parse_pattern_all(&self, value: &Value, rule_index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = value.as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} pattern-all must be an array", rule_index))
        })?;

        let mut patterns = Vec::new();
        for (index, pattern_value) in patterns_array.iter().enumerate() {
            patterns.push(self.parse_single_pattern(pattern_value, rule_index, index)?);
        }

        Ok(patterns)
    }

    /// Parse pattern-any
    fn parse_pattern_any(&self, value: &Value, rule_index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = value.as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} pattern-any must be an array", rule_index))
        })?;

        let mut patterns = Vec::new();
        for (index, pattern_value) in patterns_array.iter().enumerate() {
            patterns.push(self.parse_single_pattern(pattern_value, rule_index, index)?);
        }

        Ok(patterns)
    }

    /// Parse fix-regex field
    fn parse_fix_regex(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<Option<FixRegex>> {
        let fix_regex_value = obj.get(&Value::String("fix-regex".to_string()));

        if fix_regex_value.is_none() {
            return Ok(None);
        }

        let fix_regex_obj = fix_regex_value.unwrap().as_mapping().ok_or_else(|| {
            AnalysisError::parse_error("'fix-regex' must be an object".to_string())
        })?;

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
    fn parse_optional_string_array(
        &self,
        obj: &serde_yaml::Mapping,
        field: &str,
    ) -> Result<Vec<String>> {
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
            let item_str = item.as_str().ok_or_else(|| {
                AnalysisError::parse_error(format!("'{}' items must be strings", field))
            })?;
            result.push(item_str.to_string());
        }

        Ok(result)
    }

    /// Parse metadata field
    fn parse_metadata(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<HashMap<String, Value>> {
        let metadata_value = obj.get(&Value::String("metadata".to_string()));

        if metadata_value.is_none() {
            return Ok(HashMap::new());
        }

        let metadata_obj = metadata_value.unwrap().as_mapping().ok_or_else(|| {
            AnalysisError::parse_error("'metadata' must be an object".to_string())
        })?;

        let mut metadata = HashMap::new();
        for (key, value) in metadata_obj {
            let key_str = key.as_str().ok_or_else(|| {
                AnalysisError::parse_error("metadata keys must be strings".to_string())
            })?;
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
    fn parse_pattern_sources(
        &self,
        obj: &serde_yaml::Mapping,
        index: usize,
    ) -> Result<Vec<SourcePattern>> {
        let sources_value = obj.get(&Value::String("pattern-sources".to_string()));

        if sources_value.is_none() {
            return Ok(Vec::new());
        }

        let sources_array = sources_value.unwrap().as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} 'pattern-sources' must be an array", index))
        })?;

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
    fn parse_pattern_sinks(
        &self,
        obj: &serde_yaml::Mapping,
        index: usize,
    ) -> Result<Vec<SinkPattern>> {
        let sinks_value = obj.get(&Value::String("pattern-sinks".to_string()));

        if sinks_value.is_none() {
            return Ok(Vec::new());
        }

        let sinks_array = sinks_value.unwrap().as_sequence().ok_or_else(|| {
            AnalysisError::parse_error(format!("Rule {} 'pattern-sinks' must be an array", index))
        })?;

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
    fn parse_pattern_sanitizers(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<Vec<String>> {
        let sanitizers_value = obj.get(&Value::String("pattern-sanitizers".to_string()));

        if sanitizers_value.is_none() {
            return Ok(Vec::new());
        }

        let sanitizers_array = sanitizers_value.unwrap().as_sequence().ok_or_else(|| {
            AnalysisError::parse_error("'pattern-sanitizers' must be an array".to_string())
        })?;

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
    fn parse_pattern_propagators(
        &self,
        obj: &serde_yaml::Mapping,
        _index: usize,
    ) -> Result<Vec<PropagatorPattern>> {
        use crate::types::PropagatorPattern;

        let propagators_value = obj.get(&Value::String("pattern-propagators".to_string()));

        if propagators_value.is_none() {
            return Ok(Vec::new());
        }

        let propagators_array = propagators_value.unwrap().as_sequence().ok_or_else(|| {
            AnalysisError::parse_error("'pattern-propagators' must be an array".to_string())
        })?;

        let mut propagators = Vec::new();
        for propagator in propagators_array.iter() {
            if let Some(mapping) = propagator.as_mapping() {
                // Extract pattern (for propagators, preserve original metavariables but remove type qualifiers)
                let pattern =
                    if let Some(pattern_val) = mapping.get(&Value::String("pattern".to_string())) {
                        if let Some(s) = pattern_val.as_str() {
                            // For propagators, don't simplify metavariables - keep $X, $Y, etc.
                            // But do remove type qualifiers like "(Type $VAR)." -> "$VAR."
                            Pattern::simple(self.simplify_type_qualifiers(s))
                        } else {
                            continue;
                        }
                    } else if let Some(patterns_val) =
                        mapping.get(&Value::String("patterns".to_string()))
                    {
                        // Handle patterns array
                        if let Some(arr) = patterns_val.as_sequence() {
                            if let Some(first) = arr.first() {
                                if let Some(mapping) = first.as_mapping() {
                                    if let Some(pattern) =
                                        mapping.get(&Value::String("pattern".to_string()))
                                    {
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
    fn check_dataflow_uses_labels(&self, rule_obj: &serde_yaml::Mapping) -> bool {
        let label_key = Value::String("label".to_string());
        let requires_key = Value::String("requires".to_string());

        for array_key in &["pattern-sources", "pattern-sinks"] {
            let key = Value::String(array_key.to_string());
            if let Some(Value::Sequence(arr)) = rule_obj.get(&key) {
                for item in arr {
                    if let Some(mapping) = item.as_mapping() {
                        if mapping.contains_key(&label_key) || mapping.contains_key(&requires_key) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

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
            if let Some(pattern_either) = mapping.get(&Value::String("pattern-either".to_string()))
            {
                if let Some(arr) = pattern_either.as_sequence() {
                    let patterns: Vec<String> = arr
                        .iter()
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
                            if item_map
                                .contains_key(&Value::String("metavariable-regex".to_string()))
                            {
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
            if let Some(pattern_inside) = mapping.get(&Value::String("pattern-inside".to_string()))
            {
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
            if let Some(pattern_inside) = mapping.get(&Value::String("pattern-inside".to_string()))
            {
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
            let (pattern_str, focus_metavariables) = if let Some(patterns_value) =
                mapping.get(&Value::String("patterns".to_string()))
            {
                // Semgrep uses "patterns" array where:
                // - Elements with "pattern" field define the pattern to match
                // - Elements with "focus-metavariable" field specify which variable to track
                let patterns_array = patterns_value.as_sequence().ok_or_else(|| {
                    AnalysisError::parse_error("'patterns' must be an array".to_string())
                })?;

                if patterns_array.is_empty() {
                    return Err(AnalysisError::parse_error(
                        "'patterns' array must not be empty".to_string(),
                    ));
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
                        if let Some(f_val) =
                            elem_map.get(&Value::String("focus-metavariable".to_string()))
                        {
                            if let Some(f_str) = f_val.as_str() {
                                focus_vars.push(f_str.to_string());
                            }
                        }
                    } else if let Some(p_str) = pattern_elem.as_str() {
                        // Simple string pattern
                        pattern_str = Some(p_str.to_string());
                    }
                }

                let pattern_str = pattern_str.ok_or_else(|| {
                    AnalysisError::parse_error("No pattern found in 'patterns' array".to_string())
                })?;
                (pattern_str, focus_vars)
            } else if let Some(pattern_value) = mapping.get(&Value::String("pattern".to_string())) {
                // Standard "pattern" field
                let pattern_str = pattern_value
                    .as_str()
                    .ok_or_else(|| {
                        AnalysisError::parse_error(
                            "Source pattern must have a 'pattern' field".to_string(),
                        )
                    })?
                    .to_string();

                // Check for focus-metavariable at this level (alternate format)
                let focus_metavariables = mapping
                    .get(&Value::String("focus-metavariable".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default();

                (pattern_str, focus_metavariables)
            } else {
                return Err(AnalysisError::parse_error(
                    "Source pattern must have 'pattern' or 'patterns' field".to_string(),
                ));
            };

            // Check if fallback flag is set (optional)
            let is_fallback = mapping
                .get(&Value::String("is_fallback".to_string()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            return Ok(SourcePattern {
                pattern: Pattern::simple(pattern_str),
                focus_metavariables,
                is_fallback,
            });
        }

        Err(AnalysisError::parse_error(
            "Invalid source pattern format".to_string(),
        ))
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
            // Check for top-level pattern-either
            if let Some(pattern_either) = mapping.get(&Value::String("pattern-either".to_string()))
            {
                if let Some(arr) = pattern_either.as_sequence() {
                    let either_patterns: Vec<Pattern> = arr
                        .iter()
                        .filter_map(|v| {
                            // Recursively parse each pattern
                            self.parse_sink_pattern(v, 0).ok().map(|sp| sp.pattern)
                        })
                        .collect();
                    if !either_patterns.is_empty() {
                        return Ok(SinkPattern {
                            pattern: Pattern {
                                pattern_type: PatternType::Either(either_patterns),
                                conditions: Vec::new(),
                                metavariable_pattern: None,
                                focus: None,
                            },
                            focus_metavariables: Vec::new(),
                            is_fallback: false,
                        });
                    }
                }
            }

            // Check for "patterns" array (Semgrep format with focus-metavariable)
            let (pattern, focus_metavariables) = if let Some(patterns_value) =
                mapping.get(&Value::String("patterns".to_string()))
            {
                let patterns_array = patterns_value.as_sequence().ok_or_else(|| {
                    AnalysisError::parse_error("'patterns' must be an array".to_string())
                })?;

                if patterns_array.is_empty() {
                    return Err(AnalysisError::parse_error(
                        "'patterns' array must not be empty".to_string(),
                    ));
                }

                // Extract pattern and focus vars
                let mut pattern = None;
                let mut focus_vars = Vec::new();

                for pattern_elem in patterns_array {
                    if let Some(elem_map) = pattern_elem.as_mapping() {
                        // Look for focus-metavariable field
                        if let Some(f_val) =
                            elem_map.get(&Value::String("focus-metavariable".to_string()))
                        {
                            if let Some(f_str) = f_val.as_str() {
                                focus_vars.push(f_str.to_string());
                            }
                        }
                        // Look for pattern-either field - create Either pattern
                        if pattern.is_none() {
                            if let Some(pattern_either) =
                                elem_map.get(&Value::String("pattern-either".to_string()))
                            {
                                if let Some(arr) = pattern_either.as_sequence() {
                                    let either_patterns: Vec<Pattern> = arr
                                        .iter()
                                        .filter_map(|v| {
                                            // Handle nested patterns array within pattern-either
                                            self.extract_sink_pattern_from_either_item(v)
                                        })
                                        .collect();
                                    if !either_patterns.is_empty() {
                                        pattern = Some(Pattern {
                                            pattern_type: PatternType::Either(either_patterns),
                                            conditions: Vec::new(),
                                            metavariable_pattern: None,
                                            focus: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // If no pattern found in pattern-either, try to extract from patterns array directly
                if pattern.is_none() {
                    for pattern_elem in patterns_array {
                        if let Some(p) = self.extract_pattern_from_taint_def(pattern_elem) {
                            pattern = Some(Pattern::simple(p));
                            break;
                        }
                    }
                }

                let pattern = pattern.ok_or_else(|| {
                    AnalysisError::parse_error(
                        "No pattern found in sink 'patterns' array".to_string(),
                    )
                })?;
                (pattern, focus_vars)
            } else if let Some(pattern_value) = mapping.get(&Value::String("pattern".to_string())) {
                // Standard "pattern" field
                let pattern_str = pattern_value
                    .as_str()
                    .ok_or_else(|| {
                        AnalysisError::parse_error(
                            "Sink pattern must have a 'pattern' field".to_string(),
                        )
                    })?
                    .to_string();

                // Check for focus-metavariable at this level (alternate format)
                let focus_metavariables = mapping
                    .get(&Value::String("focus-metavariable".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default();

                (Pattern::simple(pattern_str), focus_metavariables)
            } else {
                return Err(AnalysisError::parse_error(
                    "Sink pattern must have 'pattern' or 'patterns' field".to_string(),
                ));
            };

            // Check if fallback flag is set (optional)
            let is_fallback = mapping
                .get(&Value::String("is_fallback".to_string()))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            return Ok(SinkPattern {
                pattern,
                focus_metavariables,
                is_fallback,
            });
        }

        Err(AnalysisError::parse_error(
            "Invalid sink pattern format".to_string(),
        ))
    }

    /// Extract a pattern from an item within pattern-either, handling nested patterns arrays
    fn extract_sink_pattern_from_either_item(&self, value: &Value) -> Option<Pattern> {
        // First try as a simple pattern string
        if let Some(s) = self.extract_pattern_raw(value) {
            return Some(Pattern::simple(s));
        }

        // Then try as a nested patterns array
        if let Some(mapping) = value.as_mapping() {
            if let Some(patterns_array) = mapping
                .get(&Value::String("patterns".to_string()))
                .and_then(|v| v.as_sequence())
            {
                // Look for the actual pattern within the nested patterns array
                for item in patterns_array {
                    if let Some(item_map) = item.as_mapping() {
                        // Skip pattern-not - we only want the actual pattern
                        if item_map.contains_key(&Value::String("pattern-not".to_string())) {
                            continue;
                        }
                        // Found the actual pattern
                        if let Some(pattern_val) =
                            item_map.get(&Value::String("pattern".to_string()))
                        {
                            if let Some(pattern_str) = pattern_val.as_str() {
                                return Some(Pattern::simple(pattern_str.to_string()));
                            }
                        }
                        // Also try pattern-inside as the pattern source
                        if let Some(pattern_inside_val) =
                            item_map.get(&Value::String("pattern-inside".to_string()))
                        {
                            if let Some(pattern_str) = pattern_inside_val.as_str() {
                                return Some(Pattern::simple(pattern_str.to_string()));
                            }
                        }
                    }
                    // Also try simple string pattern
                    if let Some(s) = self.extract_pattern_raw(item) {
                        return Some(Pattern::simple(s));
                    }
                }
            }

            // Also try pattern-inside at the current level
            if let Some(pattern_inside_val) =
                mapping.get(&Value::String("pattern-inside".to_string()))
            {
                if let Some(pattern_str) = pattern_inside_val.as_str() {
                    return Some(Pattern::simple(pattern_str.to_string()));
                }
            }
        }

        None
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A simple test rule
    message: A simple test rule
    severity: ERROR
    languages: [java]
    patterns:
      - "System.out.println($MSG)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.severity, Severity::Error);
        assert_eq!(rule.languages, vec![Language::Java]);
        assert_eq!(rule.patterns.len(), 1);
        if let PatternType::Simple(pattern_str) = &rule.patterns[0].pattern_type {
            assert_eq!(pattern_str, "System.out.println($MSG)");
        } else {
            panic!("Expected Simple pattern type");
        }
    }

    #[test]
    fn test_parse_enhanced_patterns() {
        let yaml = r#"
rules:
  - id: enhanced-pattern-test
    name: Enhanced Pattern Test
    description: Tests new pattern types
    message: Tests new pattern types
    severity: ERROR
    languages: [python]
    patterns:
      - pattern: "def $FUNC(...):"
      - pattern-regex: "eval\\("
        focus-metavariable: ["$FUNC", "$ARG"]
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.id, "enhanced-pattern-test");

        assert_eq!(rule.patterns.len(), 1);

        let main = &rule.patterns[0];
        if let PatternType::Simple(s) = &main.pattern_type {
            assert_eq!(s, "def $FUNC(...):");
            assert_eq!(
                main.focus,
                Some(vec!["$FUNC".to_string(), "$ARG".to_string()])
            );
        } else {
            panic!("Expected PatternType::Simple, got {:?}", main.pattern_type);
        }
    }

    #[test]
    fn test_parse_complex_rule() {
        let yaml = r#"
rules:
  - id: sql-injection
    name: SQL Injection Detection
    description: Detects potential SQL injection vulnerabilities
    message: Detects potential SQL injection vulnerabilities
    severity: CRITICAL
    confidence: HIGH
    languages: [java, python]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
        metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - "$STR + $INPUT"
          regex: "SELECT.*FROM.*"
    dataflow:
      sources:
        - "request.getParameter(...)"
      sinks:
        - "Statement.execute(...)"
      sanitizers:
        - "sanitize(...)"
      must_flow: true
      max_depth: 10
    fix: "Use PreparedStatement with parameterized queries"
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.id, "sql-injection");
        assert_eq!(rule.severity, Severity::Critical);
        assert_eq!(rule.confidence, Confidence::High);
        assert_eq!(rule.languages.len(), 2);
        assert!(rule.dataflow.is_some());
        assert!(rule.fix.is_some());
        assert_eq!(rule.metadata.len(), 2);
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    # Missing required fields
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_language() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [unknown_language]
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_strict_mode() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java]
    unknown_field: "should cause error in strict mode"
"#;

        let parser = RuleParser::strict();
        // In our current implementation, unknown fields don't cause errors
        // This test demonstrates the structure for future enhancement
        let result = parser.parse_yaml(yaml);
        assert!(result.is_ok()); // Would be Err in true strict mode
    }

    #[test]
    fn test_parse_rule_with_all_severities() {
        let yaml = r#"
rules:
  - id: info-rule
    name: Info Rule
    description: An info rule
    message: An info rule
    severity: INFO
    languages: [java]
    pattern: "foo()"
  - id: warning-rule
    name: Warning Rule
    description: A warning rule
    message: A warning rule
    severity: WARNING
    languages: [java]
    pattern: "bar()"
  - id: error-rule
    name: Error Rule
    description: An error rule
    message: An error rule
    severity: ERROR
    languages: [java]
    pattern: "baz()"
  - id: critical-rule
    name: Critical Rule
    description: A critical rule
    message: A critical rule
    severity: CRITICAL
    languages: [java]
    pattern: "qux()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].severity, Severity::Info);
        assert_eq!(rules[1].severity, Severity::Warning);
        assert_eq!(rules[2].severity, Severity::Error);
        assert_eq!(rules[3].severity, Severity::Critical);
    }

    #[test]
    fn test_parse_rule_with_confidence() {
        let yaml = r#"
rules:
  - id: high-confidence
    name: High Confidence
    description: High confidence rule
    message: High confidence rule
    severity: ERROR
    confidence: HIGH
    languages: [java]
    pattern: "foo()"
  - id: low-confidence
    name: Low Confidence
    description: Low confidence rule
    message: Low confidence rule
    severity: ERROR
    confidence: LOW
    languages: [java]
    pattern: "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].confidence, Confidence::High);
        assert_eq!(rules[1].confidence, Confidence::Low);
    }

    #[test]
    fn test_parse_rule_with_metadata() {
        let yaml = r#"
rules:
  - id: meta-rule
    name: Meta Rule
    description: Rule with metadata
    message: Rule with metadata
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021"
      references:
        - "https://example.com"
        - "https://example.org"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].metadata.get("cwe"), Some(&Value::String("CWE-89".to_string())));
        assert_eq!(rules[0].metadata.get("owasp"), Some(&Value::String("A03:2021".to_string())));
        assert!(rules[0].metadata.contains_key("references"));
    }

    #[test]
    fn test_parse_rule_with_fix_and_fix_regex() {
        let yaml = r#"
rules:
  - id: fix-rule
    name: Fix Rule
    description: Rule with fix
    message: Rule with fix
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    fix: "bar()"
    fix-regex:
      regex: "foo"
      replacement: "bar"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].fix, Some("bar()".to_string()));
        assert!(rules[0].fix_regex.is_some());
        let fix_regex = rules[0].fix_regex.as_ref().unwrap();
        assert_eq!(fix_regex.regex, "foo");
        assert_eq!(fix_regex.replacement, "bar");
    }

    #[test]
    fn test_parse_rule_with_paths_filter() {
        let yaml = r#"
rules:
  - id: paths-rule
    name: Paths Rule
    description: Rule with paths filter
    message: Rule with paths filter
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    paths:
      include:
        - "src/**/*.java"
      exclude:
        - "test/**/*.java"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert!(rules[0].paths.is_some());
        let paths = rules[0].paths.as_ref().unwrap();
        assert_eq!(paths.includes, vec!["src/**/*.java"]);
        assert_eq!(paths.excludes, vec!["test/**/*.java"]);
    }

    #[test]
    fn test_parse_rule_disabled() {
        let yaml = r#"
rules:
  - id: disabled-rule
    name: Disabled Rule
    description: A disabled rule
    message: A disabled rule
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    enabled: false
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert!(!rules[0].enabled);
    }

    #[test]
    fn test_parse_pattern_either() {
        let yaml = r#"
rules:
  - id: either-rule
    name: Either Rule
    description: Rule with pattern-either
    message: Rule with pattern-either
    severity: ERROR
    languages: [java]
    pattern-either:
      - "foo()"
      - "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::Either(sub_patterns) = &rules[0].patterns[0].pattern_type {
            assert_eq!(sub_patterns.len(), 2);
        } else {
            panic!("Expected Either pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_inside() {
        let yaml = r#"
rules:
  - id: inside-rule
    name: Inside Rule
    description: Rule with pattern-inside
    message: Rule with pattern-inside
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-inside: "class $CLASS { ... }"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::Inside(inner) = &rules[0].patterns[0].pattern_type {
            if let PatternType::Simple(s) = &inner.pattern_type {
                assert_eq!(s, "class $CLASS { ... }");
            } else {
                panic!("Expected Simple inside Inside");
            }
        } else {
            panic!("Expected Inside pattern type, got {:?}", rules[0].patterns[0].pattern_type);
        }
    }

    #[test]
    fn test_parse_pattern_not() {
        let yaml = r#"
rules:
  - id: not-rule
    name: Not Rule
    description: Rule with pattern-not
    message: Rule with pattern-not
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "foo()"
      - pattern-not: "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::All(sub_patterns) = &rules[0].patterns[0].pattern_type {
            if let PatternType::Not(inner) = &sub_patterns[1].pattern_type {
                if let PatternType::Simple(s) = &inner.pattern_type {
                    assert_eq!(s, "bar()");
                } else {
                    panic!("Expected Simple inside Not");
                }
            } else {
                panic!("Expected Not pattern type, got {:?}", sub_patterns[1].pattern_type);
            }
        } else {
            panic!("Expected All pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_regex() {
        let yaml = r#"
rules:
  - id: regex-rule
    name: Regex Rule
    description: Rule with pattern-regex
    message: Rule with pattern-regex
    severity: ERROR
    languages: [java]
    pattern-regex: "eval\\("
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::Regex(regex_str) = &rules[0].patterns[0].pattern_type {
            assert_eq!(regex_str, "eval\\(");
        } else {
            panic!("Expected Regex pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_all_nested() {
        let yaml = r#"
rules:
  - id: all-rule
    name: All Rule
    description: Rule with pattern-all
    message: Rule with pattern-all
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-all:
          - "foo()"
          - "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::All(sub_patterns) = &rules[0].patterns[0].pattern_type {
            assert_eq!(sub_patterns.len(), 2);
        } else {
            panic!("Expected All pattern type");
        }
    }

    #[test]
    fn test_parse_match_field() {
        let yaml = r#"
rules:
  - id: match-rule
    name: Match Rule
    description: Rule with match field
    message: Rule with match field
    severity: ERROR
    languages: [java]
    match:
      pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::Simple(s) = &rules[0].patterns[0].pattern_type {
            assert_eq!(s, "foo()");
        } else {
            panic!("Expected Simple pattern type");
        }
    }

    #[test]
    fn test_parse_metavariable_pattern() {
        let yaml = r#"
rules:
  - id: metavar-pattern-rule
    name: Metavariable Pattern Rule
    description: Rule with metavariable-pattern
    message: Rule with metavariable-pattern
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
        metavariable-pattern:
          metavariable: "$QUERY"
          pattern: "$STR + $INPUT"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariablePattern(mp) = &main_pattern.conditions[0] {
            assert_eq!(mp.metavariable, "$QUERY");
            assert_eq!(mp.patterns, vec!["$STR + $INPUT"]);
        } else {
            panic!("Expected MetavariablePattern condition");
        }
    }

    #[test]
    fn test_parse_metavariable_pattern_in_patterns_array() {
        let yaml = r#"
rules:
  - id: metavar-pattern-array-rule
    name: Metavariable Pattern Array Rule
    description: Rule with metavariable-pattern in patterns array
    message: Rule with metavariable-pattern in patterns array
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable-pattern:
          metavariable: "$QUERY"
          pattern: "$STR + $INPUT"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariablePattern(mp) = &main_pattern.conditions[0] {
            assert_eq!(mp.metavariable, "$QUERY");
            assert_eq!(mp.patterns, vec!["$STR + $INPUT"]);
        } else {
            panic!("Expected MetavariablePattern condition");
        }
    }

    #[test]
    fn test_parse_metavariable_regex() {
        let yaml = r#"
rules:
  - id: metavar-regex-rule
    name: Metavariable Regex Rule
    description: Rule with metavariable-regex
    message: Rule with metavariable-regex
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
        metavariable-regex:
          metavariable: "$X"
          regex: "^[A-Z].*"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!rules[0].patterns[0].conditions.is_empty());
        if let Condition::MetavariableRegex(mr) = &rules[0].patterns[0].conditions[0] {
            assert_eq!(mr.metavariable, "$X");
            assert_eq!(mr.regex, "^[A-Z].*");
        } else {
            panic!("Expected MetavariableRegex condition");
        }
    }

    #[test]
    fn test_parse_metavariable_type() {
        let yaml = r#"
rules:
  - id: metavar-type-rule
    name: Metavariable Type Rule
    description: Rule with metavariable-type
    message: Rule with metavariable-type
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
      - metavariable-type:
          metavariable: "$X"
          type: "String"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariableType(mt) = &main_pattern.conditions[0] {
            assert_eq!(mt.metavariable, "X");
            assert_eq!(mt.var_type, "String");
        } else {
            panic!("Expected MetavariableType condition");
        }
    }

    #[test]
    fn test_parse_metavariable_comparison() {
        let yaml = r#"
rules:
  - id: metavar-comp-rule
    name: Metavariable Comparison Rule
    description: Rule with metavariable-comparison
    message: Rule with metavariable-comparison
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
      - metavariable-comparison:
          metavariable: "$X"
          comparison: "int($X) > 10"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariableComparison(mc) = &main_pattern.conditions[0] {
            assert_eq!(mc.metavariable, "X");
        } else {
            panic!("Expected MetavariableComparison condition");
        }
    }

    #[test]
    fn test_parse_metavariable_name() {
        let yaml = r#"
rules:
  - id: metavar-name-rule
    name: Metavariable Name Rule
    description: Rule with metavariable-name
    message: Rule with metavariable-name
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
        metavariable-name:
          metavariable: "$X"
          name: "^get.*"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!rules[0].patterns[0].conditions.is_empty());
        if let Condition::MetavariableName(mn) = &rules[0].patterns[0].conditions[0] {
            assert_eq!(mn.metavariable, "$X");
            assert_eq!(mn.name_pattern, "^get.*");
        } else {
            panic!("Expected MetavariableName condition");
        }
    }

    #[test]
    fn test_parse_focus_metavariable() {
        let yaml = r#"
rules:
  - id: focus-rule
    name: Focus Rule
    description: Rule with focus-metavariable
    message: Rule with focus-metavariable
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X.foo()"
        focus-metavariable: "$X"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns[0].focus, Some(vec!["$X".to_string()]));
    }

    #[test]
    fn test_parse_focus_metavariable_array() {
        let yaml = r#"
rules:
  - id: focus-array-rule
    name: Focus Array Rule
    description: Rule with focus-metavariable array
    message: Rule with focus-metavariable array
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X.foo($Y)"
        focus-metavariable:
          - "$X"
          - "$Y"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns[0].focus, Some(vec!["$X".to_string(), "$Y".to_string()]));
    }

    #[test]
    fn test_parse_missing_rules_key() {
        let yaml = r#"
not_rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java]
"#;

        let parser = RuleParser::new();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rules_not_array() {
        let yaml = r#"
rules:
  id: test-rule
"#;

        let parser = RuleParser::new();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rule_not_object() {
        let yaml = r#"
rules:
  - "not an object"
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field_id() {
        let yaml = r#"
rules:
  - name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java]
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field_severity() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    languages: [java]
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field_languages() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_severity() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: INVALID
    languages: [java]
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_confidence() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    confidence: INVALID
    languages: [java]
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_ok());
        let rules = result.unwrap();
        assert_eq!(rules[0].confidence, Confidence::Medium);
    }

    #[test]
    fn test_parse_empty_languages_array() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: []
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_languages_not_array() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: "java"
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_yaml() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java
"#;

        let parser = RuleParser::new();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataflow_with_sources_and_sinks() {
        let yaml = r#"
rules:
  - id: dataflow-rule
    name: Dataflow Rule
    description: Rule with dataflow
    message: Rule with dataflow
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    dataflow:
      sources:
        - "request.getParameter(...)"
      sinks:
        - "Statement.execute(...)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].dataflow.is_some());
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sources.len(), 1);
        assert_eq!(df.sinks.len(), 1);
    }

    #[test]
    fn test_parse_dataflow_with_sanitizers() {
        let yaml = r#"
rules:
  - id: dataflow-sanitizer-rule
    name: Dataflow Sanitizer Rule
    description: Rule with dataflow sanitizers
    message: Rule with dataflow sanitizers
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    dataflow:
      sources:
        - "source()"
      sinks:
        - "sink()"
      sanitizers:
        - "sanitize()"
        - "escape()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sanitizers.len(), 2);
        assert_eq!(df.sanitizers[0], "sanitize()");
        assert_eq!(df.sanitizers[1], "escape()");
    }

    #[test]
    fn test_parse_dataflow_with_options() {
        let yaml = r#"
rules:
  - id: dataflow-options-rule
    name: Dataflow Options Rule
    description: Rule with dataflow options
    message: Rule with dataflow options
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    dataflow:
      sources:
        - "source()"
      sinks:
        - "sink()"
      must_flow: true
      max_depth: 10
      taint_assume_safe_booleans: true
      taint_assume_safe_numbers: false
      taint_only_propagate_through_assignments: true
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.must_flow, true);
        assert_eq!(df.max_depth, Some(10));
        assert_eq!(df.taint_assume_safe_booleans, Some(true));
        assert_eq!(df.taint_assume_safe_numbers, Some(false));
        assert_eq!(df.taint_only_propagate_through_assignments, Some(true));
    }

    #[test]
    fn test_parse_taint_mode_rule() {
        let yaml = r#"
rules:
  - id: taint-rule
    name: Taint Rule
    description: Rule in taint mode
    message: Rule in taint mode
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "request.getParameter($P)"
    pattern-sinks:
      - pattern: "Statement.execute($Q)"
    pattern-sanitizers:
      - pattern: "sanitize($X)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].mode, RuleMode::Taint);
        assert!(rules[0].dataflow.is_some());
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sources.len(), 1);
        assert_eq!(df.sinks.len(), 1);
        assert_eq!(df.sanitizers.len(), 1);
    }

    #[test]
    fn test_parse_options_block() {
        let yaml = r#"
rules:
  - id: options-rule
    name: Options Rule
    description: Rule with options
    message: Rule with options
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    options:
      sql_statement_boundary: true
      symbolic_propagation: "on"
      constant_propagation: "yes"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        // Options are merged into metadata
        assert!(rules[0].metadata.contains_key("sql_statement_boundary"));
        assert!(rules[0].metadata.contains_key("symbolic_propagation"));
        assert!(rules[0].metadata.contains_key("constant_propagation"));
    }

    #[test]
    fn test_parse_multiple_languages() {
        let yaml = r#"
rules:
  - id: multi-lang-rule
    name: Multi Language Rule
    description: Rule for multiple languages
    message: Rule for multiple languages
    severity: ERROR
    languages: [java, python, javascript]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].languages.len(), 3);
        assert!(rules[0].languages.contains(&Language::Java));
        assert!(rules[0].languages.contains(&Language::Python));
        assert!(rules[0].languages.contains(&Language::JavaScript));
    }

    #[test]
    fn test_parse_single_pattern_field() {
        let yaml = r#"
rules:
  - id: single-pattern-rule
    name: Single Pattern Rule
    description: Rule with single pattern field
    message: Rule with single pattern field
    severity: ERROR
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns.len(), 1);
        if let PatternType::Simple(s) = &rules[0].patterns[0].pattern_type {
            assert_eq!(s, "foo()");
        } else {
            panic!("Expected Simple pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_any() {
        let yaml = r#"
rules:
  - id: any-rule
    name: Any Rule
    description: Rule with pattern-any
    message: Rule with pattern-any
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-any:
          - "foo()"
          - "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::Any(sub_patterns) = &rules[0].patterns[0].pattern_type {
            assert_eq!(sub_patterns.len(), 2);
        } else {
            panic!("Expected Any pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_not_regex() {
        let yaml = r#"
rules:
  - id: not-regex-rule
    name: Not Regex Rule
    description: Rule with pattern-not-regex
    message: Rule with pattern-not-regex
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "foo()"
      - pattern-not-regex: "test_.*"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::All(sub_patterns) = &rules[0].patterns[0].pattern_type {
            if let PatternType::NotRegex(regex) = &sub_patterns[1].pattern_type {
                assert_eq!(regex, "test_.*");
            } else {
                panic!("Expected NotRegex pattern type");
            }
        } else {
            panic!("Expected All pattern type");
        }
    }

    #[test]
    fn test_parse_pattern_not_inside() {
        let yaml = r#"
rules:
  - id: not-inside-rule
    name: Not Inside Rule
    description: Rule with pattern-not-inside
    message: Rule with pattern-not-inside
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "foo()"
      - pattern-not-inside: "test()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        if let PatternType::All(sub_patterns) = &rules[0].patterns[0].pattern_type {
            if let PatternType::NotInside(inner) = &sub_patterns[1].pattern_type {
                if let PatternType::Simple(s) = &inner.pattern_type {
                    assert_eq!(s, "test()");
                } else {
                    panic!("Expected Simple inside NotInside");
                }
            } else {
                panic!("Expected NotInside pattern type");
            }
        } else {
            panic!("Expected All pattern type");
        }
    }

    #[test]
    fn test_parse_metavariable_analysis() {
        let yaml = r#"
rules:
  - id: metavar-analysis-rule
    name: Metavariable Analysis Rule
    description: Rule with metavariable-analysis
    message: Rule with metavariable-analysis
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
        metavariable-analysis:
          metavariable: "$X"
          entropy:
            min: 3.0
            max: 5.0
            charset: alphanumeric
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariableAnalysis(ma) = &main_pattern.conditions[0] {
            assert_eq!(ma.metavariable, "$X");
            assert!(ma.analysis.entropy.is_some());
            let entropy = ma.analysis.entropy.as_ref().unwrap();
            assert_eq!(entropy.min_entropy, 3.0);
            assert_eq!(entropy.max_entropy, Some(5.0));
            assert_eq!(entropy.charset, Some("alphanumeric".to_string()));
        } else {
            panic!("Expected MetavariableAnalysis condition");
        }
    }

    #[test]
    fn test_parse_taint_mode_with_options() {
        let yaml = r#"
rules:
  - id: taint-options-rule
    name: Taint Options Rule
    description: Taint rule with options
    message: Taint rule with options
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "source()"
    pattern-sinks:
      - pattern: "sink()"
    options:
      taint_assume_safe_booleans: true
      taint_assume_safe_numbers: false
      taint_only_propagate_through_assignments: true
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.taint_assume_safe_booleans, Some(true));
        assert_eq!(df.taint_assume_safe_numbers, Some(false));
        assert_eq!(df.taint_only_propagate_through_assignments, Some(true));
    }

    #[test]
    fn test_parse_default_values() {
        let yaml = r#"
rules:
  - id: minimal-rule
    name: Minimal Rule
    description: Minimal rule
    message: Minimal rule
    severity: ERROR
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(rule.confidence, Confidence::Medium);
        assert_eq!(rule.mode, RuleMode::Search);
        assert!(rule.enabled);
        assert!(rule.dataflow.is_none());
        assert!(rule.fix.is_none());
        assert!(rule.paths.is_none());
        assert!(rule.metadata.is_empty());
    }

    #[test]
    fn test_parse_empty_rules_array() {
        let yaml = r#"
rules: []
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_parse_non_strict_mode_skips_invalid_rules() {
        let yaml = r#"
rules:
  - id: valid-rule
    name: Valid Rule
    description: A valid rule
    message: A valid rule
    severity: ERROR
    languages: [java]
    pattern: "foo()"
  - id: invalid-rule
    name: Invalid Rule
    # Missing required fields
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "valid-rule");
    }

    #[test]
    fn test_parse_semgrep_internal_metavariable_name() {
        let yaml = r#"
rules:
  - id: internal-name-rule
    name: Internal Name Rule
    description: Rule with semgrep-internal-metavariable-name
    message: Rule with semgrep-internal-metavariable-name
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
      - semgrep-internal-metavariable-name:
          metavariable: "$X"
          fqn: "java.lang.String"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariableName(mn) = &main_pattern.conditions[0] {
            assert_eq!(mn.metavariable, "$X");
            assert_eq!(mn.fqn, Some("java.lang.String".to_string()));
        } else {
            panic!("Expected MetavariableName condition with FQN");
        }
    }

    // ======================================================================
    // NEW TESTS: coverage gaps for parser/parsing.rs public API
    // ======================================================================

    #[test]
    fn test_parse_taint_mode_with_propagators() {
        let yaml = r#"
rules:
  - id: taint-propagator-rule
    name: Taint Propagator Rule
    description: Taint rule with propagators
    message: Taint rule with propagators
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "request.getParameter($P)"
    pattern-sinks:
      - pattern: "stmt.execute($Q)"
    pattern-propagators:
      - pattern: "String $X = $Y + $Z"
        from: "$Y"
        to: "$X"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.propagators.len(), 1);
        assert_eq!(df.propagators[0].from, "$Y");
        assert_eq!(df.propagators[0].to, "$X");
        assert!(!df.propagators[0].is_fallback);
    }

    #[test]
    fn test_parse_taint_mode_with_sanitizers() {
        let yaml = r#"
rules:
  - id: taint-sanitizer-rule
    name: Taint Sanitizer Rule
    description: Taint rule with sanitizers
    message: Taint rule with sanitizers
    severity: WARNING
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getUserInput()"
    pattern-sinks:
      - pattern: "executeQuery($Q)"
    pattern-sanitizers:
      - pattern: "sanitize($X)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sanitizers.len(), 1);
        assert_eq!(df.sanitizers[0], "sanitize($VAR)");
    }

    #[test]
    fn test_parse_taint_source_with_patterns_array_and_focus() {
        let yaml = r#"
rules:
  - id: taint-source-focus-rule
    name: Source With Focus
    description: Taint source with focus metavariable
    message: Taint source with focus metavariable
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - patterns:
          - pattern: "request.getParameter($P)"
          - focus-metavariable: "$P"
    pattern-sinks:
      - pattern: "execute($Q)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sources.len(), 1);
        assert_eq!(df.sources[0].focus_metavariables, vec!["$P"]);
    }

    #[test]
    fn test_parse_taint_sink_with_pattern_either() {
        let yaml = r#"
rules:
  - id: taint-sink-either-rule
    name: Sink With Either
    description: Taint sink with pattern-either
    message: Taint sink with pattern-either
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getInput()"
    pattern-sinks:
      - pattern-either:
          - pattern: "execute($Q)"
          - pattern: "query($Q)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sinks.len(), 1);
        match &df.sinks[0].pattern.pattern_type {
            PatternType::Either(patterns) => assert_eq!(patterns.len(), 2),
            other => panic!("Expected Either pattern type, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_taint_source_as_simple_string() {
        let yaml = r#"
rules:
  - id: taint-simple-source
    name: Simple Source
    description: Source as plain string
    message: Source as plain string
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - "getInput()"
    pattern-sinks:
      - pattern: "execute($Q)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sources.len(), 1);
        assert_eq!(df.sources[0].pattern_text(), "getInput()");
    }

    #[test]
    fn test_parse_taint_sink_with_patterns_array_and_focus() {
        let yaml = r#"
rules:
  - id: taint-sink-focus
    name: Sink With Focus
    description: Sink with focus metavariable
    message: Sink with focus metavariable
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getInput()"
    pattern-sinks:
      - patterns:
          - pattern: "$STMT.execute($Q)"
          - focus-metavariable: "$Q"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sinks.len(), 1);
        assert_eq!(df.sinks[0].focus_metavariables, vec!["$Q"]);
    }

    #[test]
    fn test_parse_dataflow_with_max_depth_and_must_flow() {
        let yaml = r#"
rules:
  - id: dataflow-opts-rule
    name: Dataflow Options Rule
    description: Dataflow with numeric and boolean options
    message: Dataflow with numeric and boolean options
    severity: ERROR
    languages: [java]
    pattern: "test()"
    dataflow:
      sources:
        - "src()"
      sinks:
        - "sink()"
      must_flow: false
      max_depth: 15
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert!(!df.must_flow);
        assert_eq!(df.max_depth, Some(15));
    }

    #[test]
    fn test_parse_dataflow_with_taint_assume_options() {
        let yaml = r#"
rules:
  - id: dataflow-taint-opts
    name: Dataflow Taint Options
    description: Dataflow with taint assume options
    message: Dataflow with taint assume options
    severity: ERROR
    languages: [java]
    pattern: "test()"
    dataflow:
      sources:
        - "src()"
      sinks:
        - "sink()"
      taint_assume_safe_booleans: true
      taint_assume_safe_numbers: true
      taint_only_propagate_through_assignments: false
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.taint_assume_safe_booleans, Some(true));
        assert_eq!(df.taint_assume_safe_numbers, Some(true));
        assert_eq!(df.taint_only_propagate_through_assignments, Some(false));
    }

    #[test]
    fn test_parse_pattern_inside_object_form() {
        let yaml = r#"
rules:
  - id: inside-obj-rule
    name: Inside Object Rule
    description: Pattern-inside as object with nested patterns
    message: Pattern-inside as object
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-inside:
          patterns:
            - pattern: "class $CLASS { ... }"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].patterns[0].pattern_type {
            PatternType::Inside(inner) => match &inner.pattern_type {
                PatternType::Simple(s) => assert_eq!(s, "class $CLASS { ... }"),
                other => panic!("Expected Simple inside Inside, got {:?}", other),
            },
            other => panic!("Expected Inside pattern type, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_pattern_not_inside_object_form() {
        let yaml = r#"
rules:
  - id: not-inside-obj-rule
    name: Not Inside Object Rule
    description: Pattern-not-inside as object with nested patterns
    message: Pattern-not-inside as object
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-not-inside:
          patterns:
            - pattern: "class $CLASS { ... }"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].patterns[0].pattern_type {
            PatternType::NotInside(inner) => match &inner.pattern_type {
                PatternType::Simple(s) => assert_eq!(s, "class $CLASS { ... }"),
                other => panic!("Expected Simple inside NotInside, got {:?}", other),
            },
            other => panic!("Expected NotInside pattern type, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_metavariable_pattern_with_regex_constraint() {
        let yaml = r#"
rules:
  - id: metavar-regex-rule
    name: Metavar Regex Rule
    description: Metavariable pattern with regex
    message: Metavariable pattern with regex
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable-pattern:
          metavariable: "$QUERY"
          regex: "SELECT .* FROM .*"
          patterns:
            - pattern: "$STR + $INPUT"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.regex.as_deref() == Some("SELECT .* FROM .*"))
        });
        assert!(found, "Expected MetavariablePattern condition with regex");
    }

    #[test]
    fn test_parse_metavariable_pattern_with_type_constraint() {
        let yaml = r#"
rules:
  - id: metavar-type-rule
    name: Metavar Type Rule
    description: Metavariable pattern with type
    message: Metavariable pattern with type
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X.method()"
      - metavariable-pattern:
          metavariable: "$X"
          type: "String"
          patterns:
            - pattern: "$X"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.type_constraint.as_deref() == Some("String"))
        });
        assert!(found, "Expected MetavariablePattern condition with type");
    }

    #[test]
    fn test_parse_metavariable_pattern_with_name_constraint() {
        let yaml = r#"
rules:
  - id: metavar-name-rule
    name: Metavar Name Rule
    description: Metavariable pattern with name constraint
    message: Metavariable pattern with name constraint
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X.call()"
      - metavariable-pattern:
          metavariable: "$X"
          name: ".*Service"
          patterns:
            - pattern: "$X"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.name_constraint.as_deref() == Some(".*Service"))
        });
        assert!(found, "Expected MetavariablePattern condition with name");
    }

    #[test]
    fn test_parse_metavariable_pattern_with_analysis_block() {
        let yaml = r#"
rules:
  - id: metavar-analysis-in-pattern
    name: Metavar Analysis in Pattern
    description: Metavariable pattern with analysis
    message: Metavariable pattern with analysis
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$X"
      - metavariable-pattern:
          metavariable: "$X"
          analysis:
            entropy:
              min: 4.0
              charset: base64
          patterns:
            - pattern: "$X"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.analysis.is_some())
        });
        assert!(found, "Expected MetavariablePattern condition with analysis");
    }

    #[test]
    fn test_parse_metavariable_pattern_with_pattern_either() {
        let yaml = r#"
rules:
  - id: metavar-either-rule
    name: Metavar Either Rule
    description: Metavariable pattern with pattern-either
    message: Metavariable pattern with pattern-either
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable-pattern:
          metavariable: "$QUERY"
          pattern-either:
            - pattern: "$STR + $INPUT"
            - pattern: "String.format($FMT, $ARGS)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.patterns.len() == 2)
        });
        assert!(found, "Expected MetavariablePattern with 2 patterns from pattern-either");
    }

    #[test]
    fn test_parse_options_sql_statement_boundary() {
        let yaml = r#"
rules:
  - id: sql-boundary-rule
    name: SQL Boundary Rule
    description: Rule with SQL boundary option
    message: Rule with SQL boundary option
    severity: ERROR
    languages: [java]
    pattern: "test()"
    options:
      sql_statement_boundary: true
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(
            rule.metadata.get("sql_statement_boundary"),
            Some(&Value::String("true".to_string()))
        );
    }

    #[test]
    fn test_parse_options_symbolic_propagation() {
        let yaml = r#"
rules:
  - id: symbolic-prop-rule
    name: Symbolic Propagation Rule
    description: Rule with symbolic propagation option
    message: Rule with symbolic propagation option
    severity: ERROR
    languages: [java]
    pattern: "test()"
    options:
      symbolic_propagation: false
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(
            rule.metadata.get("symbolic_propagation"),
            Some(&Value::String("false".to_string()))
        );
    }

    #[test]
    fn test_parse_options_constant_propagation() {
        let yaml = r#"
rules:
  - id: const-prop-rule
    name: Constant Propagation Rule
    description: Rule with constant propagation option
    message: Rule with constant propagation option
    severity: ERROR
    languages: [java]
    pattern: "test()"
    options:
      constant_propagation: true
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(
            rule.metadata.get("constant_propagation"),
            Some(&Value::String("true".to_string()))
        );
    }

    #[test]
    fn test_parse_options_string_on_off_values() {
        let yaml = r#"
rules:
  - id: string-opts-rule
    name: String Options Rule
    description: Rule with string option values
    message: Rule with string option values
    severity: ERROR
    languages: [java]
    pattern: "test()"
    options:
      sql_statement_boundary: "on"
      symbolic_propagation: "off"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(
            rule.metadata.get("sql_statement_boundary"),
            Some(&Value::String("true".to_string()))
        );
        assert_eq!(
            rule.metadata.get("symbolic_propagation"),
            Some(&Value::String("false".to_string()))
        );
    }

    #[test]
    fn test_parse_strict_mode_returns_error_on_first_invalid() {
        let yaml = r#"
rules:
  - id: invalid-rule
    name: Invalid Rule
    # Missing severity, languages, message
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_non_strict_mode_warns_but_continues() {
        let yaml = r#"
rules:
  - id: invalid-rule
    name: Invalid Rule
    # Missing required fields
  - id: valid-rule
    name: Valid Rule
    description: A valid rule
    message: A valid rule
    severity: ERROR
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "valid-rule");
    }

    #[test]
    fn test_parse_multiple_rules() {
        let yaml = r#"
rules:
  - id: rule-one
    name: Rule One
    description: First rule
    message: First rule
    severity: ERROR
    languages: [java]
    pattern: "foo()"
  - id: rule-two
    name: Rule Two
    description: Second rule
    message: Second rule
    severity: WARNING
    languages: [python]
    pattern: "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "rule-one");
        assert_eq!(rules[1].id, "rule-two");
        assert_eq!(rules[0].severity, Severity::Error);
        assert_eq!(rules[1].severity, Severity::Warning);
        assert_eq!(rules[0].languages, vec![Language::Java]);
        assert_eq!(rules[1].languages, vec![Language::Python]);
    }

    #[test]
    fn test_parse_rule_with_name_and_description_override() {
        let yaml = r#"
rules:
  - id: test-rule
    name: Custom Name
    description: Custom description
    message: Default message
    severity: ERROR
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert_eq!(rule.name, "Custom Name");
        assert_eq!(rule.description, "Custom description");
    }

    #[test]
    fn test_parse_rule_defaults_name_to_id() {
        let yaml = r#"
rules:
  - id: my-rule-id
    message: Some message
    severity: ERROR
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].name, "my-rule-id");
        assert_eq!(rules[0].description, "Some message");
    }

    #[test]
    fn test_parse_pattern_regex_at_top_level() {
        let yaml = r#"
rules:
  - id: top-level-regex
    name: Top Level Regex
    description: Regex at top level
    message: Regex at top level
    severity: ERROR
    languages: [java]
    pattern-regex: "password\\s*=\\s*.*"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].patterns[0].pattern_type {
            PatternType::Regex(re) => assert_eq!(re, "password\\s*=\\s*.*"),
            other => panic!("Expected Regex pattern type, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_metavariable_analysis_with_type_and_complexity() {
        let yaml = r#"
rules:
  - id: full-analysis-rule
    name: Full Analysis Rule
    description: Rule with full analysis
    message: Rule with full analysis
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$FUNC(...)"
        metavariable-analysis:
          metavariable: "$FUNC"
          type:
            expected:
              - "String"
              - "Integer"
            forbidden:
              - "Object"
            nullable: false
          complexity:
            max_cyclomatic: 10
            max_nesting_depth: 5
            max_lines: 50
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        let main_pattern = &rules[0].patterns[0];
        assert!(!main_pattern.conditions.is_empty());
        if let Condition::MetavariableAnalysis(ma) = &main_pattern.conditions[0] {
            assert_eq!(ma.metavariable, "$FUNC");
            assert!(ma.analysis.type_analysis.is_some());
            let ta = ma.analysis.type_analysis.as_ref().unwrap();
            assert_eq!(ta.expected_types, vec!["String", "Integer"]);
            assert_eq!(ta.forbidden_types, vec!["Object"]);
            assert_eq!(ta.nullable, Some(false));
            assert!(ma.analysis.complexity.is_some());
            let ca = ma.analysis.complexity.as_ref().unwrap();
            assert_eq!(ca.max_cyclomatic, Some(10));
            assert_eq!(ca.max_nesting_depth, Some(5));
            assert_eq!(ca.max_lines, Some(50));
        } else {
            panic!("Expected MetavariableAnalysis condition");
        }
    }

    #[test]
    fn test_parse_pattern_all_with_multiple_patterns() {
        let yaml = r#"
rules:
  - id: all-multi-rule
    name: All Multi Rule
    description: Pattern-all with multiple patterns
    message: Pattern-all with multiple patterns
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-all:
          - pattern: "foo()"
          - pattern: "bar()"
          - pattern: "baz()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].patterns[0].pattern_type {
            PatternType::All(patterns) => assert_eq!(patterns.len(), 3),
            other => panic!("Expected All pattern type, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_metavariable_pattern_not_in_patterns_array() {
        let yaml = r#"
rules:
  - id: metavar-not-rule
    name: Metavar Not Rule
    description: Metavariable pattern with pattern-not
    message: Metavariable pattern with pattern-not
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable-pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern: "$STR + $INPUT"
            - pattern-not: "constantString()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let main_pattern = &rules[0].patterns[0];
        let found = main_pattern.conditions.iter().any(|c| {
            matches!(c, Condition::MetavariablePattern(mp) if mp.patterns.iter().any(|p| p.starts_with("__NOT__:")))
        });
        assert!(found, "Expected MetavariablePattern with __NOT__ prefixed pattern");
    }

    #[test]
    fn test_parse_options_not_an_object_error() {
        let yaml = r#"
rules:
  - id: bad-opts-rule
    name: Bad Options
    description: Options not an object
    message: Options not an object
    severity: ERROR
    languages: [java]
    pattern: "test()"
    options: "not_an_object"
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dataflow_not_an_object_error() {
        let yaml = r#"
rules:
  - id: bad-dataflow-rule
    name: Bad Dataflow
    description: Dataflow not an object
    message: Dataflow not an object
    severity: ERROR
    languages: [java]
    pattern: "test()"
    dataflow: "not_an_object"
"#;

        let parser = RuleParser::strict();
        let result = parser.parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_fix_regex_fields() {
        let yaml = r#"
rules:
  - id: fix-regex-rule
    name: Fix Regex Rule
    description: Rule with fix-regex
    message: Rule with fix-regex
    severity: ERROR
    languages: [java]
    pattern: "old_pattern"
    fix-regex:
      regex: "old_pattern"
      replacement: "new_pattern"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert!(rule.fix_regex.is_some());
        let fr = rule.fix_regex.as_ref().unwrap();
        assert_eq!(fr.regex, "old_pattern");
        assert_eq!(fr.replacement, "new_pattern");
    }

    #[test]
    fn test_parse_paths_with_include_and_exclude() {
        let yaml = r#"
rules:
  - id: paths-rule
    name: Paths Rule
    description: Rule with paths include and exclude
    message: Rule with paths
    severity: ERROR
    languages: [java]
    pattern: "foo()"
    paths:
      include:
        - "src/**/*.java"
        - "test/**/*.java"
      exclude:
        - "src/generated/**"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let rule = &rules[0];
        assert!(rule.paths.is_some());
        let paths = rule.paths.as_ref().unwrap();
        assert_eq!(paths.includes.len(), 2);
        assert_eq!(paths.excludes.len(), 1);
        assert_eq!(paths.includes[0], "src/**/*.java");
        assert_eq!(paths.excludes[0], "src/generated/**");
    }

    #[test]
    fn test_parse_rule_with_all_severity_levels() {
        let yaml = r#"
rules:
  - id: critical-rule
    name: Critical Rule
    description: Critical severity rule
    message: Critical
    severity: CRITICAL
    languages: [java]
    pattern: "foo()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].severity, Severity::Critical);
    }

    #[test]
    fn test_parse_rule_with_info_severity() {
        let yaml = r#"
rules:
  - id: info-rule
    name: Info Rule
    description: Info severity rule
    message: Info
    severity: INFO
    languages: [python]
    pattern: "bar()"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules[0].severity, Severity::Info);
    }

    #[test]
    fn test_parse_source_pattern_with_is_fallback() {
        let yaml = r#"
rules:
  - id: fallback-source-rule
    name: Fallback Source Rule
    description: Source with is_fallback flag
    message: Source with is_fallback
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getInput()"
        is_fallback: true
    pattern-sinks:
      - pattern: "execute($Q)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sources.len(), 1);
        assert!(df.sources[0].is_fallback);
    }

    #[test]
    fn test_parse_sink_pattern_with_is_fallback() {
        let yaml = r#"
rules:
  - id: fallback-sink-rule
    name: Fallback Sink Rule
    description: Sink with is_fallback flag
    message: Sink with is_fallback
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getInput()"
    pattern-sinks:
      - pattern: "execute($Q)"
        is_fallback: true
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.sinks.len(), 1);
        assert!(df.sinks[0].is_fallback);
    }

    #[test]
    fn test_parse_propagator_with_patterns_array() {
        let yaml = r#"
rules:
  - id: propagator-patterns-rule
    name: Propagator Patterns Rule
    description: Propagator with patterns array
    message: Propagator with patterns array
    severity: ERROR
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "getInput()"
    pattern-sinks:
      - pattern: "execute($Q)"
    pattern-propagators:
      - patterns:
          - pattern: "StringBuilder $SB = new StringBuilder($X)"
        from: "$X"
        to: "$SB"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        let df = rules[0].dataflow.as_ref().unwrap();
        assert_eq!(df.propagators.len(), 1);
        assert_eq!(df.propagators[0].from, "$X");
        assert_eq!(df.propagators[0].to, "$SB");
    }

    #[test]
    fn test_parse_taint_mode_without_sources_returns_empty_patterns() {
        let yaml = r#"
rules:
  - id: taint-no-source
    name: Taint No Source
    description: Taint mode without sources/sinks
    message: Taint without sources or sinks
    severity: ERROR
    languages: [java]
    mode: taint
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].patterns.is_empty());
        assert!(rules[0].dataflow.is_none());
    }

    #[test]
    fn test_parse_match_object_with_pattern_field() {
        let yaml = r#"
rules:
  - id: match-obj-pattern
    name: Match Object Pattern
    description: Match as object with pattern field
    message: Match as object with pattern field
    severity: ERROR
    languages: [java]
    match:
      pattern: "System.out.println($MSG)"
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns.len(), 1);
        match &rules[0].patterns[0].pattern_type {
            PatternType::Simple(s) => assert_eq!(s, "System.out.println($MSG)"),
            other => panic!("Expected Simple pattern type, got {:?}", other),
        }
    }
}
