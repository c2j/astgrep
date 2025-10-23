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
    CreateStatement,
    CreateTableStatement,
    CreateIndexStatement,
    CreateViewStatement,
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
            NodeType::CreateStatement => "create_statement",
            NodeType::CreateTableStatement => "create_table_statement",
            NodeType::CreateIndexStatement => "create_index_statement",
            NodeType::CreateViewStatement => "create_view_statement",
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

    pub fn from_str(s: &str) -> Option<Self> {
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
            "create_statement" => Some(NodeType::CreateStatement),
            "create_table_statement" => Some(NodeType::CreateTableStatement),
            "create_index_statement" => Some(NodeType::CreateIndexStatement),
            "create_view_statement" => Some(NodeType::CreateViewStatement),
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
        matches!(self,
            NodeType::BinaryExpression |
            NodeType::UnaryExpression |
            NodeType::CallExpression |
            NodeType::MemberExpression |
            NodeType::AssignmentExpression |
            NodeType::ConditionalExpression |
            NodeType::ArrayExpression |
            NodeType::ObjectExpression |
            NodeType::LambdaExpression |
            NodeType::Identifier |
            NodeType::Literal
        )
    }

    /// Check if this node type is a statement
    pub fn is_statement(&self) -> bool {
        matches!(self,
            NodeType::ExpressionStatement |
            NodeType::DeclarationStatement |
            NodeType::ControlFlowStatement |
            NodeType::ReturnStatement |
            NodeType::BlockStatement |
            NodeType::BreakStatement |
            NodeType::ContinueStatement |
            NodeType::ThrowStatement |
            NodeType::TryStatement |
            NodeType::IfStatement |
            NodeType::WhileStatement |
            NodeType::ForStatement |
            NodeType::SwitchStatement |
            NodeType::CaseStatement
        )
    }

    /// Check if this node type is a declaration
    pub fn is_declaration(&self) -> bool {
        matches!(self,
            NodeType::FunctionDeclaration |
            NodeType::VariableDeclaration |
            NodeType::ClassDeclaration |
            NodeType::ImportDeclaration |
            NodeType::ExportDeclaration |
            NodeType::InterfaceDeclaration
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
    Add, Subtract, Multiply, Divide, Modulo, Power,
    // Comparison
    Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual,
    // Logical
    And, Or,
    // Bitwise
    BitwiseAnd, BitwiseOr, BitwiseXor, LeftShift, RightShift,
    // Assignment
    Assign, AddAssign, SubtractAssign, MultiplyAssign, DivideAssign,
    // Other
    In, InstanceOf, Typeof,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOperator {
    Plus, Minus, Not, BitwiseNot, Typeof, Void, Delete,
    PreIncrement, PostIncrement, PreDecrement, PostDecrement,
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

    pub fn with_location(mut self, start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
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
        let current_interfaces = node.attributes.get("interfaces").cloned().unwrap_or_default();
        let new_interfaces = if current_interfaces.is_empty() {
            interface
        } else {
            format!("{},{}", current_interfaces, interface)
        };
        node.attributes.insert("interfaces".to_string(), new_interfaces);
        node
    }

    /// Add a specifier (for import/export)
    pub fn with_specifier(self, specifier: String) -> Self {
        let mut node = self;
        let current_specifiers = node.attributes.get("specifiers").cloned().unwrap_or_default();
        let new_specifiers = if current_specifiers.is_empty() {
            specifier
        } else {
            format!("{},{}", current_specifiers, specifier)
        };
        node.attributes.insert("specifiers".to_string(), new_specifiers);
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
        let current_params = node.attributes.get("parameters").cloned().unwrap_or_default();
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
        let current_assignments = node.attributes.get("assignments").cloned().unwrap_or_default();
        let new_assignments = if current_assignments.is_empty() {
            assignment
        } else {
            format!("{},{}", current_assignments, assignment)
        };
        node.attributes.insert("assignments".to_string(), new_assignments);
        node
    }

    /// Add a column definition (for CREATE TABLE)
    pub fn with_column_definition(self, definition: String) -> Self {
        let mut node = self;
        let current_defs = node.attributes.get("column_definitions").cloned().unwrap_or_default();
        let new_defs = if current_defs.is_empty() {
            definition
        } else {
            format!("{},{}", current_defs, definition)
        };
        node.attributes.insert("column_definitions".to_string(), new_defs);
        node
    }

    // Bash-specific methods

    /// Add an argument (for commands)
    pub fn with_argument(self, argument: String) -> Self {
        let mut node = self;
        let current_args = node.attributes.get("arguments").cloned().unwrap_or_default();
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
        assert_eq!(NodeType::FunctionDeclaration.as_str(), "function_declaration");

        assert_eq!(NodeType::from_str("identifier"), Some(NodeType::Identifier));
        assert_eq!(NodeType::from_str("binary_expression"), Some(NodeType::BinaryExpression));
        assert_eq!(NodeType::from_str("unknown"), None);
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
        assert_eq!(LiteralValue::String("hello".to_string()).to_string(), "\"hello\"");
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
        let child1 = UniversalNode::new(NodeType::Identifier)
            .with_identifier("left".to_string());
        let child2 = UniversalNode::new(NodeType::Identifier)
            .with_identifier("right".to_string());

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
        assert_eq!(node.literal(), Some(&LiteralValue::String("hello world".to_string())));
    }
}
