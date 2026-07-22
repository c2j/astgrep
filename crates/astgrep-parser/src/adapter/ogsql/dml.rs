//! DML statement conversion: ogsql DML structs → UniversalNode.
//!
//! Maps SELECT, INSERT, UPDATE, DELETE, MERGE statements for static analysis.
//! Focuses on what rules care about: table names, WHERE conditions, column refs,
//! key syntactic features (VALUES, SET, ON CONFLICT, RETURNING).

use super::expr;
use super::OgsqlAdapterError;
use astgrep_ast::{AstBuilder, UniversalNode};

/// Convert a SELECT statement.
///
/// Produces a `SelectStatement` node with:
/// - `tables` attribute: comma-separated table names from FROM/JOIN
/// - SELECT_LIST child: select column targets
/// - FROM_CLAUSE child: structured table ref nodes
/// - WHERE expression as a child (if present)
/// - GROUP_BY child (if present)
/// - HAVING_CLAUSE child (if present)
/// - ORDER_BY child (if present)
/// - Metadata flags: `has_order_by`, `has_limit`, `set_operation`, `distinct`
pub fn convert_select(
    select: &ogsql_parser::SelectStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = AstBuilder::select_statement();

    if !select.targets.is_empty() {
        let mut select_list = AstBuilder::sql_expression("SELECT_LIST");
        for target in &select.targets {
            select_list = select_list.add_child(convert_select_target(target));
        }
        node = node.add_child(select_list);
    }

    if !select.from.is_empty() {
        let mut from_node = AstBuilder::sql_expression("FROM_CLAUSE");
        for table_ref in &select.from {
            from_node = from_node.add_child(convert_table_ref_node(table_ref));
        }
        node = node.add_child(from_node);
    }

    for table_ref in &select.from {
        add_table_ref(&mut node, table_ref);
    }

    // Add WHERE clause as child (critical for SQL injection detection)
    if let Some(where_expr) = &select.where_clause {
        let where_node = expr::convert_expr(where_expr);
        node = node.add_child(where_node);
    }

    // Metadata flags for key features
    if !select.order_by.is_empty() {
        node = node.with_metadata("has_order_by".into(), "true".into());
    }
    if select.limit.is_some() || select.offset.is_some() || select.fetch.is_some() {
        node = node.with_metadata("has_limit".into(), "true".into());
    }
    if let Some(set_op) = &select.set_operation {
        let op_name = match set_op {
            ogsql_parser::ast::SetOperation::Union { .. } => "UNION",
            ogsql_parser::ast::SetOperation::Intersect { .. } => "INTERSECT",
            ogsql_parser::ast::SetOperation::Except { .. } => "EXCEPT",
        };
        node = node.with_metadata("set_operation".into(), op_name.into());
    }
    if select.distinct {
        node = node.with_metadata("distinct".into(), "true".into());
    }

    if !select.group_by.is_empty() {
        let mut group_node = AstBuilder::sql_expression("GROUP_BY");
        for item in &select.group_by {
            group_node = group_node.add_child(convert_group_by_item(item));
        }
        node = node.add_child(group_node);
        node = node.with_metadata("has_group_by".into(), "true".into());
    }

    if let Some(ref having) = select.having {
        node = node.add_child(
            AstBuilder::sql_expression("HAVING_CLAUSE")
                .add_child(expr::convert_expr(having)),
        );
        node = node.with_metadata("has_having".into(), "true".into());
    }

    if !select.order_by.is_empty() {
        let mut order_node = AstBuilder::sql_expression("ORDER_BY");
        for item in &select.order_by {
            let mut sort = AstBuilder::sql_expression("SORT_ITEM")
                .add_child(expr::convert_expr(&item.expr));
            if let Some(asc) = item.asc {
                sort = sort.with_metadata("asc".into(), asc.to_string());
            }
            order_node = order_node.add_child(sort);
        }
        node = node.add_child(order_node);
    }

    // Attach plan hints from `/*+ ... */` comments
    let hint_strings: Vec<String> = select.hints.iter().map(|h| h.name.clone()).collect();
    let mut node = super::features::add_plan_hints(node, &hint_strings);

    // SELECT ... INTO var1, var2 — variable assignment targets
    if let Some(ref into_targets) = select.into_targets {
        let var_names: Vec<String> = into_targets
            .iter()
            .filter_map(|t| match t {
                ogsql_parser::ast::SelectTarget::Expr(_, Some(alias)) => Some(alias.to_string()),
                ogsql_parser::ast::SelectTarget::Expr(expr, None) => {
                    if let ogsql_parser::ast::Expr::ColumnRef(name) = expr {
                        Some(name.join("."))
                    } else if let ogsql_parser::ast::Expr::PlVariable(name) = expr {
                        Some(name.join("."))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        if !var_names.is_empty() {
            node = node
                .with_metadata("has_into".into(), "true".into())
                .with_metadata("into_vars".into(), var_names.join(","));
            for v in &var_names {
                node = node.add_child(
                    AstBuilder::sql_expression("INTO_TARGET")
                        .with_metadata("target_var".into(), v.clone()),
                );
            }
        }
    }

    // FOR UPDATE / FOR SHARE / FOR NO KEY UPDATE / FOR KEY SHARE
    if let Some(ref lock) = select.lock_clause {
        let (lock_type, nowait, skip_locked, wait) = match lock {
            ogsql_parser::ast::LockClause::Update {
                nowait,
                skip_locked,
                wait,
                ..
            } => ("Update", *nowait, *skip_locked, wait.is_some()),
            ogsql_parser::ast::LockClause::Share {
                nowait,
                skip_locked,
                wait,
                ..
            } => ("Share", *nowait, *skip_locked, wait.is_some()),
            ogsql_parser::ast::LockClause::NoKeyUpdate {
                nowait,
                skip_locked,
                wait,
                ..
            } => ("NoKeyUpdate", *nowait, *skip_locked, wait.is_some()),
            ogsql_parser::ast::LockClause::KeyShare {
                nowait,
                skip_locked,
                wait,
                ..
            } => ("KeyShare", *nowait, *skip_locked, wait.is_some()),
        };
        node = node
            .with_metadata("has_lock".into(), "true".into())
            .with_metadata("lock_type".into(), lock_type.into());
        if nowait {
            node = node.with_metadata("lock_nowait".into(), "true".into());
        }
        if skip_locked {
            node = node.with_metadata("lock_skip_locked".into(), "true".into());
        }
        if wait {
            node = node.with_metadata("lock_wait".into(), "true".into());
        }
    }

    // BULK COLLECT
    if select.bulk_collect {
        node = node.with_metadata("bulk_collect".into(), "true".into());
    }

    Ok(node)
}

/// Convert an INSERT statement.
///
/// Produces an `InsertStatement` node with:
/// - `table` attribute: target table name
/// - `columns` attribute: comma-separated column list (if explicit)
/// - Source (VALUES/Select) as children
/// - Metadata: `on_conflict`, `has_returning`, `bulk_collect`
pub fn convert_insert(
    insert: &ogsql_parser::InsertStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = AstBuilder::insert_statement().with_table(insert.table.join("."));

    // Explicit columns
    if !insert.columns.is_empty() {
        node = node.with_metadata("columns".into(), insert.columns.join(","));
    }

    // Source: VALUES, SELECT, or DefaultValues
    match &insert.source {
        ogsql_parser::ast::InsertSource::Values(rows) => {
            let row_count = rows.len();
            let val_count = rows.first().map(|r| r.len()).unwrap_or(0);
            node = node.with_metadata("source_type".into(), "VALUES".into());
            node = node.with_metadata("value_row_count".into(), row_count.to_string());
            node = node.with_metadata("value_column_count".into(), val_count.to_string());
            let mut values_node = AstBuilder::sql_expression("VALUES_CLAUSE");
            for row in rows {
                let mut row_node = AstBuilder::sql_expression("VALUES_ROW");
                for val in row {
                    row_node = row_node.add_child(expr::convert_expr(val));
                }
                values_node = values_node.add_child(row_node);
            }
            node = node.add_child(values_node);
        }
        ogsql_parser::ast::InsertSource::Select(select) => {
            node = node.with_metadata("source_type".into(), "SELECT".into());
            let select_node = convert_select(select)?;
            node = node.add_child(select_node);
        }
        ogsql_parser::ast::InsertSource::DefaultValues => {
            node = node.with_metadata("source_type".into(), "DEFAULT_VALUES".into());
        }
        ogsql_parser::ast::InsertSource::Set(assignments) => {
            node = node.with_metadata("source_type".into(), "SET".into());
            for assign in assignments {
                node = node.add_child(convert_update_assignment(assign));
            }
        }
        ogsql_parser::ast::InsertSource::RecordVariable(_) => {
            node = node.with_metadata("source_type".into(), "RECORD_VARIABLE".into());
        }
    }

    // ON CONFLICT / ON DUPLICATE KEY
    if let Some(_on_conflict) = &insert.on_conflict {
        node = node.with_metadata("on_conflict".into(), "true".into());
    }
    if let Some(_dup) = &insert.on_duplicate_key {
        node = node.with_metadata("on_duplicate_key".into(), "true".into());
    }

    // RETURNING
    if !insert.returning.is_empty() {
        node = node.with_metadata("has_returning".into(), "true".into());
    }
    if insert.bulk_collect {
        node = node.with_metadata("bulk_collect".into(), "true".into());
    }

    // WITH
    if insert.with.is_some() {
        node = node.with_metadata("has_cte".into(), "true".into());
    }

    // Attach plan hints from `/*+ ... */` comments
    let hint_strings: Vec<String> = insert.hints.iter().map(|h| h.name.clone()).collect();
    let node = super::features::add_plan_hints(node, &hint_strings);

    Ok(node)
}

/// Convert an UPDATE statement.
///
/// Produces an `UpdateStatement` node with:
/// - `tables` attribute: target tables from the main table list
/// - SET assignments as children
/// - WHERE expression as child (if present)
/// - Metadata: `has_order_by`, `has_limit`, `has_returning`
pub fn convert_update(
    update: &ogsql_parser::UpdateStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = AstBuilder::update_statement();

    // Target tables
    for table_ref in &update.tables {
        add_table_ref(&mut node, table_ref);
    }

    if !update.assignments.is_empty() {
        let mut set_node = AstBuilder::sql_expression("SET_CLAUSE");
        for assign in &update.assignments {
            let value_expr = expr::convert_expr(&assign.value);
            let col_names: Vec<String> = assign.columns.iter().map(|c| c.join(".")).collect();
            let col_str = col_names.join(", ");
            let mut value_with_meta = value_expr;
            if let Some(ref vt) = value_with_meta.text.clone() {
                if !vt.is_empty() {
                    value_with_meta
                        .attributes
                        .insert("target_var".into(), vt.clone());
                }
            }
            set_node = set_node.add_child(
                AstBuilder::sql_expression("SET")
                    .add_child(value_with_meta)
                    .with_metadata("column".into(), col_str),
            );
        }
        node = node.add_child(set_node);
    }

    // Additional FROM tables (GaussDB-specific: UPDATE ... FROM ...)
    for table_ref in &update.from {
        add_table_ref(&mut node, table_ref);
    }

    // WHERE clause
    if let Some(where_expr) = &update.where_clause {
        node = node.add_child(expr::convert_expr(where_expr));
    }

    // Metadata flags
    if update.order_by.is_some() {
        node = node.with_metadata("has_order_by".into(), "true".into());
    }
    if update.limit.is_some() {
        node = node.with_metadata("has_limit".into(), "true".into());
    }
    if !update.returning.is_empty() {
        node = node.with_metadata("has_returning".into(), "true".into());
    }
    if update.bulk_collect {
        node = node.with_metadata("bulk_collect".into(), "true".into());
    }
    if update.with.is_some() {
        node = node.with_metadata("has_cte".into(), "true".into());
    }

    Ok(node)
}

/// Convert a DELETE statement.
///
/// Produces a `DeleteStatement` node with:
/// - `tables` attribute: target tables
/// - WHERE expression as child (if present) — absence is a security concern
/// - Metadata: `has_order_by`, `has_limit`, `has_returning`
pub fn convert_delete(
    delete: &ogsql_parser::DeleteStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = AstBuilder::delete_statement();

    // Target tables
    for table_ref in &delete.tables {
        add_table_ref(&mut node, table_ref);
    }

    // USING tables
    for table_ref in &delete.using {
        add_table_ref(&mut node, table_ref);
    }

    // WHERE clause
    if let Some(where_expr) = &delete.where_clause {
        node = node.add_child(expr::convert_expr(where_expr));
    }

    // Metadata flags
    if delete.order_by.is_some() {
        node = node.with_metadata("has_order_by".into(), "true".into());
    }
    if delete.limit.is_some() {
        node = node.with_metadata("has_limit".into(), "true".into());
    }
    if !delete.returning.is_empty() {
        node = node.with_metadata("has_returning".into(), "true".into());
    }
    if delete.bulk_collect {
        node = node.with_metadata("bulk_collect".into(), "true".into());
    }
    if delete.with.is_some() {
        node = node.with_metadata("has_cte".into(), "true".into());
    }

    Ok(node)
}

/// Convert a MERGE statement.
///
/// Produces a `MergeStatement` node with:
/// - `target_table` and `source_table` attributes
/// - ON condition as child expression
/// - WHEN clauses as children
pub fn convert_merge(
    merge: &ogsql_parser::MergeStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut node = AstBuilder::merge_statement();

    // Target table
    add_target_ref(&mut node, "target_table", &merge.target);
    // Source table
    add_target_ref(&mut node, "source_table", &merge.source);

    // ON condition
    node = node.add_child(expr::convert_expr(&merge.on_condition));

    // WHEN clauses
    for when in &merge.when_clauses {
        let action_desc = match &when.action {
            ogsql_parser::ast::MergeAction::Update(_) => "UPDATE",
            ogsql_parser::ast::MergeAction::Delete => "DELETE",
            ogsql_parser::ast::MergeAction::Insert { .. } => "INSERT",
        };
        let matched_str = if when.matched {
            "MATCHED"
        } else {
            "NOT_MATCHED"
        };
        let mut when_node = AstBuilder::sql_expression(&format!("WHEN_{}", matched_str))
            .with_metadata("action".into(), action_desc.into());
        if let Some(wc) = &when.where_clause {
            when_node = when_node.add_child(expr::convert_expr(wc));
        }
        node = node.add_child(when_node);
    }

    Ok(node)
}

/// Extract table name from a TableRef and add to "tables" attribute.
fn add_table_ref(node: &mut UniversalNode, table_ref: &ogsql_parser::TableRef) {
    use ogsql_parser::TableRef;
    match table_ref {
        TableRef::Table { name, .. } => {
            append_attr(node, "tables", &name.join("."));
        }
        TableRef::Join { left, right, .. } => {
            add_table_ref(node, left);
            add_table_ref(node, right);
        }
        TableRef::Subquery { alias, .. } => {
            if let Some(a) = alias {
                append_attr(node, "tables", a);
            }
        }
        TableRef::Values { alias, .. } => {
            if let Some(a) = alias {
                append_attr(node, "tables", a);
            }
        }
        TableRef::Pivot { source, .. } | TableRef::Unpivot { source, .. } => {
            add_table_ref(node, source);
        }
        TableRef::FunctionCall { name, alias, .. } => {
            let fn_name = name.join(".");
            let display: &str = alias.as_ref().map_or(&fn_name, |a| a.as_str());
            append_attr(node, "tables", display);
        }
    }
}

/// Extract table name from a TableRef and add to a named attribute.
fn add_target_ref(node: &mut UniversalNode, attr: &str, table_ref: &ogsql_parser::TableRef) {
    use ogsql_parser::TableRef;
    match table_ref {
        TableRef::Table { name, alias, .. } => {
            let joined = name.join(".");
            let display: String = alias.as_ref().map_or(joined, |a| a.to_string());
            node.attributes.insert(attr.to_string(), display);
        }
        TableRef::Subquery { alias, .. } => {
            if let Some(a) = alias {
                node.attributes.insert(attr.to_string(), a.to_string());
            }
        }
        _ => {
            node.attributes
                .insert(attr.to_string(), format!("{:?}", table_ref));
        }
    }
}

/// Convert an UpdateAssignment to a SqlExpression node.
fn convert_update_assignment(assign: &ogsql_parser::ast::UpdateAssignment) -> UniversalNode {
    let col_names: Vec<String> = assign.columns.iter().map(|c| c.join(".")).collect();
    let col_str = col_names.join(", ");
    AstBuilder::sql_expression("SET")
        .add_child(expr::convert_expr(&assign.value))
        .with_metadata("column".into(), col_str)
}

/// Append to a comma-separated attribute.
fn append_attr(node: &mut UniversalNode, key: &str, value: &str) {
    let current = node.attributes.get(key).cloned().unwrap_or_default();
    let new_val = if current.is_empty() {
        value.to_string()
    } else {
        format!("{},{}", current, value)
    };
    node.attributes.insert(key.to_string(), new_val);
}

fn convert_select_target(target: &ogsql_parser::ast::SelectTarget) -> UniversalNode {
    match target {
        ogsql_parser::ast::SelectTarget::Expr(expr, alias) => {
            let mut node = expr::convert_expr(expr);
            if let Some(a) = alias {
                node = node.with_metadata("alias".into(), a.to_string());
            }
            node
        }
        ogsql_parser::ast::SelectTarget::Star(qualifier) => {
            let label = if let Some(q) = qualifier {
                format!("{}.*", q)
            } else {
                "*".to_string()
            };
            AstBuilder::sql_expression("STAR").with_metadata("label".into(), label)
        }
    }
}

fn convert_table_ref_node(table_ref: &ogsql_parser::TableRef) -> UniversalNode {
    use ogsql_parser::TableRef;
    match table_ref {
        TableRef::Table { name, alias, .. } => {
            let mut node = AstBuilder::sql_expression("TABLE_REF")
                .with_metadata("name".into(), name.join("."));
            if let Some(a) = alias {
                node = node.with_metadata("alias".into(), a.to_string());
            }
            node
        }
        TableRef::Join { left, right, join_type, .. } => {
            let jt = match join_type {
                ogsql_parser::ast::JoinType::Inner => "INNER",
                ogsql_parser::ast::JoinType::Left => "LEFT",
                ogsql_parser::ast::JoinType::Right => "RIGHT",
                ogsql_parser::ast::JoinType::Full => "FULL",
                ogsql_parser::ast::JoinType::Cross => "CROSS",
            };
            AstBuilder::sql_expression("TABLE_REF")
                .with_metadata("join_type".into(), jt.into())
                .add_child(convert_table_ref_node(left))
                .add_child(convert_table_ref_node(right))
        }
        TableRef::Subquery { alias, .. } => {
            let mut node = AstBuilder::sql_expression("TABLE_REF")
                .with_metadata("kind".into(), "subquery".into());
            if let Some(a) = alias {
                node = node.with_metadata("alias".into(), a.to_string());
            }
            node
        }
        _ => {
            AstBuilder::sql_expression("TABLE_REF")
                .with_metadata("kind".into(), format!("{:?}", table_ref))
        }
    }
}

fn convert_group_by_item(item: &ogsql_parser::ast::GroupByItem) -> UniversalNode {
    match item {
        ogsql_parser::ast::GroupByItem::Expr(e) => {
            AstBuilder::sql_expression("GROUP_KEY").add_child(expr::convert_expr(e))
        }
        ogsql_parser::ast::GroupByItem::GroupingSets(sets) => {
            let mut node = AstBuilder::sql_expression("GROUPING_SETS");
            for set in sets {
                let mut set_node = AstBuilder::sql_expression("GROUP_SET");
                for e in set {
                    set_node = set_node.add_child(expr::convert_expr(e));
                }
                node = node.add_child(set_node);
            }
            node
        }
        ogsql_parser::ast::GroupByItem::Rollup(exprs) => {
            let mut node = AstBuilder::sql_expression("ROLLUP");
            for e in exprs {
                node = node.add_child(expr::convert_expr(e));
            }
            node
        }
        ogsql_parser::ast::GroupByItem::Cube(exprs) => {
            let mut node = AstBuilder::sql_expression("CUBE");
            for e in exprs {
                node = node.add_child(expr::convert_expr(e));
            }
            node
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    fn parse_to_node(sql: &str) -> UniversalNode {
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        let stmt = &stmts[0];
        match stmt {
            ogsql_parser::Statement::Select(spanned) => convert_select(spanned).unwrap(),
            ogsql_parser::Statement::Insert(spanned) => convert_insert(spanned).unwrap(),
            ogsql_parser::Statement::Update(spanned) => convert_update(spanned).unwrap(),
            ogsql_parser::Statement::Delete(spanned) => convert_delete(spanned).unwrap(),
            ogsql_parser::Statement::Merge(spanned) => convert_merge(spanned).unwrap(),
            _ => panic!("unexpected statement type: {:?}", stmt),
        }
    }

    // ── SELECT tests ──

    #[test]
    fn test_select_star() {
        let node = parse_to_node("SELECT * FROM users");
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("tables"), Some(&"users".to_string()));
    }

    #[test]
    fn test_select_with_where() {
        let node = parse_to_node("SELECT id, name FROM users WHERE id = 1");
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(node.get_attribute("tables"), Some(&"users".to_string()));
        assert!(node.child_count() >= 3);
        assert_eq!(node.child(0).unwrap().node_type(), "sql_expression");
        assert_eq!(node.child(1).unwrap().node_type(), "sql_expression");
        assert_eq!(node.child(2).unwrap().node_type(), "binary_expression");
    }

    #[test]
    fn test_select_join() {
        let node = parse_to_node("SELECT a FROM t1 JOIN t2 ON t1.id = t2.id");
        assert_eq!(node.node_type(), "select_statement");
        // Both tables should be in the attribute
        let tables = node.get_attribute("tables").unwrap();
        assert!(
            tables.contains("t1"),
            "tables should contain t1, got: {}",
            tables
        );
        assert!(
            tables.contains("t2"),
            "tables should contain t2, got: {}",
            tables
        );
    }

    #[test]
    fn test_select_set_operation() {
        let node = parse_to_node("SELECT a FROM t1 UNION SELECT b FROM t2");
        assert_eq!(node.node_type(), "select_statement");
        assert_eq!(
            node.get_attribute("set_operation"),
            Some(&"UNION".to_string())
        );
    }

    #[test]
    fn test_select_with_distinct() {
        let node = parse_to_node("SELECT DISTINCT name FROM users");
        assert_eq!(node.get_attribute("distinct"), Some(&"true".to_string()));
    }

    #[test]
    fn test_select_with_order_by() {
        let node = parse_to_node("SELECT * FROM users ORDER BY id");
        assert_eq!(
            node.get_attribute("has_order_by"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_select_with_limit() {
        let node = parse_to_node("SELECT * FROM users LIMIT 10");
        assert_eq!(node.get_attribute("has_limit"), Some(&"true".to_string()));
    }

    #[test]
    fn test_select_multiple_tables() {
        let node = parse_to_node("SELECT * FROM t1, t2");
        let tables = node.get_attribute("tables").unwrap();
        assert!(tables.contains("t1"));
        assert!(tables.contains("t2"));
    }

    // ── INSERT tests ──

    #[test]
    fn test_insert_values() {
        let node = parse_to_node("INSERT INTO users (id, name) VALUES (1, 'Alice')");
        assert_eq!(node.node_type(), "insert_statement");
        assert_eq!(node.get_attribute("table"), Some(&"users".to_string()));
        assert_eq!(node.get_attribute("columns"), Some(&"id,name".to_string()));
        assert_eq!(
            node.get_attribute("source_type"),
            Some(&"VALUES".to_string())
        );
    }

    #[test]
    fn test_insert_select() {
        let node = parse_to_node("INSERT INTO users SELECT * FROM temp");
        assert_eq!(node.node_type(), "insert_statement");
        assert_eq!(
            node.get_attribute("source_type"),
            Some(&"SELECT".to_string())
        );
        // Should have a select child
        assert_eq!(node.child_count(), 1);
        assert_eq!(node.child(0).unwrap().node_type(), "select_statement");
    }

    #[test]
    fn test_insert_on_duplicate_key() {
        let node = parse_to_node("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE id = 1");
        assert_eq!(
            node.get_attribute("on_duplicate_key"),
            Some(&"true".to_string())
        );
    }

    // ── UPDATE tests ──

    #[test]
    fn test_update_with_where() {
        let node = parse_to_node("UPDATE users SET name = 'Bob' WHERE id = 1");
        assert_eq!(node.node_type(), "update_statement");
        let tables = node.get_attribute("tables").unwrap();
        assert!(tables.contains("users"), "tables should contain users");
        // Has WHERE child
        assert!(
            node.child_count() >= 2,
            "expected at least 2 children (SET + WHERE)"
        );
    }

    #[test]
    fn test_update_without_where() {
        let node = parse_to_node("UPDATE users SET name = 'Bob'");
        assert_eq!(node.node_type(), "update_statement");
        let tables = node.get_attribute("tables").unwrap();
        assert!(tables.contains("users"));
        // No WHERE child
        assert_eq!(node.child_count(), 1, "expected 1 child (SET only)");
    }

    #[test]
    fn test_update_returning() {
        let node = parse_to_node("UPDATE users SET name = 'Bob' WHERE id = 1 RETURNING id");
        assert_eq!(
            node.get_attribute("has_returning"),
            Some(&"true".to_string())
        );
    }

    // ── DELETE tests ──

    #[test]
    fn test_delete_with_where() {
        let node = parse_to_node("DELETE FROM users WHERE id = 1");
        assert_eq!(node.node_type(), "delete_statement");
        let tables = node.get_attribute("tables").unwrap();
        assert!(tables.contains("users"));
        assert_eq!(node.child_count(), 1);
        assert_eq!(node.child(0).unwrap().node_type(), "binary_expression");
    }

    #[test]
    fn test_delete_without_where() {
        let node = parse_to_node("DELETE FROM users");
        assert_eq!(node.node_type(), "delete_statement");
        let tables = node.get_attribute("tables").unwrap();
        assert!(tables.contains("users"));
        // No WHERE child — security concern (DELETE without WHERE)
        assert_eq!(node.child_count(), 0, "expected 0 children (no WHERE)");
    }

    #[test]
    fn test_delete_returning() {
        let node = parse_to_node("DELETE FROM users WHERE id = 1 RETURNING id");
        assert_eq!(
            node.get_attribute("has_returning"),
            Some(&"true".to_string())
        );
    }

    // ── MERGE tests ──

    #[test]
    fn test_merge_update() {
        let sql = "MERGE INTO target t USING source s ON t.id = s.id \
                   WHEN MATCHED THEN UPDATE SET t.name = s.name";
        let node = parse_to_node(sql);
        assert_eq!(node.node_type(), "merge_statement");
        assert!(node.get_attribute("target_table").is_some());
        assert!(node.get_attribute("source_table").is_some());
        // ON condition as child
        assert!(node.child_count() >= 1);
        // Look for WHEN children
        let when_nodes: Vec<_> = node
            .children
            .iter()
            .filter(|c| c.get_attribute("action").is_some())
            .collect();
        assert!(!when_nodes.is_empty(), "expected WHEN clause child");
    }

    #[test]
    fn test_merge_insert() {
        let sql = "MERGE INTO target t USING source s ON t.id = s.id \
                   WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)";
        let node = parse_to_node(sql);
        assert_eq!(node.node_type(), "merge_statement");
        let when_nodes: Vec<_> = node
            .children
            .iter()
            .filter(|c| c.get_attribute("action").is_some())
            .collect();
        assert!(!when_nodes.is_empty(), "expected WHEN clause child");
    }

    #[test]
    fn test_merge_both_branches() {
        let sql = "MERGE INTO target t USING source s ON t.id = s.id \
                   WHEN MATCHED THEN UPDATE SET t.name = s.name \
                   WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)";
        let node = parse_to_node(sql);
        assert_eq!(node.node_type(), "merge_statement");
        let when_nodes: Vec<_> = node
            .children
            .iter()
            .filter(|c| c.get_attribute("action").is_some())
            .collect();
        assert_eq!(when_nodes.len(), 2, "expected 2 WHEN clauses");
    }

    // ── SELECT INTO / FOR UPDATE tests ──

    #[test]
    fn test_select_into_for_update_metadata() {
        let node = parse_to_node("SELECT cnt FROM accounts WHERE id = 1 FOR UPDATE");
        // lock_clause works standalone
        assert_eq!(node.get_attribute("has_lock"), Some(&"true".to_string()));
        assert_eq!(node.get_attribute("lock_type"), Some(&"Update".to_string()));
        // into_targets only populated in PL/pgSQL context (tested in pl.rs after Phase B)
    }

    #[test]
    fn test_select_without_lock() {
        let node = parse_to_node("SELECT cnt FROM accounts WHERE id = 1");
        assert!(node.get_attribute("has_lock").is_none());
    }

    #[test]
    fn test_select_for_update_without_into() {
        let node = parse_to_node("SELECT cnt FROM accounts WHERE id = 1 FOR UPDATE");
        assert!(node.get_attribute("has_into").is_none());
        assert_eq!(node.get_attribute("has_lock"), Some(&"true".to_string()));
    }

    #[test]
    fn test_select_for_update_nowait_metadata() {
        let node = parse_to_node("SELECT cnt FROM t FOR UPDATE NOWAIT");
        assert_eq!(node.get_attribute("has_lock"), Some(&"true".to_string()));
        assert_eq!(node.get_attribute("lock_nowait"), Some(&"true".to_string()));
    }

    #[test]
    fn test_select_for_share_metadata() {
        let node = parse_to_node("SELECT cnt FROM t FOR SHARE");
        assert_eq!(node.get_attribute("lock_type"), Some(&"Share".to_string()));
    }

    #[test]
    fn test_select_bulk_collect_metadata() {
        let node = parse_to_node("SELECT cnt BULK COLLECT INTO v FROM t FOR UPDATE");
        assert_eq!(
            node.get_attribute("bulk_collect"),
            Some(&"true".to_string())
        );
        assert_eq!(node.get_attribute("has_into"), Some(&"true".to_string()));
        assert_eq!(node.get_attribute("has_lock"), Some(&"true".to_string()));
    }
}
