//! AST builder utilities
//! 
//! This module provides convenient builders for creating AST nodes.

use crate::nodes::{BinaryOperator, LiteralValue, NodeType, UnaryOperator, UniversalNode};
use astgrep_core::Result;

/// Builder for creating AST nodes with a fluent interface
pub struct AstBuilder;

impl AstBuilder {
    /// Create a new identifier node
    pub fn identifier(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::Identifier)
            .with_identifier(name.to_string())
            .with_text(name.to_string())
    }

    /// Create a string literal node
    pub fn string_literal(value: &str) -> UniversalNode {
        UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::String(value.to_string()))
            .with_text(format!("\"{}\"", value))
    }

    /// Create a number literal node
    pub fn number_literal(value: f64) -> UniversalNode {
        UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::Number(value))
            .with_text(value.to_string())
    }

    /// Create an integer literal node
    pub fn integer_literal(value: i64) -> UniversalNode {
        UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::Integer(value))
            .with_text(value.to_string())
    }

    /// Create a boolean literal node
    pub fn boolean_literal(value: bool) -> UniversalNode {
        UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::Boolean(value))
            .with_text(value.to_string())
    }

    /// Create a null literal node
    pub fn null_literal() -> UniversalNode {
        UniversalNode::new(NodeType::Literal)
            .with_literal(LiteralValue::Null)
            .with_text("null".to_string())
    }

    /// Create a binary expression node
    pub fn binary_expression(
        operator: BinaryOperator,
        left: UniversalNode,
        right: UniversalNode,
    ) -> UniversalNode {
        UniversalNode::new(NodeType::BinaryExpression)
            .with_binary_operator(operator)
            .add_child(left)
            .add_child(right)
    }

    /// Create a unary expression node
    pub fn unary_expression(operator: UnaryOperator, operand: UniversalNode) -> UniversalNode {
        UniversalNode::new(NodeType::UnaryExpression)
            .with_unary_operator(operator)
            .add_child(operand)
    }

    /// Create a call expression node
    pub fn call_expression(callee: UniversalNode, arguments: Vec<UniversalNode>) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::CallExpression).add_child(callee);
        for arg in arguments {
            node = node.add_child(arg);
        }
        node
    }

    /// Create a member expression node (e.g., obj.prop)
    pub fn member_expression(object: UniversalNode, property: UniversalNode) -> UniversalNode {
        UniversalNode::new(NodeType::MemberExpression)
            .add_child(object)
            .add_child(property)
    }

    /// Create an assignment expression node
    pub fn assignment_expression(left: UniversalNode, right: UniversalNode) -> UniversalNode {
        UniversalNode::new(NodeType::AssignmentExpression)
            .add_child(left)
            .add_child(right)
    }

    /// Create a function declaration node
    pub fn function_declaration(
        name: &str,
        parameters: Vec<UniversalNode>,
        body: UniversalNode,
    ) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::FunctionDeclaration)
            .with_identifier(name.to_string());
        
        for param in parameters {
            node = node.add_child(param);
        }
        node.add_child(body)
    }

    /// Create a simple function declaration (for parsers)
    pub fn simple_function_declaration(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::FunctionDeclaration)
            .with_identifier(name.to_string())
    }

    /// Create a simple class declaration (for parsers)
    pub fn simple_class_declaration(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::ClassDeclaration)
            .with_identifier(name.to_string())
    }

    /// Create a simple if statement (for parsers)
    pub fn simple_if_statement(condition: &str) -> UniversalNode {
        UniversalNode::new(NodeType::IfStatement)
            .with_attribute("condition".to_string(), condition.to_string())
    }

    /// Create a simple for statement (for parsers)
    pub fn simple_for_statement(loop_spec: &str) -> UniversalNode {
        UniversalNode::new(NodeType::ForStatement)
            .with_attribute("loop_spec".to_string(), loop_spec.to_string())
    }

    /// Create a simple while statement (for parsers)
    pub fn simple_while_statement(condition: &str) -> UniversalNode {
        UniversalNode::new(NodeType::WhileStatement)
            .with_attribute("condition".to_string(), condition.to_string())
    }

    /// Create a variable declaration node
    pub fn variable_declaration(name: &str, initializer: Option<UniversalNode>) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::VariableDeclaration)
            .with_identifier(name.to_string());
        
        if let Some(init) = initializer {
            node = node.add_child(init);
        }
        node
    }

    /// Create a class declaration node
    pub fn class_declaration(
        name: &str,
        superclass: Option<UniversalNode>,
        body: Vec<UniversalNode>,
    ) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::ClassDeclaration)
            .with_identifier(name.to_string());
        
        if let Some(super_class) = superclass {
            node = node.add_child(super_class);
        }
        
        for member in body {
            node = node.add_child(member);
        }
        node
    }

    /// Create an if statement node
    pub fn if_statement(
        condition: UniversalNode,
        then_branch: UniversalNode,
        else_branch: Option<UniversalNode>,
    ) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::IfStatement)
            .add_child(condition)
            .add_child(then_branch);
        
        if let Some(else_stmt) = else_branch {
            node = node.add_child(else_stmt);
        }
        node
    }

    /// Create a while statement node
    pub fn while_statement(condition: UniversalNode, body: UniversalNode) -> UniversalNode {
        UniversalNode::new(NodeType::WhileStatement)
            .add_child(condition)
            .add_child(body)
    }

    /// Create a for statement node
    pub fn for_statement(
        init: Option<UniversalNode>,
        condition: Option<UniversalNode>,
        update: Option<UniversalNode>,
        body: UniversalNode,
    ) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::ForStatement);
        
        if let Some(init_stmt) = init {
            node = node.add_child(init_stmt);
        }
        if let Some(cond) = condition {
            node = node.add_child(cond);
        }
        if let Some(upd) = update {
            node = node.add_child(upd);
        }
        node.add_child(body)
    }

    /// Create a return statement node
    pub fn return_statement(value: Option<UniversalNode>) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::ReturnStatement);
        if let Some(val) = value {
            node = node.add_child(val);
        }
        node
    }

    /// Create a block statement node
    pub fn block_statement(statements: Vec<UniversalNode>) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::BlockStatement);
        for stmt in statements {
            node = node.add_child(stmt);
        }
        node
    }

    /// Create an expression statement node
    pub fn expression_statement(expression: UniversalNode) -> UniversalNode {
        UniversalNode::new(NodeType::ExpressionStatement)
            .add_child(expression)
    }

    /// Create a program/module root node
    pub fn program(statements: Vec<UniversalNode>) -> UniversalNode {
        let mut node = UniversalNode::new(NodeType::Program);
        for stmt in statements {
            node = node.add_child(stmt);
        }
        node
    }

    // Additional AST node builders for language-specific constructs

    /// Create a package declaration
    pub fn package_declaration(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::PackageDeclaration)
            .with_attribute("name".to_string(), name.to_string())
    }

    /// Create an import declaration
    pub fn import_declaration(path: &str, is_static: bool) -> UniversalNode {
        UniversalNode::new(NodeType::ImportDeclaration)
            .with_attribute("path".to_string(), path.to_string())
            .with_attribute("static".to_string(), is_static.to_string())
    }

    /// Create a field declaration
    pub fn field_declaration(name: &str, field_type: &str) -> UniversalNode {
        UniversalNode::new(NodeType::FieldDeclaration)
            .with_attribute("name".to_string(), name.to_string())
            .with_attribute("type".to_string(), field_type.to_string())
    }

    /// Create an export declaration
    pub fn export_declaration(name: &str, is_default: bool) -> UniversalNode {
        UniversalNode::new(NodeType::ExportDeclaration)
            .with_attribute("name".to_string(), name.to_string())
            .with_attribute("default".to_string(), is_default.to_string())
    }

    /// Create an arrow function
    pub fn arrow_function() -> UniversalNode {
        UniversalNode::new(NodeType::ArrowFunction)
    }

    /// Create a decorator
    pub fn decorator(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::Decorator)
            .with_attribute("name".to_string(), name.to_string())
    }

    /// Create an elif statement
    pub fn elif_statement(condition: &str) -> UniversalNode {
        UniversalNode::new(NodeType::ElifStatement)
            .with_attribute("condition".to_string(), condition.to_string())
    }

    /// Create an else statement
    pub fn else_statement() -> UniversalNode {
        UniversalNode::new(NodeType::ElseStatement)
    }

    /// Create a try statement
    pub fn try_statement() -> UniversalNode {
        UniversalNode::new(NodeType::TryStatement)
    }

    /// Create an except statement
    pub fn except_statement(exception_type: &str) -> UniversalNode {
        UniversalNode::new(NodeType::ExceptStatement)
            .with_attribute("exception_type".to_string(), exception_type.to_string())
    }

    /// Create a finally statement
    pub fn finally_statement() -> UniversalNode {
        UniversalNode::new(NodeType::FinallyStatement)
    }

    // SQL-specific builders

    /// Create a SQL expression
    pub fn sql_expression(expression: &str) -> UniversalNode {
        UniversalNode::new(NodeType::SqlExpression)
            .with_attribute("expression".to_string(), expression.to_string())
    }

    /// Create a SELECT statement
    pub fn select_statement() -> UniversalNode {
        UniversalNode::new(NodeType::SelectStatement)
    }

    /// Create an INSERT statement
    pub fn insert_statement() -> UniversalNode {
        UniversalNode::new(NodeType::InsertStatement)
    }

    /// Create an UPDATE statement
    pub fn update_statement() -> UniversalNode {
        UniversalNode::new(NodeType::UpdateStatement)
    }

    /// Create a DELETE statement
    pub fn delete_statement() -> UniversalNode {
        UniversalNode::new(NodeType::DeleteStatement)
    }

    /// Create a CREATE statement
    pub fn create_statement(object_type: &str) -> UniversalNode {
        UniversalNode::new(NodeType::CreateStatement)
            .with_attribute("object_type".to_string(), object_type.to_string())
    }

    /// Create a CREATE TABLE statement
    pub fn create_table_statement() -> UniversalNode {
        UniversalNode::new(NodeType::CreateTableStatement)
    }

    /// Create a CREATE INDEX statement
    pub fn create_index_statement() -> UniversalNode {
        UniversalNode::new(NodeType::CreateIndexStatement)
    }

    /// Create a CREATE VIEW statement
    pub fn create_view_statement() -> UniversalNode {
        UniversalNode::new(NodeType::CreateViewStatement)
    }

    /// Create a DROP statement
    pub fn drop_statement() -> UniversalNode {
        UniversalNode::new(NodeType::DropStatement)
    }

    /// Create an ALTER statement
    pub fn alter_statement() -> UniversalNode {
        UniversalNode::new(NodeType::AlterStatement)
    }

    // Bash-specific builders

    /// Create a shebang
    pub fn shebang(line: &str) -> UniversalNode {
        UniversalNode::new(NodeType::Shebang)
            .with_attribute("line".to_string(), line.to_string())
    }

    /// Create a case statement
    pub fn case_statement(variable: &str) -> UniversalNode {
        UniversalNode::new(NodeType::CaseStatement)
            .with_attribute("variable".to_string(), variable.to_string())
    }

    /// Create an export statement
    pub fn export_statement(variable: &str) -> UniversalNode {
        UniversalNode::new(NodeType::ExportStatement)
            .with_attribute("variable".to_string(), variable.to_string())
    }

    /// Create a source statement
    pub fn source_statement(file_path: &str) -> UniversalNode {
        UniversalNode::new(NodeType::SourceStatement)
            .with_attribute("file_path".to_string(), file_path.to_string())
    }

    /// Create a command
    pub fn command(name: &str) -> UniversalNode {
        UniversalNode::new(NodeType::Command)
            .with_attribute("name".to_string(), name.to_string())
    }
}

