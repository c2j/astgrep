//! Bash language parser and adapter
//! 
//! This module provides Bash-specific parsing and AST adaptation.

use crate::adapters::{AdapterContext, AdapterMetadata, AstAdapter};
use astgrep_ast::{AstBuilder, UniversalNode};
use astgrep_core::{AstNode, Language, LanguageParser, Result};
use std::path::Path;

/// Bash AST adapter
pub struct BashAdapter;

impl BashAdapter {
    /// Create a new Bash adapter
    pub fn new() -> Self {
        Self
    }

    /// Parse Bash-specific constructs
    fn parse_bash_construct(&self, source: &str, _context: &AdapterContext) -> Result<UniversalNode> {
        let trimmed = source.trim();
        
        if trimmed.starts_with("#!/") {
            self.parse_shebang(trimmed)
        } else if trimmed.starts_with("if ") || trimmed.starts_with("if[") {
            self.parse_if_statement(trimmed)
        } else if trimmed.starts_with("for ") {
            self.parse_for_loop(trimmed)
        } else if trimmed.starts_with("while ") {
            self.parse_while_loop(trimmed)
        } else if trimmed.starts_with("function ") || trimmed.contains("() {") {
            self.parse_function_definition(trimmed)
        } else if trimmed.starts_with("case ") {
            self.parse_case_statement(trimmed)
        } else if trimmed.contains('=') && !trimmed.contains(' ') {
            self.parse_variable_assignment(trimmed)
        } else if trimmed.starts_with("export ") {
            self.parse_export_statement(trimmed)
        } else if trimmed.starts_with("source ") || trimmed.starts_with(". ") {
            self.parse_source_statement(trimmed)
        } else {
            // Default to command
            self.parse_command(trimmed)
        }
    }

    /// Parse shebang line
    fn parse_shebang(&self, source: &str) -> Result<UniversalNode> {
        Ok(AstBuilder::shebang(source)
            .with_text(source.to_string()))
    }

    /// Parse if statement
    fn parse_if_statement(&self, source: &str) -> Result<UniversalNode> {
        // if [ condition ]; then ... fi
        let mut condition = "";
        
        if let Some(then_pos) = source.find("; then") {
            let condition_part = &source[3..then_pos]; // Skip "if "
            condition = condition_part.trim();
        } else if let Some(then_pos) = source.find(" then") {
            let condition_part = &source[3..then_pos]; // Skip "if "
            condition = condition_part.trim();
        }

        Ok(AstBuilder::simple_if_statement(condition)
            .with_text(source.to_string()))
    }

    /// Parse for loop
    fn parse_for_loop(&self, source: &str) -> Result<UniversalNode> {
        // for var in list; do ... done
        let mut variable = "";
        let mut iterable = "";
        
        if let Some(in_pos) = source.find(" in ") {
            let var_part = &source[4..in_pos]; // Skip "for "
            variable = var_part.trim();
            
            if let Some(do_pos) = source[in_pos..].find("; do") {
                let iter_part = &source[in_pos + 4..in_pos + do_pos]; // Skip " in "
                iterable = iter_part.trim();
            } else if let Some(do_pos) = source[in_pos..].find(" do") {
                let iter_part = &source[in_pos + 4..in_pos + do_pos]; // Skip " in "
                iterable = iter_part.trim();
            }
        }

        Ok(AstBuilder::simple_for_statement(&format!("{} in {}", variable, iterable))
            .with_text(source.to_string()))
    }

    /// Parse while loop
    fn parse_while_loop(&self, source: &str) -> Result<UniversalNode> {
        // while [ condition ]; do ... done
        let mut condition = "";
        
        if let Some(do_pos) = source.find("; do") {
            let condition_part = &source[6..do_pos]; // Skip "while "
            condition = condition_part.trim();
        } else if let Some(do_pos) = source.find(" do") {
            let condition_part = &source[6..do_pos]; // Skip "while "
            condition = condition_part.trim();
        }

        Ok(AstBuilder::simple_while_statement(condition)
            .with_text(source.to_string()))
    }

