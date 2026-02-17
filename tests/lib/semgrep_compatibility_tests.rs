//! Semgrep compatibility tests for Phase 1
//! 
//! Tests for:
//! - pattern-all support
//! - pattern-any support
//! - fix-regex support
//! - paths support

use astgrep_rules::parser::RuleParser;

#[test]
fn test_pattern_all_support() {
    let yaml = r#"
rules:
  - id: test-pattern-all
    message: Test pattern-all
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-all:
          - pattern: "System.out.println($MSG)"
          - pattern-not: "System.out.println(\"debug\")"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "test-pattern-all");
    assert_eq!(rule.patterns.len(), 1);
}

#[test]
fn test_pattern_any_support() {
    let yaml = r#"
rules:
  - id: test-pattern-any
    message: Test pattern-any
    severity: WARNING
    languages: [python]
    patterns:
      - pattern-any:
          - pattern: "print($MSG)"
          - pattern: "sys.stdout.write($MSG)"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "test-pattern-any");
    assert_eq!(rule.patterns.len(), 1);
}

#[test]
fn test_fix_regex_support() {
    let yaml = r#"
rules:
  - id: test-fix-regex
    message: Test fix-regex
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "hardcoded_password = \"$PASSWORD\""
    fix-regex:
      regex: 'password\s*=\s*"[^"]*"'
      replacement: 'password = "***"'
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "test-fix-regex");
    assert!(rule.fix_regex.is_some());
    
    let fix_regex = rule.fix_regex.as_ref().unwrap();
    assert_eq!(fix_regex.regex, r#"password\s*=\s*"[^"]*""#);
    assert_eq!(fix_regex.replacement, "password = \"***\"");
}

#[test]
fn test_paths_support() {
    let yaml = r#"
rules:
  - id: test-paths
    message: Test paths
    severity: WARNING
    languages: [java]
    patterns:
      - pattern: "System.out.println($MSG)"
    paths:
      include:
        - "src/**/*.java"
        - "test/**/*.java"
      exclude:
        - "src/generated/**"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "test-paths");
    assert!(rule.paths.is_some());
    
    let paths = rule.paths.as_ref().unwrap();
    assert_eq!(paths.includes.len(), 2);
    assert_eq!(paths.excludes.len(), 1);
    assert_eq!(paths.includes[0], "src/**/*.java");
    assert_eq!(paths.excludes[0], "src/generated/**");
}

#[test]
fn test_combined_semgrep_features() {
    let yaml = r#"
rules:
  - id: sql-injection-combined
    message: SQL injection with combined features
    languages: [java, python]
    severity: ERROR
    confidence: HIGH
    patterns:
      - pattern-either:
          - pattern: "$STMT.execute($QUERY)"
          - pattern: "$CONN.query($QUERY)"
    fix: "Use prepared statements"
    fix-regex:
      regex: 'execute\("([^"]*)"\)'
      replacement: 'execute(preparedStatement)'
    paths:
      include:
        - "src/**"
      exclude:
        - "test/**"
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "sql-injection-combined");
    assert!(rule.fix.is_some());
    assert!(rule.fix_regex.is_some());
    assert!(rule.paths.is_some());
    assert_eq!(rule.metadata.get("cwe"), Some(&"CWE-89".to_string()));
}

#[test]
fn test_pattern_all_with_multiple_patterns() {
    let yaml = r#"
rules:
  - id: test-pattern-all-multiple
    message: Test pattern-all with multiple patterns
    severity: ERROR
    languages: [java]
    patterns:
      - pattern-all:
          - pattern: "class $CLASS"
          - pattern-inside: "public class $CLASS"
          - pattern-not: "abstract class $CLASS"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.patterns.len(), 1);
}

#[test]
fn test_nested_pattern_any_and_all() {
    let yaml = r#"
rules:
  - id: test-nested-patterns
    message: Test nested pattern-any and pattern-all
    severity: WARNING
    languages: [python]
    patterns:
      - pattern-any:
          - pattern-all:
              - pattern: "def $FUNC(...)"
              - pattern-not: "def test_$FUNC(...)"
          - pattern: "class $CLASS"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.patterns.len(), 1);
}

#[test]
fn test_backward_compatibility() {
    // Ensure old rules still work
    let yaml = r#"
rules:
  - id: old-style-rule
    message: Old style rule
    severity: WARNING
    languages: [java]
    patterns:
      - pattern: "System.out.println($MSG)"
    fix: "Use logger instead"
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "old-style-rule");
    assert!(rule.fix.is_some());
    assert!(rule.fix_regex.is_none());
    assert!(rule.paths.is_none());
}

#[test]
fn test_empty_paths() {
    let yaml = r#"
rules:
  - id: test-empty-paths
    message: Test empty paths
    severity: WARNING
    languages: [java]
    patterns:
      - pattern: "System.out.println($MSG)"
    paths:
      include: []
      exclude: []
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert!(rule.paths.is_some());
    
    let paths = rule.paths.as_ref().unwrap();
    assert_eq!(paths.includes.len(), 0);
    assert_eq!(paths.excludes.len(), 0);
}

#[test]
fn test_fix_regex_with_special_characters() {
    let yaml = r#"
rules:
  - id: test-fix-regex-special
    message: Test fix-regex with special characters
    severity: ERROR
    languages: [java]
    patterns:
      - pattern: "eval($CODE)"
    fix-regex:
      regex: 'eval\(([^)]*)\)'
      replacement: 'safeEval($1)'
"#;

    let parser = RuleParser::new();
    let rules = parser.parse_yaml(yaml).expect("Failed to parse YAML");
    
    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert!(rule.fix_regex.is_some());
    
    let fix_regex = rule.fix_regex.as_ref().unwrap();
    assert_eq!(fix_regex.regex, r#"eval\(([^)]*)\)"#);
    assert_eq!(fix_regex.replacement, "safeEval($1)");
}

