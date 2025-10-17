//! Sanitizer detection for data flow analysis
//! 
//! This module identifies sanitizers that clean or validate tainted data.

use crate::graph::{DataFlowGraph, DataFlowNode, NodeId};
use cr_core::{Location, Result};
use std::collections::HashMap;

/// Represents a sanitizer that cleans tainted data
#[derive(Debug, Clone)]
pub struct Sanitizer {
    pub id: NodeId,
    pub sanitizer_type: SanitizerType,
    pub location: Option<Location>,
    pub description: String,
    pub effectiveness: f32,
    pub vulnerability_types: Vec<String>,
}

impl Sanitizer {
    /// Create a new sanitizer
    pub fn new(id: NodeId, sanitizer_type: SanitizerType, description: String) -> Self {
        Self {
            id,
            sanitizer_type,
            location: None,
            description,
            effectiveness: 1.0,
            vulnerability_types: Vec::new(),
        }
    }

    /// Set location
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Set effectiveness level
    pub fn with_effectiveness(mut self, effectiveness: f32) -> Self {
        self.effectiveness = effectiveness.clamp(0.0, 1.0);
        self
    }

    /// Add vulnerability types this sanitizer protects against
    pub fn with_vulnerability_types(mut self, types: Vec<String>) -> Self {
        self.vulnerability_types = types;
        self
    }

    /// Check if this sanitizer protects against a specific vulnerability type
    pub fn protects_against(&self, vulnerability_type: &str) -> bool {
        self.vulnerability_types.iter().any(|vt| vt == vulnerability_type) ||
        self.sanitizer_type.default_protections().contains(&vulnerability_type.to_string())
    }

    /// Check if this sanitizer is highly effective
    pub fn is_highly_effective(&self) -> bool {
        self.effectiveness > 0.8
    }

    /// Get effectiveness against a specific source type and vulnerability
    pub fn effectiveness_against(&self, source_type: &crate::sources::SourceType, vulnerability_type: &str) -> f32 {
        if self.protects_against(vulnerability_type) {
            // Adjust effectiveness based on source type
            match source_type {
                crate::sources::SourceType::UserInput => self.effectiveness,
                crate::sources::SourceType::DatabaseInput => self.effectiveness * 0.9,
                crate::sources::SourceType::FileInput => self.effectiveness * 0.8,
                _ => self.effectiveness * 0.7,
            }
        } else {
            0.0
        }
    }

    // protects_against and is_highly_effective methods are already defined above

    /// Get the strength of this sanitizer type
    pub fn strength(&self) -> SanitizerStrength {
        self.sanitizer_type.strength()
    }
}

/// Types of sanitizers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SanitizerType {
    /// Input validation
    InputValidation,
    /// Output encoding
    OutputEncoding,
    /// SQL parameter binding
    SqlParameterBinding,
    /// HTML encoding
    HtmlEncoding,
    /// URL encoding
    UrlEncoding,
    /// JavaScript encoding
    JavaScriptEncoding,
    /// Path normalization
    PathNormalization,
    /// Regular expression validation
    RegexValidation,
    /// Whitelist filtering
    WhitelistFiltering,
    /// Blacklist filtering
    BlacklistFiltering,
    /// Length validation
    LengthValidation,
    /// Type validation
    TypeValidation,
}

impl SanitizerType {
    /// Get the strength of this sanitizer type
    pub fn strength(&self) -> SanitizerStrength {
        match self {
            SanitizerType::SqlParameterBinding | SanitizerType::OutputEncoding => SanitizerStrength::Strong,
            SanitizerType::InputValidation | SanitizerType::HtmlEncoding | SanitizerType::WhitelistFiltering => SanitizerStrength::Strong,
            SanitizerType::UrlEncoding | SanitizerType::JavaScriptEncoding | SanitizerType::PathNormalization => SanitizerStrength::Medium,
            SanitizerType::RegexValidation | SanitizerType::LengthValidation | SanitizerType::TypeValidation => SanitizerStrength::Medium,
            SanitizerType::BlacklistFiltering => SanitizerStrength::Weak,
        }
    }

    /// Get default vulnerability types this sanitizer protects against
    pub fn default_protections(&self) -> Vec<String> {
        match self {
            SanitizerType::SqlParameterBinding => vec!["SQL_INJECTION".to_string()],
            SanitizerType::HtmlEncoding | SanitizerType::OutputEncoding => vec!["XSS".to_string()],
            SanitizerType::UrlEncoding => vec!["XSS".to_string(), "HTTP_RESPONSE_SPLITTING".to_string()],
            SanitizerType::JavaScriptEncoding => vec!["XSS".to_string(), "CODE_INJECTION".to_string()],
            SanitizerType::PathNormalization => vec!["PATH_TRAVERSAL".to_string()],
            SanitizerType::InputValidation | SanitizerType::WhitelistFiltering => {
                vec!["XSS".to_string(), "SQL_INJECTION".to_string(), "COMMAND_INJECTION".to_string()]
            }
            SanitizerType::RegexValidation | SanitizerType::LengthValidation | SanitizerType::TypeValidation => {
                vec!["INPUT_VALIDATION".to_string()]
            }
            SanitizerType::BlacklistFiltering => vec!["BASIC_FILTERING".to_string()],
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SanitizerType::InputValidation => "input_validation",
            SanitizerType::OutputEncoding => "output_encoding",
            SanitizerType::SqlParameterBinding => "sql_parameter_binding",
            SanitizerType::HtmlEncoding => "html_encoding",
            SanitizerType::UrlEncoding => "url_encoding",
            SanitizerType::JavaScriptEncoding => "javascript_encoding",
            SanitizerType::PathNormalization => "path_normalization",
            SanitizerType::RegexValidation => "regex_validation",
            SanitizerType::WhitelistFiltering => "whitelist_filtering",
            SanitizerType::BlacklistFiltering => "blacklist_filtering",
            SanitizerType::LengthValidation => "length_validation",
            SanitizerType::TypeValidation => "type_validation",
        }
    }
}

