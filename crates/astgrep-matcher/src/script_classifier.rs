//! Script categorization logic for functional classification

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

use astgrep_core::models::test_asset::{AssetType, ScriptType, TestAsset};

/// Configuration for script categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    /// Enable keyword-based classification
    pub keyword_matching: bool,
    /// Enable content analysis classification
    pub content_analysis: bool,
    /// Enable shebang-based classification
    pub shebang_analysis: bool,
    /// Enable filename-based classification
    pub filename_analysis: bool,
    /// Enable dependency analysis for classification
    pub dependency_analysis: bool,
    /// Custom classification rules
    pub custom_rules: Vec<ClassificationRule>,
    /// Minimum confidence threshold for classification
    pub confidence_threshold: f64,
    /// Enable fallback classification
    pub enable_fallback: bool,
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            keyword_matching: true,
            content_analysis: true,
            shebang_analysis: true,
            filename_analysis: true,
            dependency_analysis: false, // Disabled by default as it's expensive
            custom_rules: Vec::new(),
            confidence_threshold: 0.5,
            enable_fallback: true,
        }
    }
}

/// Classification result with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub script_type: ScriptType,
    pub confidence: f64,
    pub classification_method: String,
    pub supporting_evidence: Vec<String>,
    pub alternative_types: Vec<(ScriptType, f64)>,
    pub metadata: ClassificationMetadata,
}

/// Additional metadata from classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    pub keywords_found: Vec<String>,
    pub file_patterns_matched: Vec<String>,
    pub shebang_detected: Option<String>,
    pub dependencies_found: Vec<String>,
    pub content_analyzed: bool,
    pub processing_time_ms: u64,
}

/// Custom classification rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    pub name: String,
    pub script_type: ScriptType,
    pub conditions: Vec<ClassificationCondition>,
    pub weight: f64,
    pub description: String,
}

/// Classification condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationCondition {
    FilenameContains {
        pattern: String,
        case_sensitive: bool,
    },
    ContentContains {
        pattern: String,
        case_sensitive: bool,
    },
    ShebangMatches {
        pattern: String,
    },
    FileExtension {
        extension: String,
    },
    FileSize {
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    },
    CustomCondition {
        name: String,
        parameters: HashMap<String, String>,
    },
}

/// Script classification engine
pub struct ScriptClassifier {
    config: ClassificationConfig,
    keyword_patterns: HashMap<ScriptType, Vec<Regex>>,
    filename_patterns: HashMap<ScriptType, Vec<Regex>>,
    shebang_patterns: HashMap<String, ScriptType>,
}

impl ScriptClassifier {
    /// Create a new script classifier with default configuration
    pub fn new() -> Self {
        let classifier = Self {
            config: ClassificationConfig::default(),
            keyword_patterns: Self::build_keyword_patterns(),
            filename_patterns: Self::build_filename_patterns(),
            shebang_patterns: Self::build_shebang_patterns(),
        };

        info!("Script classifier initialized with default configuration");
        classifier
    }

    /// Create a script classifier with custom configuration
    pub fn with_config(config: ClassificationConfig) -> Self {
        let classifier = Self {
            config,
            keyword_patterns: Self::build_keyword_patterns(),
            filename_patterns: Self::build_filename_patterns(),
            shebang_patterns: Self::build_shebang_patterns(),
        };

        info!("Script classifier initialized with custom configuration");
        classifier
    }

    /// Classify a test script based on its content and metadata
    pub fn classify_script(&self, asset: &TestAsset) -> Result<ClassificationResult> {
        let start_time = std::time::Instant::now();

        debug!(
            "Classifying script: {} ({})",
            asset.name,
            asset.current_path.display()
        );

        let mut results = Vec::new();

        // Filename-based classification
        if self.config.filename_analysis {
            if let Ok(result) = self.classify_by_filename(asset) {
                results.push(result);
            }
        }

        // Shebang-based classification
        if self.config.shebang_analysis {
            if let Ok(result) = self.classify_by_shebang(asset) {
                results.push(result);
            }
        }

        // Keyword-based classification
        if self.config.keyword_matching {
            if let Ok(result) = self.classify_by_keywords(asset) {
                results.push(result);
            }
        }

        // Content analysis classification
        if self.config.content_analysis {
            if let Ok(result) = self.classify_by_content(asset) {
                results.push(result);
            }
        }

        // Custom rules classification
        for rule in &self.config.custom_rules {
            if let Ok(result) = self.classify_by_custom_rule(asset, rule) {
                results.push(result);
            }
        }

        // Combine results and determine best classification
        let final_result = self.combine_classification_results(results, &start_time)?;

        info!(
            "Script classified as {:?} with confidence {:.2}",
            final_result.script_type, final_result.confidence
        );

        Ok(final_result)
    }

    /// Classify multiple scripts
    pub fn classify_scripts(&self, assets: &[TestAsset]) -> Result<Vec<ClassificationResult>> {
        let mut results = Vec::new();

        for asset in assets {
            if asset.asset_type == AssetType::Script {
                let result = self.classify_script(asset)?;
                results.push(result);
            }
        }

        info!("Classified {} scripts", results.len());
        Ok(results)
    }

