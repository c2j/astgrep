//! Tree-sitter based parser implementation
//! 
//! This module provides tree-sitter based parsing for various languages.

use cr_ast::{UniversalNode, NodeType};
use cr_core::{Language, Result};
use tree_sitter::{Parser, Tree, Node};
use std::collections::HashMap;

/// Pattern types for AST-based matching
#[derive(Debug, Clone)]
enum PatternType {
    StringLiteral(String),
    NumericLiteral(String),
    FunctionCall(String),
    ImportStatement(String),
    MethodCall(String, String), // (object, method)
    Identifier(String),
    // Advanced pattern types
    MetaVariable(String),                    // $VAR, $FUNC
    MetaFunctionCall(String, Vec<String>),   // $FUNC($ARG1, $ARG2)
    PatternEither(Vec<PatternType>),         // pattern-either
    PatternNot(Box<PatternType>),            // pattern-not
    PatternInside(Box<PatternType>, Box<PatternType>), // pattern-inside
    PatternWhere(Box<PatternType>, String), // pattern-where
    Generic(String),
}

/// Metavariable bindings for pattern matching
#[derive(Debug, Clone)]
pub struct MetaVariableBindings {
    bindings: std::collections::HashMap<String, String>,
}

/// Precise location information for improved positioning
#[derive(Debug, Clone)]
struct PreciseLocation {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    start_byte: usize,
    end_byte: usize,
}

/// Character position for UTF-8 support
#[derive(Debug, Clone)]
struct CharPosition {
    line: usize,
    column: usize,
}

impl MetaVariableBindings {
    pub fn new() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
        }
    }

    pub fn bind(&mut self, var_name: &str, value: &str) -> bool {
        if let Some(existing) = self.bindings.get(var_name) {
            // Check if binding is consistent
            existing == value
        } else {
            self.bindings.insert(var_name.to_string(), value.to_string());
            true
        }
    }

    pub fn get(&self, var_name: &str) -> Option<&String> {
        self.bindings.get(var_name)
    }
}

/// Tree-sitter based parser
pub struct TreeSitterParser {
    parsers: HashMap<Language, Parser>,
}

impl TreeSitterParser {
    /// Create a new tree-sitter parser
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        
        // Initialize Python parser
        let mut parser = Parser::new();
        if parser.set_language(tree_sitter_python::language()).is_ok() {
            parsers.insert(Language::Python, parser);
        }

        // Initialize JavaScript parser
        let mut parser = Parser::new();
        if parser.set_language(tree_sitter_javascript::language()).is_ok() {
            parsers.insert(Language::JavaScript, parser);
        }

        // Initialize Java parser
        let mut parser = Parser::new();
        if parser.set_language(tree_sitter_java::language()).is_ok() {
            parsers.insert(Language::Java, parser);
        }

        // Initialize PHP parser (if available)
        #[cfg(feature = "php")]
        {
            let mut parser = Parser::new();
            if let Ok(php_lang) = tree_sitter_php::language() {
                if parser.set_language(php_lang).is_ok() {
                    parsers.insert(Language::PHP, parser);
                }
            }
        }

        // Initialize SQL parser (using basic parsing for now)
        // Note: SQL will use the basic SQL adapter instead of tree-sitter
        // {
        //     let mut parser = Parser::new();
        //     if parser.set_language(tree_sitter_sql::language()).is_ok() {
        //         parsers.insert(Language::Sql, parser);
        //     }
        // }

        // Initialize Bash parser
        {
            let mut parser = Parser::new();
            if parser.set_language(tree_sitter_bash::language()).is_ok() {
                parsers.insert(Language::Bash, parser);
            }
        }

