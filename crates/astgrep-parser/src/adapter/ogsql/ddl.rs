//! DDL conversion: ogsql DDL structs → UniversalNode.
//! Maps CREATE TABLE/INDEX/VIEW/FUNCTION/PROCEDURE/PACKAGE, DROP, ALTER TABLE.

use super::OgsqlAdapterError;
use astgrep_ast::{AstBuilder, NodeType, UniversalNode};
use astgrep_core::AstNode;

use super::pl;

pub fn convert_create_table(
    stmt: &ogsql_parser::CreateTableStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_table_statement()
        .with_metadata("table_name".into(), stmt.name.join("."));
    for col in &stmt.columns {
        append_attr(
            &mut n,
            "columns",
            &format!("{} {}", col.name, data_type_to_string(&col.data_type)),
        );
        for c in &col.constraints {
            append_attr(&mut n, "constraints", &column_constraint_to_string(c));
        }
    }
    for tc in &stmt.constraints {
        append_attr(&mut n, "constraints", &table_constraint_to_string(tc));
    }
    if stmt.temporary {
        n = n.with_metadata("temporary".into(), "true".into());
    }
    if stmt.if_not_exists {
        n = n.with_metadata("if_not_exists".into(), "true".into());
    }
    if stmt.partition_by.is_some() {
        n = n.with_metadata("has_partition".into(), "true".into());
    }
    for (k, v) in &stmt.options {
        n = n.with_metadata(format!("option_{}", k), v.clone());
    }
    Ok(n)
}

pub fn convert_create_index(
    stmt: &ogsql_parser::CreateIndexStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_index_statement()
        .with_metadata("table_name".into(), stmt.table.join("."));
    if let Some(name) = &stmt.name {
        n = n.with_metadata("index_name".into(), name.join("."));
    }
    let cols: Vec<_> = stmt
        .columns
        .iter()
        .map(|c| {
            c.name.clone().unwrap_or_else(|| {
                c.expr
                    .as_ref()
                    .map(|e| format!("{:?}", e))
                    .unwrap_or_else(|| "?".into())
            })
        })
        .collect();
    if !cols.is_empty() {
        n = n.with_metadata("columns".into(), cols.join(","));
    }
    if stmt.unique {
        n = n.with_metadata("unique".into(), "true".into());
    }
    if stmt.if_not_exists {
        n = n.with_metadata("if_not_exists".into(), "true".into());
    }
    if stmt.concurrent {
        n = n.with_metadata("concurrent".into(), "true".into());
    }
    Ok(n)
}

pub fn convert_create_global_index(
    stmt: &ogsql_parser::CreateGlobalIndexStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_index_statement()
        .with_metadata("table_name".into(), stmt.table.join("."))
        .with_metadata("index_type".into(), "GLOBAL".into());
    if let Some(name) = &stmt.name {
        n = n.with_metadata("index_name".into(), name.join("."));
    }
    let cols: Vec<_> = stmt.columns.iter().map(|c| c.name.clone()).collect();
    if !cols.is_empty() {
        n = n.with_metadata("columns".into(), cols.join(","));
    }
    if stmt.unique {
        n = n.with_metadata("unique".into(), "true".into());
    }
    Ok(n)
}

pub fn convert_create_view(
    stmt: &ogsql_parser::CreateViewStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n =
        AstBuilder::create_view_statement().with_metadata("view_name".into(), stmt.name.join("."));
    if !stmt.columns.is_empty() {
        n = n.with_metadata("columns".into(), stmt.columns.join(","));
    }
    n = n.add_child(super::dml::convert_select(&stmt.query)?);
    if stmt.replace {
        n = n.with_metadata("or_replace".into(), "true".into());
    }
    if stmt.temporary {
        n = n.with_metadata("temporary".into(), "true".into());
    }
    Ok(n)
}

