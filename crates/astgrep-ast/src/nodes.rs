//! Universal AST node definitions
//!
//! This module defines the universal AST node types that can represent
//! constructs from all supported programming languages.

use astgrep_core::AstNode;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Universal AST node types based on the design document
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // Basic nodes
    Identifier,
    Literal,
    Comment,

    // Expression nodes
    BinaryExpression,
    UnaryExpression,
    CallExpression,
    MemberExpression,
    AssignmentExpression,
    ConditionalExpression,
    ArrayExpression,
    ObjectExpression,
    LambdaExpression,

    // Statement nodes
    ExpressionStatement,
    DeclarationStatement,
    ControlFlowStatement,
    ReturnStatement,
    BlockStatement,
    BreakStatement,
    ContinueStatement,
    ThrowStatement,
    TryStatement,

    // Declaration nodes
    FunctionDeclaration,
    VariableDeclaration,
    ClassDeclaration,
    ImportDeclaration,
    ExportDeclaration,
    InterfaceDeclaration,

    // Control flow nodes
    IfStatement,
    WhileStatement,
    ForStatement,
    SwitchStatement,
    CaseStatement,

    // Special language-specific nodes
    SqlQuery,
    SqlProcedure,
    ShellCommand,

    // Generic container nodes
    Program,
    Module,
    Package,

    // Additional language-specific nodes
    PackageDeclaration,
    FieldDeclaration,
    MethodDeclaration,
    ArrowFunction,
    Decorator,
    ElifStatement,
    ElseStatement,
    ExceptStatement,
    FinallyStatement,

    // Unknown or unrecognized nodes
    Unknown,

    // Additional JavaScript-specific nodes
    TemplateString,

    // SQL-specific nodes
    SqlExpression,
    SelectStatement,
    InsertStatement,
    UpdateStatement,
    DeleteStatement,
    MergeStatement,
    CreateStatement,
    CreateTableStatement,
    CreateIndexStatement,
    CreateViewStatement,
    CreateSequenceStatement,
    CreateFunctionStatement,
    CreateProcedureStatement,
    CreatePackageStatement,
    DropStatement,
    AlterStatement,

    // Bash-specific nodes
    Shebang,
    ExportStatement,
    SourceStatement,
    Command,

    // Additional literals
    StringLiteral,
    IntegerLiteral,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Identifier => "identifier",
            NodeType::Literal => "literal",
            NodeType::Comment => "comment",
            NodeType::BinaryExpression => "binary_expression",
            NodeType::UnaryExpression => "unary_expression",
            NodeType::CallExpression => "call_expression",
            NodeType::MemberExpression => "member_expression",
            NodeType::AssignmentExpression => "assignment_expression",
            NodeType::ConditionalExpression => "conditional_expression",
            NodeType::ArrayExpression => "array_expression",
            NodeType::ObjectExpression => "object_expression",
            NodeType::LambdaExpression => "lambda_expression",
            NodeType::ExpressionStatement => "expression_statement",
            NodeType::DeclarationStatement => "declaration_statement",
            NodeType::ControlFlowStatement => "control_flow_statement",
            NodeType::ReturnStatement => "return_statement",
            NodeType::BlockStatement => "block_statement",
            NodeType::BreakStatement => "break_statement",
            NodeType::ContinueStatement => "continue_statement",
            NodeType::ThrowStatement => "throw_statement",
            NodeType::TryStatement => "try_statement",
            NodeType::FunctionDeclaration => "function_declaration",
            NodeType::VariableDeclaration => "variable_declaration",
            NodeType::ClassDeclaration => "class_declaration",
            NodeType::ImportDeclaration => "import_declaration",
            NodeType::ExportDeclaration => "export_declaration",
            NodeType::InterfaceDeclaration => "interface_declaration",
            NodeType::IfStatement => "if_statement",
            NodeType::WhileStatement => "while_statement",
            NodeType::ForStatement => "for_statement",
            NodeType::SwitchStatement => "switch_statement",
            NodeType::CaseStatement => "case_statement",
            NodeType::SqlQuery => "sql_query",
            NodeType::SqlProcedure => "sql_procedure",
            NodeType::ShellCommand => "shell_command",
            NodeType::Program => "program",
            NodeType::Module => "module",
            NodeType::Package => "package",
            NodeType::PackageDeclaration => "package_declaration",
            NodeType::FieldDeclaration => "field_declaration",
            NodeType::MethodDeclaration => "method_declaration",
            NodeType::ArrowFunction => "arrow_function",
            NodeType::Decorator => "decorator",
            NodeType::ElifStatement => "elif_statement",
            NodeType::ElseStatement => "else_statement",
            NodeType::ExceptStatement => "except_statement",
            NodeType::FinallyStatement => "finally_statement",
            NodeType::SqlExpression => "sql_expression",
            NodeType::SelectStatement => "select_statement",
            NodeType::InsertStatement => "insert_statement",
            NodeType::UpdateStatement => "update_statement",
            NodeType::DeleteStatement => "delete_statement",
            NodeType::MergeStatement => "merge_statement",
            NodeType::CreateStatement => "create_statement",
            NodeType::CreateTableStatement => "create_table_statement",
            NodeType::CreateIndexStatement => "create_index_statement",
            NodeType::CreateViewStatement => "create_view_statement",
            NodeType::CreateSequenceStatement => "create_sequence_statement",
            NodeType::CreateFunctionStatement => "create_function_statement",
            NodeType::CreateProcedureStatement => "create_procedure_statement",
            NodeType::CreatePackageStatement => "create_package_statement",
            NodeType::DropStatement => "drop_statement",
            NodeType::AlterStatement => "alter_statement",
            NodeType::Shebang => "shebang",
            NodeType::ExportStatement => "export_statement",
            NodeType::SourceStatement => "source_statement",
            NodeType::Command => "command",
            NodeType::StringLiteral => "string_literal",
            NodeType::IntegerLiteral => "integer_literal",
            NodeType::Unknown => "unknown",
            NodeType::TemplateString => "template_string",
        }
    }

    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "identifier" => Some(NodeType::Identifier),
            "literal" => Some(NodeType::Literal),
            "comment" => Some(NodeType::Comment),
            "binary_expression" => Some(NodeType::BinaryExpression),
            "unary_expression" => Some(NodeType::UnaryExpression),
            "call_expression" => Some(NodeType::CallExpression),
            "member_expression" => Some(NodeType::MemberExpression),
            "assignment_expression" => Some(NodeType::AssignmentExpression),
            "conditional_expression" => Some(NodeType::ConditionalExpression),
            "array_expression" => Some(NodeType::ArrayExpression),
            "object_expression" => Some(NodeType::ObjectExpression),
            "lambda_expression" => Some(NodeType::LambdaExpression),
            "expression_statement" => Some(NodeType::ExpressionStatement),
            "declaration_statement" => Some(NodeType::DeclarationStatement),
            "control_flow_statement" => Some(NodeType::ControlFlowStatement),
            "return_statement" => Some(NodeType::ReturnStatement),
            "block_statement" => Some(NodeType::BlockStatement),
            "break_statement" => Some(NodeType::BreakStatement),
            "continue_statement" => Some(NodeType::ContinueStatement),
            "throw_statement" => Some(NodeType::ThrowStatement),
            "try_statement" => Some(NodeType::TryStatement),
            "function_declaration" => Some(NodeType::FunctionDeclaration),
            "variable_declaration" => Some(NodeType::VariableDeclaration),
            "class_declaration" => Some(NodeType::ClassDeclaration),
            "import_declaration" => Some(NodeType::ImportDeclaration),
            "export_declaration" => Some(NodeType::ExportDeclaration),
            "interface_declaration" => Some(NodeType::InterfaceDeclaration),
            "if_statement" => Some(NodeType::IfStatement),
            "while_statement" => Some(NodeType::WhileStatement),
            "for_statement" => Some(NodeType::ForStatement),
            "switch_statement" => Some(NodeType::SwitchStatement),
            "case_statement" => Some(NodeType::CaseStatement),
            "sql_query" => Some(NodeType::SqlQuery),
            "sql_procedure" => Some(NodeType::SqlProcedure),
            "shell_command" => Some(NodeType::ShellCommand),
            "program" => Some(NodeType::Program),
            "module" => Some(NodeType::Module),
            "package" => Some(NodeType::Package),
            "package_declaration" => Some(NodeType::PackageDeclaration),
            "field_declaration" => Some(NodeType::FieldDeclaration),
            "method_declaration" => Some(NodeType::MethodDeclaration),
            "arrow_function" => Some(NodeType::ArrowFunction),
            "decorator" => Some(NodeType::Decorator),
            "elif_statement" => Some(NodeType::ElifStatement),
            "else_statement" => Some(NodeType::ElseStatement),
            "except_statement" => Some(NodeType::ExceptStatement),
            "finally_statement" => Some(NodeType::FinallyStatement),
            "sql_expression" => Some(NodeType::SqlExpression),
            "select_statement" => Some(NodeType::SelectStatement),
            "insert_statement" => Some(NodeType::InsertStatement),
            "update_statement" => Some(NodeType::UpdateStatement),
            "delete_statement" => Some(NodeType::DeleteStatement),
            "merge_statement" => Some(NodeType::MergeStatement),
            "create_statement" => Some(NodeType::CreateStatement),
            "create_table_statement" => Some(NodeType::CreateTableStatement),
            "create_index_statement" => Some(NodeType::CreateIndexStatement),
            "create_view_statement" => Some(NodeType::CreateViewStatement),
            "create_sequence_statement" => Some(NodeType::CreateSequenceStatement),
            "create_function_statement" => Some(NodeType::CreateFunctionStatement),
            "create_procedure_statement" => Some(NodeType::CreateProcedureStatement),
            "create_package_statement" => Some(NodeType::CreatePackageStatement),
            "drop_statement" => Some(NodeType::DropStatement),
            "alter_statement" => Some(NodeType::AlterStatement),
            "shebang" => Some(NodeType::Shebang),
            "export_statement" => Some(NodeType::ExportStatement),
            "source_statement" => Some(NodeType::SourceStatement),
            "command" => Some(NodeType::Command),
            "string_literal" => Some(NodeType::StringLiteral),
            "integer_literal" => Some(NodeType::IntegerLiteral),
            "unknown" => Some(NodeType::Unknown),
            "template_string" => Some(NodeType::TemplateString),
            _ => None,
        }
    }

    /// Check if this node type is an expression
    pub fn is_expression(&self) -> bool {
        matches!(
            self,
            NodeType::BinaryExpression
                | NodeType::UnaryExpression
                | NodeType::CallExpression
                | NodeType::MemberExpression
                | NodeType::AssignmentExpression
                | NodeType::ConditionalExpression
                | NodeType::ArrayExpression
                | NodeType::ObjectExpression
                | NodeType::LambdaExpression
                | NodeType::Identifier
                | NodeType::Literal
        )
    }

    /// Check if this node type is a statement
    pub fn is_statement(&self) -> bool {
        matches!(
            self,
            NodeType::ExpressionStatement
                | NodeType::DeclarationStatement
                | NodeType::ControlFlowStatement
                | NodeType::ReturnStatement
                | NodeType::BlockStatement
                | NodeType::BreakStatement
                | NodeType::ContinueStatement
                | NodeType::ThrowStatement
                | NodeType::TryStatement
                | NodeType::IfStatement
                | NodeType::WhileStatement
                | NodeType::ForStatement
                | NodeType::SwitchStatement
                | NodeType::CaseStatement
        )
    }

    /// Check if this node type is a declaration
    pub fn is_declaration(&self) -> bool {
        matches!(
            self,
            NodeType::FunctionDeclaration
                | NodeType::VariableDeclaration
                | NodeType::ClassDeclaration
                | NodeType::ImportDeclaration
                | NodeType::ExportDeclaration
                | NodeType::InterfaceDeclaration
        )
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Literal value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Null,
    Undefined,
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiteralValue::String(s) => write!(f, "\"{}\"", s),
            LiteralValue::Number(n) => write!(f, "{}", n),
            LiteralValue::Integer(i) => write!(f, "{}", i),
            LiteralValue::Boolean(b) => write!(f, "{}", b),
            LiteralValue::Null => write!(f, "null"),
            LiteralValue::Undefined => write!(f, "undefined"),
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    // Logical
    And,
    Or,
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    // Assignment
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    // Other
    In,
    InstanceOf,
    Typeof,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
    BitwiseNot,
    Typeof,
    Void,
    Delete,
    PreIncrement,
    PostIncrement,
    PreDecrement,
    PostDecrement,
}

