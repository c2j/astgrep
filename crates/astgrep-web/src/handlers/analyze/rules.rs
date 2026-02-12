use astgrep_core::Language;
use astgrep_rules::RuleEngine;
use tracing::warn;

use crate::{WebConfig, WebError, WebResult};

/// Parse language string to Language enum
pub fn parse_language(language_str: &str) -> WebResult<Language> {
    match language_str.to_lowercase().as_str() {
        "java" => Ok(Language::Java),
        "javascript" | "js" => Ok(Language::JavaScript),
        "python" | "py" => Ok(Language::Python),
        "sql" => Ok(Language::Sql),
        "bash" | "sh" => Ok(Language::Bash),
        "php" => Ok(Language::Php),
        "csharp" | "c#" | "cs" => Ok(Language::CSharp),
        "c" => Ok(Language::C),
        "ruby" | "rb" => Ok(Language::Ruby),
        "kotlin" | "kt" => Ok(Language::Kotlin),
        "swift" => Ok(Language::Swift),
        "xml" => Ok(Language::Xml),
        _ => Err(WebError::bad_request(&format!(
            "Unsupported language: {}",
            language_str
        ))),
    }
}

/// Load default rules for a specific language
pub async fn load_default_rules_for_language(
    rule_engine: &mut RuleEngine,
    language: Language,
    config: &WebConfig,
) -> WebResult<()> {
    use tokio::fs;

    // Construct path to default rules for the language
    let language_str = match language {
        Language::Java => "java",
        Language::JavaScript => "javascript",
        Language::Python => "python",
        Language::Sql => "sql",
        Language::Bash => "bash",
        Language::Php => "php",
        Language::CSharp => "csharp",
        Language::C => "c",
        Language::Ruby => "ruby",
        Language::Kotlin => "kotlin",
        Language::Swift => "swift",
        Language::Xml => "xml",
    };

    let rules_path = config.rules_directory.join(format!("{}.yaml", language_str));

    // Try to load language-specific rules
    if rules_path.exists() {
        match fs::read_to_string(&rules_path).await {
            Ok(rules_content) => {
                if let Err(e) = rule_engine.load_rules_from_yaml(&rules_content) {
                    warn!("Failed to load rules from {}: {}", rules_path.display(), e);
                }
            }
            Err(e) => {
                warn!("Failed to read rules file {}: {}", rules_path.display(), e);
            }
        }
    }

    // Also try to load general rules
    let general_rules_path = config.rules_directory.join("general.yaml");
    if general_rules_path.exists() {
        match fs::read_to_string(&general_rules_path).await {
            Ok(rules_content) => {
                if let Err(e) = rule_engine.load_rules_from_yaml(&rules_content) {
                    warn!("Failed to load general rules: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to read general rules file: {}", e);
            }
        }
    }

    // If no rules were loaded, create some basic default rules
    if rule_engine.rule_count() == 0 {
        load_builtin_rules_for_language(rule_engine, language)?;
    }

    Ok(())
}

/// Load built-in rules for a language when no external rules are available
pub fn load_builtin_rules_for_language(
    rule_engine: &mut RuleEngine,
    language: Language,
) -> WebResult<()> {
    // Create a simple default rule for the language
    let builtin_rules = create_default_rule_for_language(language);

    rule_engine
        .load_rules_from_yaml(&builtin_rules)
        .map_err(|e| WebError::analysis_error(format!("Failed to load builtin rules: {}", e)))?;

    Ok(())
}

/// Create a default rule for a language
pub fn create_default_rule_for_language(language: Language) -> String {
    match language {
        Language::Java => r#"
rules:
  - id: java-system-out
    name: "Avoid System.out usage"
    description: "Detects usage of System.out.print* methods"
    severity: WARNING
    confidence: HIGH
    languages: [java]
    patterns:
      - "System.out.println"
      - "System.out.print"
    message: "Use proper logging instead of System.out"
"#
        .to_string(),
        Language::JavaScript => r#"
rules:
  - id: js-console-log
    name: "Console.log Usage"
    description: "Detects console.log statements"
    severity: WARNING
    confidence: HIGH
    languages: [javascript]
    patterns:
      - "console.log"
      - "console.warn"
    message: "Remove console statements before production"
"#
        .to_string(),
        Language::Python => r#"
rules:
  - id: python-print-usage
    name: "Print Statement Usage"
    description: "Detects print statements"
    severity: WARNING
    confidence: HIGH
    languages: [python]
    patterns:
      - "print("
    message: "Use logging instead of print statements"
"#
        .to_string(),
        Language::Sql => r#"
rules:
  - id: sql-select-star
    name: "SELECT * Usage"
    description: "Detects SELECT * queries"
    severity: WARNING
    confidence: MEDIUM
    languages: [sql]
    patterns:
      - "SELECT *"
    message: "Avoid SELECT * in production queries"
"#
        .to_string(),
        Language::Bash => r#"
rules:
  - id: bash-unquoted-variable
    name: "Unquoted Variable"
    description: "Detects unquoted variables"
    severity: WARNING
    confidence: MEDIUM
    languages: [bash]
    patterns:
      - "echo $"
    message: "Quote variables to prevent word splitting"
"#
        .to_string(),
        Language::Php => r#"
rules:
  - id: php-sql-injection
    name: "SQL Injection Risk"
    description: "Detects potential SQL injection"
    severity: ERROR
    confidence: HIGH
    languages: [php]
    patterns:
      - "mysql_query("
      - "mysqli_query("
    message: "Use prepared statements"
"#
        .to_string(),
        Language::CSharp => r#"
rules:
  - id: csharp-console-writeline
    name: "Console.WriteLine Usage"
    description: "Detects Console.WriteLine"
    severity: WARNING
    confidence: HIGH
    languages: [csharp]
    patterns:
      - "Console.WriteLine"
    message: "Use proper logging framework"
"#
        .to_string(),
        Language::C => r#"
rules:
  - id: c-buffer-overflow
    name: "Buffer Overflow Risk"
    description: "Detects unsafe functions"
    severity: ERROR
    confidence: HIGH
    languages: [c]
    patterns:
      - "strcpy("
      - "gets("
    message: "Use safer alternatives"
"#
        .to_string(),
        Language::Ruby => r#"
rules:
  - id: ruby-puts-usage
    name: "Puts Usage"
    description: "Detects puts statements"
    severity: WARNING
    confidence: HIGH
    languages: [ruby]
    patterns:
      - "puts "
    message: "Use proper logging instead of puts"
"#
        .to_string(),
        Language::Kotlin => r#"
rules:
  - id: kotlin-println-usage
    name: "println Usage"
    description: "Detects println statements"
    severity: WARNING
    confidence: MEDIUM
    languages: [kotlin]
    patterns:
      - "println("
    message: "Use logging instead of println"
"#
        .to_string(),
        Language::Swift => r#"
rules:
  - id: swift-print-usage
    name: "print Usage"
    description: "Detects print statements"
    severity: WARNING
    confidence: MEDIUM
    languages: [swift]
    patterns:
      - "print("
    message: "Use proper logging instead of print"
"#
        .to_string(),
        Language::Xml => r#"
rules:
  - id: xml-hardcoded-credential
    name: "Hardcoded Credential"
    description: "Detects hardcoded credentials in XML"
    severity: ERROR
    confidence: MEDIUM
    languages: [xml]
    patterns:
      - "password=\""
    message: "Avoid hardcoded credentials"
"#
        .to_string(),
    }
}

/// Detect programming language from filename
pub fn detect_language_from_filename(filename: &str) -> String {
    let extension = filename
        .split('.')
        .last()
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "java" => "java".to_string(),
        "js" | "jsx" => "javascript".to_string(),
        "ts" | "tsx" => "typescript".to_string(),
        "py" => "python".to_string(),
        "sql" => "sql".to_string(),
        "sh" | "bash" => "bash".to_string(),
        "c" => "c".to_string(),
        "cpp" | "cc" | "cxx" => "cpp".to_string(),
        "cs" => "csharp".to_string(),
        "php" => "php".to_string(),
        "rb" => "ruby".to_string(),
        "kt" | "kts" => "kotlin".to_string(),
        "swift" => "swift".to_string(),
        "xml" => "xml".to_string(),
        _ => "text".to_string(),
    }
}