pub fn convert_create_function(
    stmt: &ogsql_parser::ast::CreateFunctionStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_function_statement()
        .with_metadata("function_name".into(), stmt.name.join("."));
    if !stmt.parameters.is_empty() {
        let params: Vec<_> = stmt
            .parameters
            .iter()
            .map(|p| {
                let m = p
                    .mode
                    .as_ref()
                    .map(|m| format!("{} ", m))
                    .unwrap_or_default();
                format!("{}{} {}", m, p.name, p.data_type)
            })
            .collect();
        n = n.with_metadata("parameters".into(), params.join("; "));
    }
    if let Some(ref rt) = stmt.return_type {
        n = n.with_metadata("return_type".into(), rt.clone());
    }
    if let Some(ref lang) = stmt.options.language {
        n = n.with_metadata("language".into(), lang.clone());
    }
    if stmt.replace {
        n = n.with_metadata("or_replace".into(), "true".into());
    }
    if let Some(ref block) = stmt.block {
        use crate::adapter::ogsql::pl;
        n = n.add_child(pl::convert_pl_block(block, NodeType::BlockStatement, None)?);
    }
    Ok(n)
}

pub fn convert_create_procedure(
    stmt: &ogsql_parser::ast::CreateProcedureStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_procedure_statement()
        .with_metadata("procedure_name".into(), stmt.name.join("."));
    if !stmt.parameters.is_empty() {
        let params: Vec<_> = stmt
            .parameters
            .iter()
            .map(|p| {
                let m = p
                    .mode
                    .as_ref()
                    .map(|m| format!("{} ", m))
                    .unwrap_or_default();
                format!("{}{} {}", m, p.name, p.data_type)
            })
            .collect();
        n = n.with_metadata("parameters".into(), params.join("; "));
    }
    if let Some(ref lang) = stmt.options.language {
        n = n.with_metadata("language".into(), lang.clone());
    }
    if stmt.replace {
        n = n.with_metadata("or_replace".into(), "true".into());
    }
    // Add PL/pgSQL block body as child for TreeMatcher matching
    if let Some(ref block) = stmt.block {
        use crate::adapter::ogsql::pl;
        n = n.add_child(pl::convert_pl_block(block, NodeType::BlockStatement, None)?);
    }
    Ok(n)
}

pub fn convert_create_package(
    stmt: &ogsql_parser::ast::CreatePackageStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_package_statement()
        .with_metadata("package_name".into(), stmt.name.join("."));
    for item in &stmt.items {
        for child in convert_package_item(item)? {
            n = n.add_child(child);
        }
    }
    if stmt.replace {
        n = n.with_metadata("or_replace".into(), "true".into());
    }
    Ok(n)
}

pub fn convert_create_package_body(
    stmt: &ogsql_parser::ast::CreatePackageBodyStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_package_statement()
        .with_metadata("package_name".into(), stmt.name.join("."))
        .with_metadata("is_body".into(), "true".into());
    for item in &stmt.items {
        for child in convert_package_item(item)? {
            n = n.add_child(child);
        }
    }
    if stmt.replace {
        n = n.with_metadata("or_replace".into(), "true".into());
    }
    Ok(n)
}

