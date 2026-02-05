//! Script dependency resolution for ASTGreP
//!
//! This module provides functionality to analyze and resolve dependencies
//! between test scripts, ensuring proper execution order and identifying
//! circular dependencies.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    fmt,
};
use tracing::{debug, info, warn, error, instrument};
use regex::Regex;
use anyhow::{Result, anyhow};

/// Types of script dependencies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// Script depends on another script (source/include)
    ScriptDependency,
    /// Script depends on external tool or command
    ToolDependency,
    /// Script depends on specific file or data
    FileDependency,
    /// Script depends on environment variable
    EnvironmentDependency,
    /// Script depends on network resource
    NetworkDependency,
    /// Script depends on specific interpreter version
    InterpreterDependency,
}

/// Direction of dependency relationship
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyDirection {
    /// Required dependency (must be available before execution)
    Required,
    /// Optional dependency (can execute without, but with reduced functionality)
    Optional,
    /// Runtime dependency (checked during execution)
    Runtime,
}

/// Represents a dependency between scripts or external resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Type of dependency
    pub dependency_type: DependencyType,
    /// Name or identifier of the dependency
    pub name: String,
    /// Version requirement (if applicable)
    pub version: Option<String>,
    /// Direction of dependency
    pub direction: DependencyDirection,
    /// Source location in script where dependency is referenced
    pub source_location: Option<SourceLocation>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Source location within a script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Content of the line containing the dependency reference
    pub line_content: String,
}

/// Script dependency graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDependencyNode {
    /// Script path
    pub script_path: PathBuf,
    /// Script name
    pub name: String,
    /// Script type (inferred from extension/shebang)
    pub script_type: String,
    /// Dependencies required by this script
    pub dependencies: Vec<Dependency>,
    /// Scripts that depend on this script
    pub dependents: Vec<PathBuf>,
    /// Topological order in dependency graph
    pub execution_order: Option<usize>,
    /// Whether script is part of a cycle
    pub in_cycle: bool,
}

/// Circular dependency detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    /// Scripts involved in the cycle
    pub scripts: Vec<PathBuf>,
    /// Length of the cycle
    pub cycle_length: usize,
    /// Description of the circular dependency
    pub description: String,
}

/// Dependency resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionResult {
    /// All dependency nodes
    pub nodes: HashMap<PathBuf, ScriptDependencyNode>,
    /// Topologically sorted execution order
    pub execution_order: Vec<PathBuf>,
    /// Circular dependencies detected
    pub circular_dependencies: Vec<CircularDependency>,
    /// Missing dependencies
    pub missing_dependencies: Vec<(PathBuf, Dependency)>,
    /// Optional dependencies that could enhance functionality
    pub optional_dependencies: Vec<(PathBuf, Dependency)>,
    /// Resolution summary statistics
    pub summary: ResolutionSummary,
}

/// Resolution summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionSummary {
    /// Total number of scripts analyzed
    pub total_scripts: usize,
    /// Total dependencies found
    pub total_dependencies: usize,
    /// Scripts with no dependencies
    pub independent_scripts: usize,
    /// Scripts with dependencies
    pub dependent_scripts: usize,
    /// Maximum dependency depth
    pub max_dependency_depth: usize,
    /// Number of circular dependencies
    pub circular_dependency_count: usize,
}

/// Configuration for dependency resolution
#[derive(Debug, Clone)]
pub struct DependencyResolutionConfig {
    /// Whether to include optional dependencies in resolution
    pub include_optional: bool,
    /// Whether to analyze external tool dependencies
    pub analyze_tools: bool,
    /// Whether to analyze file dependencies
    pub analyze_files: bool,
    /// Whether to analyze environment variable dependencies
    pub analyze_environment: bool,
    /// Maximum recursion depth for dependency analysis
    pub max_depth: usize,
    /// Custom dependency patterns
    pub custom_patterns: Vec<(String, Regex)>,
}

