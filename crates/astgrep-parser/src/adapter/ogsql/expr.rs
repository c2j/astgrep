//! Expression mapping: ogsql Expr variants → UniversalNode.
//!
//! Maps common expression variants for static analysis. Rare variants fall
//! back to a generic `SqlExpression` node with debug-format text.
//!
//! # Mapped variants (14)
//!
//! | Variant | NodeType | Notes |
//! |---------|----------|-------|
//! | `Literal` | `Literal` | String, Integer, Float, Boolean, Null, etc. |
//! | `ColumnRef` | `Identifier` | ObjectName joined by `.` |
//! | `BinaryOp` | `BinaryExpression` | Only for operators in `map_binary_op` |
//! | `UnaryOp` | `UnaryExpression` | NOT, MINUS, PLUS |
//! | `FunctionCall` | `CallExpression` | Simplified (no window/filter/within_group) |
//! | `Like` | `SqlExpression` | With metadata |
//! | `Case` | `SqlExpression` | With metadata |
//! | `Between` | `SqlExpression` | With metadata |
//! | `InList` | `SqlExpression` | With metadata |
//! | `InSubquery` | `SqlExpression` | With subquery child |
//! | `Exists` | `SqlExpression` | With subquery child |
//! | `Subquery` | `SqlExpression` | With select child |
//! | `IsNull` | `SqlExpression` | With metadata |
//! | `TypeCast` | `SqlExpression` | With metadata |
//! | `QualifiedStar` | `SqlExpression` | e.g. `table.*` |
//! | `Parenthesized` | (recurse) | Unwraps parenthesis |
//! | `Parameter/JdbcParam/MyBatis*` | `SqlExpression` | Placeholder parameters |

use astgrep_ast::{AstBuilder, BinaryOperator, UnaryOperator, UniversalNode};

