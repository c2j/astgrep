//! Sink detection for data flow analysis
//! 
//! This module identifies sinks where tainted data could cause security vulnerabilities.

use crate::graph::{DataFlowGraph, DataFlowNode, NodeId};
use cr_core::{Location, Result};
use std::collections::HashMap;

/// Represents a sink where tainted data could cause vulnerabilities
#[derive(Debug, Clone)]
pub struct Sink {
    pub id: NodeId,
    pub sink_type: SinkType,
    pub location: Option<Location>,
    pub description: String,
    pub vulnerability_type: String,
    pub confidence: f32,
}

impl Sink {
    /// Create a new sink
    pub fn new(id: NodeId, sink_type: SinkType, vulnerability_type: String, description: String) -> Self {
        Self {
            id,
            sink_type,
            location: None,
            description,
            vulnerability_type,
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

    /// Check if this is a high-confidence sink
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Get the severity of this sink type
    pub fn severity(&self) -> SinkSeverity {
        self.sink_type.severity()
    }
}

/// Types of data sinks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkType {
    /// SQL query execution
    SqlExecution,
    /// Command execution
    CommandExecution,
    /// File operations
    FileOperation,
    /// Network operations
    NetworkOperation,
    /// HTML output
    HtmlOutput,
    /// JavaScript evaluation
    JavaScriptEvaluation,
    /// Log output
    LogOutput,
    /// Database operations
    DatabaseOperation,
    /// XML/XPath operations
    XmlOperation,
    /// LDAP operations
    LdapOperation,
}

impl SinkType {
    /// Get the severity of this sink type
    pub fn severity(&self) -> SinkSeverity {
        match self {
            SinkType::SqlExecution | SinkType::CommandExecution | SinkType::JavaScriptEvaluation => SinkSeverity::Critical,
            SinkType::FileOperation | SinkType::XmlOperation | SinkType::LdapOperation => SinkSeverity::High,
            SinkType::NetworkOperation | SinkType::HtmlOutput | SinkType::DatabaseOperation => SinkSeverity::Medium,
            SinkType::LogOutput => SinkSeverity::Low,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SinkType::SqlExecution => "sql_execution",
            SinkType::CommandExecution => "command_execution",
            SinkType::FileOperation => "file_operation",
            SinkType::NetworkOperation => "network_operation",
            SinkType::HtmlOutput => "html_output",
            SinkType::JavaScriptEvaluation => "javascript_evaluation",
            SinkType::LogOutput => "log_output",
            SinkType::DatabaseOperation => "database_operation",
            SinkType::XmlOperation => "xml_operation",
            SinkType::LdapOperation => "ldap_operation",
        }
    }

    /// Get the typical vulnerability type for this sink
    pub fn vulnerability_type(&self) -> &'static str {
        match self {
            SinkType::SqlExecution => "SQL_INJECTION",
            SinkType::CommandExecution => "COMMAND_INJECTION",
            SinkType::FileOperation => "PATH_TRAVERSAL",
            SinkType::NetworkOperation => "SSRF",
            SinkType::HtmlOutput => "XSS",
            SinkType::JavaScriptEvaluation => "CODE_INJECTION",
            SinkType::LogOutput => "LOG_INJECTION",
            SinkType::DatabaseOperation => "SQL_INJECTION",
            SinkType::XmlOperation => "XML_INJECTION",
            SinkType::LdapOperation => "LDAP_INJECTION",
        }
    }
}

/// Severity levels for sinks
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SinkSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Sink detector
pub struct SinkDetector {
    patterns: HashMap<String, Vec<SinkPattern>>,
}