/// Universal AST node implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalNode {
    pub node_type: NodeType,
    pub children: Vec<UniversalNode>,
    pub location: Option<(usize, usize, usize, usize)>, // (start_line, start_col, end_line, end_col)
    pub text: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,

    // Node-specific data
    pub literal_value: Option<LiteralValue>,
    pub binary_operator: Option<BinaryOperator>,
    pub unary_operator: Option<UnaryOperator>,
    pub identifier_name: Option<String>,
}

impl UniversalNode {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            children: Vec::new(),
            location: None,
            text: None,
            attributes: std::collections::HashMap::new(),
            literal_value: None,
            binary_operator: None,
            unary_operator: None,
            identifier_name: None,
        }
    }

    pub fn with_location(
        mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Self {
        self.location = Some((start_line, start_col, end_line, end_col));
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    pub fn add_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    pub fn children_mut(&mut self) -> &mut Vec<UniversalNode> {
        &mut self.children
    }

    pub fn node_type_mut(&mut self) -> &mut NodeType {
        &mut self.node_type
    }

    pub fn with_literal(mut self, value: LiteralValue) -> Self {
        self.literal_value = Some(value);
        self
    }

    pub fn with_binary_operator(mut self, op: BinaryOperator) -> Self {
        self.binary_operator = Some(op);
        self
    }

    pub fn with_unary_operator(mut self, op: UnaryOperator) -> Self {
        self.unary_operator = Some(op);
        self
    }

    pub fn with_identifier(mut self, name: String) -> Self {
        self.identifier_name = Some(name);
        self
    }

    pub fn add_child(mut self, child: UniversalNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn add_children(mut self, children: Vec<UniversalNode>) -> Self {
        self.children.extend(children);
        self
    }

    // Additional builder methods for language-specific features

    /// Add a modifier (for Java/C# access modifiers, etc.)
    pub fn with_modifier(self, modifier: &str) -> Self {
        self.with_attribute("modifier".to_string(), modifier.to_string())
    }

    /// Add a parent class (for inheritance)
    pub fn with_parent(self, parent: String) -> Self {
        self.with_attribute("parent".to_string(), parent)
    }

    /// Add an interface (for Java implements)
    pub fn with_interface(self, interface: String) -> Self {
        let mut node = self;
        let current_interfaces = node
            .attributes
            .get("interfaces")
            .cloned()
            .unwrap_or_default();
        let new_interfaces = if current_interfaces.is_empty() {
            interface
        } else {
            format!("{},{}", current_interfaces, interface)
        };
        node.attributes
            .insert("interfaces".to_string(), new_interfaces);
        node
    }

    /// Add a specifier (for import/export)
    pub fn with_specifier(self, specifier: String) -> Self {
        let mut node = self;
        let current_specifiers = node
            .attributes
            .get("specifiers")
            .cloned()
            .unwrap_or_default();
        let new_specifiers = if current_specifiers.is_empty() {
            specifier
        } else {
            format!("{},{}", current_specifiers, specifier)
        };
        node.attributes
            .insert("specifiers".to_string(), new_specifiers);
        node
    }

    /// Add a namespace (for import * as name)
    pub fn with_namespace(self, namespace: String) -> Self {
        self.with_attribute("namespace".to_string(), namespace)
    }

    /// Add a default import/export
    pub fn with_default(self, default: String) -> Self {
        self.with_attribute("default".to_string(), default)
    }

    /// Add a wildcard flag
    pub fn with_wildcard(self, wildcard: bool) -> Self {
        self.with_attribute("wildcard".to_string(), wildcard.to_string())
    }

    /// Add an alias (for import as)
    pub fn with_alias(self, original: String, alias: String) -> Self {
        self.with_attribute("original".to_string(), original)
            .with_attribute("alias".to_string(), alias)
    }

    /// Add a module (for import)
    pub fn with_module(self, module: String) -> Self {
        self.with_attribute("module".to_string(), module)
    }

    /// Add a decorator flag
    pub fn with_decorator(self, decorator: &str) -> Self {
        self.with_attribute("decorator".to_string(), decorator.to_string())
    }

    /// Add a parameter (for functions)
    pub fn with_parameter(self, parameter: String) -> Self {
        let mut node = self;
        let current_params = node
            .attributes
            .get("parameters")
            .cloned()
            .unwrap_or_default();
        let new_params = if current_params.is_empty() {
            parameter
        } else {
            format!("{},{}", current_params, parameter)
        };
        node.attributes.insert("parameters".to_string(), new_params);
        node
    }

    // SQL-specific methods

    /// Add a column (for SELECT)
    pub fn with_column(self, column: String) -> Self {
        let mut node = self;
        let current_columns = node.attributes.get("columns").cloned().unwrap_or_default();
        let new_columns = if current_columns.is_empty() {
            column
        } else {
            format!("{},{}", current_columns, column)
        };
        node.attributes.insert("columns".to_string(), new_columns);
        node
    }

    /// Add a table (for FROM)
    pub fn with_table(self, table: String) -> Self {
        self.with_attribute("table".to_string(), table)
    }

    /// Add a WHERE clause
    pub fn with_where(self, condition: String) -> Self {
        self.with_attribute("where".to_string(), condition)
    }

    /// Add an assignment (for UPDATE SET)
    pub fn with_assignment(self, assignment: String) -> Self {
        let mut node = self;
        let current_assignments = node
            .attributes
            .get("assignments")
            .cloned()
            .unwrap_or_default();
        let new_assignments = if current_assignments.is_empty() {
            assignment
        } else {
            format!("{},{}", current_assignments, assignment)
        };
        node.attributes
            .insert("assignments".to_string(), new_assignments);
        node
    }

    /// Add a column definition (for CREATE TABLE)
    pub fn with_column_definition(self, definition: String) -> Self {
        let mut node = self;
        let current_defs = node
            .attributes
            .get("column_definitions")
            .cloned()
            .unwrap_or_default();
        let new_defs = if current_defs.is_empty() {
            definition
        } else {
            format!("{},{}", current_defs, definition)
        };
        node.attributes
            .insert("column_definitions".to_string(), new_defs);
        node
    }

    /// Add a sequence name (for CREATE SEQUENCE)
    pub fn with_sequence_name(self, sequence_name: String) -> Self {
        self.with_attribute("sequence_name".to_string(), sequence_name)
    }

    // Bash-specific methods

    /// Add an argument (for commands)
    pub fn with_argument(self, argument: String) -> Self {
        let mut node = self;
        let current_args = node
            .attributes
            .get("arguments")
            .cloned()
            .unwrap_or_default();
        let new_args = if current_args.is_empty() {
            argument
        } else {
            format!("{},{}", current_args, argument)
        };
        node.attributes.insert("arguments".to_string(), new_args);
        node
    }

    /// Add a pipe flag
    pub fn with_pipe(self, has_pipe: bool) -> Self {
        self.with_attribute("pipe".to_string(), has_pipe.to_string())
    }

    /// Add a redirection flag
    pub fn with_redirection(self, has_redirection: bool) -> Self {
        self.with_attribute("redirection".to_string(), has_redirection.to_string())
    }

    /// Add a value (for variable assignments)
    pub fn with_value(self, value: String) -> Self {
        self.with_attribute("value".to_string(), value)
    }

    /// Get attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Check if node has a specific attribute
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get the identifier name if this is an identifier node
    pub fn identifier(&self) -> Option<&String> {
        self.identifier_name.as_ref()
    }

    /// Get the literal value if this is a literal node
    pub fn literal(&self) -> Option<&LiteralValue> {
        self.literal_value.as_ref()
    }
}

impl AstNode for UniversalNode {
    fn node_type(&self) -> &str {
        self.node_type.as_str()
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn child(&self, index: usize) -> Option<&dyn AstNode> {
        self.children.get(index).map(|c| c as &dyn AstNode)
    }

    fn location(&self) -> Option<(usize, usize, usize, usize)> {
        self.location
    }

    fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    fn get_attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    fn clone_node(&self) -> Box<dyn AstNode> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[test]
    fn test_node_type_string_conversion() {
        assert_eq!(NodeType::Identifier.as_str(), "identifier");
        assert_eq!(NodeType::BinaryExpression.as_str(), "binary_expression");
        assert_eq!(
            NodeType::FunctionDeclaration.as_str(),
            "function_declaration"
        );

        assert_eq!(
            NodeType::parse_name("identifier"),
            Some(NodeType::Identifier)
        );
        assert_eq!(
            NodeType::parse_name("binary_expression"),
            Some(NodeType::BinaryExpression)
        );
        assert_eq!(NodeType::parse_name("not_a_type"), None);
    }

    #[test]
    fn test_node_type_categories() {
        assert!(NodeType::BinaryExpression.is_expression());
        assert!(NodeType::Identifier.is_expression());
        assert!(!NodeType::IfStatement.is_expression());

        assert!(NodeType::IfStatement.is_statement());
        assert!(NodeType::ReturnStatement.is_statement());
        assert!(!NodeType::Identifier.is_statement());

        assert!(NodeType::FunctionDeclaration.is_declaration());
        assert!(NodeType::VariableDeclaration.is_declaration());
        assert!(!NodeType::Identifier.is_declaration());
    }

    #[test]
    fn test_literal_value_display() {
        assert_eq!(
            LiteralValue::String("hello".to_string()).to_string(),
            "\"hello\""
        );
        assert_eq!(LiteralValue::Number(42.5).to_string(), "42.5");
        assert_eq!(LiteralValue::Integer(42).to_string(), "42");
        assert_eq!(LiteralValue::Boolean(true).to_string(), "true");
        assert_eq!(LiteralValue::Null.to_string(), "null");
        assert_eq!(LiteralValue::Undefined.to_string(), "undefined");
    }

    #[test]
    fn test_universal_node_creation() {
        let node = UniversalNode::new(NodeType::Identifier)
            .with_identifier("test_var".to_string())
            .with_location(1, 5, 1, 13)
            .with_text("test_var".to_string())
            .with_attribute("scope".to_string(), "local".to_string());

        assert_eq!(node.node_type(), "identifier");
        assert_eq!(node.identifier(), Some(&"test_var".to_string()));
        assert_eq!(node.location(), Some((1, 5, 1, 13)));
        assert_eq!(node.text(), Some("test_var"));
        assert_eq!(node.get_attribute("scope"), Some(&"local".to_string()));
        assert!(node.has_attribute("scope"));
        assert!(!node.has_attribute("nonexistent"));
    }

    #[test]
    fn test_universal_node_with_children() {
        let child1 = UniversalNode::new(NodeType::Identifier).with_identifier("left".to_string());
        let child2 = UniversalNode::new(NodeType::Identifier).with_identifier("right".to_string());

        let parent = UniversalNode::new(NodeType::BinaryExpression)
            .with_binary_operator(BinaryOperator::Add)
            .add_child(child1)
            .add_child(child2);

        assert_eq!(parent.child_count(), 2);
        assert!(parent.child(0).is_some());
        assert!(parent.child(1).is_some());
        assert!(parent.child(2).is_none());

        if let Some(first_child) = parent.child(0) {
            assert_eq!(first_child.node_type(), "identifier");
        }
    }

    #[test]
    fn test_literal_node() {
        let node = UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::String("hello world".to_string()));

        assert_eq!(node.node_type(), "literal");
        assert_eq!(
            node.literal(),
            Some(&LiteralValue::String("hello world".to_string()))
        );
    }

    #[test]
    fn test_all_node_type_variants_construct_and_compare() {
        let all = vec![
            NodeType::Identifier,
            NodeType::Literal,
            NodeType::Comment,
            NodeType::BinaryExpression,
            NodeType::UnaryExpression,
            NodeType::CallExpression,
            NodeType::MemberExpression,
            NodeType::AssignmentExpression,
            NodeType::ConditionalExpression,
            NodeType::ArrayExpression,
            NodeType::ObjectExpression,
            NodeType::LambdaExpression,
            NodeType::ExpressionStatement,
            NodeType::DeclarationStatement,
            NodeType::ControlFlowStatement,
            NodeType::ReturnStatement,
            NodeType::BlockStatement,
            NodeType::BreakStatement,
            NodeType::ContinueStatement,
            NodeType::ThrowStatement,
            NodeType::TryStatement,
            NodeType::FunctionDeclaration,
            NodeType::VariableDeclaration,
            NodeType::ClassDeclaration,
            NodeType::ImportDeclaration,
            NodeType::ExportDeclaration,
            NodeType::InterfaceDeclaration,
            NodeType::IfStatement,
            NodeType::WhileStatement,
            NodeType::ForStatement,
            NodeType::SwitchStatement,
            NodeType::CaseStatement,
            NodeType::SqlQuery,
            NodeType::SqlProcedure,
            NodeType::ShellCommand,
            NodeType::Program,
            NodeType::Module,
            NodeType::Package,
            NodeType::PackageDeclaration,
            NodeType::FieldDeclaration,
            NodeType::MethodDeclaration,
            NodeType::ArrowFunction,
            NodeType::Decorator,
            NodeType::ElifStatement,
            NodeType::ElseStatement,
            NodeType::ExceptStatement,
            NodeType::FinallyStatement,
            NodeType::Unknown,
            NodeType::TemplateString,
            NodeType::SqlExpression,
            NodeType::SelectStatement,
            NodeType::InsertStatement,
            NodeType::UpdateStatement,
            NodeType::DeleteStatement,
            NodeType::MergeStatement,
            NodeType::CreateStatement,
            NodeType::CreateTableStatement,
            NodeType::CreateIndexStatement,
            NodeType::CreateViewStatement,
            NodeType::CreateSequenceStatement,
            NodeType::CreateFunctionStatement,
            NodeType::CreateProcedureStatement,
            NodeType::CreatePackageStatement,
            NodeType::DropStatement,
            NodeType::AlterStatement,
            NodeType::Shebang,
            NodeType::ExportStatement,
            NodeType::SourceStatement,
            NodeType::Command,
            NodeType::StringLiteral,
            NodeType::IntegerLiteral,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_all_node_type_roundtrip() {
        let all = vec![
            NodeType::Identifier,
            NodeType::Literal,
            NodeType::Comment,
            NodeType::BinaryExpression,
            NodeType::UnaryExpression,
            NodeType::CallExpression,
            NodeType::MemberExpression,
            NodeType::AssignmentExpression,
            NodeType::ConditionalExpression,
            NodeType::ArrayExpression,
            NodeType::ObjectExpression,
            NodeType::LambdaExpression,
            NodeType::ExpressionStatement,
            NodeType::DeclarationStatement,
            NodeType::ControlFlowStatement,
            NodeType::ReturnStatement,
            NodeType::BlockStatement,
            NodeType::BreakStatement,
            NodeType::ContinueStatement,
            NodeType::ThrowStatement,
            NodeType::TryStatement,
            NodeType::FunctionDeclaration,
            NodeType::VariableDeclaration,
            NodeType::ClassDeclaration,
            NodeType::ImportDeclaration,
            NodeType::ExportDeclaration,
            NodeType::InterfaceDeclaration,
            NodeType::IfStatement,
            NodeType::WhileStatement,
            NodeType::ForStatement,
            NodeType::SwitchStatement,
            NodeType::CaseStatement,
            NodeType::SqlQuery,
            NodeType::SqlProcedure,
            NodeType::ShellCommand,
            NodeType::Program,
            NodeType::Module,
            NodeType::Package,
            NodeType::PackageDeclaration,
            NodeType::FieldDeclaration,
            NodeType::MethodDeclaration,
            NodeType::ArrowFunction,
            NodeType::Decorator,
            NodeType::ElifStatement,
            NodeType::ElseStatement,
            NodeType::ExceptStatement,
            NodeType::FinallyStatement,
            NodeType::Unknown,
            NodeType::TemplateString,
            NodeType::SqlExpression,
            NodeType::SelectStatement,
            NodeType::InsertStatement,
            NodeType::UpdateStatement,
            NodeType::DeleteStatement,
            NodeType::MergeStatement,
            NodeType::CreateStatement,
            NodeType::CreateTableStatement,
            NodeType::CreateIndexStatement,
            NodeType::CreateViewStatement,
            NodeType::CreateSequenceStatement,
            NodeType::CreateFunctionStatement,
            NodeType::CreateProcedureStatement,
            NodeType::CreatePackageStatement,
            NodeType::DropStatement,
            NodeType::AlterStatement,
            NodeType::Shebang,
            NodeType::ExportStatement,
            NodeType::SourceStatement,
            NodeType::Command,
            NodeType::StringLiteral,
            NodeType::IntegerLiteral,
        ];
        for nt in &all {
            let s = nt.as_str();
            let parsed = NodeType::parse_name(s);
            assert_eq!(parsed, Some(nt.clone()), "roundtrip failed for {:?}", nt);
        }
        assert_eq!(NodeType::parse_name("not_a_type"), None);
    }

    #[test]
    fn test_node_type_display() {
        assert_eq!(format!("{}", NodeType::Identifier), "identifier");
        assert_eq!(
            format!("{}", NodeType::BinaryExpression),
            "binary_expression"
        );
    }

    #[test]
    fn test_literal_value_all_variants() {
        let s = LiteralValue::String("x".to_string());
        let n = LiteralValue::Number(3.14);
        let i = LiteralValue::Integer(42);
        let b = LiteralValue::Boolean(false);
        let null = LiteralValue::Null;
        let undef = LiteralValue::Undefined;
        assert_ne!(s, n);
        assert_eq!(s, LiteralValue::String("x".to_string()));
        assert_eq!(n, LiteralValue::Number(3.14));
        assert_eq!(i, LiteralValue::Integer(42));
        assert_eq!(b, LiteralValue::Boolean(false));
        assert_eq!(null, LiteralValue::Null);
        assert_eq!(undef, LiteralValue::Undefined);
    }

    #[test]
    fn test_binary_operator_all_variants() {
        let ops = vec![
            BinaryOperator::Add,
            BinaryOperator::Subtract,
            BinaryOperator::Multiply,
            BinaryOperator::Divide,
            BinaryOperator::Modulo,
            BinaryOperator::Power,
            BinaryOperator::Equal,
            BinaryOperator::NotEqual,
            BinaryOperator::LessThan,
            BinaryOperator::LessThanOrEqual,
            BinaryOperator::GreaterThan,
            BinaryOperator::GreaterThanOrEqual,
            BinaryOperator::And,
            BinaryOperator::Or,
            BinaryOperator::BitwiseAnd,
            BinaryOperator::BitwiseOr,
            BinaryOperator::BitwiseXor,
            BinaryOperator::LeftShift,
            BinaryOperator::RightShift,
            BinaryOperator::Assign,
            BinaryOperator::AddAssign,
            BinaryOperator::SubtractAssign,
            BinaryOperator::MultiplyAssign,
            BinaryOperator::DivideAssign,
            BinaryOperator::In,
            BinaryOperator::InstanceOf,
            BinaryOperator::Typeof,
        ];
        for (i, a) in ops.iter().enumerate() {
            for (j, b) in ops.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_unary_operator_all_variants() {
        let ops = vec![
            UnaryOperator::Plus,
            UnaryOperator::Minus,
            UnaryOperator::Not,
            UnaryOperator::BitwiseNot,
            UnaryOperator::Typeof,
            UnaryOperator::Void,
            UnaryOperator::Delete,
            UnaryOperator::PreIncrement,
            UnaryOperator::PostIncrement,
            UnaryOperator::PreDecrement,
            UnaryOperator::PostDecrement,
        ];
        for (i, a) in ops.iter().enumerate() {
            for (j, b) in ops.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_universal_node_with_text_and_location() {
        let node = UniversalNode::new(NodeType::Identifier)
            .with_text("foo".to_string())
            .with_location(2, 3, 2, 6);
        assert_eq!(node.text(), Some("foo"));
        assert_eq!(node.location(), Some((2, 3, 2, 6)));
    }

    #[test]
    fn test_universal_node_add_child_and_counts() {
        let mut parent = UniversalNode::new(NodeType::BlockStatement);
        assert_eq!(parent.child_count(), 0);
        parent = parent.add_child(UniversalNode::new(NodeType::ReturnStatement));
        assert_eq!(parent.child_count(), 1);
        parent = parent.add_child(UniversalNode::new(NodeType::BreakStatement));
        assert_eq!(parent.child_count(), 2);
        let kids: Vec<_> = parent.children.iter().collect();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].node_type(), "return_statement");
        assert_eq!(kids[1].node_type(), "break_statement");
    }

    #[test]
    fn test_universal_node_children_method() {
        let root = UniversalNode::new(NodeType::Program)
            .add_child(UniversalNode::new(NodeType::Identifier))
            .add_child(UniversalNode::new(NodeType::Literal))
            .add_child(UniversalNode::new(NodeType::Comment));
        let types: Vec<_> = root
            .children
            .iter()
            .map(|c| c.node_type().to_string())
            .collect();
        assert_eq!(types, vec!["identifier", "literal", "comment"]);
    }

    #[test]
    fn test_universal_node_parent_child_relationships() {
        let child = UniversalNode::new(NodeType::Identifier).with_identifier("child".to_string());
        let parent = UniversalNode::new(NodeType::BinaryExpression).add_child(child);
        assert_eq!(parent.child_count(), 1);
        assert_eq!(parent.child(0).unwrap().node_type(), "identifier");
        assert!(parent.child(1).is_none());
    }

    #[test]
    fn test_universal_node_node_type_checking_methods() {
        assert!(NodeType::BinaryExpression.is_expression());
        assert!(NodeType::UnaryExpression.is_expression());
        assert!(NodeType::CallExpression.is_expression());
        assert!(NodeType::MemberExpression.is_expression());
        assert!(NodeType::AssignmentExpression.is_expression());
        assert!(NodeType::ConditionalExpression.is_expression());
        assert!(NodeType::ArrayExpression.is_expression());
        assert!(NodeType::ObjectExpression.is_expression());
        assert!(NodeType::LambdaExpression.is_expression());
        assert!(NodeType::Identifier.is_expression());
        assert!(NodeType::Literal.is_expression());
        assert!(!NodeType::IfStatement.is_expression());
        assert!(!NodeType::Program.is_expression());

        assert!(NodeType::ExpressionStatement.is_statement());
        assert!(NodeType::DeclarationStatement.is_statement());
        assert!(NodeType::ControlFlowStatement.is_statement());
        assert!(NodeType::ReturnStatement.is_statement());
        assert!(NodeType::BlockStatement.is_statement());
        assert!(NodeType::BreakStatement.is_statement());
        assert!(NodeType::ContinueStatement.is_statement());
        assert!(NodeType::ThrowStatement.is_statement());
        assert!(NodeType::TryStatement.is_statement());
        assert!(NodeType::IfStatement.is_statement());
        assert!(NodeType::WhileStatement.is_statement());
        assert!(NodeType::ForStatement.is_statement());
        assert!(NodeType::SwitchStatement.is_statement());
        assert!(NodeType::CaseStatement.is_statement());
        assert!(!NodeType::Identifier.is_statement());
        assert!(!NodeType::Program.is_statement());

        assert!(NodeType::FunctionDeclaration.is_declaration());
        assert!(NodeType::VariableDeclaration.is_declaration());
        assert!(NodeType::ClassDeclaration.is_declaration());
        assert!(NodeType::ImportDeclaration.is_declaration());
        assert!(NodeType::ExportDeclaration.is_declaration());
        assert!(NodeType::InterfaceDeclaration.is_declaration());
        assert!(!NodeType::Identifier.is_declaration());
        assert!(!NodeType::Program.is_declaration());
    }

    #[test]
    fn test_universal_node_builder_methods() {
        let node = UniversalNode::new(NodeType::FunctionDeclaration)
            .with_modifier("public")
            .with_parent("BaseClass".to_string())
            .with_interface("Serializable".to_string())
            .with_interface("Cloneable".to_string())
            .with_specifier("foo".to_string())
            .with_specifier("bar".to_string())
            .with_namespace("ns".to_string())
            .with_default("default_export".to_string())
            .with_wildcard(true)
            .with_alias("orig".to_string(), "ali".to_string())
            .with_module("mod".to_string())
            .with_decorator("@Override")
            .with_parameter("a".to_string())
            .with_parameter("b".to_string())
            .with_column("col1".to_string())
            .with_column("col2".to_string())
            .with_table("users".to_string())
            .with_where("id = 1".to_string())
            .with_assignment("x = 1".to_string())
            .with_assignment("y = 2".to_string())
            .with_column_definition("id INT".to_string())
            .with_column_definition("name VARCHAR".to_string())
            .with_sequence_name("seq".to_string())
            .with_argument("arg1".to_string())
            .with_argument("arg2".to_string())
            .with_pipe(true)
            .with_redirection(false)
            .with_value("42".to_string());

        assert_eq!(node.get_attribute("modifier"), Some(&"public".to_string()));
        assert_eq!(node.get_attribute("parent"), Some(&"BaseClass".to_string()));
        assert_eq!(
            node.get_attribute("interfaces"),
            Some(&"Serializable,Cloneable".to_string())
        );
        assert_eq!(
            node.get_attribute("specifiers"),
            Some(&"foo,bar".to_string())
        );
        assert_eq!(node.get_attribute("namespace"), Some(&"ns".to_string()));
        assert_eq!(
            node.get_attribute("default"),
            Some(&"default_export".to_string())
        );
        assert_eq!(node.get_attribute("wildcard"), Some(&"true".to_string()));
        assert_eq!(node.get_attribute("original"), Some(&"orig".to_string()));
        assert_eq!(node.get_attribute("alias"), Some(&"ali".to_string()));
        assert_eq!(node.get_attribute("module"), Some(&"mod".to_string()));
        assert_eq!(
            node.get_attribute("decorator"),
            Some(&"@Override".to_string())
        );
        assert_eq!(node.get_attribute("parameters"), Some(&"a,b".to_string()));
        assert_eq!(
            node.get_attribute("columns"),
            Some(&"col1,col2".to_string())
        );
        assert_eq!(node.get_attribute("table"), Some(&"users".to_string()));
        assert_eq!(node.get_attribute("where"), Some(&"id = 1".to_string()));
        assert_eq!(
            node.get_attribute("assignments"),
            Some(&"x = 1,y = 2".to_string())
        );
        assert_eq!(
            node.get_attribute("column_definitions"),
            Some(&"id INT,name VARCHAR".to_string())
        );
        assert_eq!(
            node.get_attribute("sequence_name"),
            Some(&"seq".to_string())
        );
        assert_eq!(
            node.get_attribute("arguments"),
            Some(&"arg1,arg2".to_string())
        );
        assert_eq!(node.get_attribute("pipe"), Some(&"true".to_string()));
        assert_eq!(
            node.get_attribute("redirection"),
            Some(&"false".to_string())
        );
        assert_eq!(node.get_attribute("value"), Some(&"42".to_string()));
    }

    #[test]
    fn test_universal_node_mut_methods() {
        let mut node = UniversalNode::new(NodeType::Identifier);
        *node.node_type_mut() = NodeType::Literal;
        assert_eq!(node.node_type(), "literal");
        node.children_mut()
            .push(UniversalNode::new(NodeType::Comment));
        assert_eq!(node.child_count(), 1);
    }

    #[test]
    fn test_universal_node_add_attribute_and_metadata() {
        let mut node = UniversalNode::new(NodeType::Identifier);
        node.add_attribute("key1".to_string(), "val1".to_string());
        assert_eq!(node.get_attribute("key1"), Some(&"val1".to_string()));
        let node2 = UniversalNode::new(NodeType::Identifier)
            .with_metadata("key2".to_string(), "val2".to_string());
        assert_eq!(node2.get_attribute("key2"), Some(&"val2".to_string()));
    }

    #[test]
    fn test_universal_node_add_children() {
        let node = UniversalNode::new(NodeType::Program).add_children(vec![
            UniversalNode::new(NodeType::Identifier),
            UniversalNode::new(NodeType::Literal),
        ]);
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn test_universal_node_clone_node() {
        let node = UniversalNode::new(NodeType::Identifier)
            .with_text("clone_me".to_string())
            .with_location(1, 1, 1, 8);
        let cloned = node.clone_node();
        assert_eq!(cloned.node_type(), "identifier");
        assert_eq!(cloned.text(), Some("clone_me"));
        assert_eq!(cloned.location(), Some((1, 1, 1, 8)));
    }

    #[test]
    fn test_universal_node_with_literal_binary_unary_operators() {
        let lit = UniversalNode::new(NodeType::Literal).with_literal(LiteralValue::Integer(7));
        assert_eq!(lit.literal(), Some(&LiteralValue::Integer(7)));

        let bin = UniversalNode::new(NodeType::BinaryExpression)
            .with_binary_operator(BinaryOperator::Multiply);
        assert_eq!(bin.binary_operator, Some(BinaryOperator::Multiply));

        let un =
            UniversalNode::new(NodeType::UnaryExpression).with_unary_operator(UnaryOperator::Not);
        assert_eq!(un.unary_operator, Some(UnaryOperator::Not));
    }

    #[test]
    fn test_node_type_hash_usage() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeType::Identifier);
        set.insert(NodeType::BinaryExpression);
        set.insert(NodeType::Identifier);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&NodeType::Identifier));
        assert!(set.contains(&NodeType::BinaryExpression));
        assert!(!set.contains(&NodeType::Literal));
    }

    #[test]
    fn test_binary_operator_hash_usage() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BinaryOperator::Add);
        set.insert(BinaryOperator::Equal);
        set.insert(BinaryOperator::Add);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&BinaryOperator::Add));
        assert!(set.contains(&BinaryOperator::Equal));
        assert!(!set.contains(&BinaryOperator::Subtract));
    }

    #[test]
    fn test_unary_operator_hash_usage() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(UnaryOperator::Not);
        set.insert(UnaryOperator::Minus);
        set.insert(UnaryOperator::Not);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&UnaryOperator::Not));
        assert!(set.contains(&UnaryOperator::Minus));
        assert!(!set.contains(&UnaryOperator::Plus));
    }

    #[test]
    fn test_universal_node_clone_equality() {
        let original = UniversalNode::new(NodeType::FunctionDeclaration)
            .with_identifier("foo".to_string())
            .with_location(1, 1, 10, 2)
            .with_text("function foo() {}".to_string())
            .with_attribute("scope".to_string(), "global".to_string())
            .add_child(UniversalNode::new(NodeType::Identifier).with_identifier("x".to_string()));
        let cloned = original.clone();
        assert_eq!(cloned.node_type, original.node_type);
        assert_eq!(cloned.identifier_name, original.identifier_name);
        assert_eq!(cloned.location, original.location);
        assert_eq!(cloned.text, original.text);
        assert_eq!(cloned.child_count(), original.child_count());
        assert_eq!(cloned.attributes, original.attributes);
    }

    #[test]
    fn test_literal_value_number_equality() {
        assert_eq!(LiteralValue::Number(1.0), LiteralValue::Number(1.0));
        assert_ne!(LiteralValue::Number(1.0), LiteralValue::Number(2.0));
    }
}
