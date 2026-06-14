# SQL Dialect Support

astgrep supports multi-dialect SQL analysis. Each dialect uses a specialized parser for accurate AST construction, enabling dialect-aware rule matching.

## Supported Dialects

| Dialect | `--dialect` value | Parser | Coverage |
|---|---|---|---|
| **Standard SQL** | `standard` (default) | tree-sitter-sequel 0.3.11 | Generic SQL (ANSI) |
| **GaussDB** | `gaussdb` | ogsql-parser v0.6.20 | Full GaussDB DML/DDL + PREDICT BY / TIMECAPSULE / SHRINK / Plan Hints |
| **OpenGauss** | `opengauss` | ogsql-parser v0.6.20 | Shares GaussDB implementation |
| **PolarDB-MySQL** | `polardb-mysql` | sqlparser-rs v0.62 (MySqlDialect) | MySQL DML/DDL + PolarDB keyword detection |

## CLI Usage

```bash
# GaussDB compatibility scan
astgrep analyze --dialect gaussdb --rules rules/gaussdb/ *.sql

# OpenGauss
astgrep analyze --dialect opengauss --rules rules/gaussdb/ *.sql

# PolarDB-MySQL
astgrep analyze --dialect polardb-mysql --rules rules/polardb/ *.sql

# Standard SQL (default, backward compatible)
astgrep analyze *.sql
```

## Writing Dialect-Aware Rules

### The `dialects:` field

Rules can declare which dialects they apply to:

```yaml
rules:
  - id: gaussdb-no-on-conflict
    name: "GaussDB does not support ON CONFLICT"
    languages: [sql]
    dialects: [gaussdb, opengauss]    # only fires for these dialects
    patterns:
      - pattern: "ON CONFLICT"
    message: "Use MERGE INTO instead"
    severity: ERROR
```

**Rules without `dialects:`** apply to ALL dialects (backward compatible).

### Rule patterns

astgrep supports three pattern types for SQL:

**Literal patterns** (text-based, work across all parsers):
```yaml
patterns:
  - pattern: "VARCHAR2"
```

**Metavariable patterns** (structural matching via tree-sitter):
```yaml
patterns:
  - pattern: "SELECT * FROM $TABLE"
```

**Structural patterns with negation** (detect absence of clauses):
```yaml
patterns:
  - pattern: "UPDATE $T SET $S"
  - pattern-not: "UPDATE $T SET $S WHERE $W"
```

## Built-in Rule Libraries

### GaussDB / OpenGauss (14 rules)

Located at `tests/categories/rules/sql_dialects/gaussdb/`:

| Rule ID | Category | Description |
|---|---|---|
| GAUSSDB-TYPE-001 | Type | VARCHAR2 Oracle-compat type |
| GAUSSDB-TYPE-002 | Type | NUMBER Oracle-compat type |
| GAUSSDB-CONFLICT-001 | Compat | ON CONFLICT not supported |
| GAUSSDB-STORE-001 | Storage | ustore/astore detection |
| GAUSSDB-PREDICT-001 | AI | PREDICT BY feature |
| GAUSSDB-TIMECAPSULE-001 | Compat | TIMECAPSULE flashback |
| GAUSSDB-SHRINK-001 | Compat | SHRINK TABLE/INDEX |
| GAUSSDB-SEC-001 | Security | SELECT * warning |
| GAUSSDB-SEC-002 | Security | UPDATE without WHERE |
| GAUSSDB-SEC-003 | Security | DELETE without WHERE |
| GAUSSDB-HINT-001 | Perf | Plan Hint usage |
| GAUSSDB-MERGE-001 | Semantic | MERGE DELETE not supported (validator) |
| GAUSSDB-MERGE-002 | Semantic | ON column updated (validator) |
| GAUSSDB-MERGE-003 | Semantic | DUAL table (validator) |

### PolarDB-MySQL (6 rules)

Located at `tests/categories/rules/sql_dialects/polardb_mysql/`:

| Rule ID | Category | Description |
|---|---|---|
| POLARDB-GSI-001 | Compat | GLOBAL INDEX syntax |
| POLARDB-SHARD-001 | Compat | DBPARTITION sharding |
| POLARDB-VERCOMMENT-001 | Compat | /*!99990 versioned comment |
| POLARDB-SEC-001 | Security | SELECT * warning |
| POLARDB-SEC-002 | Security | UPDATE without WHERE |
| POLARDB-SEC-003 | Security | DELETE without WHERE |

## Architecture

```
CLI (--dialect gaussdb)
  → AnalysisConfig.sql_dialect = Some(GaussDB)
  → analyze_with_rule_engine()
    → if SQL + non-Standard dialect:
        dispatch(SqlDialect::GaussDB).parse(source) → UniversalNode
    → Rule engine (filters by dialects: field)
    → Pattern matching (literal + metavariable + pattern-not)
    → ogsql validators (GaussDB semantic checks)
```

### Parser dispatch

| Dialect | Adapter | Underlying parser |
|---|---|---|
| Standard | `SqlParser` (existing) | tree-sitter-sequel |
| GaussDB/OpenGauss | `OgsqlAdapter` | ogsql-parser (hand-written Rust) |
| PolarDB-MySQL | `SqlparserAdapter` | sqlparser-rs (Apache DataFusion) |

All adapters produce `UniversalNode` — the canonical AST type. Rule matching operates on `UniversalNode` regardless of source parser.

### Ogsql validator integration

For GaussDB/OpenGauss, ogsql-parser's built-in semantic validators run after pattern matching:

- `validate_merge_semantics()` — detects GaussDB-specific MERGE restrictions
- Runs independently of rule files (always active for GaussDB dialect)

## Extending

### Adding a new dialect

1. Add variant to `SqlDialect` enum in `crates/astgrep-core/src/types.rs`
2. Create adapter in `crates/astgrep-parser/src/adapter/{name}/mod.rs`
3. Create dialect parser in `crates/astgrep-parser/src/dialect/{name}.rs`
4. Wire into `dispatch()` in `crates/astgrep-parser/src/dialect/mod.rs`
5. Apply `.with_text(source)` in dialect `parse()` for metavariable support
6. Write rules with `dialects: [{name}]`

### Adding rules

Create YAML files under `tests/categories/rules/sql_dialects/{dialect}/`:

```yaml
rules:
  - id: {DIALECT}-{CATEGORY}-{NUM}
    name: "Rule name"
    languages: [sql]
    dialects: [{dialect}]
    patterns:
      - pattern: "..."
    message: "Description"
    severity: WARNING  # INFO | WARNING | ERROR | CRITICAL
    confidence: HIGH    # LOW | MEDIUM | HIGH
```