impl SinkDetector {
    /// Create a new sink detector
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: HashMap::new(),
        };
        detector.load_default_patterns();
        detector
    }

    /// Detect sinks in a data flow graph
    pub fn detect_sinks(&self, graph: &DataFlowGraph) -> Result<Vec<Sink>> {
        let mut sinks = Vec::new();

        for (node_id, node) in graph.nodes() {
            if let Some(sink) = self.check_node_for_sink(*node_id, node) {
                sinks.push(sink);
            }
        }

        Ok(sinks)
    }

    /// Check if a node is a sink
    fn check_node_for_sink(&self, node_id: NodeId, node: &DataFlowNode) -> Option<Sink> {
        // Check function calls
        if node.is_function_call() {
            if let Some(function_name) = node.function_name() {
                for pattern in self.get_patterns_for_type(&node.node_type) {
                    if pattern.matches(function_name, node) {
                        return Some(
                            Sink::new(
                                node_id,
                                pattern.sink_type.clone(),
                                pattern.vulnerability_type.clone(),
                                pattern.description.clone(),
                            )
                            .with_confidence(pattern.confidence)
                        );
                    }
                }
            }
        }

        None
    }

    /// Get patterns for a node type
    fn get_patterns_for_type(&self, node_type: &str) -> &[SinkPattern] {
        self.patterns.get(node_type).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Add a custom sink pattern
    pub fn add_pattern(&mut self, node_type: String, pattern: SinkPattern) {
        self.patterns.entry(node_type).or_insert_with(Vec::new).push(pattern);
    }

    /// Load default sink patterns
    fn load_default_patterns(&mut self) {
        // SQL execution patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "executeQuery".to_string(),
            sink_type: SinkType::SqlExecution,
            vulnerability_type: "SQL_INJECTION".to_string(),
            description: "SQL query execution".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "query".to_string(),
            sink_type: SinkType::SqlExecution,
            vulnerability_type: "SQL_INJECTION".to_string(),
            description: "Database query".to_string(),
            confidence: 0.8,
            attributes: HashMap::new(),
        });

        // Command execution patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "exec".to_string(),
            sink_type: SinkType::CommandExecution,
            vulnerability_type: "COMMAND_INJECTION".to_string(),
            description: "Command execution".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "system".to_string(),
            sink_type: SinkType::CommandExecution,
            vulnerability_type: "COMMAND_INJECTION".to_string(),
            description: "System command execution".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        // File operation patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "writeFile".to_string(),
            sink_type: SinkType::FileOperation,
            vulnerability_type: "PATH_TRAVERSAL".to_string(),
            description: "File write operation".to_string(),
            confidence: 0.8,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "open".to_string(),
            sink_type: SinkType::FileOperation,
            vulnerability_type: "PATH_TRAVERSAL".to_string(),
            description: "File open operation".to_string(),
            confidence: 0.7,
            attributes: HashMap::new(),
        });

        // HTML output patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "innerHTML".to_string(),
            sink_type: SinkType::HtmlOutput,
            vulnerability_type: "XSS".to_string(),
            description: "HTML content insertion".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "document.write".to_string(),
            sink_type: SinkType::HtmlOutput,
            vulnerability_type: "XSS".to_string(),
            description: "Document write operation".to_string(),
            confidence: 0.9,
            attributes: HashMap::new(),
        });

        // JavaScript evaluation patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "eval".to_string(),
            sink_type: SinkType::JavaScriptEvaluation,
            vulnerability_type: "CODE_INJECTION".to_string(),
            description: "JavaScript eval function".to_string(),
            confidence: 0.95,
            attributes: HashMap::new(),
        });

        // Log output patterns
        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "log".to_string(),
            sink_type: SinkType::LogOutput,
            vulnerability_type: "LOG_INJECTION".to_string(),
            description: "Log output".to_string(),
            confidence: 0.6,
            attributes: HashMap::new(),
        });

        self.add_pattern("call_expression".to_string(), SinkPattern {
            name_pattern: "console.log".to_string(),
            sink_type: SinkType::LogOutput,
            vulnerability_type: "LOG_INJECTION".to_string(),
            description: "Console log output".to_string(),
            confidence: 0.5,
            attributes: HashMap::new(),
        });
    }
}

