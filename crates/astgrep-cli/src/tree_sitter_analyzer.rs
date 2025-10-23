//! Tree-sitter based analysis engine
//! 
//! This module provides tree-sitter based pattern matching and analysis.

use astgrep_core::{Finding, Location, Language, Severity, Confidence, Result};
use astgrep_parser::tree_sitter_parser::TreeSitterParser;
use std::path::PathBuf;

/// Tree-sitter based analyzer
pub struct TreeSitterAnalyzer {
    parser: TreeSitterParser,
}

impl TreeSitterAnalyzer {
    /// Create a new tree-sitter analyzer
    pub fn new() -> Result<Self> {
        let parser = TreeSitterParser::new()?;
        Ok(Self { parser })
    }
    
    /// Apply a rule using tree-sitter parsing
    pub fn apply_rule_with_tree_sitter(
        &mut self,
        rule_id: &str,
        message: &str,
        severity: &Severity,
        patterns: &[String],
        file_path: &PathBuf,
        source_code: &str,
        language: Language,
        fix: &Option<String>,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        
        // Parse the source code with tree-sitter
        if let Some(tree) = self.parser.parse(source_code, language)? {
            for pattern in patterns {
                let matches = self.parser.find_pattern_matches(&tree, source_code, pattern)?;
                
                for node in matches {
                    let finding = Finding {
                        rule_id: rule_id.to_string(),
                        message: message.to_string(),
                        severity: severity.clone(),
                        confidence: Confidence::High, // Tree-sitter gives us high confidence
                        location: Location {
                            file: file_path.clone(),
                            start_line: node.start_position().row + 1,
                            start_column: node.start_position().column + 1,
                            end_line: node.end_position().row + 1,
                            end_column: node.end_position().column + 1,
                        },
                        metadata: std::collections::HashMap::new(),
                        fix_suggestion: fix.clone(),
                    };
                    findings.push(finding);
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Check if tree-sitter parsing is available for a language
    pub fn supports_language(&self, language: Language) -> bool {
        matches!(language, Language::Python | Language::JavaScript | Language::Java | Language::Bash)
    }
}

impl Default for TreeSitterAnalyzer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            parser: TreeSitterParser::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_python_analysis() {
        let mut analyzer = TreeSitterAnalyzer::new().unwrap();
        let source = r#"
print("hello")
x = 42
eval(code)
"#;
        
        let findings = analyzer.apply_rule_with_tree_sitter(
            "test-rule",
            "Test message",
            &Severity::Error,
            &[r#""hello""#.to_string()],
            &PathBuf::from("test.py"),
            source,
            Language::Python,
            &None,
        ).unwrap();
        
        assert!(!findings.is_empty());
        assert_eq!(findings[0].rule_id, "test-rule");
    }
    
    #[test]
    fn test_numeric_literal_precision() {
        let mut analyzer = TreeSitterAnalyzer::new().unwrap();
        let source = r#"
x = 42
y = "42"
print(42)
"#;
        
        // Test numeric literal matching - should only match actual numbers, not strings
        let findings = analyzer.apply_rule_with_tree_sitter(
            "test-number",
            "Found number",
            &Severity::Warning,
            &["42".to_string()],
            &PathBuf::from("test.py"),
            source,
            Language::Python,
            &None,
        ).unwrap();
        
        // Should find 2 matches: x = 42 and print(42), but not "42"
        assert_eq!(findings.len(), 2);
    }
    
    #[test]
    fn test_function_call_matching() {
        let mut analyzer = TreeSitterAnalyzer::new().unwrap();
        let source = r#"
eval("code")
evaluate("something")
eval(user_input)
"#;
        
        let findings = analyzer.apply_rule_with_tree_sitter(
            "test-eval",
            "Found eval",
            &Severity::Error,
            &["eval(...)".to_string()],
            &PathBuf::from("test.py"),
            source,
            Language::Python,
            &None,
        ).unwrap();
        
        // Should find 2 eval calls, but not evaluate
        assert_eq!(findings.len(), 2);
    }
}