/// Convert an ogsql expression to a UniversalNode.
///
/// Common variants receive dedicated node types with structured children.
/// Rare variants are mapped to a generic `SqlExpression` node whose
/// `expression` attribute contains a debug representation.
pub fn convert_expr(expr: &ogsql_parser::Expr) -> UniversalNode {
    use ogsql_parser::Expr;
    match expr {
        Expr::Literal(lit) => convert_literal(lit),
        Expr::ColumnRef(name) => {
            // ObjectName is Vec<Ident> — join with "."
            let joined = name.join(".");
            AstBuilder::identifier(&joined)
        }
        Expr::QualifiedStar(table) => AstBuilder::sql_expression(&format!("{}.*", table)),
        Expr::BinaryOp { left, op, right } => convert_binary_op(left, op, right),
        Expr::Like {
            expr,
            pattern,
            negated,
            ..
        } => {
            let mut node = AstBuilder::sql_expression("LIKE")
                .add_child(convert_expr(expr))
                .add_child(convert_expr(pattern));
            if *negated {
                node = node.with_metadata("negated".into(), "true".into());
            }
            node
        }
        Expr::UnaryOp { op, expr } => convert_unary_op(op, expr),
        Expr::FunctionCall { name, args, .. } => {
            let callee = AstBuilder::identifier(&name.join("."));
            let arg_nodes: Vec<UniversalNode> = args.iter().map(convert_expr).collect();
            AstBuilder::call_expression(callee, arg_nodes)
        }
        Expr::Case {
            operand,
            whens,
            else_expr,
        } => {
            let mut node = AstBuilder::sql_expression("CASE");
            if let Some(op) = operand {
                node = node.add_child(convert_expr(op));
            }
            for when in whens {
                let when_node = AstBuilder::sql_expression("WHEN")
                    .add_child(convert_expr(&when.condition))
                    .add_child(convert_expr(&when.result));
                node = node.add_child(when_node);
            }
            if let Some(els) = else_expr {
                node =
                    node.add_child(AstBuilder::sql_expression("ELSE").add_child(convert_expr(els)));
            }
            node
        }
        Expr::Between {
            expr,
            low,
            high,
            negated,
        } => {
            let mut node = AstBuilder::sql_expression("BETWEEN")
                .add_child(convert_expr(expr))
                .add_child(convert_expr(low))
                .add_child(convert_expr(high));
            if *negated {
                node = node.with_metadata("negated".into(), "true".into());
            }
            node
        }
        Expr::InList {
            expr,
            list,
            negated,
        } => {
            let mut node = AstBuilder::sql_expression("IN").add_child(convert_expr(expr));
            for item in list {
                node = node.add_child(convert_expr(item));
            }
            if *negated {
                node = node.with_metadata("negated".into(), "true".into());
            }
            node
        }
        Expr::InSubquery {
            expr,
            subquery,
            negated,
        } => {
            let mut node = AstBuilder::sql_expression("IN_SUBQUERY")
                .add_child(convert_expr(expr))
                .add_child(convert_subquery_expr(subquery));
            if *negated {
                node = node.with_metadata("negated".into(), "true".into());
            }
            node
        }
        Expr::Exists(subquery) => {
            AstBuilder::sql_expression("EXISTS").add_child(convert_subquery_expr(subquery))
        }
        Expr::Subquery(subquery) => convert_subquery_expr(subquery),
        Expr::ScalarSublink {
            expr,
            op,
            sublink_type,
            subquery,
        } => AstBuilder::sql_expression(&format!("SCALAR_SUBLINK({})", op))
            .add_child(convert_expr(expr))
            .add_child(convert_subquery_expr(subquery))
            .with_metadata("sublink_type".into(), format!("{:?}", sublink_type)),
        Expr::IsNull { expr, negated } => {
            let tag = if *negated { "IS NOT NULL" } else { "IS NULL" };
            AstBuilder::sql_expression(tag).add_child(convert_expr(expr))
        }
        Expr::IsBoolean {
            expr,
            value,
            negated,
        } => {
            let mut node = AstBuilder::sql_expression("IS").add_child(convert_expr(expr));
            if *negated {
                node = node.with_metadata("negated".into(), "true".into());
            }
            node.with_metadata("boolean_value".into(), value.to_string())
        }
        Expr::TypeCast {
            expr, type_name, ..
        } => AstBuilder::sql_expression("CAST")
            .add_child(convert_expr(expr))
            .with_metadata("target_type".into(), format_type(type_name)),
        Expr::Treat { expr, type_name } => AstBuilder::sql_expression("TREAT")
            .add_child(convert_expr(expr))
            .with_metadata("target_type".into(), format_type(type_name)),
        Expr::CollationFor { expr } => {
            AstBuilder::sql_expression("COLLATION_FOR").add_child(convert_expr(expr))
        }
        Expr::Parameter(idx) => AstBuilder::sql_expression(&format!("${}", idx)),
        Expr::MyBatisParam(name) => {
            AstBuilder::sql_expression(name).with_metadata("is_mybatis_param".into(), "true".into())
        }
        Expr::MyBatisRawExpr(raw) => AstBuilder::sql_expression(&format!("mybatis_raw:{}", raw))
            .with_metadata("is_mybatis_raw".into(), "true".into()),
        Expr::JdbcParam => {
            AstBuilder::sql_expression("?").with_metadata("is_jdbc_param".into(), "true".into())
        }
        Expr::Array(elements) => {
            let mut node = AstBuilder::sql_expression("ARRAY");
            for el in elements {
                node = node.add_child(convert_expr(el));
            }
            node
        }
        Expr::Subscript {
            object,
            lower,
            upper,
            is_slice,
        } => {
            let mut node = AstBuilder::sql_expression("SUBSCRIPT").add_child(convert_expr(object));
            if let Some(l) = lower {
                node = node.add_child(convert_expr(l));
            }
            if let Some(u) = upper {
                node = node.add_child(convert_expr(u));
            }
            if *is_slice {
                node = node.with_metadata("is_slice".into(), "true".into());
            }
            node
        }
        Expr::FieldAccess { object, field } => AstBuilder::sql_expression("FIELD_ACCESS")
            .add_child(convert_expr(object))
            .with_metadata("field".into(), field.clone()),
        Expr::Parenthesized(inner) => {
            // Unwrap parenthesis — the inner expression is what rules care about
            convert_expr(inner)
        }
        Expr::RowConstructor(items) => {
            let mut node = AstBuilder::sql_expression("ROW");
            for item in items {
                node = node.add_child(convert_expr(item));
            }
            node
        }
        Expr::Prior(inner) => AstBuilder::sql_expression("PRIOR").add_child(convert_expr(inner)),
        Expr::Default => AstBuilder::sql_expression("DEFAULT"),
        Expr::SpecialFunction { name, args, .. } => {
            let callee = AstBuilder::identifier(name);
            let arg_nodes: Vec<UniversalNode> = args.iter().map(convert_expr).collect();
            AstBuilder::call_expression(callee, arg_nodes)
                .with_metadata("is_special_function".into(), "true".into())
        }
        Expr::CurrentOf { cursor_name } => AstBuilder::sql_expression("CURRENT_OF")
            .with_metadata("cursor".into(), cursor_name.clone()),
        Expr::PredictBy { model_name, .. } => AstBuilder::sql_expression("PREDICT_BY")
            .with_metadata("model".into(), model_name.clone()),
        Expr::SysDate => AstBuilder::sql_expression("SYSDATE"),
        Expr::SequenceValue { sequence, function } => {
            AstBuilder::sql_expression(&format!("{:?}", function))
                .with_metadata("sequence".into(), sequence.join("."))
        }
        Expr::CursorAttribute { cursor, attribute } => {
            AstBuilder::sql_expression(&format!("CURSOR_ATTR({:?})", attribute))
                .add_child(convert_expr(cursor))
        }
        Expr::PlVariable(name) => AstBuilder::sql_expression(&name.join("."))
            .with_metadata("is_pl_variable".into(), "true".into()),
        // XML and other rare variants → generic fallback
        _ => AstBuilder::sql_expression(&format!("{:?}", expr)),
    }
}