/// Convert a package item, returning one or more UniversalNode children.
///
/// For procedures/functions with a body block, returns both a metadata node
/// (PACKAGE_PROCEDURE / PACKAGE_FUNCTION) and the BlockStatement as a sibling
/// so the pattern matcher can traverse into the body statements at the same
/// depth as standalone CREATE PROCEDURE (CREATE_PROCEDURE → BlockStatement).
fn convert_package_item(
    item: &ogsql_parser::ast::PackageItem,
) -> Result<Vec<UniversalNode>, OgsqlAdapterError> {
    use ogsql_parser::ast::PackageItem;
    match item {
        PackageItem::Procedure(proc) => {
            let mut nodes = Vec::new();
            if let Some(ref block) = proc.block {
                let mut block_node = pl::convert_pl_block(block, NodeType::BlockStatement, None)?
                    .with_metadata("package_procedure_name".into(), proc.name.join("."))
                    .with_metadata(
                        "package_procedure_params".into(),
                        proc.parameters
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.data_type))
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                // Set location from ogsql parser line info (if available)
                if proc.start_line > 0 {
                    let loc = (proc.start_line, 1, proc.end_line.max(proc.start_line), 1);
                    block_node.location = Some(loc);
                    for child in block_node.children.iter_mut() {
                        if child.location.is_none() {
                            child.location = Some(loc);
                        }
                    }
                }
                // Filter out variable declarations so the target statements (SELECT,
                // assignment, UPDATE) appear as consecutive siblings starting at index 0.
                block_node
                    .children
                    .retain(|c| c.node_type() != "variable_declaration");
                nodes.push(block_node);
            } else {
                let meta = AstBuilder::sql_expression("PACKAGE_PROCEDURE")
                    .with_metadata("name".into(), proc.name.join("."))
                    .with_metadata(
                        "parameters".into(),
                        proc.parameters
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.data_type))
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                nodes.push(meta);
            }
            Ok(nodes)
        }
        PackageItem::Function(func) => {
            let mut nodes = Vec::new();
            if let Some(ref block) = func.block {
                let mut block_node = pl::convert_pl_block(block, NodeType::BlockStatement, None)?
                    .with_metadata("package_function_name".into(), func.name.join("."))
                    .with_metadata(
                        "package_function_params".into(),
                        func.parameters
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.data_type))
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                if let Some(ref rt) = func.return_type {
                    block_node = block_node.with_metadata("return_type".into(), rt.clone());
                }
                nodes.push(block_node);
            } else {
                let mut meta = AstBuilder::sql_expression("PACKAGE_FUNCTION")
                    .with_metadata("name".into(), func.name.join("."))
                    .with_metadata(
                        "parameters".into(),
                        func.parameters
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.data_type))
                            .collect::<Vec<_>>()
                            .join("; "),
                    );
                if let Some(ref rt) = func.return_type {
                    meta = meta.with_metadata("return_type".into(), rt.clone());
                }
                nodes.push(meta);
            }
            Ok(nodes)
        }
        PackageItem::Variable(_) => Ok(vec![AstBuilder::sql_expression("PACKAGE_VARIABLE")]),
        PackageItem::Type(_) => Ok(vec![AstBuilder::sql_expression("PACKAGE_TYPE")]),
        PackageItem::Cursor(c) => Ok(vec![AstBuilder::sql_expression("PACKAGE_CURSOR")
            .with_metadata("name".into(), c.name.clone())]),
        PackageItem::Raw(text) => Ok(vec![AstBuilder::sql_expression("PACKAGE_RAW")
            .with_metadata(
                "text".into(),
                if text.len() > 80 {
                    format!("{}...", &text[..80])
                } else {
                    text.clone()
                },
            )]),
    }
}

pub fn convert_drop(
    stmt: &ogsql_parser::DropStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::drop_statement()
        .with_metadata(
            "object_type".into(),
            format!("{:?}", stmt.object_type).to_uppercase(),
        )
        .with_metadata(
            "names".into(),
            stmt.names
                .iter()
                .map(|n| n.join("."))
                .collect::<Vec<_>>()
                .join(","),
        );
    if stmt.if_exists {
        n = n.with_metadata("if_exists".into(), "true".into());
    }
    if stmt.cascade {
        n = n.with_metadata("cascade".into(), "true".into());
    }
    if stmt.purge {
        n = n.with_metadata("purge".into(), "true".into());
    }
    Ok(n)
}

pub fn convert_alter_table(
    stmt: &ogsql_parser::AlterTableStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n =
        AstBuilder::alter_statement().with_metadata("table_name".into(), stmt.name.join("."));
    if stmt.if_exists {
        n = n.with_metadata("if_exists".into(), "true".into());
    }
    for action in &stmt.actions {
        append_attr(&mut n, "actions", &alter_action_to_string(action));
    }
    Ok(n)
}