impl Default for DependencyResolutionConfig {
    fn default() -> Self {
        Self {
            include_optional: true,
            analyze_tools: true,
            analyze_files: true,
            analyze_environment: true,
            max_depth: 10,
            custom_patterns: Vec::new(),
        }
    }
}

/// Script dependency resolver
pub struct ScriptDependencyResolver {
    config: DependencyResolutionConfig,
    // Pre-compiled regex patterns for dependency detection
    script_patterns: HashMap<String, Vec<Regex>>,
    tool_patterns: Vec<Regex>,
    file_patterns: Vec<Regex>,
    env_patterns: Vec<Regex>,
    network_patterns: Vec<Regex>,
}

impl ScriptDependencyResolver {
    /// Create a new script dependency resolver
    pub fn new(config: DependencyResolutionConfig) -> Self {
        let mut resolver = Self {
            config,
            script_patterns: HashMap::new(),
            tool_patterns: Vec::new(),
            file_patterns: Vec::new(),
            env_patterns: Vec::new(),
            network_patterns: Vec::new(),
        };

        resolver.initialize_patterns();
        resolver
    }

    /// Initialize regex patterns for dependency detection
    fn initialize_patterns(&mut self) {
        // Script sourcing patterns (source, include, etc.)
        self.script_patterns.insert("bash".to_string(), vec![
            Regex::new(r#"source\s+["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"\.\s+["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"source\s+([^"'\s]+)"#).unwrap(),
        ]);

        self.script_patterns.insert("python".to_string(), vec![
            Regex::new(r#"import\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)"#).unwrap(),
            Regex::new(r#"from\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s+import"#).unwrap(),
            Regex::new(r#"exec\s*\(\s*open\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap(),
        ]);

        // External tool/command patterns
        self.tool_patterns.extend_from_slice(&[
            Regex::new(r#"^\s*(\w+)\s+"#).unwrap(), // Command at start of line
            Regex::new(r#"`([^`]+)`"#).unwrap(),    // Backtick command substitution
            Regex::new(r#"\$\(([^)]+)\)"#).unwrap(), // $(command) substitution
            Regex::new(r#"subprocess\.run\(\s*["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"exec\s*\(\s*["']([^"']+)["']"#).unwrap(),
        ]);

        // File dependency patterns
        self.file_patterns.extend_from_slice(&[
            Regex::new(r#"["']([^"']+\.(?:txt|json|yaml|yml|xml|csv|data|config|conf|ini))["']"#).unwrap(),
            Regex::new(r#"open\s*\(\s*["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"cat\s+["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"read\s+.*<\s*["']?([^"'\s]+)"#).unwrap(),
        ]);

        // Environment variable patterns
        self.env_patterns.extend_from_slice(&[
            Regex::new(r#"\$\{([A-Za-z_][A-Za-z0-9_]*)\}"#).unwrap(),
            Regex::new(r#"\$([A-Za-z_][A-Za-z0-9_]*)\b"#).unwrap(),
            Regex::new(r#"os\.getenv\s*\(\s*["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"getenv\s*\(\s*["']([^"']+)["']"#).unwrap(),
        ]);

        // Network dependency patterns
        self.network_patterns.extend_from_slice(&[
            Regex::new(r"https?://[^\s]+").unwrap(),
            Regex::new(r#"curl\s+["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"wget\s+["']([^"']+)["']"#).unwrap(),
            Regex::new(r#"requests\.(?:get|post|put|delete|patch)\s*\(\s*["']([^"']+)["']"#).unwrap(),
        ]);
    }

    /// Resolve dependencies for a collection of scripts
    #[instrument(skip(self, scripts))]
    pub async fn resolve_dependencies(
        &self,
        scripts: &[PathBuf],
    ) -> Result<DependencyResolutionResult> {
        info!("Starting dependency resolution for {} scripts", scripts.len());

        // Step 1: Build dependency nodes for all scripts
        let mut nodes = HashMap::new();
        for script_path in scripts {
            let node = self.analyze_script_dependencies(script_path).await?;
            nodes.insert(script_path.clone(), node);
        }

        // Step 2: Build dependency graph by linking nodes
        self.link_dependencies(&mut nodes)?;

        // Step 3: Perform topological sort to determine execution order
        let (execution_order, circular_dependencies) = self.topological_sort(&nodes)?;

        // Step 4: Identify missing and optional dependencies
        let (missing_dependencies, optional_dependencies) =
            self.categorize_dependencies(&nodes);

        // Step 5: Generate summary statistics
        let summary = self.generate_summary(&nodes, &execution_order, &circular_dependencies);

        let result = DependencyResolutionResult {
            nodes,
            execution_order,
            circular_dependencies,
            missing_dependencies,
            optional_dependencies,
            summary,
        };

        info!("Dependency resolution completed: {} scripts, {} dependencies, {} circular dependencies found",
              result.summary.total_scripts,
              result.summary.total_dependencies,
              result.summary.circular_dependency_count);

        Ok(result)
    }

    /// Analyze dependencies for a single script
    async fn analyze_script_dependencies(&self, script_path: &PathBuf) -> Result<ScriptDependencyNode> {
        debug!("Analyzing dependencies for: {}", script_path.display());

        let content = std::fs::read_to_string(script_path)?;
        let script_type = self.detect_script_type(script_path, &content);
        let mut dependencies = Vec::new();

        // Analyze script-specific dependencies
        if let Some(patterns) = self.script_patterns.get(&script_type) {
            for (line_num, line) in content.lines().enumerate() {
                for pattern in patterns {
                    for caps in pattern.captures_iter(line) {
                        if let Some(dep_match) = caps.get(1) {
                            let dependency = Dependency {
                                dependency_type: DependencyType::ScriptDependency,
                                name: dep_match.as_str().to_string(),
                                version: None,
                                direction: DependencyDirection::Required,
                                source_location: Some(SourceLocation {
                                    line: line_num + 1,
                                    column: dep_match.start(),
                                    line_content: line.to_string(),
                                }),
                                metadata: HashMap::new(),
                            };
                            dependencies.push(dependency);
                        }
                    }
                }
            }
        }

        // Analyze tool dependencies if enabled
        if self.config.analyze_tools {
            self.analyze_tool_dependencies(&content, &mut dependencies)?;
        }

        // Analyze file dependencies if enabled
        if self.config.analyze_files {
            self.analyze_file_dependencies(script_path, &content, &mut dependencies)?;
        }

        // Analyze environment dependencies if enabled
        if self.config.analyze_environment {
            self.analyze_environment_dependencies(&content, &mut dependencies)?;
        }

        let node = ScriptDependencyNode {
            script_path: script_path.clone(),
            name: script_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            script_type,
            dependencies,
            dependents: Vec::new(),
            execution_order: None,
            in_cycle: false,
        };

        Ok(node)
    }

    /// Detect script type from path and content
    fn detect_script_type(&self, script_path: &PathBuf, content: &str) -> String {
        // Check shebang first
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                if first_line.contains("bash") || first_line.contains("sh") {
                    return "bash".to_string();
                } else if first_line.contains("python") || first_line.contains("python3") {
                    return "python".to_string();
                } else if first_line.contains("node") || first_line.contains("nodejs") {
                    return "javascript".to_string();
                }
            }
        }

        // Fallback to file extension
        if let Some(extension) = script_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "sh" | "bash" => "bash".to_string(),
                "py" => "python".to_string(),
                "js" | "ts" => "javascript".to_string(),
                "pl" => "perl".to_string(),
                "rb" => "ruby".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Analyze tool dependencies
    fn analyze_tool_dependencies(&self, content: &str, dependencies: &mut Vec<Dependency>) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &self.tool_patterns {
                for caps in pattern.captures_iter(line) {
                    if let Some(tool_match) = caps.get(1) {
                        let tool_name = tool_match.as_str().split_whitespace().next()
                            .unwrap_or(tool_match.as_str());

                        let dependency = Dependency {
                            dependency_type: DependencyType::ToolDependency,
                            name: tool_name.to_string(),
                            version: None,
                            direction: DependencyDirection::Required,
                            source_location: Some(SourceLocation {
                                line: line_num + 1,
                                column: tool_match.start(),
                                line_content: line.to_string(),
                            }),
                            metadata: HashMap::new(),
                        };
                        dependencies.push(dependency);
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze file dependencies
    fn analyze_file_dependencies(&self, script_path: &PathBuf, content: &str, dependencies: &mut Vec<Dependency>) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &self.file_patterns {
                for caps in pattern.captures_iter(line) {
                    if let Some(file_match) = caps.get(1) {
                        let file_path = file_match.as_str();

                        // Skip non-file patterns
                        if file_path.contains("http") || file_path.contains("$") {
                            continue;
                        }

                        let resolved_path = if file_path.starts_with('/') {
                            PathBuf::from(file_path)
                        } else {
                            script_path.parent()
                                .unwrap_or_else(|| Path::new("."))
                                .join(file_path)
                        };

                        let dependency = Dependency {
                            dependency_type: DependencyType::FileDependency,
                            name: resolved_path.to_string_lossy().to_string(),
                            version: None,
                            direction: DependencyDirection::Required,
                            source_location: Some(SourceLocation {
                                line: line_num + 1,
                                column: file_match.start(),
                                line_content: line.to_string(),
                            }),
                            metadata: HashMap::new(),
                        };
                        dependencies.push(dependency);
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze environment variable dependencies
    fn analyze_environment_dependencies(&self, content: &str, dependencies: &mut Vec<Dependency>) -> Result<()> {
        let mut env_vars = HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            for pattern in &self.env_patterns {
                for caps in pattern.captures_iter(line) {
                    if let Some(var_match) = caps.get(1) {
                        env_vars.insert((var_match.as_str().to_string(), line_num + 1));
                    }
                }
            }
        }

        for (var_name, line_num) in env_vars {
            let dependency = Dependency {
                dependency_type: DependencyType::EnvironmentDependency,
                name: var_name,
                version: None,
                direction: DependencyDirection::Runtime,
                source_location: None, // Multiple references possible
                metadata: HashMap::new(),
            };
            dependencies.push(dependency);
        }

        Ok(())
    }

    /// Link dependencies between script nodes
    fn link_dependencies(&self, nodes: &mut HashMap<PathBuf, ScriptDependencyNode>) -> Result<()> {
        let script_paths: HashSet<PathBuf> = nodes.keys().cloned().collect();

        // First collect all the relationships we need to add
        let mut relationships_to_add = Vec::new();
        for (script_path, node) in nodes.iter() {
            for dependency in &node.dependencies {
                if dependency.dependency_type == DependencyType::ScriptDependency {
                    let dep_path = script_path.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&dependency.name);

                    let normalized_dep = dep_path.canonicalize().unwrap_or(dep_path);

                    if script_paths.contains(&normalized_dep) {
                        relationships_to_add.push((normalized_dep, script_path.clone()));
                    }
                }
            }
        }

        // Then add all the relationships
        for (dep_path, script_path) in relationships_to_add {
            if let Some(dep_node) = nodes.get_mut(&dep_path) {
                dep_node.dependents.push(script_path);
            }
        }

        Ok(())
    }

    /// Perform topological sort to determine execution order
    fn topological_sort(
        &self,
        nodes: &HashMap<PathBuf, ScriptDependencyNode>,
    ) -> Result<(Vec<PathBuf>, Vec<CircularDependency>)> {
        let mut in_degree: HashMap<PathBuf, usize> = HashMap::new();
        let mut adj_list: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let mut all_scripts: HashSet<PathBuf> = HashSet::new();

        // Initialize data structures
        for script_path in nodes.keys() {
            all_scripts.insert(script_path.clone());
            in_degree.insert(script_path.clone(), 0);
            adj_list.insert(script_path.clone(), Vec::new());
        }

        // Build adjacency list and in-degree count
        for (script_path, node) in nodes {
            for dependency in &node.dependencies {
                if dependency.dependency_type == DependencyType::ScriptDependency {
                    let dep_path = script_path.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&dependency.name);
                    let normalized_dep = dep_path.canonicalize().unwrap_or(dep_path);

                    if all_scripts.contains(&normalized_dep) {
                        if let Some(degree) = in_degree.get_mut(script_path) {
                            *degree += 1;
                        }
                        if let Some(adjacents) = adj_list.get_mut(&normalized_dep) {
                            adjacents.push(script_path.clone());
                        }
                    }
                }
            }
        }

        // Kahn's algorithm for topological sort with cycle detection
        let mut queue: Vec<PathBuf> = in_degree.iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(path, _)| path.clone())
            .collect();
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            result.push(current.clone());

            if let Some(adjacents) = adj_list.get(&current) {
                for adjacent in adjacents {
                    if let Some(degree) = in_degree.get_mut(adjacent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(adjacent.clone());
                        }
                    }
                }
            }
        }

        // Detect circular dependencies
        let mut circular_dependencies = Vec::new();
        if result.len() != all_scripts.len() {
            let remaining_scripts: HashSet<PathBuf> = all_scripts.difference(&visited).cloned().collect();
            if !remaining_scripts.is_empty() {
                let cycle = self.detect_cycle(nodes, &remaining_scripts)?;
                circular_dependencies.push(cycle);
            }
        }

        Ok((result, circular_dependencies))
    }

    /// Detect a circular dependency in the remaining scripts
    fn detect_cycle(
        &self,
        nodes: &HashMap<PathBuf, ScriptDependencyNode>,
        remaining_scripts: &HashSet<PathBuf>,
    ) -> Result<CircularDependency> {
        // Simple cycle detection - in practice, you'd use DFS for more complex cycles
        let cycle_scripts: Vec<PathBuf> = remaining_scripts.iter().cloned().collect();

        Ok(CircularDependency {
            scripts: cycle_scripts.clone(),
            cycle_length: cycle_scripts.len(),
            description: format!("Circular dependency detected among {} scripts", cycle_scripts.len()),
        })
    }

    /// Categorize dependencies into missing and optional
    fn categorize_dependencies(
        &self,
        nodes: &HashMap<PathBuf, ScriptDependencyNode>,
    ) -> (Vec<(PathBuf, Dependency)>, Vec<(PathBuf, Dependency)>) {
        let mut missing_dependencies = Vec::new();
        let mut optional_dependencies = Vec::new();

        for (script_path, node) in nodes {
            for dependency in &node.dependencies {
                match dependency.direction {
                    DependencyDirection::Required => {
                        missing_dependencies.push((script_path.clone(), dependency.clone()));
                    }
                    DependencyDirection::Optional => {
                        optional_dependencies.push((script_path.clone(), dependency.clone()));
                    }
                    DependencyDirection::Runtime => {
                        // Runtime dependencies are handled separately
                    }
                }
            }
        }

        (missing_dependencies, optional_dependencies)
    }

    /// Generate summary statistics
    fn generate_summary(
        &self,
        nodes: &HashMap<PathBuf, ScriptDependencyNode>,
        execution_order: &[PathBuf],
        circular_dependencies: &[CircularDependency],
    ) -> ResolutionSummary {
        let total_scripts = nodes.len();
        let mut total_dependencies = 0;
        let mut independent_scripts = 0;
        let mut dependent_scripts = 0;

        for node in nodes.values() {
            total_dependencies += node.dependencies.len();
            if node.dependencies.is_empty() {
                independent_scripts += 1;
            } else {
                dependent_scripts += 1;
            }
        }

        ResolutionSummary {
            total_scripts,
            total_dependencies,
            independent_scripts,
            dependent_scripts,
            max_dependency_depth: execution_order.len(),
            circular_dependency_count: circular_dependencies.len(),
        }
    }
}

impl fmt::Display for DependencyResolutionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Dependency Resolution Summary")?;
        writeln!(f, "=============================")?;
        writeln!(f, "Total Scripts: {}", self.summary.total_scripts)?;
        writeln!(f, "Total Dependencies: {}", self.summary.total_dependencies)?;
        writeln!(f, "Independent Scripts: {}", self.summary.independent_scripts)?;
        writeln!(f, "Dependent Scripts: {}", self.summary.dependent_scripts)?;
        writeln!(f, "Max Execution Depth: {}", self.summary.max_dependency_depth)?;
        writeln!(f, "Circular Dependencies: {}", self.summary.circular_dependency_count)?;

        if !self.circular_dependencies.is_empty() {
            writeln!(f, "\nCircular Dependencies:")?;
            for cycle in &self.circular_dependencies {
                writeln!(f, "- {}: {}", cycle.description, cycle.cycle_length)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_dependency_resolution_config_default() {
        let config = DependencyResolutionConfig::default();
        assert!(config.include_optional);
        assert!(config.analyze_tools);
        assert!(config.analyze_files);
        assert!(config.analyze_environment);
        assert_eq!(config.max_depth, 10);
    }

    #[test]
    fn test_script_type_detection() {
        let resolver = ScriptDependencyResolver::new(DependencyResolutionConfig::default());

        // Test bash script detection
        let bash_content = "#!/bin/bash\necho 'test'";
        assert_eq!(resolver.detect_script_type(&PathBuf::from("test.sh"), bash_content), "bash");

        // Test python script detection
        let python_content = "#!/usr/bin/env python3\nprint('test')";
        assert_eq!(resolver.detect_script_type(&PathBuf::from("test.py"), python_content), "python");

        // Test extension-based detection
        assert_eq!(resolver.detect_script_type(&PathBuf::from("test.js"), ""), "javascript");
        assert_eq!(resolver.detect_script_type(&PathBuf::from("unknown.xyz"), ""), "unknown");
    }

    #[tokio::test]
    async fn test_simple_dependency_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let script1_path = temp_dir.path().join("script1.sh");
        let script2_path = temp_dir.path().join("script2.sh");

        // Create test scripts with dependencies
        fs::write(&script1_path, "#!/bin/bash\nsource script2.sh\necho 'test1'").unwrap();
        fs::write(&script2_path, "#!/bin/bash\necho 'test2'").unwrap();

        let resolver = ScriptDependencyResolver::new(DependencyResolutionConfig::default());
        let result = resolver.resolve_dependencies(&[script1_path, script2_path]).await.unwrap();

        assert_eq!(result.summary.total_scripts, 2);
        assert!(result.summary.total_dependencies > 0);
    }

    #[test]
    fn test_dependency_types() {
        let dep = Dependency {
            dependency_type: DependencyType::ScriptDependency,
            name: "helper.sh".to_string(),
            version: None,
            direction: DependencyDirection::Required,
            source_location: None,
            metadata: HashMap::new(),
        };

        assert_eq!(dep.dependency_type, DependencyType::ScriptDependency);
        assert_eq!(dep.name, "helper.sh");
        assert_eq!(dep.direction, DependencyDirection::Required);
    }

    #[test]
    fn test_circular_dependency_display() {
        let cycle = CircularDependency {
            scripts: vec![
                PathBuf::from("a.sh"),
                PathBuf::from("b.sh"),
                PathBuf::from("c.sh"),
            ],
            cycle_length: 3,
            description: "Test cycle".to_string(),
        };

        let display = format!("{}", cycle.description);
        assert!(display.contains("Test cycle"));
    }
}