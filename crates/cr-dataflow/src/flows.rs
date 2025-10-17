//! Data flow analysis utilities
//! 
//! This module provides utilities for analyzing and reporting data flows.

use crate::taint::{TaintFlow, FlowSeverity};
use crate::sources::{Source, SourceType, SourceSeverity};
use crate::sinks::{Sink, SinkType, SinkSeverity};
use crate::sanitizers::{Sanitizer, SanitizerType};
use crate::graph::{DataFlowGraph, NodeId};
use cr_core::{Location, Result};
use std::collections::{HashMap, HashSet};

/// Flow analyzer for analyzing patterns in data flows
pub struct FlowAnalyzer {
    vulnerability_patterns: HashMap<String, VulnerabilityPattern>,
}

impl FlowAnalyzer {
    /// Create a new flow analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            vulnerability_patterns: HashMap::new(),
        };
        analyzer.load_default_patterns();
        analyzer
    }

    /// Analyze flows and categorize vulnerabilities
    pub fn analyze_flows(&self, flows: &[TaintFlow]) -> FlowAnalysisResult {
        let mut result = FlowAnalysisResult::new();

        for flow in flows {
            let category = self.categorize_flow(flow);
            result.add_flow(flow.clone(), category);
        }

        result.calculate_statistics();
        result
    }

    /// Categorize a flow based on vulnerability patterns
    fn categorize_flow(&self, flow: &TaintFlow) -> VulnerabilityCategory {
        if let Some(_pattern) = self.vulnerability_patterns.get(&flow.vulnerability_type) {
            if flow.is_vulnerable() {
                match flow.severity() {
                    FlowSeverity::Critical => VulnerabilityCategory::Critical,
                    FlowSeverity::High => VulnerabilityCategory::High,
                    FlowSeverity::Medium => VulnerabilityCategory::Medium,
                    FlowSeverity::Low => VulnerabilityCategory::Low,
                    FlowSeverity::Info => VulnerabilityCategory::Info,
                }
            } else {
                VulnerabilityCategory::Sanitized
            }
        } else {
            VulnerabilityCategory::Unknown
        }
    }

    /// Load default vulnerability patterns
    fn load_default_patterns(&mut self) {
        self.vulnerability_patterns.insert(
            "SQL_INJECTION".to_string(),
            VulnerabilityPattern {
                name: "SQL Injection".to_string(),
                description: "Untrusted data used in SQL queries".to_string(),
                cwe_id: Some(89),
                owasp_category: Some("A03:2021 – Injection".to_string()),
                severity: FlowSeverity::Critical,
            },
        );

        self.vulnerability_patterns.insert(
            "XSS".to_string(),
            VulnerabilityPattern {
                name: "Cross-Site Scripting".to_string(),
                description: "Untrusted data rendered in HTML without proper encoding".to_string(),
                cwe_id: Some(79),
                owasp_category: Some("A03:2021 – Injection".to_string()),
                severity: FlowSeverity::High,
            },
        );

        self.vulnerability_patterns.insert(
            "COMMAND_INJECTION".to_string(),
            VulnerabilityPattern {
                name: "Command Injection".to_string(),
                description: "Untrusted data used in system commands".to_string(),
                cwe_id: Some(78),
                owasp_category: Some("A03:2021 – Injection".to_string()),
                severity: FlowSeverity::Critical,
            },
        );

        self.vulnerability_patterns.insert(
            "PATH_TRAVERSAL".to_string(),
            VulnerabilityPattern {
                name: "Path Traversal".to_string(),
                description: "Untrusted data used in file paths".to_string(),
                cwe_id: Some(22),
                owasp_category: Some("A01:2021 – Broken Access Control".to_string()),
                severity: FlowSeverity::High,
            },
        );

        self.vulnerability_patterns.insert(
            "CODE_INJECTION".to_string(),
            VulnerabilityPattern {
                name: "Code Injection".to_string(),
                description: "Untrusted data executed as code".to_string(),
                cwe_id: Some(94),
                owasp_category: Some("A03:2021 – Injection".to_string()),
                severity: FlowSeverity::Critical,
            },
        );
    }

    /// Get vulnerability pattern by type
    pub fn get_pattern(&self, vulnerability_type: &str) -> Option<&VulnerabilityPattern> {
        self.vulnerability_patterns.get(vulnerability_type)
    }

    /// Add custom vulnerability pattern
    pub fn add_pattern(&mut self, vulnerability_type: String, pattern: VulnerabilityPattern) {
        self.vulnerability_patterns.insert(vulnerability_type, pattern);
    }
}

impl Default for FlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Vulnerability pattern information
#[derive(Debug, Clone)]
pub struct VulnerabilityPattern {
    pub name: String,
    pub description: String,
    pub cwe_id: Option<u32>,
    pub owasp_category: Option<String>,
    pub severity: FlowSeverity,
}

/// Categories for vulnerability flows
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VulnerabilityCategory {
    Critical,
    High,
    Medium,
    Low,
    Info,
    Sanitized,
    Unknown,
}

