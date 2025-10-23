//! JavaScript-specific optimizations for better pattern matching
//!
//! This module provides JavaScript-specific optimizations to improve
//! the accuracy of semgrep-style pattern matching for JavaScript/TypeScript code.

use astgrep_ast::{UniversalNode, NodeType};
use astgrep_core::{Result, AnalysisError, AstNode};
use std::collections::{HashMap, HashSet};
use regex::Regex;

/// JavaScript-specific AST optimizer
pub struct JavaScriptOptimizer {
    /// JavaScript-specific patterns
    js_patterns: HashMap<String, JsPattern>,
    /// DOM API tracker
    dom_tracker: DomApiTracker,
    /// Async/await tracker
    async_tracker: AsyncTracker,
    /// Module system tracker
    module_tracker: ModuleTracker,
    /// Framework detector
    framework_detector: FrameworkDetector,
}

/// JavaScript-specific pattern information
#[derive(Debug, Clone)]
struct JsPattern {
    pattern_type: JsPatternType,
    regex: Regex,
    node_type: NodeType,
    security_impact: SecurityImpact,
}

/// Types of JavaScript-specific patterns
#[derive(Debug, Clone)]
enum JsPatternType {
    DomManipulation,
    EventHandler,
    AjaxCall,
    LocalStorage,
    SessionStorage,
    Cookie,
    Eval,
    InnerHtml,
    DocumentWrite,
    WindowOpen,
    PostMessage,
    WebSocket,
    FetchApi,
    XmlHttpRequest,
    ImportStatement,
    RequireCall,
    AsyncFunction,
    PromiseChain,
    ArrowFunction,
    TemplateString,
    Destructuring,
    SpreadOperator,
}

/// Security impact levels
#[derive(Debug, Clone)]
enum SecurityImpact {
    High,
    Medium,
    Low,
    Info,
}

/// DOM API usage tracker
#[derive(Debug, Clone)]
struct DomApiTracker {
    dangerous_apis: HashSet<String>,
    sink_apis: HashSet<String>,
    source_apis: HashSet<String>,
}

/// Async/await pattern tracker
#[derive(Debug, Clone)]
struct AsyncTracker {
    async_functions: HashSet<String>,
    promise_chains: Vec<PromiseChain>,
}

/// Promise chain information
#[derive(Debug, Clone)]
struct PromiseChain {
    start_location: (usize, usize),
    chain_length: usize,
    has_error_handling: bool,
}

/// Module system tracker
#[derive(Debug, Clone)]
struct ModuleTracker {
    imports: HashMap<String, ImportInfo>,
    exports: HashMap<String, ExportInfo>,
    requires: HashMap<String, RequireInfo>,
}

/// Import information
#[derive(Debug, Clone)]
struct ImportInfo {
    module_name: String,
    imported_names: Vec<String>,
    is_default: bool,
    is_namespace: bool,
}

/// Export information
#[derive(Debug, Clone)]
struct ExportInfo {
    exported_name: String,
    is_default: bool,
    source_module: Option<String>,
}

/// Require information
#[derive(Debug, Clone)]
struct RequireInfo {
    module_name: String,
    assigned_to: Option<String>,
}

/// Framework detection
#[derive(Debug, Clone)]
struct FrameworkDetector {
    detected_frameworks: HashSet<String>,
    framework_patterns: HashMap<String, Regex>,
}

impl JavaScriptOptimizer {
    /// Create a new JavaScript optimizer
    pub fn new() -> Self {
        let mut optimizer = Self {
            js_patterns: HashMap::new(),
            dom_tracker: DomApiTracker::new(),
            async_tracker: AsyncTracker::new(),
            module_tracker: ModuleTracker::new(),
            framework_detector: FrameworkDetector::new(),
        };
        
        optimizer.initialize_patterns();
        optimizer
    }