impl Default for SinkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for detecting sinks
#[derive(Debug, Clone)]
pub struct SinkPattern {
    pub name_pattern: String,
    pub sink_type: SinkType,
    pub vulnerability_type: String,
    pub description: String,
    pub confidence: f32,
    pub attributes: HashMap<String, String>,
}

impl SinkPattern {
    /// Check if this pattern matches a node
    pub fn matches(&self, name: &str, _node: &DataFlowNode) -> bool {
        // Simple string matching for now
        name.contains(&self.name_pattern)
    }

    /// Create a new sink pattern
    pub fn new(
        name_pattern: String,
        sink_type: SinkType,
        vulnerability_type: String,
        description: String,
        confidence: f32,
    ) -> Self {
        Self {
            name_pattern,
            sink_type,
            vulnerability_type,
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
    fn test_sink_creation() {
        let sink = Sink::new(
            0,
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "Test sink".to_string(),
        )
        .with_confidence(0.9);
        
        assert_eq!(sink.id, 0);
        assert_eq!(sink.sink_type, SinkType::SqlExecution);
        assert_eq!(sink.vulnerability_type, "SQL_INJECTION");
        assert_eq!(sink.confidence, 0.9);
        assert!(sink.is_high_confidence());
    }

    #[test]
    fn test_sink_type_severity() {
        assert_eq!(SinkType::SqlExecution.severity(), SinkSeverity::Critical);
        assert_eq!(SinkType::FileOperation.severity(), SinkSeverity::High);
        assert_eq!(SinkType::HtmlOutput.severity(), SinkSeverity::Medium);
        assert_eq!(SinkType::LogOutput.severity(), SinkSeverity::Low);
    }

    #[test]
    fn test_sink_type_vulnerability() {
        assert_eq!(SinkType::SqlExecution.vulnerability_type(), "SQL_INJECTION");
        assert_eq!(SinkType::CommandExecution.vulnerability_type(), "COMMAND_INJECTION");
        assert_eq!(SinkType::HtmlOutput.vulnerability_type(), "XSS");
    }

    #[test]
    fn test_sink_detector() {
        let detector = SinkDetector::new();
        assert!(!detector.patterns.is_empty());
    }

    #[test]
    fn test_sink_pattern_matching() {
        let pattern = SinkPattern::new(
            "executeQuery".to_string(),
            SinkType::SqlExecution,
            "SQL_INJECTION".to_string(),
            "SQL execution".to_string(),
            0.9,
        );

        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("executeQuery".to_string());

        assert!(pattern.matches("executeQuery", &node));
        assert!(!pattern.matches("other.method", &node));
    }

    #[test]
    fn test_detect_sinks_in_graph() {
        let mut graph = DataFlowGraph::new();
        let detector = SinkDetector::new();

        // Add a node that should be detected as a sink
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("executeQuery".to_string());
        let node_id = graph.add_node(node);

        let sinks = detector.detect_sinks(&graph).unwrap();
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].id, node_id);
        assert_eq!(sinks[0].sink_type, SinkType::SqlExecution);
        assert_eq!(sinks[0].vulnerability_type, "SQL_INJECTION");
    }

    #[test]
    fn test_custom_sink_pattern() {
        let mut detector = SinkDetector::new();
        
        let custom_pattern = SinkPattern::new(
            "customSink".to_string(),
            SinkType::NetworkOperation,
            "CUSTOM_VULN".to_string(),
            "Custom sink".to_string(),
            0.8,
        );
        
        detector.add_pattern("call_expression".to_string(), custom_pattern);
        
        let mut graph = DataFlowGraph::new();
        let node = DataFlowNode::new("call_expression".to_string())
            .with_text("customSink".to_string());
        graph.add_node(node);

        let sinks = detector.detect_sinks(&graph).unwrap();
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].sink_type, SinkType::NetworkOperation);
        assert_eq!(sinks[0].vulnerability_type, "CUSTOM_VULN");
    }
}
