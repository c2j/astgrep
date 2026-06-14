# Test Case Conventions

All rule-driven test categories in `tests/categories/` follow a self-describing
pattern adapted from the GaussDB SQL dialect tests. Each test file declares
what it tests via structured annotations, enabling automated validation.

## Directory Structure

```
tests/categories/{category}/
├── rules/                          # Rule YAML files
│   └── {concern}.yaml
└── cases/                          # Test case source files
    └── {concern}/
        ├── {RULE_ID}_{scenario}.{ext}          # positive (rule SHOULD fire)
        ├── {RULE_ID}_{scenario}.neg.{ext}      # negative (rule should NOT fire)
        └── {RULE_ID}_{scenario}.{other_ext}    # multi-language variant
```

## Rule ID Scheme

```
{LANG}-{CATEGORY}-{NNN}
```

| LANG     | Languages                         |
|----------|-----------------------------------|
| `JAVA`   | Java                              |
| `JS`     | JavaScript / TypeScript           |
| `PY`     | Python                            |
| `SQL`    | SQL (standard)                    |
| `BASH`   | Bash / Shell                      |
| `XML`    | XML / HTML                        |
| `GAUSSDB`| GaussDB / OpenGauss SQL dialect   |
| `POLARDB`| PolarDB-MySQL SQL dialect         |

Examples:
- `JAVA-SQLI-001` — First SQL injection rule for Java
- `JS-XSS-003` — Third XSS rule for JavaScript
- `PY-EVAL-002` — Second eval injection rule for Python

## Test Case Annotations

Every test file MUST start with these three annotations in the
appropriate comment syntax for the language:

**SQL** (`--` comments):
```sql
-- @rule GAUSSDB-SET-001
-- @desc 3-column SET subquery (should trigger)
-- @expect MATCH
```

**Java / JS / C++ / Rust** (`//` comments):
```java
// @rule JAVA-SQLI-001
// @desc PreparedStatement with string concatenation
// @expect MATCH
```

**Python / Ruby / Bash** (`#` comments):
```python
# @rule PY-EVAL-001
# @desc eval() with user input
# @expect MATCH
```

**XML / HTML** (`<!-- -->` comments):
```xml
<!-- @rule XML-XPATH-001 -->
<!-- @desc User input in XPath expression -->
<!-- @expect MATCH -->
```

| Annotation  | Required | Values                            |
|-------------|----------|-----------------------------------|
| `@rule`     | Yes      | Rule ID (e.g., `GAUSSDB-SET-001`) |
| `@expect`   | Yes      | `MATCH` or `NO_MATCH`             |
| `@desc`     | Yes      | Human-readable scenario summary   |
| `@dialect`  | No       | SQL dialect override (default: inferred from path) |

## File Naming

```
{RULE_ID}_{short_description}.{ext}          ← positive case
{RULE_ID}_{short_description}.neg.{ext}      ← negative case
```

- `short_description`: lowercase, underscores, max 30 chars
- `.neg.` infix marks negative cases
- Each rule should have at least 1 positive + 1 negative test case
- Cross-language variants share the same RULE_ID but use different extensions

## Validation

```bash
# Validate ALL annotated test cases
python3 tests/scripts/validate_annotations.py

# Validate specific category
python3 tests/scripts/validate_annotations.py --category gaussdb

# Dry run (list discovered cases without running astgrep)
python3 tests/scripts/validate_annotations.py --dry-run

# Verbose (show passing cases too)
python3 tests/scripts/validate_annotations.py --verbose
```

## Exemptions — Legacy semgrep-core Format

The following directories intentionally use the legacy semgrep-core format
(`// MATCH:` / `// ERROR:` annotations) and are **NOT** subject to these
conventions. See [tests/README.md](README.md) for that format.

- `tests/categories/patterns/` — Semgrep pattern matching compatibility tests
- `tests/categories/semgrep-core/` — Semgrep-core upstream test snapshots
- `tests/categories/semgrep-core-e2e/` — End-to-end semgrep compatibility
- `tests/categories/comparison/` — astgrep vs semgrep comparison

**Do NOT** reorganize, rename, or re-annotate files in these directories.