/// Convert a Literal to a UniversalNode.
fn convert_literal(lit: &ogsql_parser::Literal) -> UniversalNode {
    use ogsql_parser::Literal;
    match lit {
        Literal::Integer(i) => AstBuilder::integer_literal(*i),
        Literal::Float(s) => {
            // Parse float or fall back to string
            s.parse::<f64>()
                .map(AstBuilder::number_literal)
                .unwrap_or_else(|_| AstBuilder::string_literal(s))
        }
        Literal::String(s) => AstBuilder::string_literal(s),
        Literal::EscapeString(s) => {
            AstBuilder::string_literal(s).with_metadata("escape_string".into(), "true".into())
        }
        Literal::BitString(s) => AstBuilder::sql_expression(&format!("b'{}'", s)),
        Literal::HexString(s) => AstBuilder::sql_expression(&format!("x'{}'", s)),
        Literal::NationalString(s) => AstBuilder::sql_expression(&format!("N'{}'", s)),
        Literal::DollarString { tag, body } => {
            let tag_str = tag.as_deref().unwrap_or("");
            AstBuilder::sql_expression(&format!("${}${}${}$", tag_str, body, tag_str))
                .with_metadata("dollar_tag".into(), tag_str.to_string())
        }
        Literal::Boolean(b) => AstBuilder::boolean_literal(*b),
        Literal::Null => AstBuilder::null_literal(),
    }
}

/// Convert a BinaryOp expression.
///
/// Operators that map to `BinaryOperator` produce `BinaryExpression` nodes.
/// SQL-specific operators (LIKE, ILIKE, etc.) produce `SqlExpression` nodes.
fn convert_binary_op(
    left: &ogsql_parser::Expr,
    op: &str,
    right: &ogsql_parser::Expr,
) -> UniversalNode {
    let left_node = convert_expr(left);
    let right_node = convert_expr(right);
    if let Some(mapped) = map_binary_op(op) {
        AstBuilder::binary_expression(mapped, left_node, right_node)
    } else {
        // SQL-specific operators: LIKE, ILIKE, IS, IS DISTINCT FROM, etc.
        AstBuilder::sql_expression(op)
            .add_child(left_node)
            .add_child(right_node)
    }
}

/// Map a SQL binary operator string to the generic BinaryOperator enum.
fn map_binary_op(op: &str) -> Option<BinaryOperator> {
    match op {
        "=" => Some(BinaryOperator::Equal),
        "<>" | "!=" => Some(BinaryOperator::NotEqual),
        "<" => Some(BinaryOperator::LessThan),
        "<=" => Some(BinaryOperator::LessThanOrEqual),
        ">" => Some(BinaryOperator::GreaterThan),
        ">=" => Some(BinaryOperator::GreaterThanOrEqual),
        "+" => Some(BinaryOperator::Add),
        "-" => Some(BinaryOperator::Subtract),
        "*" => Some(BinaryOperator::Multiply),
        "/" => Some(BinaryOperator::Divide),
        "%" => Some(BinaryOperator::Modulo),
        "AND" => Some(BinaryOperator::And),
        "OR" => Some(BinaryOperator::Or),
        "IN" => Some(BinaryOperator::In),
        _ => None,
    }
}

/// Convert a UnaryOp expression.
fn convert_unary_op(op: &str, expr: &ogsql_parser::Expr) -> UniversalNode {
    let operand = convert_expr(expr);
    let mapped = match op {
        "-" | "MINUS" => UnaryOperator::Minus,
        "+" | "PLUS" => UnaryOperator::Plus,
        "NOT" => UnaryOperator::Not,
        "~" => UnaryOperator::BitwiseNot,
        _ => {
            // Unknown unary op → fallback to sql_expression
            return AstBuilder::sql_expression(&format!("{} {:?}", op, expr)).add_child(operand);
        }
    };
    AstBuilder::unary_expression(mapped, operand)
}

