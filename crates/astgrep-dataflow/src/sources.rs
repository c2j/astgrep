//! Source detection for data flow analysis
//! 
//! This module identifies sources of potentially tainted data in the program.

use crate::graph::{DataFlowGraph, DataFlowNode, NodeId};
use astgrep_core::{Location, Result};
use std::collections::HashMap;

/// Represents a source of tainted data
#[derive(Debug, Clone)]
pub struct Source {
    pub id: NodeId,
    pub source_type: SourceType,
    pub location: Option<Location>,
    pub description: String,
    pub confidence: f32,
}

impl Source {
    /// Create a new source
    pub fn new(id: NodeId, source_type: SourceType, description: String) -> Self {
        Self {
            id,
            source_type,
            location: None,
            description,
            confidence: 1.0,
        }
    }

    /// Set location
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Check if this is a high-confidence source
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Get the severity of this source type
    pub fn severity(&self) -> SourceSeverity {
        self.source_type.severity()
    }
}

/// Types of data sources
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    /// User input (HTTP parameters, form data, etc.)
    UserInput,
    /// File input
    FileInput,
    /// Network input
    NetworkInput,
    /// Database query results
    DatabaseInput,
    /// Environment variables
    EnvironmentInput,
    /// Command line arguments
    CommandLineInput,
    /// External API responses
    ExternalApiInput,
    /// Cookies
    CookieInput,
    /// Headers
    HeaderInput,
    /// URL parameters
    UrlParameterInput,
}

impl SourceType {
    /// Get the severity of this source type
    pub fn severity(&self) -> SourceSeverity {
        match self {
            SourceType::UserInput | SourceType::UrlParameterInput | SourceType::HeaderInput => SourceSeverity::High,
            SourceType::NetworkInput | SourceType::ExternalApiInput | SourceType::CookieInput => SourceSeverity::High,
            SourceType::FileInput | SourceType::DatabaseInput => SourceSeverity::Medium,
            SourceType::EnvironmentInput | SourceType::CommandLineInput => SourceSeverity::Low,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::UserInput => "user_input",
            SourceType::FileInput => "file_input",
            SourceType::NetworkInput => "network_input",
            SourceType::DatabaseInput => "database_input",
            SourceType::EnvironmentInput => "environment_input",
            SourceType::CommandLineInput => "command_line_input",
            SourceType::ExternalApiInput => "external_api_input",
            SourceType::CookieInput => "cookie_input",
            SourceType::HeaderInput => "header_input",
            SourceType::UrlParameterInput => "url_parameter_input",
        }
    }
}

/// Severity levels for sources
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceSeverity {
    Low,
    Medium,
    High,
}

/// Source detector
pub struct SourceDetector {
    patterns: HashMap<String, Vec<SourcePattern>>,
}

impl SourceDetector {
    /// Create a new source detector
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: HashMap::new(),
        };
        detector.load_default_patterns();
        detector
    }

    /// Detect sources in a data flow graph
    pub fn detect_sources(&self, graph: &DataFlowGraph) -> Result<Vec<Source>> {
        let mut sources = Vec::new();

        for (node_id, node) in graph.nodes() {
            if let Some(source) = self.check_node_for_source(*node_id, node) {
                sources.push(source);
            }
        }

        Ok(sources)
    }

    /// Check if a node is a source
    fn check_node_for_source(&self, node_id: NodeId, node: &DataFlowNode) -> Option<Source> {
        // Check function calls
        if node.is_function_call() {
            if let Some(function_name) = node.function_name() {
                for pattern in self.get_patterns_for_type(&node.node_type) {
                    if pattern.matches(function_name, node) {
                        return Some(
                            Source::new(node_id, pattern.source_type.clone(), pattern.description.clone())
                                .with_confidence(pattern.confidence)
                        );
                    }
                }
            }
        }

        // Check variable access
        if node.is_variable() {
            if let Some(var_name) = node.variable_name() {
                for pattern in self.get_patterns_for_type(&node.node_type) {
                    if pattern.matches(var_name, node) {
                        return Some(
                            Source::new(node_id, pattern.source_type.clone(), pattern.description.clone())
                                .with_confidence(pattern.confidence)
                        );
                    }
                }
            }
        }

        None
    }

    /// Get patterns for a node type
    fn get_patterns_for_type(&self, node_type: &str) -> &[SourcePattern] {
        self.patterns.get(node_type).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Add a custom source pattern
    pub fn add_pattern(&mut self, node_type: String, pattern: SourcePattern) {
        self.patterns.entry(node_type).or_insert_with(Vec::new).push(pattern);
    }

    /// Load default source patterns
    fn load_default_patterns(&mut self) {
        // HTTP request patterns
        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "request.getParameter".to_string(),
            source_type: SourceType::UserInput,
            description: "HTTP request parameter".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "request.getHeader".to_string(),
            source_type: SourceType::HeaderInput,
            description: "HTTP request header".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "request.getCookies".to_string(),
            source_type: SourceType::CookieInput,
            description: "HTTP cookies".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        // File input patterns
        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "readFile".to_string(),
            source_type: SourceType::FileInput,
            description: "File read operation".to_string(),
            confidence: 0.8,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "fs.readFileSync".to_string(),
            source_type: SourceType::FileInput,
            description: "Synchronous file read".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        // Database patterns
        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "executeQuery".to_string(),
            source_type: SourceType::DatabaseInput,
            description: "Database query result".to_string(),
            confidence: 0.7,
            attributes: HashMap::new(),
        });

        // Environment variables
        self.add_pattern("call_expression".to_string(), SourcePattern {
            name_pattern: "getenv".to_string(),
            source_type: SourceType::EnvironmentInput,
            description: "Environment variable".to_string(),
            confidence: 0.6,
            attributes: HashMap::new(),
        });

        self.add_pattern("identifier".to_string(), SourcePattern {
            name_pattern: "process.env".to_string(),
            source_type: SourceType::EnvironmentInput,
            description: "Node.js environment variable".to_string(),
            confidence: 0.7,
            attributes: HashMap::new(),
        });

        // Command line arguments
        self.add_pattern("identifier".to_string(), SourcePattern {
            name_pattern: "sys.argv".to_string(),
            source_type: SourceType::CommandLineInput,
            description: "Python command line arguments".to_string(),
            confidence: 0.8,
            attributes: HashMap::new(),
        });

        self.add_pattern("identifier".to_string(), SourcePattern {
            name_pattern: "process.argv".to_string(),
            source_type: SourceType::CommandLineInput,
            description: "Node.js command line arguments".to_string(),
            confidence: 0.8,
            attributes: HashMap::new(),
        });
    }
}