/// Strength levels for sanitizers
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SanitizerStrength {
    Weak,
    Medium,
    Strong,
}

/// Sanitizer detector
pub struct SanitizerDetector {
    patterns: HashMap<String, Vec<SanitizerPattern>>,
}

impl SanitizerDetector {
    /// Create a new sanitizer detector
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: HashMap::new(),
        };
        detector.load_default_patterns();
        detector
    }

    /// Detect sanitizers in a data flow graph
    pub fn detect_sanitizers(&self, graph: &DataFlowGraph) -> Result<Vec<Sanitizer>> {
        let mut sanitizers = Vec::new();

        for (node_id, node) in graph.nodes() {
            if let Some(sanitizer) = self.check_node_for_sanitizer(*node_id, node) {
                sanitizers.push(sanitizer);
            }
        }

        Ok(sanitizers)
    }

    /// Check if a node is a sanitizer
    fn check_node_for_sanitizer(&self, node_id: NodeId, node: &DataFlowNode) -> Option<Sanitizer> {
        // Check function calls
        if node.is_function_call() {
            if let Some(function_name) = node.function_name() {
                for pattern in self.get_patterns_for_type(&node.node_type) {
                    if pattern.matches(function_name, node) {
                        return Some(
                            Sanitizer::new(node_id, pattern.sanitizer_type.clone(), pattern.description.clone())
                                .with_effectiveness(pattern.effectiveness)
                                .with_vulnerability_types(pattern.vulnerability_types.clone())
                        );
                    }
                }
            }
        }

        None
    }

    /// Get patterns for a node type
    fn get_patterns_for_type(&self, node_type: &str) -> &[SanitizerPattern] {
        self.patterns.get(node_type).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Add a custom sanitizer pattern
    pub fn add_pattern(&mut self, node_type: String, pattern: SanitizerPattern) {
        self.patterns.entry(node_type).or_insert_with(Vec::new).push(pattern);
    }

    /// Load default sanitizer patterns
    fn load_default_patterns(&mut self) {
        // SQL parameter binding
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "prepareStatement".to_string(),
            sanitizer_type: SanitizerType::SqlParameterBinding,
            description: "SQL prepared statement".to_string(),
            effectiveness: 0.95,
            vulnerability_types: vec!["SQL_INJECTION".to_string()],
            attributes: HashMap::new(),
        });

        // HTML encoding
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "htmlEncode".to_string(),
            sanitizer_type: SanitizerType::HtmlEncoding,
            description: "HTML encoding function".to_string(),
            effectiveness: 0.9,
            vulnerability_types: vec!["XSS".to_string()],
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "escapeHtml".to_string(),
            sanitizer_type: SanitizerType::HtmlEncoding,
            description: "HTML escape function".to_string(),
            effectiveness: 0.9,
            vulnerability_types: vec!["XSS".to_string()],
            attributes: HashMap::new(),
        });

        // URL encoding
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "encodeURIComponent".to_string(),
            sanitizer_type: SanitizerType::UrlEncoding,
            description: "URL component encoding".to_string(),
            effectiveness: 0.8,
            vulnerability_types: vec!["XSS".to_string()],
            attributes: HashMap::new(),
        });

        // Input validation
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "validate".to_string(),
            sanitizer_type: SanitizerType::InputValidation,
            description: "Input validation function".to_string(),
            effectiveness: 0.8,
            vulnerability_types: vec!["INPUT_VALIDATION".to_string()],
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "sanitize".to_string(),
            sanitizer_type: SanitizerType::InputValidation,
            description: "Input sanitization function".to_string(),
            effectiveness: 0.7,
            vulnerability_types: vec!["XSS".to_string(), "SQL_INJECTION".to_string()],
            attributes: HashMap::new(),
        });

        // Path normalization
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "path.normalize".to_string(),
            sanitizer_type: SanitizerType::PathNormalization,
            description: "Path normalization".to_string(),
            effectiveness: 0.8,
            vulnerability_types: vec!["PATH_TRAVERSAL".to_string()],
            attributes: HashMap::new(),
        });

        // Regular expression validation
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "match".to_string(),
            sanitizer_type: SanitizerType::RegexValidation,
            description: "Regular expression matching".to_string(),
            effectiveness: 0.6,
            vulnerability_types: vec!["INPUT_VALIDATION".to_string()],
            attributes: HashMap::new(),
        });

        // Length validation
        self.add_pattern("call_expression".to_string(), SanitizerPattern {
            name_pattern: "length".to_string(),
            sanitizer_type: SanitizerType::LengthValidation,
            description: "Length validation".to_string(),
            effectiveness: 0.5,
            vulnerability_types: vec!["BUFFER_OVERFLOW".to_string()],
            attributes: HashMap::new(),
        });
    }
}