    /// Classify by filename patterns
    fn classify_by_filename(&self, asset: &TestAsset) -> Result<ClassificationResult> {
        let filename = asset
            .current_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut best_match = (ScriptType::Utility, 0.0, Vec::new());

        for (script_type, patterns) in &self.filename_patterns {
            for pattern in patterns {
                if pattern.is_match(filename) {
                    let confidence = 0.8; // High confidence for filename matches
                    if confidence > best_match.1 {
                        best_match = (
                            script_type.clone(),
                            confidence,
                            vec![format!("Filename pattern: {}", pattern)],
                        );
                    }
                }
            }
        }

        Ok(ClassificationResult {
            script_type: best_match.0,
            confidence: best_match.1,
            classification_method: "filename_analysis".to_string(),
            supporting_evidence: best_match.2,
            alternative_types: Vec::new(),
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: false,
                processing_time_ms: 0,
            },
        })
    }

    /// Classify by shebang
    fn classify_by_shebang(&self, asset: &TestAsset) -> Result<ClassificationResult> {
        let shebang = self.read_shebang(&asset.current_path)?;

        if let Some(ref shebang_str) = shebang {
            for (shebang_pattern, script_type) in &self.shebang_patterns {
                if shebang_str.contains(shebang_pattern) {
                    return Ok(ClassificationResult {
                        script_type: script_type.clone(),
                        confidence: 0.9, // Very high confidence for shebang
                        classification_method: "shebang_analysis".to_string(),
                        supporting_evidence: vec![format!("Shebang: {}", shebang_str)],
                        alternative_types: Vec::new(),
                        metadata: ClassificationMetadata {
                            keywords_found: Vec::new(),
                            file_patterns_matched: Vec::new(),
                            shebang_detected: Some(shebang_str.to_string()),
                            dependencies_found: Vec::new(),
                            content_analyzed: false,
                            processing_time_ms: 0,
                        },
                    });
                }
            }
        }

        // No shebang or no match found
        Ok(ClassificationResult {
            script_type: ScriptType::Utility,
            confidence: 0.0,
            classification_method: "shebang_analysis".to_string(),
            supporting_evidence: Vec::new(),
            alternative_types: Vec::new(),
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: shebang,
                dependencies_found: Vec::new(),
                content_analyzed: false,
                processing_time_ms: 0,
            },
        })
    }

    /// Classify by keyword analysis
    fn classify_by_keywords(&self, asset: &TestAsset) -> Result<ClassificationResult> {
        let content = fs::read_to_string(&asset.current_path)?;
        let content_lower = content.to_lowercase();

        let mut scores = HashMap::new();
        let mut evidence = HashMap::new();

        for (script_type, patterns) in &self.keyword_patterns {
            let mut type_score = 0.0;
            let mut type_evidence = Vec::new();

            for pattern in patterns {
                if pattern.is_match(&content_lower) {
                    type_score += 0.1;
                    type_evidence.push(format!("Keyword: {}", pattern));
                }
            }

            if type_score > 0.0 {
                scores.insert(script_type.clone(), type_score);
                evidence.insert(script_type.clone(), type_evidence);
            }
        }

        if scores.is_empty() {
            return Ok(ClassificationResult {
                script_type: ScriptType::Utility,
                confidence: 0.0,
                classification_method: "keyword_matching".to_string(),
                supporting_evidence: Vec::new(),
                alternative_types: Vec::new(),
                metadata: ClassificationMetadata {
                    keywords_found: Vec::new(),
                    file_patterns_matched: Vec::new(),
                    shebang_detected: None,
                    dependencies_found: Vec::new(),
                    content_analyzed: true,
                    processing_time_ms: 0,
                },
            });
        }

        // Find the best match
        let (best_type, best_score) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let best_evidence = evidence.get(best_type).unwrap_or(&Vec::new()).clone();

        // Calculate confidence based on score
        let confidence = (best_score / 1.0_f64).min(1.0_f64);

        Ok(ClassificationResult {
            script_type: best_type.clone(),
            confidence,
            classification_method: "keyword_matching".to_string(),
            supporting_evidence: best_evidence.clone(),
            alternative_types: Vec::new(),
            metadata: ClassificationMetadata {
                keywords_found: best_evidence.to_vec(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: true,
                processing_time_ms: 0,
            },
        })
    }

    /// Classify by content analysis (more sophisticated than just keywords)
    fn classify_by_content(&self, asset: &TestAsset) -> Result<ClassificationResult> {
        let content = fs::read_to_string(&asset.current_path)?;

        let mut validator_score = 0.0;
        let mut runner_score = 0.0;
        let mut ci_score = 0.0;
        let mut utility_score = 0.1; // Base score for utility

        // Analyze content for patterns
        if self.contains_validation_patterns(&content) {
            validator_score += 0.7;
        }

        if self.contains_runner_patterns(&content) {
            runner_score += 0.7;
        }

        if self.contains_ci_patterns(&content) {
            ci_score += 0.7;
        }

        if self.contains_utility_patterns(&content) {
            utility_score += 0.3;
        }

        let scores = [
            (ScriptType::Validator, validator_score),
            (ScriptType::Runner, runner_score),
            (ScriptType::CiIntegration, ci_score),
            (ScriptType::Utility, utility_score),
        ];

        let (best_type, best_score) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let confidence = *best_score;

        Ok(ClassificationResult {
            script_type: best_type.clone(),
            confidence,
            classification_method: "content_analysis".to_string(),
            supporting_evidence: vec![format!("Content analysis score: {:.2}", best_score)],
            alternative_types: Vec::new(),
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: true,
                processing_time_ms: 0,
            },
        })
    }

    /// Classify by custom rule
    fn classify_by_custom_rule(
        &self,
        asset: &TestAsset,
        rule: &ClassificationRule,
    ) -> Result<ClassificationResult> {
        let mut matched_conditions = Vec::new();
        let mut conditions_met = 0;

        for condition in &rule.conditions {
            if self.evaluate_condition(asset, condition)? {
                matched_conditions.push(format!("Condition met: {:?}", condition));
                conditions_met += 1;
            }
        }

        let confidence = if conditions_met > 0 {
            (conditions_met as f64 / rule.conditions.len() as f64) * rule.weight
        } else {
            0.0
        };

        Ok(ClassificationResult {
            script_type: rule.script_type.clone(),
            confidence,
            classification_method: format!("custom_rule: {}", rule.name),
            supporting_evidence: matched_conditions,
            alternative_types: Vec::new(),
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: false,
                processing_time_ms: 0,
            },
        })
    }

    /// Evaluate a classification condition
    fn evaluate_condition(
        &self,
        asset: &TestAsset,
        condition: &ClassificationCondition,
    ) -> Result<bool> {
        match condition {
            ClassificationCondition::FilenameContains {
                pattern,
                case_sensitive,
            } => {
                let filename = asset
                    .current_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if *case_sensitive {
                    Ok(filename.contains(pattern))
                } else {
                    Ok(filename.to_lowercase().contains(&pattern.to_lowercase()))
                }
            }
            ClassificationCondition::ContentContains {
                pattern,
                case_sensitive,
            } => {
                let content = fs::read_to_string(&asset.current_path)?;
                if *case_sensitive {
                    Ok(content.contains(pattern))
                } else {
                    Ok(content.to_lowercase().contains(&pattern.to_lowercase()))
                }
            }
            ClassificationCondition::ShebangMatches { pattern } => {
                let shebang = self.read_shebang(&asset.current_path)?;
                Ok(shebang.is_some_and(|s| s.contains(pattern)))
            }
            ClassificationCondition::FileExtension { extension } => {
                let file_extension = asset
                    .current_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                Ok(file_extension == extension)
            }
            ClassificationCondition::FileSize {
                min_bytes,
                max_bytes,
            } => {
                let file_size = asset.get_file_size().unwrap_or(0);
                let min_ok = min_bytes.is_none_or(|min| file_size >= min);
                let max_ok = max_bytes.is_none_or(|max| file_size <= max);
                Ok(min_ok && max_ok)
            }
            ClassificationCondition::CustomCondition { .. } => {
                // Custom conditions would require additional implementation
                Ok(false)
            }
        }
    }

    /// Combine multiple classification results into a final result
    fn combine_classification_results(
        &self,
        results: Vec<ClassificationResult>,
        start_time: &std::time::Instant,
    ) -> Result<ClassificationResult> {
        if results.is_empty() {
            // Fallback classification
            return Ok(ClassificationResult {
                script_type: ScriptType::Utility,
                confidence: if self.config.enable_fallback {
                    0.1
                } else {
                    0.0
                },
                classification_method: "fallback".to_string(),
                supporting_evidence: vec!["No classification methods available".to_string()],
                alternative_types: Vec::new(),
                metadata: ClassificationMetadata {
                    keywords_found: Vec::new(),
                    file_patterns_matched: Vec::new(),
                    shebang_detected: None,
                    dependencies_found: Vec::new(),
                    content_analyzed: false,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                },
            });
        }

        // Sort by confidence
        let mut sorted_results = results;
        sorted_results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_result = sorted_results.into_iter().next().unwrap();

        // Check if confidence meets threshold
        if best_result.confidence < self.config.confidence_threshold && self.config.enable_fallback
        {
            return Ok(ClassificationResult {
                script_type: ScriptType::Utility,
                confidence: best_result.confidence.max(0.1),
                classification_method: "fallback".to_string(),
                supporting_evidence: vec![format!(
                    "Low confidence ({}), using fallback",
                    best_result.confidence
                )],
                alternative_types: vec![(best_result.script_type.clone(), best_result.confidence)],
                metadata: ClassificationMetadata {
                    keywords_found: Vec::new(),
                    file_patterns_matched: Vec::new(),
                    shebang_detected: None,
                    dependencies_found: Vec::new(),
                    content_analyzed: false,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                },
            });
        }

        Ok(best_result)
    }

    /// Read shebang from file
    fn read_shebang(&self, path: &Path) -> Result<Option<String>> {
        let content = fs::read_to_string(path)?;

        if let Some(rest) = content.strip_prefix("#!") {
            if let Some(end_line) = content.find('\n') {
                let shebang_line = &rest[..end_line - 2];
                return Ok(Some(shebang_line.trim().to_string()));
            } else {
                return Ok(Some(rest.trim().to_string()));
            }
        }

        Ok(None)
    }

    /// Check if content contains validation patterns
    fn contains_validation_patterns(&self, content: &str) -> bool {
        let validation_keywords = [
            "validate",
            "check",
            "verify",
            "assert",
            "test",
            "spec",
            "expect",
            "assertEqual",
            "assertNotNull",
            "assertTrue",
            "assertFalse",
        ];

        let content_lower = content.to_lowercase();
        validation_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains runner patterns
    fn contains_runner_patterns(&self, content: &str) -> bool {
        let runner_keywords = [
            "run",
            "execute",
            "start",
            "launch",
            "invoke",
            "call",
            "perform",
            "main",
            "test_all",
            "run_all",
            "execute_all",
        ];

        let content_lower = content.to_lowercase();
        runner_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains CI patterns
    fn contains_ci_patterns(&self, content: &str) -> bool {
        let ci_keywords = [
            "ci",
            "build",
            "deploy",
            "pipeline",
            "continuous",
            "integration",
            "github",
            "jenkins",
            "travis",
            "circleci",
            "actions",
        ];

        let content_lower = content.to_lowercase();
        ci_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains utility patterns
    fn contains_utility_patterns(&self, content: &str) -> bool {
        let utility_keywords = [
            "helper",
            "util",
            "tool",
            "function",
            "method",
            "procedure",
            "library",
            "module",
            "package",
            "import",
            "require",
        ];

        let content_lower = content.to_lowercase();
        utility_keywords
            .iter()
            .any(|&keyword| content_lower.contains(keyword))
    }

    /// Build keyword patterns for different script types
    fn build_keyword_patterns() -> HashMap<ScriptType, Vec<Regex>> {
        let mut patterns = HashMap::new();

        // Validator patterns
        patterns.insert(
            ScriptType::Validator,
            vec![
                Regex::new(r"validate").unwrap(),
                Regex::new(r"check").unwrap(),
                Regex::new(r"verify").unwrap(),
                Regex::new(r"assert").unwrap(),
                Regex::new(r"test").unwrap(),
                Regex::new(r"spec").unwrap(),
            ],
        );

        // Runner patterns
        patterns.insert(
            ScriptType::Runner,
            vec![
                Regex::new(r"run").unwrap(),
                Regex::new(r"execute").unwrap(),
                Regex::new(r"start").unwrap(),
                Regex::new(r"launch").unwrap(),
            ],
        );

        // CI Integration patterns
        patterns.insert(
            ScriptType::CiIntegration,
            vec![
                Regex::new(r"ci").unwrap(),
                Regex::new(r"build").unwrap(),
                Regex::new(r"deploy").unwrap(),
                Regex::new(r"pipeline").unwrap(),
            ],
        );

        patterns
    }

    /// Build filename patterns for different script types
    fn build_filename_patterns() -> HashMap<ScriptType, Vec<Regex>> {
        let mut patterns = HashMap::new();

        // Validator patterns
        patterns.insert(
            ScriptType::Validator,
            vec![
                Regex::new(r"(?i)validate").unwrap(),
                Regex::new(r"(?i)check").unwrap(),
                Regex::new(r"(?i)test").unwrap(),
                Regex::new(r"(?i)spec").unwrap(),
            ],
        );

        // Runner patterns
        patterns.insert(
            ScriptType::Runner,
            vec![
                Regex::new(r"(?i)run").unwrap(),
                Regex::new(r"(?i)execute").unwrap(),
                Regex::new(r"(?i)start").unwrap(),
            ],
        );

        // CI Integration patterns
        patterns.insert(
            ScriptType::CiIntegration,
            vec![
                Regex::new(r"(?i)ci").unwrap(),
                Regex::new(r"(?i)build").unwrap(),
                Regex::new(r"(?i)deploy").unwrap(),
            ],
        );

        patterns
    }

    /// Build shebang patterns for different script types
    fn build_shebang_patterns() -> HashMap<String, ScriptType> {
        let mut patterns = HashMap::new();

        // Shell scripts are typically utilities or validators
        patterns.insert("/bin/bash".to_string(), ScriptType::Utility);
        patterns.insert("/bin/sh".to_string(), ScriptType::Utility);
        patterns.insert("/usr/bin/bash".to_string(), ScriptType::Utility);

        // Python scripts can be any type, default to utility
        patterns.insert("/usr/bin/python".to_string(), ScriptType::Utility);
        patterns.insert("/usr/bin/python3".to_string(), ScriptType::Utility);
        patterns.insert("/usr/local/bin/python".to_string(), ScriptType::Utility);

        // Node.js scripts are often runners or utilities
        patterns.insert("/usr/bin/node".to_string(), ScriptType::Runner);

        patterns
    }
}

impl Default for ScriptClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::models::test_asset::AssetType;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_classification_config_default() {
        let config = ClassificationConfig::default();
        assert!(config.keyword_matching);
        assert!(config.content_analysis);
        assert!(config.shebang_analysis);
        assert!(config.filename_analysis);
        assert!(!config.dependency_analysis);
        assert_eq!(config.confidence_threshold, 0.5);
        assert!(config.enable_fallback);
    }

    #[test]
    fn test_script_classifier_creation() {
        let classifier = ScriptClassifier::new();
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_filename_classification() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("validate_test.sh");

        // Create a test script
        fs::write(&script_path, "#!/bin/bash\necho 'validation'\n")?;

        let asset = TestAsset::new(
            "test-001".to_string(),
            "Validation Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_filename(&asset)?;

        assert!(result.confidence > 0.5); // Should detect "validate" in filename
        assert_eq!(result.classification_method, "filename_analysis");

        Ok(())
    }

    #[test]
    fn test_shebang_classification() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");

        // Create a test script with Python shebang
        fs::write(&script_path, "#!/usr/bin/python3\nprint('test')\n")?;

        let asset = TestAsset::new(
            "test-002".to_string(),
            "Test Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_shebang(&asset)?;

        assert_eq!(
            result.metadata.shebang_detected,
            Some("/usr/bin/python3".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_keyword_classification() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test_script.sh");

        // Create a script with validation keywords
        fs::write(
            &script_path,
            "#!/bin/bash\nvalidate_function() {\n  check_output\n  verify_result\n}\n",
        )?;

        let asset = TestAsset::new(
            "test-003".to_string(),
            "Test Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_keywords(&asset)?;

        assert!(result.confidence > 0.0); // Should detect validation keywords
        assert!(result.metadata.content_analyzed);

        Ok(())
    }

    #[test]
    fn test_script_classifier_default() {
        let classifier: ScriptClassifier = Default::default();
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_script_classifier_with_config() {
        let config = ClassificationConfig {
            keyword_matching: false,
            content_analysis: false,
            shebang_analysis: false,
            filename_analysis: false,
            dependency_analysis: false,
            custom_rules: Vec::new(),
            confidence_threshold: 0.8,
            enable_fallback: false,
        };
        let classifier = ScriptClassifier::with_config(config);
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_classify_script_empty_file() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("empty.sh");

        // Create an empty script
        fs::write(&script_path, "")?;

        let asset = TestAsset::new(
            "test-empty".to_string(),
            "Empty Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_script(&asset)?;

        // Empty file should fall back to Utility with low confidence
        assert!(matches!(result.script_type, ScriptType::Utility));
        assert!(result.confidence >= 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_script_no_shebang() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("no_shebang.sh");

        // Create a script without shebang
        fs::write(&script_path, "echo 'hello world'\n")?;

        let asset = TestAsset::new(
            "test-no-shebang".to_string(),
            "No Shebang Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_script(&asset)?;

        // Should still classify without panicking
        assert!(result.confidence >= 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_script_bash_shebang() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("bash_script.sh");

        // Create a script with bash shebang
        fs::write(&script_path, "#!/bin/bash\necho 'hello'\n")?;

        let asset = TestAsset::new(
            "test-bash".to_string(),
            "Bash Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_script(&asset)?;

        // Bash shebang should be detected
        assert_eq!(
            result.metadata.shebang_detected,
            Some("/bin/bash".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_classify_script_python_shebang() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("python_script.py");

        // Create a script with python shebang
        fs::write(&script_path, "#!/usr/bin/python3\nprint('hello')\n")?;

        let asset = TestAsset::new(
            "test-python".to_string(),
            "Python Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_script(&asset)?;

        // Python shebang should be detected
        assert_eq!(
            result.metadata.shebang_detected,
            Some("/usr/bin/python3".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_classify_script_node_shebang() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("node_script.js");

        // Create a script with node shebang
        fs::write(&script_path, "#!/usr/bin/node\nconsole.log('hello')\n")?;

        let asset = TestAsset::new(
            "test-node".to_string(),
            "Node Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_script(&asset)?;

        // Node shebang should be detected
        assert_eq!(
            result.metadata.shebang_detected,
            Some("/usr/bin/node".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_classify_script_runner_keywords() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("runner_script.sh");

        // Create a script with runner keywords
        fs::write(
            &script_path,
            "#!/bin/bash\nrun_tests() {\n  execute_all\n  launch_app\n}\n",
        )?;

        let asset = TestAsset::new(
            "test-runner".to_string(),
            "Runner Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_keywords(&asset)?;

        // Should detect runner keywords
        assert!(result.confidence > 0.0);
        assert!(result.metadata.content_analyzed);

        Ok(())
    }

    #[test]
    fn test_classify_script_ci_keywords() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("ci_script.sh");

        // Create a script with CI keywords
        fs::write(
            &script_path,
            "#!/bin/bash\necho 'ci pipeline build deploy'\n",
        )?;

        let asset = TestAsset::new(
            "test-ci".to_string(),
            "CI Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_keywords(&asset)?;

        // Should detect CI keywords
        assert!(result.confidence > 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_script_utility_content() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("util_script.sh");

        fs::write(
            &script_path,
            "#!/bin/bash\nhelper_function() {\n  util\n  tool\n  library\n  import\n}\n",
        )?;

        let asset = TestAsset::new(
            "test-utility".to_string(),
            "Utility Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_content(&asset)?;

        assert!(result.confidence > 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_script_by_content_validation() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("validate_content.sh");

        // Create a script with validation patterns
        fs::write(&script_path, "#!/bin/bash\nvalidate_input() {\n  assert_equal\n  check_result\n  verify_output\n  expect_true\n}\n")?;

        let asset = TestAsset::new(
            "test-validate-content".to_string(),
            "Validate Content Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_content(&asset)?;

        // Should detect validation patterns in content
        assert!(result.confidence > 0.0);
        assert!(result.metadata.content_analyzed);

        Ok(())
    }

    #[test]
    fn test_classify_script_by_content_runner() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("run_content.sh");

        // Create a script with runner patterns
        fs::write(
            &script_path,
            "#!/bin/bash\nrun_all_tests() {\n  execute_command\n  start_service\n  launch_app\n}\n",
        )?;

        let asset = TestAsset::new(
            "test-run-content".to_string(),
            "Run Content Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_content(&asset)?;

        // Should detect runner patterns
        assert!(result.confidence > 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_script_by_content_ci() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("build_content.sh");

        // Create a script with CI patterns
        fs::write(
            &script_path,
            "#!/bin/bash\necho 'ci build pipeline deploy jenkins'\n",
        )?;

        let asset = TestAsset::new(
            "test-ci-content".to_string(),
            "CI Content Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_content(&asset)?;

        // Should detect CI patterns
        assert!(result.confidence > 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_scripts_multiple() -> Result<()> {
        let temp_dir = tempdir()?;

        let script1_path = temp_dir.path().join("validate_test.sh");
        fs::write(&script1_path, "#!/bin/bash\nvalidate\n")?;

        let script2_path = temp_dir.path().join("run_test.sh");
        fs::write(&script2_path, "#!/bin/bash\nrun\n")?;

        let fixture_path = temp_dir.path().join("fixture.txt");
        fs::write(&fixture_path, "fixture data\n")?;

        let assets = vec![
            TestAsset::new(
                "test-001".to_string(),
                "Validate Script".to_string(),
                AssetType::Script,
                script1_path.clone(),
                script1_path.clone(),
            ),
            TestAsset::new(
                "test-002".to_string(),
                "Run Script".to_string(),
                AssetType::Script,
                script2_path.clone(),
                script2_path.clone(),
            ),
            TestAsset::new(
                "test-003".to_string(),
                "Fixture".to_string(),
                AssetType::Fixture,
                fixture_path.clone(),
                fixture_path.clone(),
            ),
        ];

        let classifier = ScriptClassifier::new();
        let results = classifier.classify_scripts(&assets)?;

        // Should only classify Script assets
        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[test]
    fn test_classification_result_fields() {
        let result = ClassificationResult {
            script_type: ScriptType::Validator,
            confidence: 0.95,
            classification_method: "test_method".to_string(),
            supporting_evidence: vec!["evidence1".to_string()],
            alternative_types: vec![(ScriptType::Runner, 0.3)],
            metadata: ClassificationMetadata {
                keywords_found: vec!["validate".to_string()],
                file_patterns_matched: vec!["*validate*".to_string()],
                shebang_detected: Some("/bin/bash".to_string()),
                dependencies_found: vec![],
                content_analyzed: true,
                processing_time_ms: 100,
            },
        };

        assert!(matches!(result.script_type, ScriptType::Validator));
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.classification_method, "test_method");
        assert_eq!(result.supporting_evidence.len(), 1);
        assert_eq!(result.alternative_types.len(), 1);
        assert_eq!(result.metadata.keywords_found.len(), 1);
        assert_eq!(result.metadata.processing_time_ms, 100);
    }

    #[test]
    fn test_classification_metadata_default() {
        let metadata = ClassificationMetadata {
            keywords_found: Vec::new(),
            file_patterns_matched: Vec::new(),
            shebang_detected: None,
            dependencies_found: Vec::new(),
            content_analyzed: false,
            processing_time_ms: 0,
        };

        assert!(metadata.keywords_found.is_empty());
        assert!(metadata.shebang_detected.is_none());
        assert!(!metadata.content_analyzed);
    }

    #[test]
    fn test_script_type_variants() {
        let validator = ScriptType::Validator;
        let runner = ScriptType::Runner;
        let utility = ScriptType::Utility;
        let ci = ScriptType::CiIntegration;

        let _ = validator.clone();
        let _ = runner.clone();
        let _ = utility.clone();
        let _ = ci.clone();
    }

    #[test]
    fn test_classification_rule_creation() {
        let rule = ClassificationRule {
            name: "test_rule".to_string(),
            script_type: ScriptType::Validator,
            conditions: vec![ClassificationCondition::FilenameContains {
                pattern: "validate".to_string(),
                case_sensitive: false,
            }],
            weight: 1.0,
            description: "Test rule".to_string(),
        };

        assert_eq!(rule.name, "test_rule");
        assert!(matches!(rule.script_type, ScriptType::Validator));
        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.weight, 1.0);
    }

    #[test]
    fn test_classification_condition_variants() {
        let filename = ClassificationCondition::FilenameContains {
            pattern: "test".to_string(),
            case_sensitive: true,
        };
        let content = ClassificationCondition::ContentContains {
            pattern: "test".to_string(),
            case_sensitive: false,
        };
        let shebang = ClassificationCondition::ShebangMatches {
            pattern: "bash".to_string(),
        };
        let extension = ClassificationCondition::FileExtension {
            extension: "sh".to_string(),
        };
        let size = ClassificationCondition::FileSize {
            min_bytes: Some(10),
            max_bytes: Some(1000),
        };
        let custom = ClassificationCondition::CustomCondition {
            name: "custom".to_string(),
            parameters: std::collections::HashMap::new(),
        };

        let _ = filename;
        let _ = content;
        let _ = shebang;
        let _ = extension;
        let _ = size;
        let _ = custom;
    }

    #[test]
    fn test_custom_rule_classification() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("my_custom_script.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'hello'\n")?;

        let asset = TestAsset::new(
            "test-custom".to_string(),
            "Custom Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let config = ClassificationConfig {
            keyword_matching: false,
            content_analysis: false,
            shebang_analysis: false,
            filename_analysis: false,
            dependency_analysis: false,
            custom_rules: vec![ClassificationRule {
                name: "custom_validator".to_string(),
                script_type: ScriptType::Validator,
                conditions: vec![ClassificationCondition::FilenameContains {
                    pattern: "custom".to_string(),
                    case_sensitive: false,
                }],
                weight: 1.0,
                description: "Custom validator rule".to_string(),
            }],
            confidence_threshold: 0.5,
            enable_fallback: true,
        };

        let classifier = ScriptClassifier::with_config(config);
        let result = classifier.classify_script(&asset)?;

        // Custom rule should match
        assert!(result.confidence > 0.0);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_filename_contains() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("validate_test.sh");
        fs::write(&script_path, "#!/bin/bash\n")?;

        let asset = TestAsset::new(
            "test-eval".to_string(),
            "Eval Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::FilenameContains {
            pattern: "validate".to_string(),
            case_sensitive: false,
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(result);

        let condition_case = ClassificationCondition::FilenameContains {
            pattern: "VALIDATE".to_string(),
            case_sensitive: true,
        };
        let result_case = classifier.evaluate_condition(&asset, &condition_case)?;
        assert!(!result_case);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_content_contains() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'validate_result'\n")?;

        let asset = TestAsset::new(
            "test-content".to_string(),
            "Content Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::ContentContains {
            pattern: "validate".to_string(),
            case_sensitive: false,
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(result);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_file_extension() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\n")?;

        let asset = TestAsset::new(
            "test-ext".to_string(),
            "Extension Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::FileExtension {
            extension: "sh".to_string(),
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(result);

        let condition_bad = ClassificationCondition::FileExtension {
            extension: "py".to_string(),
        };
        let result_bad = classifier.evaluate_condition(&asset, &condition_bad)?;
        assert!(!result_bad);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_file_size() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'hello'\n")?;

        let asset = TestAsset::new(
            "test-size".to_string(),
            "Size Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::FileSize {
            min_bytes: Some(1),
            max_bytes: Some(10000),
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(result);

        let condition_too_small = ClassificationCondition::FileSize {
            min_bytes: Some(10000),
            max_bytes: None,
        };
        let result_small = classifier.evaluate_condition(&asset, &condition_too_small)?;
        assert!(!result_small);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_shebang_matches() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/usr/bin/python3\nprint('hello')\n")?;

        let asset = TestAsset::new(
            "test-shebang-cond".to_string(),
            "Shebang Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::ShebangMatches {
            pattern: "python".to_string(),
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(result);

        let condition_bad = ClassificationCondition::ShebangMatches {
            pattern: "ruby".to_string(),
        };
        let result_bad = classifier.evaluate_condition(&asset, &condition_bad)?;
        assert!(!result_bad);

        Ok(())
    }

    #[test]
    fn test_evaluate_condition_custom() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\n")?;

        let asset = TestAsset::new(
            "test-custom-cond".to_string(),
            "Custom Cond Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();

        let condition = ClassificationCondition::CustomCondition {
            name: "always_false".to_string(),
            parameters: std::collections::HashMap::new(),
        };
        let result = classifier.evaluate_condition(&asset, &condition)?;
        assert!(!result);

        Ok(())
    }

    #[test]
    fn test_combine_classification_results_empty_with_fallback() {
        let classifier = ScriptClassifier::new();
        let results: Vec<ClassificationResult> = Vec::new();
        let start_time = std::time::Instant::now();

        let result = classifier
            .combine_classification_results(results, &start_time)
            .unwrap();

        assert!(matches!(result.script_type, ScriptType::Utility));
        assert!(result.confidence > 0.0); // Fallback enabled by default
        assert_eq!(result.classification_method, "fallback");
    }

    #[test]
    fn test_combine_classification_results_empty_no_fallback() {
        let config = ClassificationConfig {
            keyword_matching: true,
            content_analysis: true,
            shebang_analysis: true,
            filename_analysis: true,
            dependency_analysis: false,
            custom_rules: Vec::new(),
            confidence_threshold: 0.5,
            enable_fallback: false,
        };
        let classifier = ScriptClassifier::with_config(config);
        let results: Vec<ClassificationResult> = Vec::new();
        let start_time = std::time::Instant::now();

        let result = classifier
            .combine_classification_results(results, &start_time)
            .unwrap();

        assert!(matches!(result.script_type, ScriptType::Utility));
        assert_eq!(result.confidence, 0.0); // No fallback
    }

    #[test]
    fn test_combine_classification_results_low_confidence() {
        let classifier = ScriptClassifier::new();
        let results = vec![ClassificationResult {
            script_type: ScriptType::Runner,
            confidence: 0.1,
            classification_method: "test".to_string(),
            supporting_evidence: vec![],
            alternative_types: vec![],
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: false,
                processing_time_ms: 0,
            },
        }];
        let start_time = std::time::Instant::now();

        let result = classifier
            .combine_classification_results(results, &start_time)
            .unwrap();

        // Low confidence with fallback should return Utility
        assert!(matches!(result.script_type, ScriptType::Utility));
    }

    #[test]
    fn test_combine_classification_results_high_confidence() {
        let classifier = ScriptClassifier::new();
        let results = vec![ClassificationResult {
            script_type: ScriptType::Validator,
            confidence: 0.9,
            classification_method: "test".to_string(),
            supporting_evidence: vec!["evidence".to_string()],
            alternative_types: vec![],
            metadata: ClassificationMetadata {
                keywords_found: Vec::new(),
                file_patterns_matched: Vec::new(),
                shebang_detected: None,
                dependencies_found: Vec::new(),
                content_analyzed: false,
                processing_time_ms: 0,
            },
        }];
        let start_time = std::time::Instant::now();

        let result = classifier
            .combine_classification_results(results, &start_time)
            .unwrap();

        // High confidence should return the original result
        assert!(matches!(result.script_type, ScriptType::Validator));
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_read_shebang_none() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("no_shebang.txt");
        fs::write(&script_path, "just some text\n")?;

        let classifier = ScriptClassifier::new();
        let shebang = classifier.read_shebang(&script_path)?;

        assert!(shebang.is_none());

        Ok(())
    }

    #[test]
    fn test_read_shebang_with_newline() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("with_shebang.sh");
        fs::write(&script_path, "#!/usr/bin/env python3\nprint('hello')\n")?;

        let classifier = ScriptClassifier::new();
        let shebang = classifier.read_shebang(&script_path)?;

        assert_eq!(shebang, Some("/usr/bin/env python3".to_string()));

        Ok(())
    }

    #[test]
    fn test_read_shebang_no_newline() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("shebang_only.sh");
        fs::write(&script_path, "#!/bin/bash")?;

        let classifier = ScriptClassifier::new();
        let shebang = classifier.read_shebang(&script_path)?;

        assert_eq!(shebang, Some("/bin/bash".to_string()));

        Ok(())
    }

    #[test]
    fn test_contains_validation_patterns() {
        let classifier = ScriptClassifier::new();
        assert!(classifier.contains_validation_patterns("validate input"));
        assert!(classifier.contains_validation_patterns("check output"));
        assert!(classifier.contains_validation_patterns("verify result"));
        assert!(classifier.contains_validation_patterns("assertEqual"));
        assert!(!classifier.contains_validation_patterns("hello world"));
    }

    #[test]
    fn test_contains_runner_patterns() {
        let classifier = ScriptClassifier::new();
        assert!(classifier.contains_runner_patterns("run tests"));
        assert!(classifier.contains_runner_patterns("execute command"));
        assert!(classifier.contains_runner_patterns("launch app"));
        assert!(classifier.contains_runner_patterns("main function"));
        assert!(!classifier.contains_runner_patterns("hello world"));
    }

    #[test]
    fn test_contains_ci_patterns() {
        let classifier = ScriptClassifier::new();
        assert!(classifier.contains_ci_patterns("ci pipeline"));
        assert!(classifier.contains_ci_patterns("build project"));
        assert!(classifier.contains_ci_patterns("deploy to production"));
        assert!(classifier.contains_ci_patterns("github actions"));
        assert!(!classifier.contains_ci_patterns("hello world"));
    }

    #[test]
    fn test_contains_utility_patterns() {
        let classifier = ScriptClassifier::new();
        assert!(classifier.contains_utility_patterns("helper function"));
        assert!(classifier.contains_utility_patterns("util module"));
        assert!(classifier.contains_utility_patterns("library import"));
        assert!(classifier.contains_utility_patterns("package tool"));
        assert!(!classifier.contains_utility_patterns("hello world"));
    }

    #[test]
    fn test_classify_by_filename_no_match() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("random_name.sh");
        fs::write(&script_path, "#!/bin/bash\n")?;

        let asset = TestAsset::new(
            "test-no-match".to_string(),
            "No Match Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_filename(&asset)?;

        // No filename match should return Utility with 0 confidence
        assert!(matches!(result.script_type, ScriptType::Utility));
        assert_eq!(result.confidence, 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_by_shebang_no_match() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/usr/bin/ruby\nputs 'hello'\n")?;

        let asset = TestAsset::new(
            "test-shebang-no-match".to_string(),
            "Shebang No Match Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_shebang(&asset)?;

        // Unknown shebang should return Utility with 0 confidence
        assert!(matches!(result.script_type, ScriptType::Utility));
        assert_eq!(result.confidence, 0.0);
        assert_eq!(
            result.metadata.shebang_detected,
            Some("/usr/bin/ruby".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_classify_by_keywords_no_match() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("plain.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'hello world'\n")?;

        let asset = TestAsset::new(
            "test-keywords-no-match".to_string(),
            "Plain Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let result = classifier.classify_by_keywords(&asset)?;

        // No keyword match should return Utility with 0 confidence
        assert!(matches!(result.script_type, ScriptType::Utility));
        assert_eq!(result.confidence, 0.0);

        Ok(())
    }

    #[test]
    fn test_classify_by_custom_rule_no_match() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\n")?;

        let asset = TestAsset::new(
            "test-custom-no-match".to_string(),
            "Custom No Match Script".to_string(),
            AssetType::Script,
            script_path.clone(),
            script_path.clone(),
        );

        let classifier = ScriptClassifier::new();
        let rule = ClassificationRule {
            name: "no_match_rule".to_string(),
            script_type: ScriptType::Validator,
            conditions: vec![ClassificationCondition::FilenameContains {
                pattern: "nonexistent".to_string(),
                case_sensitive: true,
            }],
            weight: 1.0,
            description: "Rule that won't match".to_string(),
        };

        let result = classifier.classify_by_custom_rule(&asset, &rule)?;

        // No conditions met should return 0 confidence
        assert_eq!(result.confidence, 0.0);

        Ok(())
    }
}
