//! Rule validation
//! 
//! This module provides functionality to validate rules for correctness and consistency.

use crate::types::*;
use astgrep_core::{AnalysisError, Result};
use std::collections::{HashMap, HashSet};

/// Rule validator
pub struct RuleValidator {
    strict_validation: bool,
    custom_validators: HashMap<String, Box<dyn Fn(&Rule) -> Result<()> + Send + Sync>>,
}

impl RuleValidator {
    /// Create a new rule validator
    pub fn new() -> Self {
        Self {
            strict_validation: false,
            custom_validators: HashMap::new(),
        }
    }

    /// Create a validator with strict validation enabled
    pub fn strict() -> Self {
        Self {
            strict_validation: true,
            custom_validators: HashMap::new(),
        }
    }

    /// Add a custom validator function
    pub fn add_custom_validator<F>(&mut self, name: String, validator: F)
    where
        F: Fn(&Rule) -> Result<()> + Send + Sync + 'static,
    {
        self.custom_validators.insert(name, Box::new(validator));
    }

    /// Validate a single rule
    pub fn validate_rule(&self, rule: &Rule) -> Result<()> {
        self.validate_basic_fields(rule)?;
        self.validate_patterns(rule)?;
        self.validate_dataflow(rule)?;
        self.validate_metadata(rule)?;
        self.validate_consistency(rule)?;

        // Run custom validators
        for (name, validator) in &self.custom_validators {
            validator(rule).map_err(|e| {
                AnalysisError::rule_validation_error(format!("Custom validator '{}' failed: {}", name, e))
            })?;
        }

        Ok(())
    }

    /// Validate multiple rules for consistency
    pub fn validate_rules(&self, rules: &[Rule]) -> Result<()> {
        // Validate each rule individually
        for rule in rules {
            self.validate_rule(rule)?;
        }

        // Validate rule set consistency
        self.validate_rule_ids(rules)?;
        self.validate_rule_dependencies(rules)?;

        Ok(())
    }

    /// Validate basic rule fields
    fn validate_basic_fields(&self, rule: &Rule) -> Result<()> {
        // Validate ID
        if rule.id.is_empty() {
            return Err(AnalysisError::rule_validation_error("Rule ID cannot be empty"));
        }

        if !rule.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(AnalysisError::rule_validation_error(
                "Rule ID can only contain alphanumeric characters, hyphens, and underscores"
            ));
        }

        // Validate name (note: name is auto-generated from id if not provided during parsing)
        if rule.name.is_empty() {
            return Err(AnalysisError::rule_validation_error("Rule name cannot be empty"));
        }

        if rule.name.len() > 100 {
            return Err(AnalysisError::rule_validation_error("Rule name too long (max 100 characters)"));
        }

        // Validate description (note: description is auto-generated from message if not provided during parsing)
        if rule.description.is_empty() {
            return Err(AnalysisError::rule_validation_error("Rule description cannot be empty"));
        }

        if rule.description.len() > 1000 {
            return Err(AnalysisError::rule_validation_error("Rule description too long (max 1000 characters)"));
        }

        // Validate languages
        if rule.languages.is_empty() {
            return Err(AnalysisError::rule_validation_error("Rule must specify at least one language"));
        }