    /// Parse function definition
    fn parse_function_definition(&self, source: &str) -> Result<UniversalNode> {
        let mut function_name = "unknown";
        
        if source.starts_with("function ") {
            // function name() { ... }
            if let Some(paren_pos) = source.find("()") {
                let name_part = &source[9..paren_pos]; // Skip "function "
                function_name = name_part.trim();
            }
        } else if let Some(paren_pos) = source.find("() {") {
            // name() { ... }
            function_name = source[..paren_pos].trim();
        }

        Ok(AstBuilder::simple_function_declaration(function_name)
            .with_text(source.to_string()))
    }

    /// Parse case statement
    fn parse_case_statement(&self, source: &str) -> Result<UniversalNode> {
        // case $var in ... esac
        let mut variable = "";
        
        if let Some(in_pos) = source.find(" in") {
            let var_part = &source[5..in_pos]; // Skip "case "
            variable = var_part.trim();
        }

        Ok(AstBuilder::case_statement(variable)
            .with_text(source.to_string()))
    }

    /// Parse variable assignment
    fn parse_variable_assignment(&self, source: &str) -> Result<UniversalNode> {
        // VAR=value
        if let Some(eq_pos) = source.find('=') {
            let var_name = &source[..eq_pos];
            let var_value = &source[eq_pos + 1..];
            
            Ok(AstBuilder::variable_declaration(var_name, None)
                .with_attribute("type".to_string(), "bash_var".to_string())
                .with_value(var_value.to_string())
                .with_text(source.to_string()))
        } else {
            Err(astgrep_core::AnalysisError::parse_error("Invalid variable assignment"))
        }
    }

    /// Parse export statement
    fn parse_export_statement(&self, source: &str) -> Result<UniversalNode> {
        // export VAR=value or export VAR
        let export_part = &source[7..]; // Skip "export "
        
        if export_part.contains('=') {
            // export VAR=value
            if let Some(eq_pos) = export_part.find('=') {
                let var_name = &export_part[..eq_pos];
                let var_value = &export_part[eq_pos + 1..];
                
                Ok(AstBuilder::export_statement(var_name)
                    .with_value(var_value.to_string())
                    .with_text(source.to_string()))
            } else {
                Err(astgrep_core::AnalysisError::parse_error("Invalid export assignment"))
            }
        } else {
            // export VAR
            Ok(AstBuilder::export_statement(export_part.trim())
                .with_text(source.to_string()))
        }
    }

    /// Parse source statement
    fn parse_source_statement(&self, source: &str) -> Result<UniversalNode> {
        let file_path = if source.starts_with("source ") {
            &source[7..] // Skip "source "
        } else if source.starts_with(". ") {
            &source[2..] // Skip ". "
        } else {
            ""
        };

        Ok(AstBuilder::source_statement(file_path.trim())
            .with_text(source.to_string()))
    }

    /// Parse command
    fn parse_command(&self, source: &str) -> Result<UniversalNode> {
        // Parse command with arguments
        let parts: Vec<&str> = source.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(AstBuilder::command("").with_text(source.to_string()));
        }

        let command_name = parts[0];
        let mut command_node = AstBuilder::command(command_name);
        
        // Add arguments
        for arg in &parts[1..] {
            command_node = command_node.with_argument(arg.to_string());
        }

        // Check for pipes
        if source.contains(" | ") {
            command_node = command_node.with_pipe(true);
        }

        // Check for redirections
        if source.contains(" > ") || source.contains(" >> ") || source.contains(" < ") {
            command_node = command_node.with_redirection(true);
        }

        Ok(command_node.with_text(source.to_string()))
    }
}

impl AstAdapter for BashAdapter {
    fn adapt_node(&self, _node: &dyn std::any::Any, context: &AdapterContext) -> Result<UniversalNode> {
        self.parse_bash_construct(&context.source_code, context)
    }

    fn language(&self) -> Language {
        Language::Bash
    }

    fn metadata(&self) -> AdapterMetadata {
        AdapterMetadata::new(
            "BashAdapter".to_string(),
            "1.0.0".to_string(),
            "Bash AST adapter with shell scripting support".to_string(),
        )
        .with_feature("control_flow".to_string())
        .with_feature("functions".to_string())
        .with_feature("variables".to_string())
        .with_feature("commands".to_string())
        .with_feature("pipes_redirections".to_string())
        .with_feature("case_statements".to_string())
    }
}