// ---- helper functions ----
fn data_type_to_string(dt: &ogsql_parser::DataType) -> String {
    use ogsql_parser::DataType;
    match dt {
        DataType::Boolean => "BOOLEAN",
        DataType::TinyInt(None) => "TINYINT",
        DataType::TinyInt(Some(n)) => return format!("TINYINT({})", n),
        DataType::SmallInt(_) => "SMALLINT",
        DataType::Integer(None) => "INTEGER",
        DataType::Integer(Some(n)) => return format!("INTEGER({})", n),
        DataType::BigInt(_) => "BIGINT",
        DataType::Real => "REAL",
        DataType::Double => "DOUBLE PRECISION",
        DataType::Serial => "SERIAL",
        DataType::SmallSerial => "SMALLSERIAL",
        DataType::BigSerial => "BIGSERIAL",
        DataType::Float(_) => "FLOAT",
        DataType::Text => "TEXT",
        DataType::Bytea => "BYTEA",
        DataType::Date => "DATE",
        DataType::Json => "JSON",
        DataType::Jsonb => "JSONB",
        DataType::Uuid => "UUID",
        DataType::BinaryFloat => "BINARY_FLOAT",
        DataType::BinaryDouble => "BINARY_DOUBLE",
        DataType::Char(None) => "CHAR",
        DataType::Char(Some(n)) => return format!("CHAR({})", n),
        DataType::Varchar(None) => "VARCHAR",
        DataType::Varchar(Some(n)) => return format!("VARCHAR({})", n),
        DataType::Numeric(_, _) => "NUMERIC",
        DataType::Timestamp(_, _) => "TIMESTAMP",
        DataType::Timestamptz(_) => "TIMESTAMPTZ",
        DataType::Time(_, _) => "TIME",
        DataType::Interval(_) => "INTERVAL",
        DataType::Bit(_) => "BIT",
        DataType::Varbit(_) => "VARBIT",
        DataType::Array(inner) => return format!("{}[]", data_type_to_string(inner)),
        DataType::Custom(name, _) => return name.join("."),
    }
    .to_string()
}

fn column_constraint_to_string(c: &ogsql_parser::ColumnConstraint) -> String {
    use ogsql_parser::ColumnConstraint;
    match c {
        ColumnConstraint::NotNull => "NOT NULL",
        ColumnConstraint::Null => "NULL",
        ColumnConstraint::Default(_) => "DEFAULT",
        ColumnConstraint::Unique => "UNIQUE",
        ColumnConstraint::PrimaryKey => "PRIMARY KEY",
        ColumnConstraint::Check(_) => "CHECK",
        ColumnConstraint::References {
            ref_table,
            ref_columns,
            ..
        } => {
            return format!(
                "REFERENCES {}({})",
                ref_table.join("."),
                ref_columns.join(",")
            )
        }
    }
    .to_string()
}

fn table_constraint_to_string(tc: &ogsql_parser::TableConstraint) -> String {
    use ogsql_parser::TableConstraint;
    match tc {
        TableConstraint::PrimaryKey { columns, .. } => {
            format!("PRIMARY KEY ({})", columns.join(","))
        }
        TableConstraint::Unique { columns, .. } => format!("UNIQUE ({})", columns.join(",")),
        TableConstraint::Check(_) => "CHECK".into(),
        TableConstraint::ForeignKey {
            columns,
            ref_table,
            ref_columns,
            ..
        } => {
            format!(
                "FOREIGN KEY ({}) REFERENCES {}({})",
                columns.join(","),
                ref_table.join("."),
                ref_columns.join(",")
            )
        }
    }
}