impl VulnerabilityCategory {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            VulnerabilityCategory::Critical => "critical",
            VulnerabilityCategory::High => "high",
            VulnerabilityCategory::Medium => "medium",
            VulnerabilityCategory::Low => "low",
            VulnerabilityCategory::Info => "info",
            VulnerabilityCategory::Sanitized => "sanitized",
            VulnerabilityCategory::Unknown => "unknown",
        }
    }
}

/// Result of flow analysis
#[derive(Debug, Clone)]
pub struct FlowAnalysisResult {
    pub flows_by_category: HashMap<VulnerabilityCategory, Vec<TaintFlow>>,
    pub statistics: FlowStatistics,
}

impl FlowAnalysisResult {
    /// Create a new flow analysis result
    pub fn new() -> Self {
        Self {
            flows_by_category: HashMap::new(),
            statistics: FlowStatistics::default(),
        }
    }

    /// Add a flow to the result
    pub fn add_flow(&mut self, flow: TaintFlow, category: VulnerabilityCategory) {
        self.flows_by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push(flow);
    }

    /// Calculate statistics
    pub fn calculate_statistics(&mut self) {
        let mut stats = FlowStatistics::default();

        for (category, flows) in &self.flows_by_category {
            let count = flows.len();
            match category {
                VulnerabilityCategory::Critical => stats.critical_count = count,
                VulnerabilityCategory::High => stats.high_count = count,
                VulnerabilityCategory::Medium => stats.medium_count = count,
                VulnerabilityCategory::Low => stats.low_count = count,
                VulnerabilityCategory::Info => stats.info_count = count,
                VulnerabilityCategory::Sanitized => stats.sanitized_count = count,
                VulnerabilityCategory::Unknown => stats.unknown_count = count,
            }
        }

        stats.total_flows = stats.critical_count + stats.high_count + stats.medium_count + 
                           stats.low_count + stats.info_count + stats.sanitized_count + stats.unknown_count;
        stats.vulnerable_flows = stats.critical_count + stats.high_count + stats.medium_count + stats.low_count;

        self.statistics = stats;
    }

    /// Get flows by category
    pub fn get_flows(&self, category: &VulnerabilityCategory) -> &[TaintFlow] {
        self.flows_by_category
            .get(category)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all vulnerable flows
    pub fn vulnerable_flows(&self) -> Vec<&TaintFlow> {
        let mut flows = Vec::new();
        flows.extend(self.get_flows(&VulnerabilityCategory::Critical));
        flows.extend(self.get_flows(&VulnerabilityCategory::High));
        flows.extend(self.get_flows(&VulnerabilityCategory::Medium));
        flows.extend(self.get_flows(&VulnerabilityCategory::Low));
        flows
    }

    /// Check if there are any critical vulnerabilities
    pub fn has_critical_vulnerabilities(&self) -> bool {
        self.statistics.critical_count > 0
    }

    /// Get vulnerability types present
    pub fn vulnerability_types(&self) -> HashSet<String> {
        let mut types = HashSet::new();
        for flows in self.flows_by_category.values() {
            for flow in flows {
                types.insert(flow.vulnerability_type.clone());
            }
        }
        types
    }
}

impl Default for FlowAnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about flows
#[derive(Debug, Clone, Default)]
pub struct FlowStatistics {
    pub total_flows: usize,
    pub vulnerable_flows: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub sanitized_count: usize,
    pub unknown_count: usize,
}

impl FlowStatistics {
    /// Get the vulnerability rate
    pub fn vulnerability_rate(&self) -> f32 {
        if self.total_flows == 0 {
            0.0
        } else {
            self.vulnerable_flows as f32 / self.total_flows as f32
        }
    }

    /// Get the sanitization rate
    pub fn sanitization_rate(&self) -> f32 {
        if self.total_flows == 0 {
            0.0
        } else {
            self.sanitized_count as f32 / self.total_flows as f32
        }
    }

    /// Get the most severe category with flows
    pub fn highest_severity(&self) -> Option<VulnerabilityCategory> {
        if self.critical_count > 0 {
            Some(VulnerabilityCategory::Critical)
        } else if self.high_count > 0 {
            Some(VulnerabilityCategory::High)
        } else if self.medium_count > 0 {
            Some(VulnerabilityCategory::Medium)
        } else if self.low_count > 0 {
            Some(VulnerabilityCategory::Low)
        } else if self.info_count > 0 {
            Some(VulnerabilityCategory::Info)
        } else {
            None
        }
    }
}

/// Flow reporter for generating reports
pub struct FlowReporter;

impl FlowReporter {
    /// Generate a summary report
    pub fn generate_summary(result: &FlowAnalysisResult) -> String {
        let stats = &result.statistics;
        let mut report = String::new();

        report.push_str("=== Data Flow Analysis Summary ===\n");
        report.push_str(&format!("Total flows: {}\n", stats.total_flows));
        report.push_str(&format!("Vulnerable flows: {}\n", stats.vulnerable_flows));
        report.push_str(&format!("Vulnerability rate: {:.1}%\n", stats.vulnerability_rate() * 100.0));
        report.push_str(&format!("Sanitization rate: {:.1}%\n", stats.sanitization_rate() * 100.0));
        report.push_str("\n");

        report.push_str("Severity breakdown:\n");
        if stats.critical_count > 0 {
            report.push_str(&format!("  Critical: {}\n", stats.critical_count));
        }
        if stats.high_count > 0 {
            report.push_str(&format!("  High: {}\n", stats.high_count));
        }
        if stats.medium_count > 0 {
            report.push_str(&format!("  Medium: {}\n", stats.medium_count));
        }
        if stats.low_count > 0 {
            report.push_str(&format!("  Low: {}\n", stats.low_count));
        }
        if stats.sanitized_count > 0 {
            report.push_str(&format!("  Sanitized: {}\n", stats.sanitized_count));
        }

        report
    }

