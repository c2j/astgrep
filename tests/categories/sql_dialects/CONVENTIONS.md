# SQL Dialect Rules — Directory & Naming Conventions

## Directory Structure

```
tests/categories/sql_dialects/
├── gaussdb/                           # GaussDB / OpenGauss (shared parser)
│   ├── rules/                         # Rule YAML files (one per concern category)
│   │   ├── merge.yaml                 # MERGE INTO rules
│   │   ├── update_set.yaml            # UPDATE SET rules
│   │   ├── types.yaml                 # Data type compatibility
│   │   ├── compatibility.yaml         # Syntax compatibility (ON CONFLICT etc.)
│   │   ├── security.yaml              # Security (SELECT *, missing WHERE)
│   │   └── hints.yaml                 # Plan Hint rules
│   └── cases/                         # Test SQL files
│       ├── merge/
│       │   ├── GAUSSDB-MERGE-001_multi_values.sql       # positive (should match)
│       │   ├── GAUSSDB-MERGE-001_multi_values.neg.sql   # negative (should NOT match)
│       │   └── GAUSSDB-MERGE-002_delete.sql
│       ├── update_set/
│       │   ├── GAUSSDB-SET-001_multicol_subquery_3col.sql
│       │   ├── GAUSSDB-SET-001_multicol_subquery_2col.neg.sql
│       │   └── GAUSSDB-SET-001_multicol_subquery_5col.sql
│       └── ...
├── polardb_mysql/                     # PolarDB-MySQL
│   ├── rules/
│   │   ├── compatibility.yaml
│   │   ├── sharding.yaml
│   │   └── security.yaml
│   └── cases/
│       └── ...
├── common/                            # Cross-dialect rules (apply to 2+ dialects)
│   ├── rules/
│   │   └── security.yaml              # SELECT *, UPDATE without WHERE (all dialects)
│   └── cases/
│       └── ...
└── validate.sh                        # Automated validation script
```

## Rule ID Scheme

```
{DB}-{CATEGORY}-{NNN}
```

| Part | Values |
|---|---|
| **DB** | `GAUSSDB` (incl. OpenGauss), `POLARDB`, `COMMON` (cross-dialect) |
| **CATEGORY** | `MERGE`, `SET`, `TYPE`, `COMPAT`, `SEC`, `HINT`, `SHARD`, `GSI`, `VERCOMMENT`, `STORE`, `PREDICT` |
| **NNN** | `001`, `002`, `003`... (zero-padded, per DB+CATEGORY) |

Examples:
- `GAUSSDB-MERGE-001` — First MERGE rule for GaussDB
- `GAUSSDB-SET-003` — Third UPDATE SET rule for GaussDB
- `POLARDB-SHARD-002` — Second sharding rule for PolarDB
- `COMMON-SEC-001` — First security rule, cross-dialect

## Test Case File Naming

```
{RULE_ID}_{short_description}.{ext}          ← positive case (rule SHOULD fire)
{RULE_ID}_{short_description}.neg.{ext}      ← negative case (rule should NOT fire)
```

| Extension | Source language | Annotation syntax |
|---|---|---|
| `.sql` | Pure SQL | `-- @rule ...` |
| `.java` | Java with embedded SQL | `// @rule ...` |
| `.xml` | iBatis/MyBatis mapper XML | `<!-- @rule ... -->` |

- `short_description`: lowercase, underscores, ≤30 chars
- `.neg.` infix marks negative cases
- Each rule should have **at least 1 positive + 1 negative** test case
- Cross-language cases share the same RULE_ID but have different extensions

## Test Case Annotations

Each test file starts with annotations in the appropriate comment syntax:

**SQL** (`.sql`):
```sql
-- @rule GAUSSDB-SET-001
-- @desc 3 columns SET subquery (should trigger)
-- @expect MATCH
```

**Java** (`.java`):
```java
// @rule GAUSSDB-SET-001
// @desc Java string concat UPDATE SET 3+ columns
// @expect MATCH
```

**iBatis XML** (`.xml`):
```xml
<!-- @rule GAUSSDB-SET-001 -->
<!-- @desc iBatis mapper UPDATE SET 3+ columns -->
<!-- @expect MATCH -->
```

| Annotation | Required | Values |
|---|---|---|
| `@rule` | Yes | Rule ID (e.g., `GAUSSDB-SET-001`) |
| `@expect` | Yes | `MATCH` (positive) or `NO_MATCH` (negative) |
| `@desc` | Yes | Human-readable description of the scenario |
| `@dialect` | No | Override dialect (default: inferred from directory) |
UPDATE employees e
SET (e.salary, e.dept, e.title) = (
    SELECT s.salary, s.dept, s.title
    FROM new_data s WHERE s.id = e.id
);
```

```sql
-- @rule GAUSSDB-SET-001
-- @desc UPDATE SET 2 columns (within limit)
-- @expect NO_MATCH
UPDATE employees e
SET (e.salary, e.dept) = (
    SELECT s.salary, s.dept
    FROM new_data s WHERE s.id = e.id
);
```

| Annotation | Purpose |
|---|---|
| `@rule` | Which rule ID this case tests |
| `@desc` | Human-readable description of the scenario |
| `@expect` | `MATCH` (positive) or `NO_MATCH` (negative) |

## Rule YAML Template

```yaml
rules:
  - id: GAUSSDB-SET-001               # {DB}-{CATEGORY}-{NNN}
    name: "GaussDB UPDATE SET 多列子查询 (>2列)"
    description: >
      检测 UPDATE SET (col1, col2, col3, ...) = (SELECT ...) 语句。
      GaussDB 对多列子查询赋值可能有限制。
    languages: [sql]
    dialects: [gaussdb, opengauss]     # Which dialects this applies to
    patterns:
      - pattern: "$SQL"
      - metavariable-regex:
          metavariable: $SQL
          regex: "(?s)SET\\s*\\([^)]*,[^)]*,"  # 3+ comma-separated items in SET (...)
    message: "UPDATE SET 子句包含3+字段子查询赋值，GaussDB可能有限制"
    severity: WARNING                  # INFO | WARNING | ERROR | CRITICAL
    confidence: HIGH                   # LOW | MEDIUM | HIGH
    metadata:
      category: compatibility
      cwe: "CWE-1106"                  # Optional: relevant CWE
      gaussdb_doc: "https://support.huaweicloud.com/..."  # Optional: doc URL
```

## Validation Workflow

```bash
# Validate ALL rules against ALL test cases
./tests/categories/sql_dialects/validate.sh

# Validate specific dialect only
./tests/categories/sql_dialects/validate.sh gaussdb

# Validate specific category only
./tests/categories/sql_dialects/validate.sh gaussdb update_set
```

The script:
1. Scans `cases/` for `.sql` and `.neg.sql` files
2. Parses `@rule` and `@expect` annotations
3. Runs the applicable rule against each case
4. Verifies positive cases produce findings, negative cases produce none
5. Reports pass/fail summary with details

## Severity Guidelines

| Severity | Use When |
|---|---|
| `INFO` | Best practice suggestion, no action required (VARCHAR2 usage) |
| `WARNING` | Potential issue, should review (UPDATE without WHERE, PREDICT BY) |
| `ERROR` | Known incompatibility, will break (ON CONFLICT in GaussDB, DELETE without WHERE) |
| `CRITICAL` | Data loss or security risk (reserved for extreme cases) |