fn alter_action_to_string(action: &ogsql_parser::AlterTableAction) -> String {
    use ogsql_parser::AlterTableAction;
    match action {
        AlterTableAction::AddColumn(col) => format!(
            "ADD COLUMN {} {}",
            col.name,
            data_type_to_string(&col.data_type)
        ),
        AlterTableAction::AddColumns(cols) => format!(
            "ADD COLUMNS ({})",
            cols.iter()
                .map(|c| format!("{} {}", c.name, data_type_to_string(&c.data_type)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        AlterTableAction::DropColumn {
            name, if_exists, ..
        } => format!(
            "DROP COLUMN{}{}",
            if *if_exists { " IF EXISTS" } else { "" },
            name
        ),
        AlterTableAction::AlterColumn { name, .. } => format!("ALTER COLUMN {}", name),
        AlterTableAction::AddConstraint { constraint, .. } => {
            format!("ADD {}", table_constraint_to_string(constraint))
        }
        AlterTableAction::DropConstraint { name, .. } => format!("DROP CONSTRAINT {}", name),
        AlterTableAction::RenameColumn { old, new } => format!("RENAME COLUMN {} TO {}", old, new),
        AlterTableAction::RenameTo { new_name } => format!("RENAME TO {}", new_name),
        AlterTableAction::OwnerTo { owner } => format!("OWNER TO {}", owner),
        AlterTableAction::SetSchema { schema } => format!("SET SCHEMA {}", schema),
        AlterTableAction::SetTablespace { tablespace } => format!("SET TABLESPACE {}", tablespace),
        _ => format!("{:?}", action),
    }
}

fn append_attr(node: &mut UniversalNode, key: &str, value: &str) {
    let cur = node.attributes.get(key).cloned().unwrap_or_default();
    node.attributes.insert(
        key.into(),
        if cur.is_empty() {
            value.into()
        } else {
            format!("{},{}", cur, value)
        },
    );
}

// ── CREATE SEQUENCE ──

pub fn convert_create_sequence(
    stmt: &ogsql_parser::ast::CreateSequenceStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::create_sequence_statement()
        .with_metadata("sequence_name".into(), stmt.name.join("."));
    if stmt.if_not_exists {
        n = n.with_metadata("if_not_exists".into(), "true".into());
    }
    if let Some(ref start) = stmt.start {
        n = n.with_metadata("start".into(), format!("{:?}", start));
    }
    if let Some(ref inc) = stmt.increment {
        n = n.with_metadata("increment".into(), format!("{:?}", inc));
    }
    if let Some(ref max) = stmt.max_value {
        n = n.with_metadata("max_value".into(), format!("{:?}", max));
    }
    if let Some(ref min) = stmt.min_value {
        n = n.with_metadata("min_value".into(), format!("{:?}", min));
    }
    if let Some(ref cache) = stmt.cache {
        n = n.with_metadata("cache".into(), format!("{:?}", cache));
    }
    if stmt.cycle {
        n = n.with_metadata("cycle".into(), "true".into());
    }
    if let Some(ref owned) = stmt.owned_by {
        n = n.with_metadata("owned_by".into(), owned.join("."));
    }
    Ok(n)
}

// ── CREATE TYPE ──

pub fn convert_create_type(
    stmt: &ogsql_parser::ast::CreateTypeStatement,
) -> Result<UniversalNode, OgsqlAdapterError> {
    let mut n = AstBuilder::sql_expression("CREATE TYPE")
        .with_metadata("type_name".into(), stmt.name.join("."));
    match &stmt.type_kind {
        ogsql_parser::ast::TypeKind::Composite { attributes } => {
            n = n.with_metadata("type_kind".into(), "composite".into());
            for attr in attributes {
                append_attr(
                    &mut n,
                    "attributes",
                    &format!("{} {}", attr.name, data_type_to_string(&attr.data_type)),
                );
            }
        }
        ogsql_parser::ast::TypeKind::Enum { labels } => {
            n = n.with_metadata("type_kind".into(), "enum".into());
            for label in labels {
                append_attr(&mut n, "labels", label);
            }
        }
        ogsql_parser::ast::TypeKind::Base { options } => {
            n = n.with_metadata("type_kind".into(), "base".into());
            for (k, v) in options {
                n = n.with_metadata(format!("option_{}", k).into(), v.clone());
            }
        }
        ogsql_parser::ast::TypeKind::Table { element_type } => {
            n = n.with_metadata("type_kind".into(), "table".into())
                .with_metadata("element_type".into(), element_type.clone());
        }
        ogsql_parser::ast::TypeKind::Range { options } => {
            n = n.with_metadata("type_kind".into(), "range".into());
            for (k, v) in options {
                n = n.with_metadata(format!("option_{}", k).into(), v.clone());
            }
        }
        ogsql_parser::ast::TypeKind::Shell => {
            n = n.with_metadata("type_kind".into(), "shell".into());
        }
    }
    Ok(n)
}

// ---- tests ----
#[cfg(test)]
mod tests {
    use super::*;
    use astgrep_core::AstNode;

    fn p(sql: &str) -> UniversalNode {
        let tokens = ogsql_parser::token::tokenizer::Tokenizer::new(sql)
            .tokenize()
            .unwrap();
        let stmts = ogsql_parser::parser::Parser::new(tokens).parse();
        let stmt = &stmts[0];
        match stmt {
            ogsql_parser::Statement::CreateTable(s) => convert_create_table(s).unwrap(),
            ogsql_parser::Statement::CreateIndex(s) => convert_create_index(s).unwrap(),
            ogsql_parser::Statement::CreateGlobalIndex(s) => {
                convert_create_global_index(s).unwrap()
            }
            ogsql_parser::Statement::CreateView(s) => convert_create_view(s).unwrap(),
            ogsql_parser::Statement::CreateFunction(s) => convert_create_function(s).unwrap(),
            ogsql_parser::Statement::CreateProcedure(s) => convert_create_procedure(s).unwrap(),
            ogsql_parser::Statement::CreatePackage(s) => convert_create_package(s).unwrap(),
            ogsql_parser::Statement::CreatePackageBody(s) => {
                convert_create_package_body(s).unwrap()
            }
            ogsql_parser::Statement::Drop(s) => convert_drop(s).unwrap(),
            ogsql_parser::Statement::AlterTable(s) => convert_alter_table(s).unwrap(),
            _ => panic!("unexpected: {:?}", stmt),
        }
    }

    #[test]
    fn test_create_table_basic() {
        let n = p("CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL)");
        assert_eq!("create_table_statement", n.node_type());
        assert_eq!(Some(&"users".to_string()), n.get_attribute("table_name"));
        assert!(n
            .get_attribute("columns")
            .unwrap()
            .contains("name VARCHAR(100)"));
        assert!(n
            .get_attribute("constraints")
            .unwrap()
            .contains("PRIMARY KEY"));
    }
    #[test]
    fn test_create_table_fk() {
        let n = p("CREATE TABLE orders (id INT, user_id INT REFERENCES users(id))");
        assert!(n
            .get_attribute("constraints")
            .unwrap()
            .contains("REFERENCES users(id)"));
    }
    #[test]
    fn test_create_table_storage() {
        let n = p("CREATE TABLE t (a INT) WITH (storage_type=ustore)");
        assert_eq!(
            Some(&"ustore".to_string()),
            n.get_attribute("option_storage_type")
        );
    }
    #[test]
    fn test_create_table_flags() {
        let n = p("CREATE TEMPORARY TABLE IF NOT EXISTS t (a INT)");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("temporary"));
        assert_eq!(Some(&"true".to_string()), n.get_attribute("if_not_exists"));
    }
    #[test]
    fn test_create_table_constraints() {
        let n = p("CREATE TABLE t (id INT, user_id INT, PRIMARY KEY (id), FOREIGN KEY (user_id) REFERENCES users(id))");
        assert!(n
            .get_attribute("constraints")
            .unwrap()
            .contains("PRIMARY KEY (id)"));
        assert!(n
            .get_attribute("constraints")
            .unwrap()
            .contains("FOREIGN KEY (user_id) REFERENCES users(id)"));
    }
    #[test]
    fn test_create_index_basic() {
        let n = p("CREATE INDEX idx_name ON users(name)");
        assert_eq!(Some(&"users".to_string()), n.get_attribute("table_name"));
        assert_eq!(Some(&"idx_name".to_string()), n.get_attribute("index_name"));
        assert!(n.get_attribute("columns").unwrap().contains("name"));
    }
    #[test]
    fn test_create_unique_index() {
        let n = p("CREATE UNIQUE INDEX idx_email ON users(email)");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("unique"));
    }
    #[test]
    fn test_create_global_index() {
        let n = p("CREATE GLOBAL INDEX idx_global ON orders(id)");
        assert_eq!(Some(&"GLOBAL".to_string()), n.get_attribute("index_type"));
    }
    #[test]
    fn test_create_view_basic() {
        let n = p("CREATE VIEW active_users AS SELECT * FROM users WHERE active = true");
        assert_eq!(
            Some(&"active_users".to_string()),
            n.get_attribute("view_name")
        );
        assert_eq!(1, n.child_count());
        assert_eq!("select_statement", n.child(0).unwrap().node_type());
    }
    #[test]
    fn test_create_view_or_replace() {
        let n = p("CREATE OR REPLACE VIEW v AS SELECT 1");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("or_replace"));
    }
    #[test]
    fn test_create_function_basic() {
        let n = p("CREATE FUNCTION add(a INT, b INT) RETURNS INT LANGUAGE plpgsql");
        assert_eq!(Some(&"add".to_string()), n.get_attribute("function_name"));
        assert!(n
            .get_attribute("return_type")
            .unwrap()
            .to_lowercase()
            .contains("int"));
    }
    #[test]
    fn test_create_function_or_replace() {
        let n = p("CREATE OR REPLACE FUNCTION f() RETURNS INT LANGUAGE plpgsql");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("or_replace"));
    }
    #[test]
    fn test_create_procedure_basic() {
        let n = p("CREATE PROCEDURE proc1(x INT) LANGUAGE plpgsql");
        assert_eq!(
            Some(&"proc1".to_string()),
            n.get_attribute("procedure_name")
        );
    }
    #[test]
    fn test_create_package_basic() {
        let n = p("CREATE PACKAGE my_pkg AS FUNCTION f1 RETURN INT; PROCEDURE p1; END my_pkg");
        assert_eq!(Some(&"my_pkg".to_string()), n.get_attribute("package_name"));
    }
    #[test]
    fn test_drop_table() {
        let n = p("DROP TABLE users");
        assert_eq!(Some(&"TABLE".to_string()), n.get_attribute("object_type"));
        assert_eq!(Some(&"users".to_string()), n.get_attribute("names"));
    }
    #[test]
    fn test_drop_if_exists() {
        let n = p("DROP TABLE IF EXISTS temp");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("if_exists"));
    }
    #[test]
    fn test_drop_index() {
        let n = p("DROP INDEX idx_name");
        assert_eq!(Some(&"INDEX".to_string()), n.get_attribute("object_type"));
    }
    #[test]
    fn test_drop_cascade() {
        let n = p("DROP TABLE t CASCADE");
        assert_eq!(Some(&"true".to_string()), n.get_attribute("cascade"));
    }
    #[test]
    fn test_alter_table_add_column() {
        let n = p("ALTER TABLE users ADD COLUMN email VARCHAR(255)");
        assert_eq!(Some(&"users".to_string()), n.get_attribute("table_name"));
        assert!(n
            .get_attribute("actions")
            .unwrap()
            .contains("ADD COLUMN email"));
    }
    #[test]
    fn test_alter_table_drop_column() {
        let n = p("ALTER TABLE users DROP COLUMN old_col");
        assert!(n.get_attribute("actions").unwrap().contains("DROP COLUMN"));
    }
    #[test]
    fn test_alter_table_rename_column() {
        let n = p("ALTER TABLE users RENAME COLUMN old TO new");
        assert!(n
            .get_attribute("actions")
            .unwrap()
            .contains("RENAME COLUMN old TO new"));
    }
    #[test]
    fn test_create_package_body_with_procedure() {
        let n = p("CREATE OR REPLACE PACKAGE BODY my_pkg AS \
             PROCEDURE do_update IS v_cnt INTEGER; \
             BEGIN \
               SELECT cnt INTO v_cnt FROM t WHERE id = 1 FOR UPDATE; \
               v_cnt := v_cnt + 1; \
               UPDATE t SET cnt = v_cnt WHERE id = 1; \
             END do_update; \
             END my_pkg;");
        assert_eq!(Some(&"my_pkg".to_string()), n.get_attribute("package_name"));
        assert_eq!(Some(&"true".to_string()), n.get_attribute("is_body"));
        assert_eq!(Some(&"true".to_string()), n.get_attribute("or_replace"));
        assert_eq!(1, n.children.len(), "expected 1 child (BlockStatement)");
        let block = &n.children[0];
        assert_eq!("block_statement", block.node_type());
        assert_eq!(
            3,
            block.children.len(),
            "expected 3 body statements after filtering declarations"
        );
    }
    #[test]
    fn test_alter_table_constraint() {
        let n = p("ALTER TABLE t ADD CONSTRAINT pk PRIMARY KEY (id)");
        assert!(n
            .get_attribute("actions")
            .unwrap()
            .contains("ADD PRIMARY KEY (id)"));
    }
}