/// Convert a subquery SelectStatement to a SqlExpression child node.
fn convert_subquery_expr(stmt: &ogsql_parser::SelectStatement) -> UniversalNode {
    // Build a simplified select representation as a child
    let mut node = AstBuilder::select_statement();
    for table_ref in &stmt.from {
        add_table_name_to_node(&mut node, table_ref);
    }
    node
}

/// Extract table names from a TableRef and add them to a node's "tables" attribute.
fn add_table_name_to_node(node: &mut UniversalNode, table_ref: &ogsql_parser::TableRef) {
    use ogsql_parser::TableRef;
    match table_ref {
        TableRef::Table { name, .. } => {
            let table_name = name.join(".");
            append_attribute(node, "tables", &table_name);
        }
        TableRef::Join { left, right, .. } => {
            add_table_name_to_node(node, left);
            add_table_name_to_node(node, right);
        }
        TableRef::Subquery { alias, .. } => {
            if let Some(a) = alias {
                append_attribute(node, "tables", a);
            }
        }
        TableRef::Values { alias, .. } => {
            if let Some(a) = alias {
                append_attribute(node, "tables", a);
            }
        }
        TableRef::Pivot { source, .. } | TableRef::Unpivot { source, .. } => {
            add_table_name_to_node(node, source);
        }
        TableRef::FunctionCall { name, alias, .. } => {
            let fn_name = name.join(".");
            let display: &str = alias.as_ref().map_or(&fn_name, |a| a.as_str());
            append_attribute(node, "tables", display);
        }
    }
}

/// Append a value to a comma-separated attribute on a node.
fn append_attribute(node: &mut UniversalNode, key: &str, value: &str) {
    let current = node.attributes.get(key).cloned().unwrap_or_default();
    let new_val = if current.is_empty() {
        value.to_string()
    } else {
        format!("{},{}", current, value)
    };
    node.attributes.insert(key.to_string(), new_val);
}

