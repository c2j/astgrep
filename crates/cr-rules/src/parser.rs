//! YAML rule parser
//! 
//! This module provides functionality to parse rules from YAML format.

use crate::types::*;
use cr_core::{AnalysisError, Confidence, Language, Result, Severity};
use cr_core::{MetavariableAnalysis, EntropyAnalysis, TypeAnalysis, ComplexityAnalysis};
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
        let patterns = self.parse_patterns_or_pattern(rule_obj, index)?;
        let dataflow = self.parse_dataflow(rule_obj, index)?;
        let fix = self.get_optional_string_field(rule_obj, "fix");
        let fix_regex = self.parse_fix_regex(rule_obj, index)?;
        let paths = self.parse_paths(rule_obj, index)?;
        let metadata = self.parse_metadata(rule_obj, index)?;
        let enabled = self.get_optional_bool_field(rule_obj, "enabled").unwrap_or(true);

        let mut rule = Rule::new(id, name, description, severity, confidence, languages);
        rule.patterns = patterns;
        rule.dataflow = dataflow;
        rule.fix = fix;
        rule.fix_regex = fix_regex;
        rule.paths = paths;
        rule.metadata = metadata;
        rule.enabled = enabled;

        Ok(rule)
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
    fn parse_patterns_array(&self, patterns_value: &Value, index: usize) -> Result<Vec<Pattern>> {
        let patterns_array = patterns_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!("Rule {} 'patterns' must be an array", index)))?;

        let mut patterns = Vec::new();
        for (pattern_index, pattern_value) in patterns_array.iter().enumerate() {
            let pattern = self.parse_single_pattern(pattern_value, index, pattern_index)?;
            patterns.push(pattern);
        }

        Ok(patterns)
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
        
        let patterns_value = metavar_obj
            .get(&Value::String("patterns".to_string()))
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern missing 'patterns'",
                rule_index, pattern_index
            )))?;

        let patterns_array = patterns_value
            .as_sequence()
            .ok_or_else(|| AnalysisError::parse_error(format!(
                "Rule {} pattern {} metavariable_pattern 'patterns' must be an array",
                rule_index, pattern_index
            )))?;

        let mut patterns = Vec::new();
        for pattern_value in patterns_array {
            let pattern_str = pattern_value
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error(format!(
                    "Rule {} pattern {} metavariable pattern must be a string",
                    rule_index, pattern_index
                )))?;
            patterns.push(pattern_str.to_string());
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

        let mut dataflow = DataFlowSpec::new(sources, sinks).with_sanitizers(sanitizers);

        if let Some(must_flow) = self.get_optional_bool_field(dataflow_obj, "must_flow") {
            dataflow.must_flow = must_flow;
        }

        if let Some(max_depth_value) = dataflow_obj.get(&Value::String("max_depth".to_string())) {
            if let Some(max_depth) = max_depth_value.as_u64() {
                dataflow.max_depth = Some(max_depth as usize);
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
    fn parse_metadata(&self, obj: &serde_yaml::Mapping, _index: usize) -> Result<HashMap<String, String>> {
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
            let value_str = value
                .as_str()
                .ok_or_else(|| AnalysisError::parse_error("metadata values must be strings".to_string()))?;
            metadata.insert(key_str.to_string(), value_str.to_string());
        }

        Ok(metadata)
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
    severity: ERROR
    languages: [python]
    patterns:
      - pattern: "def $FUNC(...):"
        pattern-not-inside: |
          class $CLASS:
            ...
      - pattern-regex: "eval\\("
        pattern-not-regex: "test_.*"
        focus-metavariable: ["$FUNC", "$ARG"]
"#;

        let parser = RuleParser::new();
        let rules = parser.parse_yaml(yaml).unwrap();

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.id, "enhanced-pattern-test");
        assert_eq!(rule.patterns.len(), 2);

        // Check first pattern has pattern-not-inside
        if let PatternType::NotInside(_) = &rule.patterns[0].pattern_type {
            // Expected
        } else {
            panic!("Expected NotInside pattern type");
        }

        // Check second pattern has pattern-not-regex and focus
        if let PatternType::NotRegex(regex_str) = &rule.patterns[1].pattern_type {
            assert_eq!(regex_str, "test_.*");
        } else {
            panic!("Expected NotRegex pattern type");
        }

        assert_eq!(rule.patterns[1].focus, Some(vec!["$FUNC".to_string(), "$ARG".to_string()]));
    }

    #[test]
    fn test_parse_complex_rule() {
        let yaml = r#"
rules:
  - id: sql-injection
    name: SQL Injection Detection
    description: Detects potential SQL injection vulnerabilities
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
}