        Ok(Self { parsers })
    }
    
    /// Parse source code using tree-sitter
    pub fn parse(&mut self, source: &str, language: Language) -> Result<Option<Tree>> {
        if let Some(parser) = self.parsers.get_mut(&language) {
            Ok(parser.parse(source, None))
        } else {
            Ok(None)
        }
    }
    
    /// Convert tree-sitter tree to universal AST
    pub fn tree_to_universal_ast(&self, tree: &Tree, source: &str) -> Result<UniversalNode> {
        let root_node = tree.root_node();
        eprintln!("🔍 Tree-sitter root node: kind={}, child_count={}", root_node.kind(), root_node.child_count());
        for i in 0..root_node.child_count() {
            if let Some(child) = root_node.child(i) {
                eprintln!("🔍   Child {}: kind={}, text={:?}", i, child.kind(), child.utf8_text(source.as_bytes()).unwrap_or(""));
            }
        }
        self.convert_node(&root_node, source)
    }
    
    /// Convert a tree-sitter node to universal node with improved precision
    fn convert_node(&self, node: &Node, source: &str) -> Result<UniversalNode> {
        let node_type = self.map_node_type(node.kind());
        let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

        // Calculate precise location information
        let location = self.calculate_precise_location(node, source);

        let mut universal_node = UniversalNode::new(node_type)
            .with_text(text.clone())
            .with_location(
                location.start_line,
                location.start_column,
                location.end_line,
                location.end_column,
            );

        // Add metadata about the original tree-sitter node
        universal_node = universal_node
            .with_metadata("ts_kind".to_string(), node.kind().to_string())
            .with_metadata("ts_id".to_string(), node.id().to_string())
            .with_metadata("byte_range".to_string(), format!("{}-{}", node.start_byte(), node.end_byte()));

        // Add syntax highlighting information if available
        if let Some(syntax_info) = self.extract_syntax_info(node, &text) {
            universal_node = universal_node.with_metadata("syntax_info".to_string(), syntax_info);
        }

        // Add children with improved filtering
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                // Skip certain auxiliary nodes that don't add semantic value
                if self.should_include_child(&child) {
                    let child_universal = self.convert_node(&child, source)?;
                    universal_node = universal_node.add_child(child_universal);
                }
            }
        }

        Ok(universal_node)
    }

    /// Calculate precise location information for a node
    fn calculate_precise_location(&self, node: &Node, source: &str) -> PreciseLocation {
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        // Convert byte positions to character positions for better Unicode support
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        // Calculate actual character positions
        let source_bytes = source.as_bytes();
        let start_char_pos = self.byte_to_char_position(source_bytes, start_byte);
        let end_char_pos = self.byte_to_char_position(source_bytes, end_byte);

        PreciseLocation {
            start_line: start_pos.row + 1,
            start_column: start_char_pos.column + 1,
            end_line: end_pos.row + 1,
            end_column: end_char_pos.column + 1,
            start_byte,
            end_byte,
        }
    }

    /// Convert byte position to character position
    fn byte_to_char_position(&self, source_bytes: &[u8], byte_pos: usize) -> CharPosition {
        let mut line = 0;
        let mut column = 0;
        let mut current_byte = 0;

        while current_byte < byte_pos && current_byte < source_bytes.len() {
            if source_bytes[current_byte] == b'\n' {
                line += 1;
                column = 0;
            } else {
                // Handle UTF-8 characters properly
                let char_len = self.utf8_char_len(source_bytes[current_byte]);
                if current_byte + char_len <= byte_pos {
                    column += 1;
                }
                current_byte += char_len - 1;
            }
            current_byte += 1;
        }

        CharPosition { line, column }
    }

    /// Get the length of a UTF-8 character from its first byte
    fn utf8_char_len(&self, first_byte: u8) -> usize {
        if first_byte < 0x80 {
            1
        } else if first_byte < 0xE0 {
            2
        } else if first_byte < 0xF0 {
            3
        } else {
            4
        }
    }

    /// Extract syntax highlighting information
    fn extract_syntax_info(&self, node: &Node, text: &str) -> Option<String> {
        match node.kind() {
            "string" | "string_literal" => Some("string".to_string()),
            "number" | "integer" | "float" => Some("number".to_string()),
            "comment" => Some("comment".to_string()),
            "identifier" => Some("identifier".to_string()),
            "keyword" => Some("keyword".to_string()),
            kind if kind.contains("keyword") => Some("keyword".to_string()),
            _ => None,
        }
    }

    /// Determine if a child node should be included in the AST
    fn should_include_child(&self, child: &Node) -> bool {
        match child.kind() {
            // Skip punctuation and whitespace nodes
            "," | ";" | "(" | ")" | "[" | "]" | "{" | "}" => false,
            // Skip certain auxiliary nodes
            "whitespace" | "newline" => false,
            // Include everything else
            _ => true,
        }
    }
    
    /// Map tree-sitter node types to universal node types with improved precision
    fn map_node_type(&self, ts_kind: &str) -> NodeType {
        match ts_kind {
            // Program structure
            "module" | "program" | "source_file" | "compilation_unit" => NodeType::Program,

            // Function definitions
            "function_definition" | "function_declaration" | "method_definition" |
            "constructor_definition" | "arrow_function" | "function_expression" => NodeType::FunctionDeclaration,

            // Function calls
            "call_expression" | "call" | "method_invocation" | "constructor_invocation" |
            "new_expression" => NodeType::CallExpression,

            // Assignments
            "assignment" | "assignment_expression" | "augmented_assignment" => NodeType::AssignmentExpression,
            "variable_declaration" | "variable_declarator" => NodeType::VariableDeclaration,

            // Identifiers and names
            "identifier" | "field_identifier" | "type_identifier" | "property_identifier" |
            "variable_name" | "function_name" | "class_name" => NodeType::Identifier,

            // Literals
            "string" | "string_literal" | "template_string" | "raw_string" |
            "character_literal" | "escape_sequence" => NodeType::Literal,
            "integer" | "number" | "integer_literal" | "float_literal" |
            "decimal_integer_literal" | "hex_integer_literal" | "binary_integer_literal" |
            "octal_integer_literal" => NodeType::Literal,
            "boolean" | "true" | "false" | "null" | "undefined" | "none" => NodeType::Literal,

            // Control flow
            "if_statement" | "conditional_expression" | "ternary_expression" => NodeType::IfStatement,
            "while_statement" | "do_statement" => NodeType::WhileStatement,
            "for_statement" | "for_in_statement" | "for_of_statement" | "enhanced_for_statement" => NodeType::ForStatement,
            "return_statement" => NodeType::ReturnStatement,
            "break_statement" => NodeType::BreakStatement,
            "continue_statement" => NodeType::ContinueStatement,
            "throw_statement" => NodeType::ThrowStatement,
            "try_statement" | "catch_clause" | "finally_clause" => NodeType::TryStatement,

            // Expressions
            "expression_statement" => NodeType::ExpressionStatement,
            "binary_expression" | "logical_expression" | "comparison_expression" => NodeType::BinaryExpression,
            "unary_expression" | "update_expression" => NodeType::UnaryExpression,
            "member_expression" | "subscript_expression" | "attribute" => NodeType::MemberExpression,
            "array" | "array_literal" | "list" | "tuple" => NodeType::ArrayExpression,
            "object" | "object_literal" | "dictionary" | "hash" => NodeType::ObjectExpression,

            // Blocks and statements
            "block" | "block_statement" | "compound_statement" | "suite" => NodeType::BlockStatement,
            "class_declaration" | "class_definition" => NodeType::ClassDeclaration,
            "interface_declaration" => NodeType::InterfaceDeclaration,
            "import_statement" | "import_declaration" | "from_import" | "include" => NodeType::ImportDeclaration,
            "export_statement" | "export_declaration" => NodeType::ExportDeclaration,

            // Comments and documentation
            "comment" | "line_comment" | "block_comment" | "documentation_comment" => NodeType::Comment,

            // Language-specific constructs
            "lambda" | "lambda_expression" | "arrow_function" => NodeType::LambdaExpression,

            // Switch statements
            "switch_statement" => NodeType::SwitchStatement,
            "case_statement" | "case_clause" => NodeType::CaseStatement,

            // Bash-specific constructs
            "command" | "simple_command" | "pipeline" | "command_substitution" => NodeType::CallExpression,
            "variable_assignment" | "assignment" => NodeType::AssignmentExpression,
            "word" | "variable_name" | "command_name" => NodeType::Identifier,
            "string" | "raw_string" | "ansi_c_quoting" | "quoted_string" => NodeType::Literal,
            "expansion" | "command_substitution" | "process_substitution" => NodeType::CallExpression,
            "if_statement" | "while_statement" | "for_statement" | "case_statement" => NodeType::ControlFlowStatement,
            "function_definition" => NodeType::FunctionDeclaration,
            "compound_statement" | "subshell" => NodeType::BlockStatement,
            "test_command" | "test_operator" => NodeType::BinaryExpression,
            "redirected_statement" | "file_redirect" => NodeType::ExpressionStatement,

            // SQL-specific constructs
            "select_statement" | "insert_statement" | "update_statement" | "delete_statement" |
            "create_statement" | "drop_statement" | "alter_statement" => NodeType::ExpressionStatement,
            "from_clause" | "where_clause" | "having_clause" | "order_by_clause" |
            "group_by_clause" | "limit_clause" => NodeType::ExpressionStatement,
            "join_clause" | "inner_join" | "left_join" | "right_join" | "full_join" => NodeType::ExpressionStatement,
            "column_reference" | "table_reference" | "field" => NodeType::Identifier,
            "function_call" | "aggregate_function" => NodeType::CallExpression,
            "binary_expression" | "comparison_predicate" | "in_predicate" |
            "like_predicate" | "between_predicate" => NodeType::BinaryExpression,
            "literal" | "string_literal" | "number_literal" | "boolean_literal" => NodeType::Literal,
            "subquery" | "parenthesized_expression" => NodeType::ExpressionStatement,
            "union" | "intersect" | "except" => NodeType::BinaryExpression,

            // Error handling
            "ERROR" => NodeType::Unknown,

            // Default case - try to infer from context
            _ => self.infer_node_type_from_context(ts_kind),
        }
    }

    /// Infer node type from context when direct mapping is not available
    fn infer_node_type_from_context(&self, ts_kind: &str) -> NodeType {
        // Check for common patterns in node names
        if ts_kind.contains("statement") {
            if ts_kind.contains("control") || ts_kind.contains("flow") {
                NodeType::ControlFlowStatement
            } else if ts_kind.contains("declaration") {
                NodeType::DeclarationStatement
            } else {
                NodeType::ExpressionStatement
            }
        } else if ts_kind.contains("expression") {
            if ts_kind.contains("binary") || ts_kind.contains("logical") {
                NodeType::BinaryExpression
            } else if ts_kind.contains("unary") {
                NodeType::UnaryExpression
            } else if ts_kind.contains("member") || ts_kind.contains("attribute") {
                NodeType::MemberExpression
            } else if ts_kind.contains("call") {
                NodeType::CallExpression
            } else if ts_kind.contains("assignment") {
                NodeType::AssignmentExpression
            } else if ts_kind.contains("conditional") {
                NodeType::ConditionalExpression
            } else {
                NodeType::BinaryExpression
            }
        } else if ts_kind.contains("declaration") {
            if ts_kind.contains("function") {
                NodeType::FunctionDeclaration
            } else if ts_kind.contains("class") {
                NodeType::ClassDeclaration
            } else if ts_kind.contains("variable") {
                NodeType::VariableDeclaration
            } else if ts_kind.contains("import") {
                NodeType::ImportDeclaration
            } else if ts_kind.contains("export") {
                NodeType::ExportDeclaration
            } else {
                NodeType::DeclarationStatement
            }
        } else if ts_kind.contains("literal") {
            NodeType::Literal
        } else if ts_kind.contains("identifier") || ts_kind.contains("name") {
            NodeType::Identifier
        } else if ts_kind.contains("call") || ts_kind.contains("invocation") {
            NodeType::CallExpression
        } else if ts_kind.contains("block") || ts_kind.contains("body") {
            NodeType::BlockStatement
        } else if ts_kind.contains("comment") {
            NodeType::Comment
        } else {
            // Last resort - classify as unknown
            NodeType::Unknown
        }
    }
    
    /// Find nodes matching a pattern in the AST
    pub fn find_pattern_matches<'a>(&self, tree: &'a Tree, source: &str, pattern: &str) -> Result<Vec<Node<'a>>> {
        let mut matches = Vec::new();
        let root = tree.root_node();
        self.find_matches_recursive(&root, source, pattern, &mut matches)?;
        Ok(matches)
    }

    /// Recursively find pattern matches
    fn find_matches_recursive<'a>(&self, node: &Node<'a>, source: &str, pattern: &str, matches: &mut Vec<Node<'a>>) -> Result<()> {
        // Check if current node matches the pattern
        if self.node_matches_pattern(node, source, pattern)? {
            // Only add this match if it's a leaf match or the most specific match
            if self.is_most_specific_match(node, source, pattern)? {
                matches.push(*node);
            }
            // Don't recurse into children if we found a match at this level
            // This prevents duplicate matches for the same semantic construct
            return Ok(());
        }

        // Check children only if current node doesn't match
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_matches_recursive(&child, source, pattern, matches)?;
            }
        }

        Ok(())
    }

    /// Check if this is the most specific match for the pattern
    fn is_most_specific_match(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        // For function calls, we want the call_expression node, not its children
        if pattern.ends_with("(...)") && matches!(node.kind(), "call_expression" | "call") {
            return Ok(true);
        }

        // For string literals, we want the string node itself
        if pattern.starts_with('"') && pattern.ends_with('"') && matches!(node.kind(), "string" | "string_literal") {
            return Ok(true);
        }

        // For numeric literals, we want the number node itself
        if pattern.chars().all(|c| c.is_ascii_digit() || c == '.') &&
           matches!(node.kind(), "integer" | "number" | "integer_literal" | "float" | "decimal_literal") {
            return Ok(true);
        }

        // For import statements, we want the import statement node
        if pattern.starts_with("import ") && matches!(node.kind(), "import_statement" | "import_from_statement") {
            return Ok(true);
        }

        // For other patterns, check if any children also match
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if self.node_matches_pattern(&child, source, pattern)? {
                    // If a child also matches, this is not the most specific
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
    
    /// Check if a node matches a pattern using AST-based matching
    fn node_matches_pattern(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        let mut bindings = MetaVariableBindings::new();
        self.node_matches_pattern_with_bindings(node, source, pattern, &mut bindings)
    }

    /// Check if a node matches a pattern with metavariable bindings
    fn node_matches_pattern_with_bindings(
        &self,
        node: &Node,
        source: &str,
        pattern: &str,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // Use AST-based pattern matching instead of text matching
        match self.classify_pattern(pattern) {
            PatternType::StringLiteral(content) => {
                self.match_string_literal(node, source, &content)
            }
            PatternType::NumericLiteral(value) => {
                self.match_numeric_literal(node, source, &value)
            }
            PatternType::FunctionCall(func_name) => {
                self.match_function_call(node, source, &func_name)
            }
            PatternType::ImportStatement(import_spec) => {
                self.match_import_statement(node, source, &import_spec)
            }
            PatternType::MethodCall(object, method) => {
                self.match_method_call(node, source, &object, &method)
            }
            PatternType::Identifier(name) => {
                self.match_identifier(node, source, &name)
            }
            PatternType::MetaVariable(var_name) => {
                self.match_metavariable(node, source, &var_name, bindings)
            }
            PatternType::MetaFunctionCall(func_name, args) => {
                self.match_meta_function_call(node, source, &func_name, &args, bindings)
            }
            PatternType::PatternEither(patterns) => {
                self.match_pattern_either(node, source, &patterns, bindings)
            }
            PatternType::PatternNot(pattern) => {
                self.match_pattern_not(node, source, &pattern, bindings)
            }
            PatternType::PatternInside(inner, outer) => {
                self.match_pattern_inside(node, source, &inner, &outer, bindings)
            }
            PatternType::PatternWhere(pattern, condition) => {
                self.match_pattern_where(node, source, &pattern, &condition, bindings)
            }
            PatternType::Generic(text) => {
                // Fallback for unrecognized patterns - but be more selective
                self.match_generic_pattern(node, source, &text)
            }
        }
    }

    /// Classify a pattern into specific types for AST-based matching
    fn classify_pattern(&self, pattern: &str) -> PatternType {
        let pattern = pattern.trim();

        // Check for metavariables first
        if pattern.starts_with('$') {
            return self.classify_metavariable_pattern(pattern);
        }

        if pattern.starts_with('"') && pattern.ends_with('"') && pattern.len() >= 2 {
            // String literal: "hello world"
            PatternType::StringLiteral(pattern[1..pattern.len()-1].to_string())
        } else if pattern.chars().all(|c| c.is_ascii_digit() || c == '.') {
            // Numeric literal: 42, 3.14
            PatternType::NumericLiteral(pattern.to_string())
        } else if pattern.ends_with("(...)") {
            // Function call: eval(...) or $FUNC(...)
            let func_name = pattern[..pattern.len()-5].to_string();
            if func_name.starts_with('$') {
                PatternType::MetaFunctionCall(func_name, vec![])
            } else {
                PatternType::FunctionCall(func_name)
            }
        } else if pattern.contains('(') && pattern.contains(')') && pattern.contains('$') {
            // Function call with metavariables: eval($CODE), $FUNC($ARG)
            return self.parse_meta_function_call(pattern);
        } else if pattern.starts_with("import ") {
            // Import statement: import foo.bar
            PatternType::ImportStatement(pattern.to_string())
        } else if pattern.contains('.') && !pattern.contains(' ') && !pattern.starts_with('"') {
            // Method call: System.out.println, obj.method, $OBJ.method
            let parts: Vec<&str> = pattern.split('.').collect();
            if parts.len() >= 2 {
                let object = parts[..parts.len()-1].join(".");
                let method = parts[parts.len()-1].to_string();
                PatternType::MethodCall(object, method)
            } else {
                PatternType::Generic(pattern.to_string())
            }
        } else if pattern.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // Simple identifier: variable_name
            PatternType::Identifier(pattern.to_string())
        } else {
            // Generic pattern
            PatternType::Generic(pattern.to_string())
        }
    }

    /// Classify metavariable patterns
    fn classify_metavariable_pattern(&self, pattern: &str) -> PatternType {
        if pattern.ends_with("(...)") {
            // Meta function call: $FUNC(...)
            let func_name = pattern[..pattern.len()-5].to_string();
            PatternType::MetaFunctionCall(func_name, vec![])
        } else if pattern.contains('(') && pattern.contains(')') {
            // Meta function call with args: $FUNC($ARG1, $ARG2)
            self.parse_meta_function_call(pattern)
        } else {
            // Simple metavariable: $VAR
            PatternType::MetaVariable(pattern.to_string())
        }
    }

    /// Parse meta function call with arguments
    fn parse_meta_function_call(&self, pattern: &str) -> PatternType {
        if let Some(open_paren) = pattern.find('(') {
            if let Some(close_paren) = pattern.rfind(')') {
                let func_name = pattern[..open_paren].to_string();
                let args_str = &pattern[open_paren+1..close_paren];
                let args: Vec<String> = args_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                return PatternType::MetaFunctionCall(func_name, args);
            }
        }
        PatternType::Generic(pattern.to_string())
    }

    /// Match string literal nodes
    fn match_string_literal(&self, node: &Node, source: &str, content: &str) -> Result<bool> {
        if matches!(node.kind(), "string" | "string_literal") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            // Remove quotes and check content
            let cleaned_text = node_text.trim_matches('"').trim_matches('\'');
            Ok(cleaned_text == content)
        } else {
            Ok(false)
        }
    }

    /// Match numeric literal nodes
    fn match_numeric_literal(&self, node: &Node, source: &str, value: &str) -> Result<bool> {
        if matches!(node.kind(), "integer" | "number" | "integer_literal" | "float" | "decimal_literal") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            Ok(node_text == value)
        } else {
            Ok(false)
        }
    }

    /// Match function call nodes
    fn match_function_call(&self, node: &Node, source: &str, func_name: &str) -> Result<bool> {
        if matches!(node.kind(), "call_expression" | "call") {
            // Check if the function name matches
            if let Some(function_node) = node.child_by_field_name("function") {
                let func_text = function_node.utf8_text(source.as_bytes()).unwrap_or("");
                Ok(func_text == func_name)
            } else {
                // Fallback: check first child
                if let Some(first_child) = node.child(0) {
                    let func_text = first_child.utf8_text(source.as_bytes()).unwrap_or("");
                    Ok(func_text == func_name)
                } else {
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Match import statement nodes
    fn match_import_statement(&self, node: &Node, source: &str, import_spec: &str) -> Result<bool> {
        if matches!(node.kind(), "import_statement" | "import_from_statement") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");

            // Extract the module path from the import specification
            if let Some(module_path) = self.extract_module_path_from_pattern(import_spec) {
                // Check if the import statement contains this module path
                Ok(node_text.contains(&module_path))
            } else {
                // Fallback to exact match
                Ok(node_text.trim() == import_spec.trim())
            }
        } else {
            Ok(false)
        }
    }

    /// Extract module path from import pattern (e.g., "foo.bar" from "import foo.bar")
    fn extract_module_path_from_pattern(&self, pattern: &str) -> Option<String> {
        let pattern = pattern.trim();
        if pattern.starts_with("import ") {
            let module_part = &pattern[7..]; // Remove "import "
            // Handle "import foo.bar as alias" -> "foo.bar"
            let module_path = module_part.split_whitespace().next().unwrap_or("");
            if !module_path.is_empty() {
                Some(module_path.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Match method call nodes (e.g., System.out.println)
    fn match_method_call(&self, node: &Node, source: &str, object: &str, method: &str) -> Result<bool> {
        if matches!(node.kind(), "call_expression" | "call") {
            // Check if this is a method call on the specified object
            if let Some(function_node) = node.child_by_field_name("function") {
                if matches!(function_node.kind(), "attribute" | "member_expression" | "field_expression") {
                    let func_text = function_node.utf8_text(source.as_bytes()).unwrap_or("");
                    let expected = format!("{}.{}", object, method);
                    Ok(func_text == expected)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Match identifier nodes
    fn match_identifier(&self, node: &Node, source: &str, name: &str) -> Result<bool> {
        if matches!(node.kind(), "identifier") {
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            Ok(node_text == name)
        } else {
            Ok(false)
        }
    }

    /// Match metavariable patterns
    fn match_metavariable(&self, node: &Node, source: &str, var_name: &str, bindings: &mut MetaVariableBindings) -> Result<bool> {
        let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");

        // Try to bind the metavariable
        if bindings.bind(var_name, node_text) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Match meta function call patterns
    fn match_meta_function_call(
        &self,
        node: &Node,
        source: &str,
        func_name: &str,
        args: &[String],
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {

        if !matches!(node.kind(), "call_expression" | "call" | "method_invocation") {
            return Ok(false);
        }

        // Get function name - handle different node types
        let actual_func_text = if node.kind() == "method_invocation" {
            // For Java method invocations, we need to get the full method chain
            // e.g., System.err.print -> we want the full chain
            let mut parts = Vec::new();

            // Get the object part (e.g., System.err)
            if let Some(object_node) = node.child_by_field_name("object") {
                parts.push(object_node.utf8_text(source.as_bytes()).unwrap_or(""));
            }

            // Get the method name
            if let Some(name_node) = node.child_by_field_name("name") {
                parts.push(name_node.utf8_text(source.as_bytes()).unwrap_or(""));
            }

            if parts.len() == 2 {
                format!("{}.{}", parts[0], parts[1])
            } else {
                // Fallback: get the entire node text up to the opening parenthesis
                let full_text = node.utf8_text(source.as_bytes()).unwrap_or("");
                if let Some(paren_pos) = full_text.find('(') {
                    full_text[..paren_pos].to_string()
                } else {
                    full_text.to_string()
                }
            }
        } else {
            // For other call types, use the function field
            if let Some(function_node) = node.child_by_field_name("function") {
                function_node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
            } else {
                "".to_string()
            }
        };

        // If func_name is a metavariable, try to bind it
        if func_name.starts_with('$') {
            if !bindings.bind(func_name, &actual_func_text) {
                return Ok(false);
            }
        } else if actual_func_text != func_name {
            return Ok(false);
        }

        // Match arguments if specified
        if !args.is_empty() {
            return self.match_function_arguments(node, source, args, bindings);
        }

        Ok(true)
    }

    /// Match function arguments
    fn match_function_arguments(
        &self,
        node: &Node,
        source: &str,
        expected_args: &[String],
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // Get arguments node
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut actual_args = Vec::new();

            // Collect actual arguments
            for i in 0..args_node.child_count() {
                if let Some(arg_node) = args_node.child(i) {
                    if arg_node.kind() != "," && arg_node.kind() != "(" && arg_node.kind() != ")" {  // Skip separators
                        let arg_text = arg_node.utf8_text(source.as_bytes()).unwrap_or("");
                        actual_args.push(arg_text);
                    }
                }
            }

            // Check if argument count matches
            if actual_args.len() != expected_args.len() {
                return Ok(false);
            }

            // Match each argument
            for (expected, actual) in expected_args.iter().zip(actual_args.iter()) {
                if expected.starts_with('$') {
                    // Metavariable argument
                    if !bindings.bind(expected, actual) {
                        return Ok(false);
                    }
                } else if expected != actual {
                    return Ok(false);
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Match pattern-either (OR logic)
    fn match_pattern_either(
        &self,
        node: &Node,
        source: &str,
        patterns: &[PatternType],
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        for pattern in patterns {
            let mut temp_bindings = bindings.clone();
            if self.match_pattern_type(node, source, pattern, &mut temp_bindings)? {
                *bindings = temp_bindings;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Match pattern-not (NOT logic)
    fn match_pattern_not(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        let mut temp_bindings = bindings.clone();
        let matches = self.match_pattern_type(node, source, pattern, &mut temp_bindings)?;
        Ok(!matches)
    }

    /// Match pattern-inside (context matching)
    fn match_pattern_inside(
        &self,
        node: &Node,
        source: &str,
        inner: &PatternType,
        outer: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // Check if inner pattern matches current node
        if self.match_pattern_type(node, source, inner, bindings)? {
            // Check if we're inside an outer pattern
            let mut current = node.parent();
            while let Some(parent) = current {
                let mut temp_bindings = bindings.clone();
                if self.match_pattern_type(&parent, source, outer, &mut temp_bindings)? {
                    *bindings = temp_bindings;
                    return Ok(true);
                }
                current = parent.parent();
            }
        }
        Ok(false)
    }

    /// Match pattern-where (conditional matching)
    fn match_pattern_where(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        condition: &str,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        // First check if the pattern matches
        if self.match_pattern_type(node, source, pattern, bindings)? {
            // Then evaluate the condition (simplified implementation)
            self.evaluate_where_condition(node, source, condition, bindings)
        } else {
            Ok(false)
        }
    }

    /// Evaluate where condition (simplified)
    fn evaluate_where_condition(
        &self,
        _node: &Node,
        _source: &str,
        condition: &str,
        bindings: &MetaVariableBindings
    ) -> Result<bool> {
        // Simplified condition evaluation
        // In a full implementation, this would parse and evaluate complex conditions
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim();

                let left_value = if left.starts_with('$') {
                    bindings.get(left).map(|s| s.as_str()).unwrap_or("")
                } else {
                    left
                };

                let right_value = if right.starts_with('$') {
                    bindings.get(right).map(|s| s.as_str()).unwrap_or("")
                } else {
                    right.trim_matches('"')
                };

                return Ok(left_value == right_value);
            }
        }

        // Default to true for unrecognized conditions
        Ok(true)
    }

    /// Helper to match a PatternType
    fn match_pattern_type(
        &self,
        node: &Node,
        source: &str,
        pattern: &PatternType,
        bindings: &mut MetaVariableBindings
    ) -> Result<bool> {
        match pattern {
            PatternType::StringLiteral(content) => self.match_string_literal(node, source, content),
            PatternType::NumericLiteral(value) => self.match_numeric_literal(node, source, value),
            PatternType::FunctionCall(func_name) => self.match_function_call(node, source, func_name),
            PatternType::ImportStatement(import_spec) => self.match_import_statement(node, source, import_spec),
            PatternType::MethodCall(object, method) => self.match_method_call(node, source, object, method),
            PatternType::Identifier(name) => self.match_identifier(node, source, name),
            PatternType::MetaVariable(var_name) => self.match_metavariable(node, source, var_name, bindings),
            PatternType::MetaFunctionCall(func_name, args) => self.match_meta_function_call(node, source, func_name, args, bindings),
            PatternType::PatternEither(patterns) => self.match_pattern_either(node, source, patterns, bindings),
            PatternType::PatternNot(pattern) => self.match_pattern_not(node, source, pattern, bindings),
            PatternType::PatternInside(inner, outer) => self.match_pattern_inside(node, source, inner, outer, bindings),
            PatternType::PatternWhere(pattern, condition) => self.match_pattern_where(node, source, pattern, condition, bindings),
            PatternType::Generic(text) => self.match_generic_pattern(node, source, text),
        }
    }

    /// Generic pattern matching (fallback)
    fn match_generic_pattern(&self, node: &Node, source: &str, pattern: &str) -> Result<bool> {
        // Only match on specific node types to avoid matching the entire file
        if matches!(node.kind(), "module" | "program" | "source_file") {
            return Ok(false);
        }

        let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
        Ok(node_text.contains(pattern))
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self { parsers: HashMap::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_core::AstNode;

    #[test]
    fn test_python_parsing() {
        let mut parser = TreeSitterParser::new().unwrap();
        let source = r#"
def hello():
    print("world")
    x = 42
"#;

        if let Ok(Some(tree)) = parser.parse(source, Language::Python) {
            let universal_ast = parser.tree_to_universal_ast(&tree, source).unwrap();
            assert_eq!(universal_ast.node_type(), "module");
        }
    }
    
    #[test]
    fn test_pattern_matching() {
        let mut parser = TreeSitterParser::new().unwrap();
        let source = r#"
print("hello")
x = 42
eval(code)
"#;
        
        if let Ok(Some(tree)) = parser.parse(source, Language::Python) {
            // Test string literal matching
            let matches = parser.find_pattern_matches(&tree, source, r#""hello""#).unwrap();
            assert!(!matches.is_empty());
            
            // Test numeric literal matching
            let matches = parser.find_pattern_matches(&tree, source, "42").unwrap();
            assert!(!matches.is_empty());
            
            // Test function call matching
            let matches = parser.find_pattern_matches(&tree, source, "eval(...)").unwrap();
            assert!(!matches.is_empty());
        }
    }
}
