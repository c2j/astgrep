use super::*;
use crate::models::AnalysisOptions;

#[test]
fn test_detect_language_from_filename() {
    assert_eq!(detect_language_from_filename("test.java"), "java");
    assert_eq!(detect_language_from_filename("test.js"), "javascript");
    assert_eq!(detect_language_from_filename("test.py"), "python");
    assert_eq!(detect_language_from_filename("test.unknown"), "text");
}

#[tokio::test]
async fn test_perform_code_analysis() {
    let request = AnalyzeRequest {
        code: "System.out.println(\"Hello World\");".to_string(),
        language: "java".to_string(),
        rules: None,
        options: Some(AnalysisOptions {
            include_metrics: Some(true),
            ..Default::default()
        }),
    };

    let config = WebConfig::default();
    let results = perform_code_analysis(&request, &config).await.unwrap();

    // The analysis engine may return multiple findings
    assert!(!results.findings.is_empty());
    assert_eq!(results.summary.total_findings, results.findings.len());
    assert_eq!(results.summary.files_analyzed, 1);
    assert!(results.metrics.is_some());
}

#[tokio::test]
async fn test_multiplication_rule_no_duplicates() {
    let yaml = r#"
rules:
  - id: multiplication_rule
    pattern: "$VAR1 * $VAR2;"
    message: "Use Math.pow(<number>, 2);"
    languages: [javascript]
    severity: INFO
"#;
    let code = r#"
const number = parseFloat(userInput);
var square = number * number;
"#;
    let request = AnalyzeRequest {
        code: code.to_string(),
        language: "javascript".to_string(),
        rules: Some(serde_json::Value::String(yaml.to_string())),
        options: None,
    };
    let config = WebConfig::default();
    let results = perform_code_analysis(&request, &config).await.unwrap();
    let matches: Vec<_> = results
        .findings
        .iter()
        .filter(|f| f.rule_id == "multiplication_rule")
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "should return exactly 1 match, got {}",
        matches.len()
    );
}