    /// Initialize JavaScript-specific patterns
    fn initialize_patterns(&mut self) {
        // DOM manipulation patterns
        self.add_pattern(
            "innerHTML",
            JsPatternType::InnerHtml,
            r"\.innerHTML\s*=",
            NodeType::AssignmentExpression,
            SecurityImpact::High,
        );

        self.add_pattern(
            "document_write",
            JsPatternType::DocumentWrite,
            r"document\.write\s*\(",
            NodeType::CallExpression,
            SecurityImpact::High,
        );

        // Event handlers
        self.add_pattern(
            "event_handler",
            JsPatternType::EventHandler,
            r"on[a-zA-Z]+\s*=",
            NodeType::AssignmentExpression,
            SecurityImpact::Medium,
        );

        // AJAX and network calls
        self.add_pattern(
            "fetch_api",
            JsPatternType::FetchApi,
            r"fetch\s*\(",
            NodeType::CallExpression,
            SecurityImpact::Medium,
        );

        self.add_pattern(
            "xhr",
            JsPatternType::XmlHttpRequest,
            r"XMLHttpRequest\s*\(",
            NodeType::CallExpression,
            SecurityImpact::Medium,
        );

        // Storage APIs
        self.add_pattern(
            "local_storage",
            JsPatternType::LocalStorage,
            r"localStorage\.(setItem|getItem)",
            NodeType::CallExpression,
            SecurityImpact::Low,
        );

        self.add_pattern(
            "session_storage",
            JsPatternType::SessionStorage,
            r"sessionStorage\.(setItem|getItem)",
            NodeType::CallExpression,
            SecurityImpact::Low,
        );

        // Dangerous functions
        self.add_pattern(
            "eval",
            JsPatternType::Eval,
            r"eval\s*\(",
            NodeType::CallExpression,
            SecurityImpact::High,
        );

        self.add_pattern(
            "window_open",
            JsPatternType::WindowOpen,
            r"window\.open\s*\(",
            NodeType::CallExpression,
            SecurityImpact::Medium,
        );

        // Modern JavaScript features
        self.add_pattern(
            "arrow_function",
            JsPatternType::ArrowFunction,
            r"=>\s*",
            NodeType::ArrowFunction,
            SecurityImpact::Info,
        );

        self.add_pattern(
            "template_string",
            JsPatternType::TemplateString,
            r"`[^`]*\$\{[^}]*\}[^`]*`",
            NodeType::TemplateString,
            SecurityImpact::Medium,
        );

        self.add_pattern(
            "async_function",
            JsPatternType::AsyncFunction,
            r"async\s+(function|\([^)]*\)\s*=>|\w+\s*\()",
            NodeType::FunctionDeclaration,
            SecurityImpact::Info,
        );

        // Module patterns
        self.add_pattern(
            "import_statement",
            JsPatternType::ImportStatement,
            r"import\s+",
            NodeType::ImportDeclaration,
            SecurityImpact::Info,
        );

        self.add_pattern(
            "require_call",
            JsPatternType::RequireCall,
            r"require\s*\(",
            NodeType::CallExpression,
            SecurityImpact::Info,
        );
    }

    /// Add a JavaScript-specific pattern
    fn add_pattern(
        &mut self,
        name: &str,
        pattern_type: JsPatternType,
        regex_str: &str,
        node_type: NodeType,
        security_impact: SecurityImpact,
    ) {
        if let Ok(regex) = Regex::new(regex_str) {
            let pattern = JsPattern {
                pattern_type,
                regex,
                node_type,
                security_impact,
            };
            self.js_patterns.insert(name.to_string(), pattern);
        }
    }

    /// Optimize JavaScript AST for better pattern matching
    pub fn optimize_js_ast(&mut self, mut ast: UniversalNode, source: &str) -> Result<UniversalNode> {
        // Phase 1: Detect framework
        self.framework_detector.detect_frameworks(source);

        // Phase 2: Enhance JavaScript-specific constructs
        self.enhance_js_constructs(&mut ast, source)?;

        // Phase 3: Track DOM API usage
        self.track_dom_apis(&mut ast, source)?;

        // Phase 4: Track async patterns
        self.track_async_patterns(&mut ast, source)?;

        // Phase 5: Track module usage
        self.track_module_usage(&mut ast, source)?;

        // Phase 6: Add framework-specific metadata
        self.add_framework_metadata(&mut ast)?;

        Ok(ast)
    }

    /// Enhance JavaScript-specific constructs
    fn enhance_js_constructs(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            let mut updates = Vec::new();

            for (name, pattern) in &self.js_patterns {
                if pattern.regex.is_match(text) {
                    updates.push((name.clone(), pattern.clone()));
                }
            }

            for (name, pattern) in updates {
                // Update node type if more specific
                if ast.node_type() == NodeType::Literal.as_str() && pattern.node_type != NodeType::Literal {
                    *ast.node_type_mut() = pattern.node_type.clone();
                }

                // Add pattern metadata
                ast.add_attribute(format!("js_pattern_{}", name), "true".to_string());
                ast.add_attribute("js_security_impact".to_string(), format!("{:?}", pattern.security_impact));

                // Add vulnerability information
                self.add_js_vulnerability_metadata(ast, &pattern.pattern_type)?;
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.enhance_js_constructs(child, source)?;
        }

        Ok(())
    }

