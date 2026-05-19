//! Script categorization logic for functional classification

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use regex::Regex;
use tracing::{debug, info};

use astgrep_core::models::test_asset::{TestAsset, ScriptType, AssetType};

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
    FilenameContains { pattern: String, case_sensitive: bool },
    ContentContains { pattern: String, case_sensitive: bool },
    ShebangMatches { pattern: String },
    FileExtension { extension: String },
    FileSize { min_bytes: Option<u64>, max_bytes: Option<u64> },
    CustomCondition { name: String, parameters: HashMap<String, String> },
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

        debug!("Classifying script: {} ({})", asset.name, asset.current_path.display());

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

        info!("Script classified as {:?} with confidence {:.2}",
              final_result.script_type, final_result.confidence);

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
        let filename = asset.current_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut best_match = (ScriptType::Utility, 0.0, Vec::new());

        for (script_type, patterns) in &self.filename_patterns {
            for pattern in patterns {
                if pattern.is_match(filename) {
                    let confidence = 0.8; // High confidence for filename matches
                    if confidence > best_match.1 {
                        best_match = (script_type.clone(), confidence, vec![format!("Filename pattern: {}", pattern)]);
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
        let (best_type, best_score) = scores.iter()
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
                keywords_found: best_evidence.iter().map(|e| e.clone()).collect(),
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

        let scores = vec![
            (ScriptType::Validator, validator_score),
            (ScriptType::Runner, runner_score),
            (ScriptType::CiIntegration, ci_score),
            (ScriptType::Utility, utility_score),
        ];

        let (best_type, best_score) = scores.iter()
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
    fn classify_by_custom_rule(&self, asset: &TestAsset, rule: &ClassificationRule) -> Result<ClassificationResult> {
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
    fn evaluate_condition(&self, asset: &TestAsset, condition: &ClassificationCondition) -> Result<bool> {
        match condition {
            ClassificationCondition::FilenameContains { pattern, case_sensitive } => {
                let filename = asset.current_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if *case_sensitive {
                    Ok(filename.contains(pattern))
                } else {
                    Ok(filename.to_lowercase().contains(&pattern.to_lowercase()))
                }
            }
            ClassificationCondition::ContentContains { pattern, case_sensitive } => {
                let content = fs::read_to_string(&asset.current_path)?;
                if *case_sensitive {
                    Ok(content.contains(pattern))
                } else {
                    Ok(content.to_lowercase().contains(&pattern.to_lowercase()))
                }
            }
            ClassificationCondition::ShebangMatches { pattern } => {
                let shebang = self.read_shebang(&asset.current_path)?;
                Ok(shebang.map_or(false, |s| s.contains(pattern)))
            }
            ClassificationCondition::FileExtension { extension } => {
                let file_extension = asset.current_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                Ok(file_extension == extension)
            }
            ClassificationCondition::FileSize { min_bytes, max_bytes } => {
                let file_size = asset.get_file_size().unwrap_or(0);
                let min_ok = min_bytes.map_or(true, |min| file_size >= min);
                let max_ok = max_bytes.map_or(true, |max| file_size <= max);
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
                confidence: self.config.enable_fallback.then_some(0.1).unwrap_or(0.0),
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
        sorted_results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        let best_result = sorted_results.into_iter().next().unwrap();

        // Check if confidence meets threshold
        if best_result.confidence < self.config.confidence_threshold && self.config.enable_fallback {
            return Ok(ClassificationResult {
                script_type: ScriptType::Utility,
                confidence: best_result.confidence.max(0.1),
                classification_method: "fallback".to_string(),
                supporting_evidence: vec![format!("Low confidence ({}), using fallback", best_result.confidence)],
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

        if content.starts_with("#!") {
            if let Some(end_line) = content.find('\n') {
                let shebang_line = &content[2..end_line]; // Skip #!
                return Ok(Some(shebang_line.trim().to_string()));
            } else {
                return Ok(Some(content[2..].trim().to_string()));
            }
        }

        Ok(None)
    }

    /// Check if content contains validation patterns
    fn contains_validation_patterns(&self, content: &str) -> bool {
        let validation_keywords = [
            "validate", "check", "verify", "assert", "test", "spec", "expect",
            "assertEqual", "assertNotNull", "assertTrue", "assertFalse",
        ];

        let content_lower = content.to_lowercase();
        validation_keywords.iter().any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains runner patterns
    fn contains_runner_patterns(&self, content: &str) -> bool {
        let runner_keywords = [
            "run", "execute", "start", "launch", "invoke", "call", "perform",
            "main", "test_all", "run_all", "execute_all",
        ];

        let content_lower = content.to_lowercase();
        runner_keywords.iter().any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains CI patterns
    fn contains_ci_patterns(&self, content: &str) -> bool {
        let ci_keywords = [
            "ci", "build", "deploy", "pipeline", "continuous", "integration",
            "github", "jenkins", "travis", "circleci", "actions",
        ];

        let content_lower = content.to_lowercase();
        ci_keywords.iter().any(|&keyword| content_lower.contains(keyword))
    }

    /// Check if content contains utility patterns
    fn contains_utility_patterns(&self, content: &str) -> bool {
        let utility_keywords = [
            "helper", "util", "tool", "function", "method", "procedure",
            "library", "module", "package", "import", "require",
        ];

        let content_lower = content.to_lowercase();
        utility_keywords.iter().any(|&keyword| content_lower.contains(keyword))
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
    use std::fs;
    use tempfile::tempdir;
    use astgrep_core::models::test_asset::{AssetType};

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

        assert_eq!(result.shebang_detected, Some("/usr/bin/python3".to_string()));

        Ok(())
    }

    #[test]
    fn test_keyword_classification() -> Result<()> {
        let temp_dir = tempdir()?;
        let script_path = temp_dir.path().join("test_script.sh");

        // Create a script with validation keywords
        fs::write(&script_path, "#!/bin/bash\nvalidate_function() {\n  check_output\n  verify_result\n}\n")?;

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
}