        Ok(())
    }

    /// Validate rule patterns
    fn validate_patterns(&self, rule: &Rule) -> Result<()> {
        if rule.patterns.is_empty() && rule.dataflow.is_none() {
            return Err(AnalysisError::rule_validation_error(
                "Rule must have either patterns or dataflow specification"
            ));
        }

        for (index, pattern) in rule.patterns.iter().enumerate() {
            self.validate_pattern(pattern, index)?;
        }

        Ok(())
    }

    /// Validate a single pattern
    fn validate_pattern(&self, pattern: &Pattern, index: usize) -> Result<()> {
        // Validate pattern syntax
        if let Some(pattern_str) = pattern.get_pattern_string() {
            if pattern_str.is_empty() {
                return Err(AnalysisError::rule_validation_error(format!(
                    "Pattern {} cannot be empty", index
                )));
            }
            // Check for balanced metavariables
            self.validate_metavariables(pattern_str, index)?;
        }

        // Validate metavariable pattern if present
        if let Some(ref metavar_pattern) = pattern.metavariable_pattern {
            self.validate_metavariable_pattern(metavar_pattern, index)?;
        }

        // Validate conditions
        for (cond_index, condition) in pattern.conditions.iter().enumerate() {
            self.validate_condition(condition, index, cond_index)?;
        }

        Ok(())
    }

    /// Validate metavariables in a pattern
    fn validate_metavariables(&self, pattern: &str, pattern_index: usize) -> Result<()> {
        let mut metavars = HashSet::new();
        let mut chars = pattern.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut metavar = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        metavar.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                
                if !metavar.is_empty() {
                    metavars.insert(metavar);
                }
            }
        }

        // Check for common metavariable naming issues
        for metavar in &metavars {
            // Allow single-character metavariables like $X for Semgrep compatibility
            // Only enforce uppercase-start in strict mode
            if !metavar.chars().next().unwrap().is_uppercase() {
                if self.strict_validation {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} metavariable '{}' should start with uppercase letter",
                        pattern_index, metavar
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate metavariable pattern
    fn validate_metavariable_pattern(&self, metavar_pattern: &MetavariablePattern, pattern_index: usize) -> Result<()> {
        if metavar_pattern.metavariable.is_empty() {
            return Err(AnalysisError::rule_validation_error(format!(
                "Pattern {} metavariable name cannot be empty", pattern_index
            )));
        }

        if !metavar_pattern.metavariable.starts_with('$') {
            return Err(AnalysisError::rule_validation_error(format!(
                "Pattern {} metavariable '{}' must start with '$'",
                pattern_index, metavar_pattern.metavariable
            )));
        }

        if metavar_pattern.patterns.is_empty() {
            return Err(AnalysisError::rule_validation_error(format!(
                "Pattern {} metavariable pattern must have at least one pattern",
                pattern_index
            )));
        }

        // Validate regex if present
        if let Some(ref regex) = metavar_pattern.regex {
            if let Err(e) = regex::Regex::new(regex) {
                return Err(AnalysisError::rule_validation_error(format!(
                    "Pattern {} metavariable regex invalid: {}",
                    pattern_index, e
                )));
            }
        }

        Ok(())
    }

    /// Validate a condition
    fn validate_condition(&self, condition: &Condition, pattern_index: usize, condition_index: usize) -> Result<()> {
        match condition {
            Condition::MetavariableRegex(metavar_regex) => {
                if metavar_regex.metavariable.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} metavariable cannot be empty",
                        pattern_index, condition_index
                    )));
                }
                if metavar_regex.regex.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} regex cannot be empty",
                        pattern_index, condition_index
                    )));
                }
                // Validate regex syntax
                if let Err(e) = regex::Regex::new(&metavar_regex.regex) {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} regex invalid: {}",
                        pattern_index, condition_index, e
                    )));
                }
            }
            Condition::MetavariableComparison(metavar_comp) => {
                if metavar_comp.metavariable.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} metavariable cannot be empty",
                        pattern_index, condition_index
                    )));
                }
                if metavar_comp.value.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} value cannot be empty",
                        pattern_index, condition_index
                    )));
                }
            }
            Condition::NodeType(node_type) => {
                if node_type.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} node type cannot be empty",
                        pattern_index, condition_index
                    )));
                }
            }
            Condition::NodeAttribute(attr_name, attr_value) => {
                if attr_name.is_empty() || attr_value.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} attribute name and value cannot be empty",
                        pattern_index, condition_index
                    )));
                }
            }
            Condition::MetavariableName(metavar_name) => {
                // Validate metavariable name constraint
                if metavar_name.metavariable.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} metavariable name cannot be empty",
                        pattern_index, condition_index
                    )));
                }
                if metavar_name.name_pattern.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} name pattern cannot be empty",
                        pattern_index, condition_index
                    )));
                }
            }
            Condition::MetavariableAnalysis(metavar_analysis) => {
                // Validate metavariable analysis constraint
                if metavar_analysis.metavariable.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} metavariable name cannot be empty",
                        pattern_index, condition_index
                    )));
                }
                // Additional validation for analysis configuration could be added here
            }
            Condition::Custom(custom_condition) => {
                if custom_condition.is_empty() {
                    return Err(AnalysisError::rule_validation_error(format!(
                        "Pattern {} condition {} custom condition cannot be empty",
                        pattern_index, condition_index
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate dataflow specification
    fn validate_dataflow(&self, rule: &Rule) -> Result<()> {
        if let Some(ref dataflow) = rule.dataflow {
            if dataflow.sources.is_empty() {
                return Err(AnalysisError::rule_validation_error("Dataflow sources cannot be empty"));
            }

            if dataflow.sinks.is_empty() {
                return Err(AnalysisError::rule_validation_error("Dataflow sinks cannot be empty"));
            }

            // Validate max_depth
            if let Some(max_depth) = dataflow.max_depth {
                if max_depth == 0 {
                    return Err(AnalysisError::rule_validation_error("Dataflow max_depth must be greater than 0"));
                }
                if max_depth > 1000 {
                    return Err(AnalysisError::rule_validation_error("Dataflow max_depth too large (max 1000)"));
                }
            }
        }

        Ok(())
    }

    /// Validate metadata
    fn validate_metadata(&self, rule: &Rule) -> Result<()> {
        for (key, value) in &rule.metadata {
            if key.is_empty() {
                return Err(AnalysisError::rule_validation_error("Metadata key cannot be empty"));
            }

            if value.is_empty() {
                return Err(AnalysisError::rule_validation_error(format!(
                    "Metadata value for key '{}' cannot be empty", key
                )));
            }

            // Validate specific metadata fields
            if key == "cwe" && !value.starts_with("CWE-") {
                return Err(AnalysisError::rule_validation_error(
                    "CWE metadata should start with 'CWE-'"
                ));
            }
        }

        Ok(())
    }

    /// Validate rule internal consistency
    fn validate_consistency(&self, rule: &Rule) -> Result<()> {
        // This validation is already done in validate_patterns
        // Keep this method for future consistency checks

        // Validate severity-confidence combinations
        if rule.severity == astgrep_core::Severity::Critical && rule.confidence == astgrep_core::Confidence::Low {
            if self.strict_validation {
                return Err(AnalysisError::rule_validation_error(
                    "Critical severity with low confidence is discouraged"
                ));
            }
        }

        Ok(())
    }

    /// Validate rule IDs for uniqueness
    fn validate_rule_ids(&self, rules: &[Rule]) -> Result<()> {
        let mut seen_ids = HashSet::new();
        
        for rule in rules {
            if seen_ids.contains(&rule.id) {
                return Err(AnalysisError::rule_validation_error(format!(
                    "Duplicate rule ID: {}", rule.id
                )));
            }
            seen_ids.insert(&rule.id);
        }

        Ok(())
    }

    /// Validate rule dependencies (placeholder for future enhancement)
    fn validate_rule_dependencies(&self, _rules: &[Rule]) -> Result<()> {
        // Future: validate rule dependencies, inheritance, etc.
        Ok(())
    }
}

impl Default for RuleValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::{Confidence, Language, Severity};

    fn create_valid_rule() -> Rule {
        Rule::new(
            "test-rule".to_string(),
            "Test Rule".to_string(),
            "A valid test rule".to_string(),
            Severity::Error,
            Confidence::High,
            vec![Language::Java],
        )
        .add_pattern(Pattern::simple("System.out.println($MSG)".to_string()))
    }

    #[test]
    fn test_validate_valid_rule() {
        let validator = RuleValidator::new();
        let rule = create_valid_rule();
        
        assert!(validator.validate_rule(&rule).is_ok());
    }

    #[test]
    fn test_validate_empty_id() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.id = String::new();
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_invalid_id_characters() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.id = "invalid@id".to_string();
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_empty_name() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.name = String::new();
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_long_name() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.name = "a".repeat(101);
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_empty_description() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.description = String::new();
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_no_languages() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.languages = Vec::new();
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_no_patterns_or_dataflow() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        rule.patterns = Vec::new();
        rule.dataflow = None;

        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_invalid_metavariable_regex() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        
        let metavar_pattern = MetavariablePattern::new(
            "$VAR".to_string(),
            vec!["pattern".to_string()],
        ).with_regex("[invalid regex".to_string());
        
        let pattern = Pattern::simple("test($VAR)".to_string());
        rule.patterns = vec![Pattern {
            pattern_type: pattern.pattern_type,
            metavariable_pattern: Some(metavar_pattern),
            conditions: Vec::new(),
            focus: None,
        }];
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_duplicate_rule_ids() {
        let validator = RuleValidator::new();
        let rule1 = create_valid_rule();
        let rule2 = create_valid_rule(); // Same ID
        
        assert!(validator.validate_rules(&[rule1, rule2]).is_err());
    }

    #[test]
    fn test_validate_dataflow_empty_sources() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        
        let dataflow = DataFlowSpec::new(Vec::new(), vec!["sink".to_string()]);
        rule.dataflow = Some(dataflow);
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_validate_dataflow_zero_max_depth() {
        let validator = RuleValidator::new();
        let mut rule = create_valid_rule();
        
        let dataflow = DataFlowSpec::new(
            vec!["source".to_string()],
            vec!["sink".to_string()],
        ).with_max_depth(0);
        rule.dataflow = Some(dataflow);
        
        assert!(validator.validate_rule(&rule).is_err());
    }

    #[test]
    fn test_custom_validator() {
        let mut validator = RuleValidator::new();
        validator.add_custom_validator(
            "test_validator".to_string(),
            |rule| {
                if rule.id.contains("forbidden") {
                    Err(AnalysisError::rule_validation_error("Forbidden ID pattern"))
                } else {
                    Ok(())
                }
            },
        );

        let mut rule = create_valid_rule();
        rule.id = "forbidden-rule".to_string();
        
        assert!(validator.validate_rule(&rule).is_err());
    }
}
