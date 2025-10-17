//! PHP-specific optimizations for better pattern matching
//!
//! This module provides PHP-specific optimizations to improve
//! the accuracy of semgrep-style pattern matching for PHP code.

use crate::adapters::{AstAdapter, AdapterContext, AdapterMetadata};
use cr_ast::{UniversalNode, NodeType};
use cr_core::{Result, AnalysisError, Language, AstNode};
use std::collections::HashMap;
use regex::Regex;

/// PHP-specific AST optimizer
pub struct PhpOptimizer {
    /// PHP-specific patterns for common constructs
    php_patterns: HashMap<String, PhpPattern>,
    /// Variable tracking for PHP's dynamic nature
    variable_tracker: VariableTracker,
    /// Function call resolver
    function_resolver: PhpFunctionResolver,
}

/// PHP-specific pattern information
#[derive(Debug, Clone)]
struct PhpPattern {
    pattern_type: PhpPatternType,
    regex: Regex,
    node_type: NodeType,
    metadata: HashMap<String, String>,
}

/// Types of PHP-specific patterns
#[derive(Debug, Clone)]
enum PhpPatternType {
    VariableAssignment,
    FunctionCall,
    ClassInstantiation,
    ArrayAccess,
    PropertyAccess,
    MethodCall,
    Include,
    Echo,
    SqlQuery,
    FileOperation,
    UserInput,
}

/// Variable tracker for PHP's dynamic variables
#[derive(Debug, Clone)]
struct VariableTracker {
    variables: HashMap<String, VariableInfo>,
    superglobals: HashMap<String, VariableInfo>,
}

/// Information about a PHP variable
#[derive(Debug, Clone)]
struct VariableInfo {
    name: String,
    var_type: PhpVariableType,
    is_tainted: bool,
    source_location: Option<(usize, usize)>,
}

/// Types of PHP variables
#[derive(Debug, Clone)]
enum PhpVariableType {
    Scalar,
    Array,
    Object,
    Resource,
    Superglobal,
    Unknown,
}

/// PHP function call resolver
#[derive(Debug, Clone)]
struct PhpFunctionResolver {
    dangerous_functions: HashMap<String, DangerousFunction>,
    sanitization_functions: HashMap<String, SanitizationFunction>,
}

/// Information about dangerous PHP functions
#[derive(Debug, Clone)]
struct DangerousFunction {
    name: String,
    vulnerability_types: Vec<String>,
    dangerous_parameters: Vec<usize>,
}

/// Information about sanitization functions
#[derive(Debug, Clone)]
struct SanitizationFunction {
    name: String,
    protected_types: Vec<String>,
    effectiveness: f32,
}

impl PhpOptimizer {
    /// Create a new PHP optimizer
    pub fn new() -> Self {
        let mut optimizer = Self {
            php_patterns: HashMap::new(),
            variable_tracker: VariableTracker::new(),
            function_resolver: PhpFunctionResolver::new(),
        };
        
        optimizer.initialize_patterns();
        optimizer
    }