impl Default for SanitizerDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for detecting sanitizers
#[derive(Debug, Clone)]
pub struct SanitizerPattern {
    pub name_pattern: String,
    pub sanitizer_type: SanitizerType,
    pub description: String,
    pub effectiveness: f32,
    pub vulnerability_types: Vec<String>,
    pub attributes: HashMap<String, String>,
}

impl SanitizerPattern {
    /// Check if this pattern matches a node
    pub fn matches(&self, name: &str, _node: &DataFlowNode) -> bool {
        // Simple string matching for now
        name.contains(&self.name_pattern)
    }

    /// Create a new sanitizer pattern
    pub fn new(
        name_pattern: String,
        sanitizer_type: SanitizerType,
        description: String,
        effectiveness: f32,
        vulnerability_types: Vec<String>,
    ) -> Self {
        Self {
            name_pattern,
            sanitizer_type,
            description,
            effectiveness: effectiveness.clamp(0.0, 1.0),
            vulnerability_types,
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute to the pattern
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::DataFlowGraph;

    #[test]
    fn test_sanitizer_creation() {
        let sanitizer = Sanitizer::new(
            0,
            SanitizerType::HtmlEncoding,
            "Test sanitizer".to_string(),
        )
        .with_effectiveness(0.9)
        .with_vulnerability_types(vec!["XSS".to_string()]);
        
        assert_eq!(sanitizer.id, 0);
        assert_eq!(sanitizer.sanitizer_type, SanitizerType::HtmlEncoding);
        assert_eq!(sanitizer.effectiveness, 0.9);
        assert!(sanitizer.protects_against("XSS"));
        assert!(sanitizer.is_highly_effective());
    }

    #[test]
    fn test_sanitizer_type_strength() {
        assert_eq!(SanitizerType::SqlParameterBinding.strength(), SanitizerStrength::Strong);
        assert_eq!(SanitizerType::UrlEncoding.strength(), SanitizerStrength::Medium);
        assert_eq!(SanitizerType::BlacklistFiltering.strength(), SanitizerStrength::Weak);
    }

    #[test]
    fn test_sanitizer_type_protections() {
        let protections = SanitizerType::HtmlEncoding.default_protections();
        assert!(protections.contains(&"XSS".to_string()));
        
        let sql_protections = SanitizerType::SqlParameterBinding.default_protections();
        assert!(sql_protections.contains(&"SQL_INJECTION".to_string()));
    }

    #[test]
    fn test_sanitizer_detector() {
        let detector = SanitizerDetector::new();
        assert!(!detector.patterns.is_empty());
    }

    #[test]
    fn test_sanitizer_pattern_matching() {
        let pattern = SanitizerPattern::new(
            "htmlEncode".to_string(),
            SanitizerType::HtmlEncoding,
            "HTML encoding".to_string(),
            0.9,
            vec!["XSS".to_string()],
        );

        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("htmlEncode".to_string());

        assert!(pattern.matches("htmlEncode", &node));
        assert!(!pattern.matches("other.method", &node));
    }

    #[test]
    fn test_detect_sanitizers_in_graph() {
        let mut graph = DataFlowGraph::new();
        let detector = SanitizerDetector::new();

        // Add a node that should be detected as a sanitizer
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("htmlEncode".to_string());
        let node_id = graph.add_node(node);

        let sanitizers = detector.detect_sanitizers(&graph).unwrap();
        assert_eq!(sanitizers.len(), 1);
        assert_eq!(sanitizers[0].id, node_id);
        assert_eq!(sanitizers[0].sanitizer_type, SanitizerType::HtmlEncoding);
        assert!(sanitizers[0].protects_against("XSS"));
    }

    #[test]
    fn test_custom_sanitizer_pattern() {
        let mut detector = SanitizerDetector::new();
        
        let custom_pattern = SanitizerPattern::new(
            "customSanitize".to_string(),
            SanitizerType::InputValidation,
            "Custom sanitizer".to_string(),
            0.8,
            vec!["CUSTOM_VULN".to_string()],
        );
        
        detector.add_pattern("call_expression".to_string(), custom_pattern);
        
        let mut graph = DataFlowGraph::new();
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("customSanitize".to_string());
        graph.add_node(node);

        let sanitizers = detector.detect_sanitizers(&graph).unwrap();
        assert_eq!(sanitizers.len(), 1);
        assert_eq!(sanitizers[0].sanitizer_type, SanitizerType::InputValidation);
        assert!(sanitizers[0].protects_against("CUSTOM_VULN"));
    }
}