/// Bash language parser
pub struct BashParser {
    adapter: BashAdapter,
}

impl BashParser {
    /// Create a new Bash parser
    pub fn new() -> Self {
        Self {
            adapter: BashAdapter::new(),
        }
    }
}

impl LanguageParser for BashParser {
    fn parse(&self, source: &str, file_path: &Path) -> Result<Box<dyn AstNode>> {
        let context = AdapterContext::new(
            file_path.to_string_lossy().to_string(),
            source.to_string(),
            Language::Bash,
        );

        let universal_node = self.adapter.parse_bash_construct(source, &context)?;
        Ok(Box::new(universal_node))
    }

    fn language(&self) -> Language {
        Language::Bash
    }

    fn supports_file(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext.to_lowercase().as_str(), "sh" | "bash" | "zsh")
        } else {
            // Check for shebang in filename (common for shell scripts without extension)
            file_path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("bash") || name.starts_with("sh"))
                .unwrap_or(false)
        }
    }
}

impl Default for BashParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_parser_creation() {
        let parser = BashParser::new();
        assert_eq!(parser.language(), Language::Bash);
    }

    #[test]
    fn test_bash_parser_supports_file() {
        let parser = BashParser::new();
        assert!(parser.supports_file(Path::new("script.sh")));
        assert!(parser.supports_file(Path::new("script.bash")));
        assert!(parser.supports_file(Path::new("script.zsh")));
        assert!(!parser.supports_file(Path::new("test.py")));
        assert!(!parser.supports_file(Path::new("test.js")));
    }

    #[test]
    fn test_parse_shebang() {
        let adapter = BashAdapter::new();
        
        let result = adapter.parse_shebang("#!/bin/bash");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "shebang");
    }

    #[test]
    fn test_parse_if_statement() {
        let adapter = BashAdapter::new();
        
        let result = adapter.parse_if_statement("if [ $x -gt 0 ]; then");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "if_statement");
    }

    #[test]
    fn test_parse_for_loop() {
        let adapter = BashAdapter::new();
        
        let result = adapter.parse_for_loop("for i in 1 2 3; do");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "for_statement");
    }

    #[test]
    fn test_parse_while_loop() {
        let adapter = BashAdapter::new();
        
        let result = adapter.parse_while_loop("while [ $count -lt 10 ]; do");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "while_statement");
    }

    #[test]
    fn test_parse_function_definition() {
        let adapter = BashAdapter::new();
        
        // Function keyword style
        let result = adapter.parse_function_definition("function my_func() {");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");

        // Simple style
        let result = adapter.parse_function_definition("my_func() {");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "function_declaration");
    }

    #[test]
    fn test_parse_variable_assignment() {
        let adapter = BashAdapter::new();
        
        let result = adapter.parse_variable_assignment("VAR=value");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "variable_declaration");
    }

    #[test]
    fn test_parse_export_statement() {
        let adapter = BashAdapter::new();
        
        // Export with assignment
        let result = adapter.parse_export_statement("export PATH=/usr/bin");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "export_statement");

        // Export existing variable
        let result = adapter.parse_export_statement("export PATH");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "export_statement");
    }

    #[test]
    fn test_parse_source_statement() {
        let adapter = BashAdapter::new();
        
        // source command
        let result = adapter.parse_source_statement("source ~/.bashrc");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "source_statement");

        // dot command
        let result = adapter.parse_source_statement(". ~/.bashrc");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "source_statement");
    }

    #[test]
    fn test_parse_command() {
        let adapter = BashAdapter::new();
        
        // Simple command
        let result = adapter.parse_command("ls -la");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "command");

        // Command with pipe
        let result = adapter.parse_command("cat file.txt | grep pattern");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "command");

        // Command with redirection
        let result = adapter.parse_command("echo hello > output.txt");
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.node_type(), "command");
    }

    #[test]
    fn test_bash_adapter_metadata() {
        let adapter = BashAdapter::new();
        let metadata = adapter.metadata();
        
        assert_eq!(metadata.name, "BashAdapter");
        assert!(metadata.supported_features.contains(&"control_flow".to_string()));
        assert!(metadata.supported_features.contains(&"functions".to_string()));
        assert!(metadata.supported_features.contains(&"pipes_redirections".to_string()));
    }
}
