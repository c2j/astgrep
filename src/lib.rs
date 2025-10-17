//! Enhanced Code Review Service
//! 
//! A comprehensive static analysis tool for security-focused code review
//! with advanced pattern matching, taint analysis, and language-specific optimizations.

// Re-export core functionality
pub use cr_core as core;
pub use cr_ast as ast;
pub use cr_matcher as matcher;
pub use cr_parser as parser;
pub use cr_dataflow as dataflow;
pub use cr_dataflow::{
    EnhancedTaintTracker, DataFlowGraph, DataFlowNode, EdgeType,
    Source, SourceType, Sink, SinkType, Sanitizer
};
pub use cr_rules as rules;
pub use cr_cli as cli;

// Re-export commonly used types for convenience
pub use cr_core::{
    Language, Severity, Confidence, Location,
    SemgrepPattern, PatternType, AstNode, Result,
    Condition, MetavariableRegex, MetavariableComparison, ComparisonOperator
};

pub use cr_ast::{UniversalNode, NodeType};

pub use cr_matcher::{
    AdvancedSemgrepMatcher, PreciseExpressionMatcher, 
    MatchingConfig, MatchResult
};

pub use cr_parser::{
    php_optimizer::PhpOptimizer,
    javascript_optimizer::JavaScriptOptimizer,
    LanguageParserRegistry,
    tree_sitter_parser::TreeSitterParser
};

pub use cr_dataflow::{
    TaintAnalysisConfig, SanitizerType
};

pub use cr_rules::{
    Rule, Pattern, RuleEngine, RuleValidator
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Enhanced Code Review Service main library
/// 
/// This library provides a comprehensive set of tools for static code analysis
/// with a focus on security vulnerabilities. It includes:
/// 
/// - Advanced pattern matching with Semgrep-compatible syntax
/// - Precise expression matching with configurable algorithms
/// - Enhanced taint analysis with field and context sensitivity
/// - Language-specific optimizations for PHP, JavaScript, Java, Python, and more
/// - Rule engine with validation and execution capabilities
/// 
/// # Examples
/// 
/// ```rust
/// use cr_semservice::{
///     AdvancedSemgrepMatcher, SemgrepPattern, PatternType,
///     UniversalNode, NodeType
/// };
/// 
/// // Create a pattern matcher
/// let mut matcher = AdvancedSemgrepMatcher::new();
/// 
/// // Define a security pattern
/// let pattern = SemgrepPattern {
///     pattern_type: PatternType::Simple("eval($CODE)".to_string()),
///     metavariable_pattern: None,
///     focus: None,
///     conditions: Vec::new(),
/// };
/// 
/// // Create an AST node to match against
/// let ast = UniversalNode::new(NodeType::CallExpression)
///     .with_text("eval(userInput)".to_string());
/// 
/// // Find matches
/// let matches = matcher.find_matches(&pattern, &ast).unwrap();
/// println!("Found {} potential security issues", matches.len());
/// ```
/// 
/// # Features
/// 
/// ## Advanced Pattern Matching
/// 
/// The library supports complex pattern matching including:
/// - Simple patterns with metavariables
/// - Pattern-either for alternative matches
/// - Pattern-inside for contextual matching
/// - Pattern-not for exclusion patterns
/// - Conditional patterns with metavariable constraints
/// 
/// ## Precise Expression Matching
/// 
/// Configurable matching algorithms:
/// - Structural matching for exact syntax matches
/// - Semantic matching for equivalent expressions
/// - Type-aware matching with type information
/// - Fuzzy matching with similarity thresholds
/// 
/// ## Enhanced Taint Analysis
/// 
/// Advanced data flow analysis:
/// - Field-sensitive analysis for object properties
/// - Context-sensitive analysis for function calls
/// - Path-sensitive analysis for control flow
/// - Configurable source, sink, and sanitizer definitions
/// 
/// ## Language-Specific Optimizations
/// 
/// Specialized handling for different languages:
/// - PHP: Superglobal detection, framework analysis
/// - JavaScript: DOM API detection, async pattern analysis
/// - Java: Annotation processing, reflection analysis
/// - Python: Dynamic feature detection
/// 
/// # Architecture
/// 
/// The library is organized into several crates:
/// 
/// - `cr-core`: Core types and traits
/// - `cr-ast`: Abstract syntax tree representation
/// - `cr-matcher`: Pattern matching engines
/// - `cr-parser`: Language parsers and optimizers
/// - `cr-dataflow`: Data flow and taint analysis
/// - `cr-rules`: Rule definition and execution
/// - `cr-cli`: Command-line interface
/// - `cr-web`: Web service interface
pub mod prelude {
    //! Prelude module with commonly used imports
    
    pub use crate::{
        Language, Severity, Confidence, Location,
        SemgrepPattern, PatternType, AstNode, Result,
        UniversalNode, NodeType,
        AdvancedSemgrepMatcher, PreciseExpressionMatcher,
        MatchingConfig, MatchResult,
        EnhancedTaintTracker, TaintAnalysisConfig,
        Source, Sink, Sanitizer, DataFlowGraph,
        SourceType, SinkType, SanitizerType,
        Rule, Pattern, RuleEngine, RuleValidator
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_basic_pattern_matching() {
        let mut matcher = AdvancedSemgrepMatcher::new();
        
        let pattern = SemgrepPattern {
            pattern_type: PatternType::Simple("$FUNC($ARG)".to_string()),
            metavariable_pattern: None,
            focus: None,
            conditions: Vec::new(),
        };
        
        let ast = UniversalNode::new(NodeType::CallExpression)
            .with_text("eval(code)".to_string());
        
        let result = matcher.find_matches(&pattern, &ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enhanced_taint_tracker() {
        let mut tracker = EnhancedTaintTracker::new();
        let mut graph = DataFlowGraph::new();

        // Test basic taint tracking functionality
        let source_node = graph.add_node(DataFlowNode::new("user_input".to_string()));
        let sink_node = graph.add_node(DataFlowNode::new("output".to_string()));
        graph.add_edge(source_node, sink_node, EdgeType::DataFlow);

        let sources = vec![Source::new(source_node, SourceType::UserInput, "Test source".to_string())];
        let sinks = vec![Sink::new(sink_node, SinkType::HtmlOutput, "XSS".to_string(), "Test sink".to_string())];
        let sanitizers = Vec::new();

        // Perform taint analysis
        let flows = tracker.analyze_taint(&graph, &sources, &sinks, &sanitizers);
        assert!(flows.is_ok());

        let flows = flows.unwrap();
        // We should find at least one flow from source to sink
        assert!(!flows.is_empty());
    }

    #[test]
    fn test_language_optimizers() {
        let _php_optimizer = PhpOptimizer::new();
        let _js_optimizer = JavaScriptOptimizer::new();
        // Test that we can create optimizers
        assert!(true);
    }
}