impl Default for SourceDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for detecting sources
#[derive(Debug, Clone)]
pub struct SourcePattern {
    pub name_pattern: String,
    pub source_type: SourceType,
    pub description: String,
    pub confidence: f32,
    pub attributes: HashMap<String, String>,
}

impl SourcePattern {
    /// Check if this pattern matches a node
    pub fn matches(&self, name: &str, _node: &DataFlowNode) -> bool {
        // Simple string matching for now
        // In a real implementation, this could use regex or more sophisticated matching
        name.contains(&self.name_pattern)
    }

    /// Create a new source pattern
    pub fn new(
        name_pattern: String,
        source_type: SourceType,
        description: String,
        confidence: f32,
    ) -> Self {
        Self {
            name_pattern,
            source_type,
            description,
            confidence: confidence.clamp(0.0, 1.0),
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
    fn test_source_creation() {
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string())
            .with_confidence(0.8);
        
        assert_eq!(source.id, 0);
        assert_eq!(source.source_type, SourceType::UserInput);
        assert_eq!(source.confidence, 0.8);
        assert!(source.is_high_confidence());
    }

    #[test]
    fn test_source_type_severity() {
        assert_eq!(SourceType::UserInput.severity(), SourceSeverity::High);
        assert_eq!(SourceType::FileInput.severity(), SourceSeverity::Medium);
        assert_eq!(SourceType::EnvironmentInput.severity(), SourceSeverity::Low);
    }

    #[test]
    fn test_source_detector() {
        let detector = SourceDetector::new();
        assert!(!detector.patterns.is_empty());
    }

    #[test]
    fn test_source_pattern_matching() {
        let pattern = SourcePattern::new(
            "getParameter".to_string(),
            SourceType::UserInput,
            "HTTP parameter".to_string(),
            0.9,
        );

        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("request.getParameter".to_string());

        assert!(pattern.matches("request.getParameter", &node));
        assert!(!pattern.matches("other.method", &node));
    }

    #[test]
    fn test_detect_sources_in_graph() {
        let mut graph = DataFlowGraph::new();
        let detector = SourceDetector::new();

        // Add a node that should be detected as a source
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("request.getParameter".to_string());
        let node_id = graph.add_node(node);

        let sources = detector.detect_sources(&graph).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, node_id);
        assert_eq!(sources[0].source_type, SourceType::UserInput);
    }

    #[test]
    fn test_custom_pattern() {
        let mut detector = SourceDetector::new();
        
        let custom_pattern = SourcePattern::new(
            "customInput".to_string(),
            SourceType::ExternalApiInput,
            "Custom API input".to_string(),
            0.7,
        );
        
        detector.add_pattern("call_expression".to_string(), custom_pattern);
        
        let mut graph = DataFlowGraph::new();
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("customInput".to_string());
        graph.add_node(node);

        let sources = detector.detect_sources(&graph).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_type, SourceType::ExternalApiInput);
    }
}