/// Format a DataType as a string.
fn format_type(dt: &ogsql_parser::DataType) -> String {
    use ogsql_parser::DataType;
    match dt {
        DataType::Boolean => "BOOLEAN".into(),
        DataType::TinyInt(_) => "TINYINT".into(),
        DataType::SmallInt(_) => "SMALLINT".into(),
        DataType::Integer(_) => "INTEGER".into(),
        DataType::BigInt(_) => "BIGINT".into(),
        DataType::Real => "REAL".into(),
        DataType::Float(_) => "FLOAT".into(),
        DataType::Double => "DOUBLE".into(),
        DataType::Numeric(..) => "NUMERIC".into(),
        DataType::Char(_) => "CHAR".into(),
        DataType::Varchar(_) => "VARCHAR".into(),
        DataType::Text => "TEXT".into(),
        DataType::Bytea => "BYTEA".into(),
        DataType::Timestamp(..) => "TIMESTAMP".into(),
        DataType::Timestamptz(_) => "TIMESTAMPTZ".into(),
        DataType::Date => "DATE".into(),
        DataType::Time(..) => "TIME".into(),
        DataType::Interval(_) => "INTERVAL".into(),
        DataType::Json => "JSON".into(),
        DataType::Jsonb => "JSONB".into(),
        DataType::Uuid => "UUID".into(),
        DataType::Bit(_) => "BIT".into(),
        DataType::Varbit(_) => "VARBIT".into(),
        DataType::Serial => "SERIAL".into(),
        DataType::SmallSerial => "SMALLSERIAL".into(),
        DataType::BigSerial => "BIGSERIAL".into(),
        DataType::BinaryFloat => "BINARY_FLOAT".into(),
        DataType::BinaryDouble => "BINARY_DOUBLE".into(),
        DataType::Array(inner) => format!("{}[]", format_type(inner)),
        DataType::Custom(name, _) => name.join("."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_ast::{LiteralValue, NodeType};
    use astgrep_core::AstNode;

    fn parse_one(sql: &str) -> UniversalNode {
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        let stmt = &stmts[0];
        match stmt {
            ogsql_parser::Statement::Select(spanned) => {
                // Get WHERE clause to test expressions
                if let Some(where_expr) = &spanned.where_clause {
                    convert_expr(where_expr)
                } else {
                    // For no WHERE, just test the first target expression
                    match &spanned.targets[0] {
                        ogsql_parser::ast::SelectTarget::Expr(expr, _) => convert_expr(expr),
                        ogsql_parser::ast::SelectTarget::Star(_) => {
                            UniversalNode::new(NodeType::SqlExpression)
                        }
                    }
                }
            }
            _ => panic!("expected select statement"),
        }
    }

    #[test]
    fn test_literal_string() {
        let node = parse_one("SELECT 'hello'");
        assert_eq!(node.node_type(), "literal");
        assert_eq!(
            node.literal(),
            Some(&LiteralValue::String("hello".to_string()))
        );
    }

    #[test]
    fn test_literal_integer() {
        let node = parse_one("SELECT 42");
        assert_eq!(node.node_type(), "literal");
        assert_eq!(node.literal(), Some(&LiteralValue::Integer(42)));
    }

    #[test]
    fn test_column_ref() {
        let node = parse_one("SELECT id FROM t");
        assert_eq!(node.node_type(), "identifier");
        assert_eq!(node.identifier(), Some(&"id".to_string()));
    }

    #[test]
    fn test_qualified_column_ref() {
        let node = parse_one("SELECT t.id FROM t");
        assert_eq!(node.node_type(), "identifier");
        assert_eq!(node.identifier(), Some(&"t.id".to_string()));
    }

    #[test]
    fn test_binary_op_equals() {
        let node = parse_one("SELECT a FROM t WHERE a = 1");
        assert_eq!(node.node_type(), "binary_expression");
        assert_eq!(node.binary_operator, Some(BinaryOperator::Equal));
        assert_eq!(node.child_count(), 2);
        // left = ColumnRef "a"
        assert_eq!(node.child(0).unwrap().node_type(), "identifier");
        // right = Literal 1
        assert_eq!(node.child(1).unwrap().node_type(), "literal");
        assert_eq!(node.children[1].literal(), Some(&LiteralValue::Integer(1)));
    }

    #[test]
    fn test_binary_op_and() {
        let node = parse_one("SELECT a FROM t WHERE a = 1 AND b = 2");
        assert_eq!(node.node_type(), "binary_expression");
        assert_eq!(node.binary_operator, Some(BinaryOperator::And));
    }

    #[test]
    fn test_function_call() {
        let node = parse_one("SELECT COUNT(*) FROM t");
        assert_eq!(node.node_type(), "call_expression");
        assert_eq!(node.child_count(), 2); // callee + 1 arg
        assert_eq!(node.child(0).unwrap().node_type(), "identifier");
    }

    #[test]
    fn test_in_list() {
        let node = parse_one("SELECT a FROM t WHERE a IN (1, 2, 3)");
        assert_eq!(node.node_type(), "sql_expression");
        // Has expression attribute "IN"
        assert_eq!(node.get_attribute("expression"), Some(&"IN".to_string()));
        // Has 4 children: the column ref + 3 list items
        assert_eq!(node.child_count(), 4);
    }

    #[test]
    fn test_between() {
        let node = parse_one("SELECT a FROM t WHERE a BETWEEN 1 AND 10");
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(
            node.get_attribute("expression"),
            Some(&"BETWEEN".to_string())
        );
        // 3 children: expr, low, high
        assert_eq!(node.child_count(), 3);
    }

    #[test]
    fn test_is_null() {
        let node = parse_one("SELECT a FROM t WHERE a IS NULL");
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(
            node.get_attribute("expression"),
            Some(&"IS NULL".to_string())
        );
    }

    #[test]
    fn test_like() {
        let node = parse_one("SELECT a FROM t WHERE a LIKE 'foo%'");
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(node.get_attribute("expression"), Some(&"LIKE".to_string()));
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn test_exists() {
        let node = parse_one("SELECT a FROM t WHERE EXISTS (SELECT 1)");
        assert_eq!(node.node_type(), "sql_expression");
        assert_eq!(
            node.get_attribute("expression"),
            Some(&"EXISTS".to_string())
        );
    }

    #[test]
    fn test_parenthesized_expression() {
        // Parentheses should be unwrapped
        let nodes = super::super::OgsqlAdapter::parse_to_universal("SELECT a FROM t WHERE (a = 1)")
            .unwrap();
        let select = &nodes[0];
        // WHERE child should be a binary_expression (not parenthesized)
        assert_eq!(select.child_count(), 1);
        let where_child = select.child(0).unwrap();
        assert_eq!(where_child.node_type(), "binary_expression");
    }
}
