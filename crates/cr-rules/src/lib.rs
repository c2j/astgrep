//! Rule parsing and execution engine for CR-SemService
//!
//! This crate provides rule parsing, validation, and execution functionality.

pub mod parser;
pub mod validator;
pub mod engine;
pub mod executor;
pub mod integration;
pub mod types;
pub mod marketplace;

pub use parser::*;
pub use validator::*;
pub use engine::*;
pub use executor::*;
pub use integration::*;
pub use types::*;
pub use marketplace::*;

use cr_core::{Finding, Language, Result};

/// Main rule engine interface
pub struct RuleEngine {
    rules: Vec<Rule>,
    pub validator: RuleValidator,
    executor: RuleExecutionEngine,
}

impl RuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            validator: RuleValidator::new(),
            executor: RuleExecutionEngine::new(),
        }
    }

    /// Load rules from YAML content
    pub fn load_rules_from_yaml(&mut self, yaml_content: &str) -> Result<usize> {
        let parser = RuleParser::new();
        let parsed_rules = parser.parse_yaml(yaml_content)?;

        // If no rules were parsed (due to errors in non-strict mode), return error
        if parsed_rules.is_empty() {
            return Err(cr_core::AnalysisError::parse_error("No valid rules found"));
        }

        // Validate all rules before adding them
        for rule in &parsed_rules {
            self.validator.validate_rule(rule)?;
        }

        let count = parsed_rules.len();
        self.rules.extend(parsed_rules);
        Ok(count)
    }

    /// Load rules from a file
    pub fn load_rules_from_file(&mut self, file_path: &std::path::Path) -> Result<usize> {
        let content = std::fs::read_to_string(file_path)?;
        self.load_rules_from_yaml(&content)
    }

    /// Get all loaded rules
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Get rules for a specific language
    pub fn rules_for_language(&self, language: Language) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|rule| rule.applies_to(language))
            .collect()
    }

    /// Clear all loaded rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Add a single rule
    pub fn add_rule(&mut self, rule: Rule) -> cr_core::Result<()> {
        self.validator.validate_rule(&rule)?;
        self.rules.push(rule);
        Ok(())
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Execute rules against an AST
    pub fn execute_rules(
        &mut self,
        ast: &dyn cr_core::AstNode,
        context: &RuleContext,
    ) -> Result<Vec<RuleResult>> {
        let applicable_rules = self.rules_for_language(context.language);
        let results = self.executor.execute_rules(&applicable_rules.into_iter().cloned().collect::<Vec<_>>(), ast, context);
        Ok(results)
    }

    /// Execute a single rule against an AST
    pub fn execute_rule(
        &mut self,
        rule_id: &str,
        ast: &dyn cr_core::AstNode,
        context: &RuleContext,
    ) -> Result<Option<RuleResult>> {
        if let Some(rule) = self.rules.iter().find(|r| r.id == rule_id) {
            if rule.applies_to(context.language) {
                let result = self.executor.execute_rule(rule, ast, context);
                Ok(Some(result))
            } else {
                Ok(None)
            }
        } else {
            Err(cr_core::AnalysisError::rule_validation_error(&format!("Rule not found: {}", rule_id)))
        }
    }

    /// Get all findings from executing all rules
    pub fn analyze(
        &mut self,
        ast: &dyn cr_core::AstNode,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        let results = self.execute_rules(ast, context)?;
        let mut findings = Vec::new();

        for result in results {
            if result.is_success() {
                findings.extend(result.findings);
            }
        }

        Ok(findings)
    }

    /// Configure the execution engine
    pub fn configure_executor(&mut self) -> &mut RuleExecutionEngine {
        &mut self.executor
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_core::{Confidence, Language, Severity};

    #[test]
    fn test_rule_engine_creation() {
        let engine = RuleEngine::new();
        assert_eq!(engine.rule_count(), 0);
        assert!(engine.rules().is_empty());
    }

    #[test]
    fn test_load_rules_from_yaml() {
        let mut engine = RuleEngine::new();
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    severity: ERROR
    languages: [java]
    patterns:
      - "System.out.println($MSG)"
"#;

        let count = engine.load_rules_from_yaml(yaml).unwrap();
        assert_eq!(count, 1);
        assert_eq!(engine.rule_count(), 1);

        let rules = engine.rules();
        assert_eq!(rules[0].id, "test-rule");
        assert_eq!(rules[0].name, "Test Rule");
    }

    #[test]
    fn test_load_invalid_yaml() {
        let mut engine = RuleEngine::new();
        engine.validator = RuleValidator::strict(); // Use strict validator
        let yaml = r#"
rules:
  - id: test-rule
    # Missing required fields
"#;

        let result = engine.load_rules_from_yaml(yaml);
        assert!(result.is_err());
        assert_eq!(engine.rule_count(), 0);
    }

    #[test]
    fn test_rules_for_language() {
        let mut engine = RuleEngine::new();
        let yaml = r#"
rules:
  - id: java-rule
    name: Java Rule
    description: A Java rule
    severity: ERROR
    languages: [java]
    patterns:
      - "test"
  - id: python-rule
    name: Python Rule
    description: A Python rule
    severity: WARNING
    languages: [python]
    patterns:
      - "test"
  - id: multi-rule
    name: Multi Language Rule
    description: A multi-language rule
    severity: INFO
    languages: [java, python]
    patterns:
      - "test"
"#;

        engine.load_rules_from_yaml(yaml).unwrap();
        assert_eq!(engine.rule_count(), 3);

        let java_rules = engine.rules_for_language(Language::Java);
        assert_eq!(java_rules.len(), 2); // java-rule and multi-rule

        let python_rules = engine.rules_for_language(Language::Python);
        assert_eq!(python_rules.len(), 2); // python-rule and multi-rule

        let js_rules = engine.rules_for_language(Language::JavaScript);
        assert_eq!(js_rules.len(), 0);
    }

    #[test]
    fn test_clear_rules() {
        let mut engine = RuleEngine::new();
        let yaml = r#"
rules:
  - id: test-rule
    name: Test Rule
    description: A test rule
    severity: ERROR
    languages: [java]
    patterns:
      - "test"
"#;

        engine.load_rules_from_yaml(yaml).unwrap();
        assert_eq!(engine.rule_count(), 1);

        engine.clear_rules();
        assert_eq!(engine.rule_count(), 0);
        assert!(engine.rules().is_empty());
    }

    #[test]
    fn test_rule_validation_during_load() {
        let mut engine = RuleEngine::new();
        let yaml = r#"
rules:
  - id: ""  # Invalid empty ID
    name: Test Rule
    description: A test rule
    severity: ERROR
    languages: [java]
"#;

        let result = engine.load_rules_from_yaml(yaml);
        assert!(result.is_err());
        assert_eq!(engine.rule_count(), 0);
    }
}