    /// Track DOM API usage
    fn track_dom_apis(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            // Check for dangerous DOM APIs
            if self.dom_tracker.is_dangerous_api(text) {
                ast.add_attribute("js_dangerous_dom_api".to_string(), "true".to_string());
            }

            // Check for DOM sinks
            if self.dom_tracker.is_sink_api(text) {
                ast.add_attribute("js_dom_sink".to_string(), "true".to_string());
            }

            // Check for DOM sources
            if self.dom_tracker.is_source_api(text) {
                ast.add_attribute("js_dom_source".to_string(), "true".to_string());
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.track_dom_apis(child, source)?;
        }

        Ok(())
    }

    /// Track async patterns
    fn track_async_patterns(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            // Track async functions
            if text.contains("async") && (text.contains("function") || text.contains("=>")) {
                ast.add_attribute("js_async_function".to_string(), "true".to_string());
                self.async_tracker.add_async_function(text);
            }

            // Track promise chains
            if text.contains(".then(") || text.contains(".catch(") || text.contains(".finally(") {
                ast.add_attribute("js_promise_chain".to_string(), "true".to_string());
            }

            // Track await expressions
            if text.contains("await ") {
                ast.add_attribute("js_await_expression".to_string(), "true".to_string());
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.track_async_patterns(child, source)?;
        }

        Ok(())
    }

    /// Track module usage
    fn track_module_usage(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            // Track ES6 imports
            if text.starts_with("import ") {
                if let Some(import_info) = self.parse_import_statement(text) {
                    self.module_tracker.add_import(import_info);
                    ast.add_attribute("js_es6_import".to_string(), "true".to_string());
                }
            }

            // Track CommonJS requires
            if text.contains("require(") {
                if let Some(require_info) = self.parse_require_statement(text) {
                    self.module_tracker.add_require(require_info);
                    ast.add_attribute("js_commonjs_require".to_string(), "true".to_string());
                }
            }

            // Track exports
            if text.starts_with("export ") {
                if let Some(export_info) = self.parse_export_statement(text) {
                    self.module_tracker.add_export(export_info);
                    ast.add_attribute("js_export".to_string(), "true".to_string());
                }
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.track_module_usage(child, source)?;
        }

        Ok(())
    }

    /// Add framework-specific metadata
    fn add_framework_metadata(&self, ast: &mut UniversalNode) -> Result<()> {
        for framework in &self.framework_detector.detected_frameworks {
            ast.add_attribute("js_framework".to_string(), framework.clone());
        }
        Ok(())
    }