    /// Initialize PHP-specific patterns
    fn initialize_patterns(&mut self) {
        // Variable assignment patterns
        self.add_pattern(
            "variable_assignment",
            PhpPatternType::VariableAssignment,
            r"\$[a-zA-Z_][a-zA-Z0-9_]*\s*=",
            NodeType::AssignmentExpression,
        );

        // Function call patterns
        self.add_pattern(
            "function_call",
            PhpPatternType::FunctionCall,
            r"[a-zA-Z_][a-zA-Z0-9_]*\s*\(",
            NodeType::CallExpression,
        );

        // Class instantiation
        self.add_pattern(
            "class_instantiation",
            PhpPatternType::ClassInstantiation,
            r"new\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\(",
            NodeType::CallExpression,
        );

        // Array access
        self.add_pattern(
            "array_access",
            PhpPatternType::ArrayAccess,
            r"\$[a-zA-Z_][a-zA-Z0-9_]*\s*\[",
            NodeType::MemberExpression,
        );

        // Property access
        self.add_pattern(
            "property_access",
            PhpPatternType::PropertyAccess,
            r"\$[a-zA-Z_][a-zA-Z0-9_]*\s*->\s*[a-zA-Z_][a-zA-Z0-9_]*",
            NodeType::MemberExpression,
        );

        // Method calls
        self.add_pattern(
            "method_call",
            PhpPatternType::MethodCall,
            r"\$[a-zA-Z_][a-zA-Z0-9_]*\s*->\s*[a-zA-Z_][a-zA-Z0-9_]*\s*\(",
            NodeType::CallExpression,
        );

        // Include/require statements
        self.add_pattern(
            "include",
            PhpPatternType::Include,
            r"(include|require|include_once|require_once)\s*\(",
            NodeType::CallExpression,
        );

        // Echo statements
        self.add_pattern(
            "echo",
            PhpPatternType::Echo,
            r"echo\s+",
            NodeType::CallExpression,
        );

        // SQL query patterns
        self.add_pattern(
            "sql_query",
            PhpPatternType::SqlQuery,
            r"(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)\s+",
            NodeType::Literal,
        );

        // File operations
        self.add_pattern(
            "file_operation",
            PhpPatternType::FileOperation,
            r"(fopen|file_get_contents|file_put_contents|readfile|include|require)\s*\(",
            NodeType::CallExpression,
        );

        // User input patterns
        self.add_pattern(
            "user_input",
            PhpPatternType::UserInput,
            r"\$_(GET|POST|REQUEST|COOKIE|SESSION|SERVER|FILES)\s*\[",
            NodeType::MemberExpression,
        );
    }

    /// Add a PHP-specific pattern
    fn add_pattern(
        &mut self,
        name: &str,
        pattern_type: PhpPatternType,
        regex_str: &str,
        node_type: NodeType,
    ) {
        if let Ok(regex) = Regex::new(regex_str) {
            let pattern = PhpPattern {
                pattern_type,
                regex,
                node_type,
                metadata: HashMap::new(),
            };
            self.php_patterns.insert(name.to_string(), pattern);
        }
    }

    /// Optimize PHP AST for better pattern matching
    pub fn optimize_php_ast(&mut self, mut ast: UniversalNode, source: &str) -> Result<UniversalNode> {
        // Phase 1: Identify and enhance PHP-specific constructs
        self.enhance_php_constructs(&mut ast, source)?;

        // Phase 2: Track variables and their taint status
        self.track_php_variables(&mut ast, source)?;

        // Phase 3: Resolve function calls and add metadata
        self.resolve_php_functions(&mut ast, source)?;

        // Phase 4: Add PHP-specific metadata
        self.add_php_metadata(&mut ast, source)?;

        Ok(ast)
    }

    /// Enhance PHP-specific constructs in the AST
    fn enhance_php_constructs(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        // Process current node
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            let mut updates = Vec::new();

            for (name, pattern) in &self.php_patterns {
                if pattern.regex.is_match(text) {
                    updates.push((name.clone(), pattern.clone()));
                }
            }

            for (name, pattern) in updates {
                // Update node type if it's more specific
                if ast.node_type() == NodeType::Literal.as_str() && pattern.node_type != NodeType::Literal {
                    *ast.node_type_mut() = pattern.node_type.clone();
                }

                // Add pattern-specific metadata
                ast.add_attribute(format!("php_pattern_{}", name), "true".to_string());

                // Add vulnerability information for dangerous patterns
                self.add_vulnerability_metadata(ast, &pattern.pattern_type)?;
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.enhance_php_constructs(child, source)?;
        }

        Ok(())
    }