/// Convenience functions for common AST patterns
impl AstBuilder {
    /// Create a simple function call: func(arg1, arg2, ...)
    pub fn simple_call(function_name: &str, arguments: Vec<UniversalNode>) -> UniversalNode {
        let callee = Self::identifier(function_name);
        Self::call_expression(callee, arguments)
    }

    /// Create a property access: obj.prop
    pub fn property_access(object_name: &str, property_name: &str) -> UniversalNode {
        let object = Self::identifier(object_name);
        let property = Self::identifier(property_name);
        Self::member_expression(object, property)
    }

    /// Create a simple assignment: var = value
    pub fn simple_assignment(variable_name: &str, value: UniversalNode) -> UniversalNode {
        let variable = Self::identifier(variable_name);
        Self::assignment_expression(variable, value)
    }

    /// Create an arithmetic expression: left op right
    pub fn arithmetic(left: UniversalNode, op: BinaryOperator, right: UniversalNode) -> UniversalNode {
        Self::binary_expression(op, left, right)
    }

    /// Create a comparison expression: left op right
    pub fn comparison(left: UniversalNode, op: BinaryOperator, right: UniversalNode) -> UniversalNode {
        Self::binary_expression(op, left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    #[test]
    fn test_identifier_builder() {
        let node = AstBuilder::identifier("test_var");
        assert_eq!(node.node_type(), "identifier");
        assert_eq!(node.identifier(), Some(&"test_var".to_string()));
        assert_eq!(node.text(), Some("test_var"));
    }

    #[test]
    fn test_literal_builders() {
        let string_node = AstBuilder::string_literal("hello");
        assert_eq!(string_node.node_type(), "literal");
        assert_eq!(string_node.literal(), Some(&LiteralValue::String("hello".to_string())));

        let number_node = AstBuilder::number_literal(42.5);
        assert_eq!(number_node.literal(), Some(&LiteralValue::Number(42.5)));

        let int_node = AstBuilder::integer_literal(42);
        assert_eq!(int_node.literal(), Some(&LiteralValue::Integer(42)));

        let bool_node = AstBuilder::boolean_literal(true);
        assert_eq!(bool_node.literal(), Some(&LiteralValue::Boolean(true)));

        let null_node = AstBuilder::null_literal();
        assert_eq!(null_node.literal(), Some(&LiteralValue::Null));
    }

    #[test]
    fn test_binary_expression_builder() {
        let left = AstBuilder::identifier("a");
        let right = AstBuilder::identifier("b");
        let expr = AstBuilder::binary_expression(BinaryOperator::Add, left, right);

        assert_eq!(expr.node_type(), "binary_expression");
        assert_eq!(expr.child_count(), 2);
        assert_eq!(expr.binary_operator, Some(BinaryOperator::Add));
    }

    #[test]
    fn test_function_declaration_builder() {
        let param1 = AstBuilder::identifier("a");
        let param2 = AstBuilder::identifier("b");
        let body = AstBuilder::block_statement(vec![]);
        
        let func = AstBuilder::function_declaration("add", vec![param1, param2], body);
        
        assert_eq!(func.node_type(), "function_declaration");
        assert_eq!(func.identifier(), Some(&"add".to_string()));
        assert_eq!(func.child_count(), 3); // 2 params + 1 body
    }

    #[test]
    fn test_call_expression_builder() {
        let callee = AstBuilder::identifier("console.log");
        let arg = AstBuilder::string_literal("Hello, World!");
        let call = AstBuilder::call_expression(callee, vec![arg]);

        assert_eq!(call.node_type(), "call_expression");
        assert_eq!(call.child_count(), 2); // callee + 1 argument
    }

    #[test]
    fn test_simple_call_builder() {
        let arg1 = AstBuilder::string_literal("test");
        let arg2 = AstBuilder::integer_literal(42);
        let call = AstBuilder::simple_call("myFunction", vec![arg1, arg2]);

        assert_eq!(call.node_type(), "call_expression");
        assert_eq!(call.child_count(), 3); // callee + 2 arguments
    }

    #[test]
    fn test_property_access_builder() {
        let access = AstBuilder::property_access("obj", "prop");
        assert_eq!(access.node_type(), "member_expression");
        assert_eq!(access.child_count(), 2);
    }

    #[test]
    fn test_if_statement_builder() {
        let condition = AstBuilder::boolean_literal(true);
        let then_branch = AstBuilder::block_statement(vec![]);
        let else_branch = AstBuilder::block_statement(vec![]);
        
        let if_stmt = AstBuilder::if_statement(condition, then_branch, Some(else_branch));
        
        assert_eq!(if_stmt.node_type(), "if_statement");
        assert_eq!(if_stmt.child_count(), 3); // condition + then + else
    }

    #[test]
    fn test_program_builder() {
        let stmt1 = AstBuilder::expression_statement(AstBuilder::identifier("x"));
        let stmt2 = AstBuilder::expression_statement(AstBuilder::identifier("y"));
        let program = AstBuilder::program(vec![stmt1, stmt2]);

        assert_eq!(program.node_type(), "program");
        assert_eq!(program.child_count(), 2);
    }
}
