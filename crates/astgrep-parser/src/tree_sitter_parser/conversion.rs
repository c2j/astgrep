//! Node conversion and type mapping
//!
//! This module provides tree-sitter node to universal node conversion
//! and node type mapping functionality.

use super::integration::TreeSitterParser;
use astgrep_ast::{NodeType, UniversalNode};
use astgrep_core::Result;
use tree_sitter::Node;

/// Character position for UTF-8 support
#[derive(Debug, Clone)]
struct CharPosition {
    line: usize,
    column: usize,
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

impl TreeSitterParser {
    /// Convert a tree-sitter node to universal node with improved precision
    pub(super) fn convert_node(&self, node: &Node, source: &str) -> Result<UniversalNode> {
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

        // Add metadata about original tree-sitter node
        universal_node = universal_node
            .with_metadata("ts_kind".to_string(), node.kind().to_string())
            .with_metadata("ts_id".to_string(), node.id().to_string())
            .with_metadata(
                "byte_range".to_string(),
                format!("{}-{}", node.start_byte(), node.end_byte()),
            );

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
    fn extract_syntax_info(&self, node: &Node, _text: &str) -> Option<String> {
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

    /// Determine if a child node should be included in AST
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
    pub(super) fn map_node_type(&self, ts_kind: &str) -> NodeType {
        match ts_kind {
            // Program structure
            "module" | "program" | "source_file" | "compilation_unit" => NodeType::Program,

            // Function definitions
            "function_definition"
            | "function_declaration"
            | "method_definition"
            | "constructor_definition"
            | "arrow_function"
            | "function_expression" => NodeType::FunctionDeclaration,

            // Function calls
            "call_expression"
            | "call"
            | "method_invocation"
            | "constructor_invocation"
            | "new_expression" => NodeType::CallExpression,

            // Assignments
            "assignment" | "assignment_expression" | "augmented_assignment" => {
                NodeType::AssignmentExpression
            }
            "variable_declaration" | "variable_declarator" => NodeType::VariableDeclaration,

            // Identifiers and names
            "identifier"
            | "field_identifier"
            | "type_identifier"
            | "property_identifier"
            | "variable_name"
            | "function_name"
            | "class_name" => NodeType::Identifier,

            // Literals
            "string" | "string_literal" | "template_string" | "raw_string"
            | "character_literal" | "escape_sequence" => NodeType::Literal,
            "integer"
            | "number"
            | "integer_literal"
            | "float_literal"
            | "decimal_integer_literal"
            | "hex_integer_literal"
            | "binary_integer_literal"
            | "octal_integer_literal" => NodeType::Literal,
            "boolean" | "true" | "false" | "null" | "undefined" | "none" => NodeType::Literal,

            // Control flow
            "if_statement" | "conditional_expression" | "ternary_expression" => {
                NodeType::IfStatement
            }
            "while_statement" | "do_statement" => NodeType::WhileStatement,
            "for_statement"
            | "for_in_statement"
            | "for_of_statement"
            | "enhanced_for_statement" => NodeType::ForStatement,
            "return_statement" => NodeType::ReturnStatement,
            "break_statement" => NodeType::BreakStatement,
            "continue_statement" => NodeType::ContinueStatement,
            "throw_statement" => NodeType::ThrowStatement,
            "try_statement" | "catch_clause" | "finally_clause" => NodeType::TryStatement,

            // Expressions
            "expression_statement" => NodeType::ExpressionStatement,
            "binary_expression" | "logical_expression" | "comparison_expression" => {
                NodeType::BinaryExpression
            }
            "unary_expression" | "update_expression" => NodeType::UnaryExpression,
            "member_expression" | "subscript_expression" | "attribute" | "field_access" => {
                NodeType::MemberExpression
            }
            "array" | "array_literal" | "list" | "tuple" => NodeType::ArrayExpression,
            "object" | "object_literal" | "dictionary" | "hash" => NodeType::ObjectExpression,

            // Blocks and statements
            "block" | "block_statement" | "compound_statement" | "suite" => {
                NodeType::BlockStatement
            }
            "class_declaration" | "class_definition" => NodeType::ClassDeclaration,
            "interface_declaration" => NodeType::InterfaceDeclaration,
            "import_statement" | "import_declaration" | "from_import" | "include" => {
                NodeType::ImportDeclaration
            }
            "export_statement" | "export_declaration" => NodeType::ExportDeclaration,

            // Comments and documentation
            "comment" | "line_comment" | "block_comment" | "documentation_comment" => {
                NodeType::Comment
            }

            // Language-specific constructs
            "lambda" | "lambda_expression" => NodeType::LambdaExpression,

            // Switch statements
            "switch_statement" => NodeType::SwitchStatement,
            "case_statement" | "case_clause" => NodeType::CaseStatement,

            // Bash-specific constructs
            "command" | "simple_command" | "pipeline" | "command_substitution" => {
                NodeType::CallExpression
            }
            "variable_assignment" => NodeType::AssignmentExpression,
            "word" | "command_name" => NodeType::Identifier,
            "ansi_c_quoting" | "quoted_string" => NodeType::Literal,
            "expansion" | "process_substitution" => NodeType::CallExpression,
            "if_statement" | "while_statement" | "for_statement" | "case_statement" => {
                NodeType::ControlFlowStatement
            }
            "function_definition" => NodeType::FunctionDeclaration,
            "subshell" => NodeType::BlockStatement,
            "test_command" | "test_operator" => NodeType::BinaryExpression,
            "redirected_statement" | "file_redirect" => NodeType::ExpressionStatement,

            // SQL-specific constructs (align with manual SQL adapter node types)
            "select_statement" => NodeType::SelectStatement,
            "insert_statement" => NodeType::InsertStatement,
            "update_statement" => NodeType::UpdateStatement,
            "delete_statement" => NodeType::DeleteStatement,
            "create_statement" => NodeType::CreateStatement,
            "create_sequence" => NodeType::CreateSequenceStatement,
            "drop_statement" => NodeType::DropStatement,
            "alter_statement" => NodeType::AlterStatement,
            // SQL clauses map to a generic SqlExpression container
            "from_clause"
            | "where_clause"
            | "having_clause"
            | "order_by_clause"
            | "group_by_clause"
            | "limit_clause"
            | "join_clause"
            | "inner_join"
            | "left_join"
            | "right_join"
            | "full_join"
            | "subquery"
            | "parenthesized_expression" => NodeType::SqlExpression,
            // Common SQL tokens
            "column_reference" | "table_reference" | "field" => NodeType::Identifier,
            "function_call" | "aggregate_function" => NodeType::CallExpression,
            "comparison_predicate"
            | "in_predicate"
            | "like_predicate"
            | "between_predicate"
            | "union"
            | "intersect"
            | "except" => NodeType::BinaryExpression,
            "literal" | "number_literal" | "boolean_literal" => NodeType::Literal,

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
            } else if ts_kind.contains("member")
                || ts_kind.contains("attribute")
                || ts_kind.contains("field_access")
            {
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
}