    /// Track PHP variables and their properties
    fn track_php_variables(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        let text_copy = ast.text().map(|s| s.to_string());
        if let Some(text) = &text_copy {
            // Track variable assignments
            if let Some(var_info) = self.extract_variable_assignment(text) {
                self.variable_tracker.add_variable(var_info);
                ast.add_attribute("php_variable_assignment".to_string(), "true".to_string());
            }

            // Track variable usage
            if let Some(var_name) = self.extract_variable_usage(text) {
                if let Some(var_info) = self.variable_tracker.get_variable(&var_name) {
                    if var_info.is_tainted {
                        ast.add_attribute("php_tainted_variable".to_string(), "true".to_string());
                    }
                    ast.add_attribute("php_variable_type".to_string(), format!("{:?}", var_info.var_type));
                }
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.track_php_variables(child, source)?;
        }

        Ok(())
    }

    /// Resolve PHP function calls and add security information
    fn resolve_php_functions(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        if ast.node_type() == NodeType::CallExpression.as_str() {
            if let Some(text) = ast.text() {
                if let Some(function_name) = self.extract_function_name(text) {
                    // Check for dangerous functions
                    if let Some(dangerous_func) = self.function_resolver.get_dangerous_function(&function_name) {
                        ast.add_attribute("php_dangerous_function".to_string(), "true".to_string());
                        ast.add_attribute("php_vulnerability_types".to_string(), 
                                        dangerous_func.vulnerability_types.join(","));
                    }

                    // Check for sanitization functions
                    if let Some(sanitizer) = self.function_resolver.get_sanitization_function(&function_name) {
                        ast.add_attribute("php_sanitization_function".to_string(), "true".to_string());
                        ast.add_attribute("php_protected_types".to_string(), 
                                        sanitizer.protected_types.join(","));
                        ast.add_attribute("php_sanitizer_effectiveness".to_string(), 
                                        sanitizer.effectiveness.to_string());
                    }
                }
            }
        }

        // Recursively process children
        for child in ast.children_mut() {
            self.resolve_php_functions(child, source)?;
        }

        Ok(())
    }

    /// Add PHP-specific metadata to AST nodes
    fn add_php_metadata(&mut self, ast: &mut UniversalNode, source: &str) -> Result<()> {
        // Add language-specific metadata
        ast.add_attribute("language".to_string(), "php".to_string());

        // Add PHP version information if detectable
        if let Some(version) = self.detect_php_version(source) {
            ast.add_attribute("php_version".to_string(), version);
        }

        // Add framework information if detectable
        if let Some(framework) = self.detect_php_framework(source) {
            ast.add_attribute("php_framework".to_string(), framework);
        }

        Ok(())
    }

    /// Add vulnerability metadata based on pattern type
    fn add_vulnerability_metadata(&self, ast: &mut UniversalNode, pattern_type: &PhpPatternType) -> Result<()> {
        match pattern_type {
            PhpPatternType::SqlQuery => {
                ast.add_attribute("vulnerability_risk".to_string(), "sql_injection".to_string());
            }
            PhpPatternType::Echo => {
                ast.add_attribute("vulnerability_risk".to_string(), "xss".to_string());
            }
            PhpPatternType::Include => {
                ast.add_attribute("vulnerability_risk".to_string(), "file_inclusion".to_string());
            }
            PhpPatternType::FileOperation => {
                ast.add_attribute("vulnerability_risk".to_string(), "path_traversal".to_string());
            }
            PhpPatternType::UserInput => {
                ast.add_attribute("vulnerability_risk".to_string(), "user_input".to_string());
                ast.add_attribute("taint_source".to_string(), "true".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    /// Extract variable assignment information
    fn extract_variable_assignment(&self, text: &str) -> Option<VariableInfo> {
        let var_regex = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)\s*=").ok()?;
        if let Some(captures) = var_regex.captures(text) {
            let var_name = captures.get(1)?.as_str().to_string();
            Some(VariableInfo {
                name: var_name,
                var_type: PhpVariableType::Unknown,
                is_tainted: self.is_assignment_tainted(text),
                source_location: None,
            })
        } else {
            None
        }
    }

    /// Extract variable usage
    fn extract_variable_usage(&self, text: &str) -> Option<String> {
        let var_regex = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").ok()?;
        var_regex.captures(text)?.get(1).map(|m| m.as_str().to_string())
    }

    /// Extract function name from call expression
    fn extract_function_name(&self, text: &str) -> Option<String> {
        let func_regex = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").ok()?;
        func_regex.captures(text)?.get(1).map(|m| m.as_str().to_string())
    }

    /// Check if an assignment involves tainted data
    fn is_assignment_tainted(&self, text: &str) -> bool {
        // Check for common taint sources
        text.contains("$_GET") || text.contains("$_POST") || 
        text.contains("$_REQUEST") || text.contains("$_COOKIE") ||
        text.contains("file_get_contents") || text.contains("fread")
    }

    /// Detect PHP version from source code
    fn detect_php_version(&self, source: &str) -> Option<String> {
        // Look for version-specific syntax
        if source.contains("<?php") {
            if source.contains("??") { // Null coalescing operator (PHP 7.0+)
                Some("7.0+".to_string())
            } else if source.contains("::class") { // Class constant (PHP 5.5+)
                Some("5.5+".to_string())
            } else {
                Some("5.0+".to_string())
            }
        } else {
            None
        }
    }

    /// Detect PHP framework from source code
    fn detect_php_framework(&self, source: &str) -> Option<String> {
        if source.contains("Laravel") || source.contains("Illuminate\\") {
            Some("Laravel".to_string())
        } else if source.contains("Symfony") || source.contains("use Symfony\\") {
            Some("Symfony".to_string())
        } else if source.contains("CodeIgniter") || source.contains("CI_") {
            Some("CodeIgniter".to_string())
        } else if source.contains("Zend") || source.contains("Zend_") {
            Some("Zend".to_string())
        } else {
            None
        }
    }
}

impl VariableTracker {
    fn new() -> Self {
        let mut tracker = Self {
            variables: HashMap::new(),
            superglobals: HashMap::new(),
        };
        tracker.initialize_superglobals();
        tracker
    }

    fn initialize_superglobals(&mut self) {
        let superglobals = ["_GET", "_POST", "_REQUEST", "_COOKIE", "_SESSION", "_SERVER", "_FILES", "_ENV"];
        for &name in &superglobals {
            self.superglobals.insert(name.to_string(), VariableInfo {
                name: name.to_string(),
                var_type: PhpVariableType::Superglobal,
                is_tainted: true, // Superglobals are considered tainted by default
                source_location: None,
            });
        }
    }

    fn add_variable(&mut self, var_info: VariableInfo) {
        self.variables.insert(var_info.name.clone(), var_info);
    }

    fn get_variable(&self, name: &str) -> Option<&VariableInfo> {
        self.variables.get(name).or_else(|| self.superglobals.get(name))
    }
}

impl PhpFunctionResolver {
    fn new() -> Self {
        let mut resolver = Self {
            dangerous_functions: HashMap::new(),
            sanitization_functions: HashMap::new(),
        };
        resolver.initialize_function_databases();
        resolver
    }

    fn initialize_function_databases(&mut self) {
        // Initialize dangerous functions
        self.add_dangerous_function("eval", vec!["CODE_INJECTION".to_string()], vec![0]);
        self.add_dangerous_function("exec", vec!["COMMAND_INJECTION".to_string()], vec![0]);
        self.add_dangerous_function("system", vec!["COMMAND_INJECTION".to_string()], vec![0]);
        self.add_dangerous_function("shell_exec", vec!["COMMAND_INJECTION".to_string()], vec![0]);
        self.add_dangerous_function("mysql_query", vec!["SQL_INJECTION".to_string()], vec![0]);
        self.add_dangerous_function("mysqli_query", vec!["SQL_INJECTION".to_string()], vec![1]);

        // Initialize sanitization functions
        self.add_sanitization_function("htmlspecialchars", vec!["XSS".to_string()], 0.9);
        self.add_sanitization_function("htmlentities", vec!["XSS".to_string()], 0.95);
        self.add_sanitization_function("mysqli_real_escape_string", vec!["SQL_INJECTION".to_string()], 0.8);
        self.add_sanitization_function("addslashes", vec!["SQL_INJECTION".to_string()], 0.6);
        self.add_sanitization_function("filter_var", vec!["XSS".to_string(), "INJECTION".to_string()], 0.85);
    }

    fn add_dangerous_function(&mut self, name: &str, vuln_types: Vec<String>, dangerous_params: Vec<usize>) {
        self.dangerous_functions.insert(name.to_string(), DangerousFunction {
            name: name.to_string(),
            vulnerability_types: vuln_types,
            dangerous_parameters: dangerous_params,
        });
    }

    fn add_sanitization_function(&mut self, name: &str, protected_types: Vec<String>, effectiveness: f32) {
        self.sanitization_functions.insert(name.to_string(), SanitizationFunction {
            name: name.to_string(),
            protected_types,
            effectiveness,
        });
    }

    fn get_dangerous_function(&self, name: &str) -> Option<&DangerousFunction> {
        self.dangerous_functions.get(name)
    }

    fn get_sanitization_function(&self, name: &str) -> Option<&SanitizationFunction> {
        self.sanitization_functions.get(name)
    }
}