    /// Generate a detailed report
    pub fn generate_detailed_report(result: &FlowAnalysisResult, analyzer: &FlowAnalyzer) -> String {
        let mut report = Self::generate_summary(result);
        
        report.push_str("\n=== Detailed Findings ===\n");
        
        for category in [
            VulnerabilityCategory::Critical,
            VulnerabilityCategory::High,
            VulnerabilityCategory::Medium,
            VulnerabilityCategory::Low,
        ] {
            let flows = result.get_flows(&category);
            if !flows.is_empty() {
                report.push_str(&format!("\n{} Severity Issues:\n", category.as_str().to_uppercase()));
                
                for (i, flow) in flows.iter().enumerate() {
                    report.push_str(&format!("  {}. {}\n", i + 1, flow.vulnerability_type));
                    
                    if let Some(pattern) = analyzer.get_pattern(&flow.vulnerability_type) {
                        report.push_str(&format!("     Description: {}\n", pattern.description));
                        if let Some(cwe) = pattern.cwe_id {
                            report.push_str(&format!("     CWE-{}\n", cwe));
                        }
                    }
                    
                    report.push_str(&format!("     Confidence: {:.1}%\n", flow.confidence * 100.0));
                    report.push_str(&format!("     Path length: {} nodes\n", flow.path_length()));
                    
                    if !flow.sanitizers.is_empty() {
                        report.push_str("     Sanitizers: ");
                        for sanitizer in &flow.sanitizers {
                            report.push_str(&format!("{} ", sanitizer.sanitizer_type.as_str()));
                        }
                        report.push_str("\n");
                    }
                    
                    report.push_str("\n");
                }
            }
        }
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Source;
    use crate::sinks::Sink;

    #[test]
    fn test_flow_analyzer_creation() {
        let analyzer = FlowAnalyzer::new();
        assert!(!analyzer.vulnerability_patterns.is_empty());
        assert!(analyzer.get_pattern("SQL_INJECTION").is_some());
    }

    #[test]
    fn test_vulnerability_pattern() {
        let analyzer = FlowAnalyzer::new();
        let pattern = analyzer.get_pattern("XSS").unwrap();
        
        assert_eq!(pattern.name, "Cross-Site Scripting");
        assert_eq!(pattern.cwe_id, Some(79));
        assert_eq!(pattern.severity, FlowSeverity::High);
    }

    #[test]
    fn test_flow_analysis_result() {
        let mut result = FlowAnalysisResult::new();
        
        let source = Source::new(0, SourceType::UserInput, "Test source".to_string());
        let sink = Sink::new(1, SinkType::SqlExecution, "SQL_INJECTION".to_string(), "Test sink".to_string());
        
        let flow = TaintFlow {
            source,
            sink,
            path: vec![0, 1],
            sanitizers: Vec::new(),
            confidence: 0.9,
            vulnerability_type: "SQL_INJECTION".to_string(),
            context: None,
            transformations: Vec::new(),
            flow_type: crate::taint::FlowType::Direct,
        };
        
        result.add_flow(flow, VulnerabilityCategory::Critical);
        result.calculate_statistics();
        
        assert_eq!(result.statistics.critical_count, 1);
        assert_eq!(result.statistics.total_flows, 1);
        assert!(result.has_critical_vulnerabilities());
    }

    #[test]
    fn test_flow_statistics() {
        let mut stats = FlowStatistics::default();
        stats.total_flows = 10;
        stats.vulnerable_flows = 3;
        stats.sanitized_count = 5;
        
        assert_eq!(stats.vulnerability_rate(), 0.3);
        assert_eq!(stats.sanitization_rate(), 0.5);
    }

    #[test]
    fn test_flow_reporter() {
        let mut result = FlowAnalysisResult::new();
        result.statistics.total_flows = 5;
        result.statistics.vulnerable_flows = 2;
        result.statistics.critical_count = 1;
        result.statistics.high_count = 1;
        
        let report = FlowReporter::generate_summary(&result);
        assert!(report.contains("Total flows: 5"));
        assert!(report.contains("Vulnerable flows: 2"));
        assert!(report.contains("Critical: 1"));
    }
}