    /// Add vulnerability metadata for JavaScript patterns
    fn add_js_vulnerability_metadata(&self, ast: &mut UniversalNode, pattern_type: &JsPatternType) -> Result<()> {
        match pattern_type {
            JsPatternType::InnerHtml | JsPatternType::DocumentWrite => {
                ast.add_attribute("vulnerability_risk".to_string(), "xss".to_string());
            }
            JsPatternType::Eval => {
                ast.add_attribute("vulnerability_risk".to_string(), "code_injection".to_string());
            }
            JsPatternType::PostMessage => {
                ast.add_attribute("vulnerability_risk".to_string(), "postmessage_xss".to_string());
            }
            JsPatternType::LocalStorage | JsPatternType::SessionStorage => {
                ast.add_attribute("vulnerability_risk".to_string(), "sensitive_data_exposure".to_string());
            }
            JsPatternType::FetchApi | JsPatternType::XmlHttpRequest => {
                ast.add_attribute("vulnerability_risk".to_string(), "ssrf".to_string());
            }
            JsPatternType::TemplateString => {
                ast.add_attribute("vulnerability_risk".to_string(), "template_injection".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    /// Parse import statement
    fn parse_import_statement(&self, text: &str) -> Option<ImportInfo> {
        // Simplified parsing - in practice would use proper AST parsing
        if let Some(from_pos) = text.find(" from ") {
            let import_part = &text[6..from_pos].trim(); // Skip "import "
            let module_part = &text[from_pos + 6..].trim(); // Skip " from "
            
            Some(ImportInfo {
                module_name: module_part.trim_matches(|c| c == '"' || c == '\'' || c == ';').to_string(),
                imported_names: vec![import_part.to_string()], // Simplified
                is_default: !import_part.contains('{'),
                is_namespace: import_part.contains('*'),
            })
        } else {
            None
        }
    }

    /// Parse require statement
    fn parse_require_statement(&self, text: &str) -> Option<RequireInfo> {
        let require_regex = Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]"#).ok()?;
        if let Some(captures) = require_regex.captures(text) {
            Some(RequireInfo {
                module_name: captures.get(1)?.as_str().to_string(),
                assigned_to: None, // Would extract variable name in full implementation
            })
        } else {
            None
        }
    }

    /// Parse export statement
    fn parse_export_statement(&self, text: &str) -> Option<ExportInfo> {
        // Simplified parsing
        Some(ExportInfo {
            exported_name: "unknown".to_string(), // Would extract actual name
            is_default: text.contains("export default"),
            source_module: None,
        })
    }
}

impl DomApiTracker {
    fn new() -> Self {
        let mut tracker = Self {
            dangerous_apis: HashSet::new(),
            sink_apis: HashSet::new(),
            source_apis: HashSet::new(),
        };
        tracker.initialize_api_sets();
        tracker
    }

    fn initialize_api_sets(&mut self) {
        // Dangerous APIs
        self.dangerous_apis.insert("innerHTML".to_string());
        self.dangerous_apis.insert("outerHTML".to_string());
        self.dangerous_apis.insert("document.write".to_string());
        self.dangerous_apis.insert("eval".to_string());

        // Sink APIs
        self.sink_apis.insert("innerHTML".to_string());
        self.sink_apis.insert("outerHTML".to_string());
        self.sink_apis.insert("insertAdjacentHTML".to_string());

        // Source APIs
        self.source_apis.insert("location.search".to_string());
        self.source_apis.insert("location.hash".to_string());
        self.source_apis.insert("document.referrer".to_string());
    }

    fn is_dangerous_api(&self, text: &str) -> bool {
        self.dangerous_apis.iter().any(|api| text.contains(api))
    }

    fn is_sink_api(&self, text: &str) -> bool {
        self.sink_apis.iter().any(|api| text.contains(api))
    }

    fn is_source_api(&self, text: &str) -> bool {
        self.source_apis.iter().any(|api| text.contains(api))
    }
}

impl AsyncTracker {
    fn new() -> Self {
        Self {
            async_functions: HashSet::new(),
            promise_chains: Vec::new(),
        }
    }

    fn add_async_function(&mut self, function_text: &str) {
        // Extract function name if possible
        self.async_functions.insert(function_text.to_string());
    }
}

impl ModuleTracker {
    fn new() -> Self {
        Self {
            imports: HashMap::new(),
            exports: HashMap::new(),
            requires: HashMap::new(),
        }
    }

    fn add_import(&mut self, import_info: ImportInfo) {
        self.imports.insert(import_info.module_name.clone(), import_info);
    }

    fn add_export(&mut self, export_info: ExportInfo) {
        self.exports.insert(export_info.exported_name.clone(), export_info);
    }

    fn add_require(&mut self, require_info: RequireInfo) {
        self.requires.insert(require_info.module_name.clone(), require_info);
    }
}

impl FrameworkDetector {
    fn new() -> Self {
        let mut detector = Self {
            detected_frameworks: HashSet::new(),
            framework_patterns: HashMap::new(),
        };
        detector.initialize_framework_patterns();
        detector
    }

    fn initialize_framework_patterns(&mut self) {
        self.framework_patterns.insert("React".to_string(), 
            Regex::new(r"(import.*react|React\.|jsx|useState|useEffect)").unwrap());
        self.framework_patterns.insert("Vue".to_string(), 
            Regex::new(r"(Vue\.|new Vue|v-|@click|<template>)").unwrap());
        self.framework_patterns.insert("Angular".to_string(), 
            Regex::new(r"(@Component|@Injectable|angular|ng-)").unwrap());
        self.framework_patterns.insert("jQuery".to_string(), 
            Regex::new(r"(\$\(|\$\.|\$\.|jQuery)").unwrap());
    }

    fn detect_frameworks(&mut self, source: &str) {
        for (framework, pattern) in &self.framework_patterns {
            if pattern.is_match(source) {
                self.detected_frameworks.insert(framework.clone());
            }
        }
    }
}
