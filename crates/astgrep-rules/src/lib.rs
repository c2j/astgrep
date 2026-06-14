//! Rule parsing and execution engine for astgrep
//!
//! This crate provides rule parsing, validation, and execution functionality.

pub mod engine;
pub mod executor;
pub mod integration;
pub mod marketplace;
pub mod parser;
pub mod types;
pub mod validator;

pub use engine::*;
pub use executor::*;
pub use integration::*;
pub use marketplace::*;
pub use parser::*;
pub use types::*;
pub use validator::*;

use astgrep_core::{Finding, Language, Result};

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
            return Err(astgrep_core::AnalysisError::parse_error(
                "No valid rules found",
            ));
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
    pub fn add_rule(&mut self, rule: Rule) -> astgrep_core::Result<()> {
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
        ast: &dyn astgrep_core::AstNode,
        context: &RuleContext,
    ) -> Result<Vec<RuleResult>> {
        let applicable_rules: Vec<Rule> = self
            .rules_for_language(context.language)
            .into_iter()
            .filter(|r| r.applies_to_dialect(context.sql_dialect))
            .cloned()
            .collect();
        let results = self.executor.execute_rules(&applicable_rules, ast, context);
        Ok(results)
    }

    /// Execute a single rule against an AST
    pub fn execute_rule(
        &mut self,
        rule_id: &str,
        ast: &dyn astgrep_core::AstNode,
        context: &RuleContext,
    ) -> Result<Option<RuleResult>> {
        if let Some(rule) = self.rules.iter().find(|r| r.id == rule_id) {
            if rule.applies_to(context.language) && rule.applies_to_dialect(context.sql_dialect) {
                let result = self.executor.execute_rule(rule, ast, context);
                Ok(Some(result))
            } else {
                Ok(None)
            }
        } else {
            Err(astgrep_core::AnalysisError::rule_validation_error(
                &format!("Rule not found: {}", rule_id),
            ))
        }
    }

    /// Get all findings from executing all rules
    pub fn analyze(
        &mut self,
        ast: &dyn astgrep_core::AstNode,
        context: &RuleContext,
    ) -> Result<Vec<Finding>> {
        if context.enable_constant_propagation {
            use astgrep_dataflow::ConstantPropagator;
            let mut propagator = ConstantPropagator::new();
            match propagator.analyze_ast(ast) {
                Ok(constants) => {
                    if !constants.is_empty() {
                        tracing::info!("Constant propagation found {} constants", constants.len());
                        self.executor.set_constant_values(constants);
                    }
                }
                Err(e) => {
                    tracing::warn!("Constant propagation analysis failed: {}", e);
                }
            }
        }

        let rules = self.rules_for_language(context.language);
        let rules: Vec<&Rule> = rules
            .into_iter()
            .filter(|r| r.applies_to_dialect(context.sql_dialect))
            .collect();

        let rules_with_conditions: Vec<Rule> = rules
            .iter()
            .filter(|r| {
                r.patterns
                    .iter()
                    .any(|p| !p.conditions.is_empty() || pattern_has_typed_metavar(p))
            })
            .map(|r| (*r).clone())
            .collect();

        let rules_without_conditions: Vec<Rule> = rules
            .iter()
            .filter(|r| {
                r.patterns
                    .iter()
                    .all(|p| p.conditions.is_empty() && !pattern_has_typed_metavar(p))
            })
            .map(|r| (*r).clone())
            .collect();

        let mut findings = Vec::new();

        // Run simple traverser for rules WITHOUT conditions
        if !rules_without_conditions.is_empty() {
            let saved_rules = std::mem::take(&mut self.rules);
            self.rules = rules_without_conditions.clone();
            let results = self.execute_rules(ast, context)?;
            self.rules = saved_rules;

            for result in results {
                if result.is_success() {
                    findings.extend(result.findings);
                }
            }
        }

        // Run advanced executor for rules WITH conditions
        if !rules_with_conditions.is_empty() {
            use crate::executor::core::AdvancedRuleExecutor;
            let mut advanced = AdvancedRuleExecutor::new();
            let comp_result = advanced.execute_comprehensive_analysis(
                &rules_with_conditions,
                ast,
                context.language,
                Some(std::path::Path::new(&context.file_path)),
                context.enable_constant_propagation,
                context.sql_dialect,
            )?;
            findings.extend(comp_result.findings);
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

/// Check if a pattern contains typed metavariable syntax `(TYPE $VAR)`.
///
/// Patterns with typed metavariables must be routed through `AdvancedRuleExecutor`
/// which handles type constraint extraction and post-filtering.
fn pattern_has_typed_metavar(pattern: &crate::types::Pattern) -> bool {
    use crate::types::PatternType;
    match &pattern.pattern_type {
        PatternType::Simple(s) => {
            let re = regex::Regex::new(r"\(([\w.]+(?:<[^>]*>)?(?:\[\])?)\s+\$(\w+)\)");
            re.map(|r| r.is_match(s)).unwrap_or(false)
        }
        PatternType::Either(patterns)
        | PatternType::All(patterns)
        | PatternType::Any(patterns) => patterns.iter().any(pattern_has_typed_metavar),
        PatternType::Inside(pattern)
        | PatternType::NotInside(pattern)
        | PatternType::Not(pattern) => pattern_has_typed_metavar(pattern),
        PatternType::Regex(_) | PatternType::NotRegex(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::{Confidence, Language, Severity, SqlDialect};

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
    message: A test rule
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
    message: A Java rule
    severity: ERROR
    languages: [java]
    patterns:
      - "test"
  - id: python-rule
    name: Python Rule
    description: A Python rule
    message: A Python rule
    severity: WARNING
    languages: [python]
    patterns:
      - "test"
  - id: multi-rule
    name: Multi Language Rule
    description: A multi-language rule
    message: A multi-language rule
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
    message: A test rule
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
    fn test_dialect_filtering_in_engine() {
        let yaml = r#"
rules:
  - id: gaussdb-only
    name: GaussDB Only
    description: A rule that only applies to GaussDB
    message: Only GaussDB
    severity: ERROR
    languages: [sql]
    patterns:
      - "SELECT"
    dialects: [gaussdb]
  - id: any-dialect
    name: Any Dialect
    description: A rule without dialect constraint
    message: Any dialect
    severity: WARNING
    languages: [sql]
    patterns:
      - "SELECT"
"#;

        let mut engine = RuleEngine::new();
        engine.load_rules_from_yaml(yaml).unwrap();
        assert_eq!(engine.rule_count(), 2);

        // When dialect is Some(GaussDB), both rules should apply
        let ctx_gaussdb = RuleContext {
            file_path: "test.sql".to_string(),
            language: Language::Sql,
            source_code: "SELECT 1".to_string(),
            custom_data: std::collections::HashMap::new(),
            enable_constant_propagation: false,
            sql_stmt_boundary: None,
            sql_dialect: Some(SqlDialect::GaussDB),
        };
        let gaussdb_rules: Vec<&Rule> = engine
            .rules()
            .iter()
            .filter(|r| {
                r.applies_to(Language::Sql) && r.applies_to_dialect(ctx_gaussdb.sql_dialect)
            })
            .collect();
        assert_eq!(
            gaussdb_rules.len(),
            2,
            "Both rules should apply for GaussDB"
        );

        // When dialect is Some(OpenGauss), only the unconstrained rule should apply
        let ctx_opengauss = RuleContext {
            file_path: "test.sql".to_string(),
            language: Language::Sql,
            source_code: "SELECT 1".to_string(),
            custom_data: std::collections::HashMap::new(),
            enable_constant_propagation: false,
            sql_stmt_boundary: None,
            sql_dialect: Some(SqlDialect::OpenGauss),
        };
        let opengauss_rules: Vec<&Rule> = engine
            .rules()
            .iter()
            .filter(|r| {
                r.applies_to(Language::Sql) && r.applies_to_dialect(ctx_opengauss.sql_dialect)
            })
            .collect();
        assert_eq!(
            opengauss_rules.len(),
            1,
            "Only the unconstrained rule should apply for OpenGauss"
        );
        assert_eq!(opengauss_rules[0].id, "any-dialect");

        // When dialect is None, only the unconstrained rule should apply
        let ctx_no_dialect = RuleContext {
            file_path: "test.sql".to_string(),
            language: Language::Sql,
            source_code: "SELECT 1".to_string(),
            custom_data: std::collections::HashMap::new(),
            enable_constant_propagation: false,
            sql_stmt_boundary: None,
            sql_dialect: None,
        };
        let no_dialect_rules: Vec<&Rule> = engine
            .rules()
            .iter()
            .filter(|r| {
                r.applies_to(Language::Sql) && r.applies_to_dialect(ctx_no_dialect.sql_dialect)
            })
            .collect();
        assert_eq!(
            no_dialect_rules.len(),
            1,
            "Only the unconstrained rule should apply for no dialect"
        );
        assert_eq!(no_dialect_rules[0].id, "any-dialect");
    }

    #[test]
    fn test_rule_validation_during_load() {
        let mut engine = RuleEngine::new();
        let yaml = r#"
rules:
  - id: ""  # Invalid empty ID
    name: Test Rule
    description: A test rule
    message: A test rule
    severity: ERROR
    languages: [java]
"#;

        let result = engine.load_rules_from_yaml(yaml);
        assert!(result.is_err());
        assert_eq!(engine.rule_count(), 0);
    }
}
