# Test Case Reorganization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize test categories in `tests/categories/` to follow the self-describing GaussDB pattern (`@rule/@expect/@desc` annotations + `rules/+cases/` structure), while preserving semgrep-compatibility tests as intentional legacy.

**Architecture:** Generalize the existing `sql_dialects/validate.sh` (which already parses annotations and validates MATCH/NO_MATCH expectations) into a universal validator. Then migrate rule-driven categories (Tier A) to the pattern in batches, clean up chaotic directories (Tier C), and document semgrep-legacy directories (Tier B) as intentionally untouched.

**Tech Stack:** Bash (validator), Python (existing test runner), Rust (astgrep CLI), YAML (rules)

---

## Working Assumptions

These are the recommended defaults for the 3 decision points from the evaluation. **Flag for user confirmation before Phase 1 begins.**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **1. Annotation validator** | Generalize existing `validate.sh` (not rebuild from scratch) | It already works for SQL dialects — extend, don't reinvent |
| **2. Annotation standard** | `@rule/@expect/@desc` for all migrated categories; keep `// MATCH:` only in semgrep-legacy dirs | Structured, machine-parseable, already proven in gaussdb |
| **3. newtest/ relationship** | Leave `newtest/` as-is; reorganize `tests/categories/` in place | Avoid double-reorganization; newtest/ is incomplete and parallel |

**If the user disagrees with any assumption, adjust the corresponding phase before execution.**

---

## Tier Classification (from evaluation)

| Tier | Categories | Action |
|------|-----------|--------|
| **A — Migrate** | `sql`, `simple`, `errors`, `advanced_patterns`, `explanations`, `tainting_rules`, `rules`, `rules_v2`, `autofix`, `naming`, `metachecks`, `typing` | Apply GaussDB pattern |
| **B — Preserve** | `patterns`, `semgrep-core`, `semgrep-core-e2e`, `semgrep_output`, `comparison` | Document as intentional legacy |
| **C — Cleanup** | `TODO`, `osemgrep`, `perf`, `eval`, `e-rules`, `parsing*` (5 dirs), misc unknowns | Archive/merge/delete |
| **D — Defer** | `bash-sql`, `ci`, `cpp`, `irrelevant_rules`, `jsonnet`, `login`, `misc`, `precommit_dogfooding`, `rule_formats`, `rules_error_recovery`, `syntax_v2`, `taint_maturity`, `validation_reports`, `windows`, `xml` | Triage in Phase 3 |

---

## Phase 0: Universal Annotation Validator (Prerequisite)

**Why first:** Without a validator that works for ALL languages (not just SQL dialects), migration is cosmetic. The existing `validate.sh` only handles `gaussdb` and `polardb-mysql` dialects.

### Task 0.1: Audit existing validate.sh capabilities

**Files:**
- Read: `tests/categories/sql_dialects/validate.sh` (109 lines, already analyzed)
- Read: `tests/scripts/utils/comprehensive_test_runner.py` (384 lines)

**Step 1: Document what validate.sh already does**

Create a capability matrix:

| Capability | Status in validate.sh |
|-----------|----------------------|
| Parse `@rule` | ✅ Line 31 |
| Parse `@expect` | ✅ Line 32 |
| Parse `@desc` | ✅ Line 33 |
| Run astgrep | ✅ Line 76 |
| Verify MATCH expectation | ✅ Lines 84-91 |
| Verify NO_MATCH expectation | ✅ Lines 92-100 |
| Multi-language extraction (Java/XML) | ✅ Lines 59-74 (SQL-specific) |
| Non-SQL language support | ❌ Hardcoded to gaussdb/polardb-mysql |
| Rule YAML discovery | ✅ Via `$DIALECT_ROOT/rules/` convention |
| Pass/fail summary | ✅ Lines 104-108 |

**Step 2: Identify gaps for generalization**

The script is hardcoded to SQL dialects in 3 places:
- Lines 39-45: Dialect detection from path (`gaussdb`, `polardb_mysql`)
- Lines 59-74: Language extraction (only SQL/Java/XML → SQL)
- Line 76: `--dialect` flag (SQL-specific)

For non-SQL languages (Java, Python, JS, Bash), we need:
- No `--dialect` flag (it's SQL-only)
- No SQL extraction from Java/XML (analyze the source file directly)
- Language detection from file extension, not path

**Step 3: Commit findings**

No code changes. Document in this plan.

---

### Task 0.2: Create universal validator script

**Files:**
- Create: `tests/scripts/validate_annotations.py`

**Design:**

```python
#!/usr/bin/env python3
"""
Universal annotation validator for astgrep test cases.

Validates that @rule/@expect/@desc annotations in test files
match actual astgrep analysis results.

Usage:
    python tests/scripts/validate_annotations.py                    # validate all
    python tests/scripts/validate_annotations.py --category sql     # specific category
    python tests/scripts/validate_annotations.py --dry-run          # scan only, no execution
    python tests/scripts/validate_annotations.py --verbose          # show all cases
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

PROJECT_ROOT = Path(__file__).resolve().parents[2]
ASTGREP_CMD = ["cargo", "run", "--quiet", "--", "analyze"]

# Annotation patterns per comment style
ANNOTATION_PATTERNS = {
    # comment_prefix: regex_pattern
    "--":   r"--\s*@(\w+)\s+(.+?)\s*$",        # SQL, Lua
    "//":   r"//\s*@(\w+)\s+(.+?)\s*$",         # Java, JS, C++, Rust
    "#":    r"#\s*@(\w+)\s+(.+?)\s*$",          # Python, Ruby, Bash
    "<!--": r"<!--\s*@(\w+)\s+(.+?)\s*-->",      # XML, HTML
}

# Extension → language mapping for astgrep
EXTENSION_MAP = {
    ".sql": "sql", ".java": "java", ".js": "javascript",
    ".py": "python", ".ts": "javascript", ".tsx": "javascript",
    ".xml": "xml", ".sh": "bash", ".rb": "ruby",
    ".go": "go", ".c": "c", ".cpp": "cpp",
}

# SQL dialect detection from path
DIALECT_MAP = {
    "gaussdb": "gaussdb",
    "polardb_mysql": "polardb-mysql",
    "opengauss": "opengauss",
}

@dataclass
class TestCase:
    file_path: Path
    rule_id: str
    expect: str          # "MATCH" or "NO_MATCH"
    desc: str
    language: str
    dialect: Optional[str]
    rules_dir: Path

def detect_comment_style(file_path: Path) -> str:
    """Detect which comment style to use based on file extension."""
    ext = file_path.suffix
    if ext in (".sql", ".lua"):
        return "--"
    elif ext in (".java", ".js", ".ts", ".tsx", ".c", ".cpp", ".go", ".rs"):
        return "//"
    elif ext in (".py", ".rb", ".sh"):
        return "#"
    elif ext in (".xml", ".html"):
        return "<!--"
    return "--"  # default

def parse_annotations(file_path: Path) -> dict:
    """Parse @rule, @expect, @desc from file header comments."""
    comment_prefix = detect_comment_style(file_path)
    pattern = ANNOTATION_PATTERNS[comment_prefix]
    annotations = {}
    with open(file_path, "r", errors="replace") as f:
        for line in f:
            m = re.match(pattern, line)
            if m:
                key, value = m.group(1), m.group(2).strip()
                annotations[f"@{key}"] = value
            # Stop after first 30 lines (annotations are in header)
            if len(annotations) >= 3:
                break
    return annotations

def detect_language(file_path: Path) -> tuple[str, Optional[str]]:
    """Detect language and SQL dialect from file path + extension."""
    ext = file_path.suffix
    language = EXTENSION_MAP.get(ext, "unknown")
    dialect = None
    for dialect_key, dialect_val in DIALECT_MAP.items():
        if f"/{dialect_key}/" in str(file_path):
            dialect = dialect_val
            break
    return language, dialect

def find_rules_dir(file_path: Path) -> Optional[Path]:
    """Find the rules/ directory for a test case file.

    Convention: walk up from the case file until we find a `rules/` sibling
    to the `cases/` directory.

    Example:
        cases/update_set/GAUSSDB-SET-001_xxx.sql
        → cases/ = parent of update_set/
        → dialect_root/ = parent of cases/
        → rules/ = dialect_root/rules/
    """
    current = file_path.parent
    while current != current.parent:
        # Check if current is a cases/ subdirectory
        if current.parent and current.parent.name == "cases":
            dialect_root = current.parent.parent
            rules_dir = dialect_root / "rules"
            if rules_dir.is_dir():
                return rules_dir
        # Also check if current IS cases/
        if current.name == "cases":
            dialect_root = current.parent
            rules_dir = dialect_root / "rules"
            if rules_dir.is_dir():
                return rules_dir
        current = current.parent
    return None

def run_astgrep(test_case: TestCase) -> int:
    """Run astgrep against the test case, return finding count."""
    cmd = list(ASTGREP_CMD)
    if test_case.dialect:
        cmd.extend(["--dialect", test_case.dialect])
    cmd.extend(["--rules", str(test_case.rules_dir) + "/"])
    cmd.append(str(test_case.file_path))

    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    # Count findings matching the rule_id
    finding_count = result.stdout.count(f'"rule_id": "{test_case.rule_id}"')
    return finding_count

def validate_case(test_case: TestCase) -> tuple[bool, str]:
    """Validate a single test case. Returns (passed, message)."""
    finding_count = run_astgrep(test_case)

    if test_case.expect == "MATCH":
        if finding_count > 0:
            return True, f"  PASS  {test_case.rule_id}  {test_case.desc}"
        else:
            return False, f"  FAIL  {test_case.rule_id}  {test_case.desc}  (expected MATCH, got 0)"
    elif test_case.expect == "NO_MATCH":
        if finding_count == 0:
            return True, f"  PASS  {test_case.rule_id}  {test_case.desc}"
        else:
            return False, f"  FAIL  {test_case.rule_id}  {test_case.desc}  (expected NO_MATCH, got {finding_count})"
    else:
        return False, f"  SKIP  {test_case.rule_id}  (unknown @expect: {test_case.expect})"

def discover_cases(root: Path, category_filter: Optional[str] = None) -> list[TestCase]:
    """Discover all annotated test cases under root."""
    cases = []
    for f in root.rglob("*"):
        if not f.is_file():
            continue
        if f.suffix not in EXTENSION_MAP:
            continue
        annotations = parse_annotations(f)
        if "@rule" not in annotations or "@expect" not in annotations:
            continue
        if category_filter and category_filter not in str(f):
            continue

        language, dialect = detect_language(f)
        rules_dir = find_rules_dir(f)
        if not rules_dir:
            continue

        cases.append(TestCase(
            file_path=f,
            rule_id=annotations["@rule"],
            expect=annotations["@expect"],
            desc=annotations.get("@desc", ""),
            language=language,
            dialect=dialect,
            rules_dir=rules_dir,
        ))
    return cases

def main():
    parser = argparse.ArgumentParser(description="Validate astgrep test annotations")
    parser.add_argument("--category", "-c", help="Filter to specific category path substring")
    parser.add_argument("--dry-run", action="store_true", help="Scan only, don't run astgrep")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show all cases including passes")
    args = parser.parse_args()

    categories_root = PROJECT_ROOT / "tests" / "categories"
    cases = discover_cases(categories_root, args.category)

    if args.dry_run:
        print(f"Found {len(cases)} annotated test cases:")
        for tc in cases:
            print(f"  {tc.rule_id:30s}  {tc.expect:10s}  {tc.file_path.relative_to(PROJECT_ROOT)}")
        return 0

    passed, failed = 0, 0
    for tc in cases:
        ok, msg = validate_case(tc)
        if ok:
            passed += 1
            if args.verbose:
                print(msg)
        else:
            failed += 1
            print(msg)

    print()
    print("===============================================")
    print(f" Results: {passed} passed, {failed} failed")
    print("===============================================")
    return 1 if failed else 0

if __name__ == "__main__":
    sys.exit(main())
```

**Step 1: Create the script**

Write the full script to `tests/scripts/validate_annotations.py`.

**Step 2: Make it executable**

```bash
chmod +x tests/scripts/validate_annotations.py
```

**Step 3: Dry-run test (scan only, no execution)**

```bash
python tests/scripts/validate_annotations.py --dry-run
```

Expected: Lists all existing gaussdb annotated cases (13 files). 0 failures.

**Step 4: Full validation test against existing gaussdb cases**

```bash
python tests/scripts/validate_annotations.py --category gaussdb --verbose
```

Expected: Same pass/fail results as `./tests/categories/sql_dialects/validate.sh gaussdb`.

**Step 5: Compare output with existing validate.sh**

```bash
# Run both and compare pass counts
./tests/categories/sql_dialects/validate.sh gaussdb 2>&1 | tail -3
python tests/scripts/validate_annotations.py --category gaussdb 2>&1 | tail -3
```

Expected: Same pass/fail counts.

**Step 6: Commit**

```bash
git add tests/scripts/validate_annotations.py
git commit -m "test: add universal annotation validator

Generalizes sql_dialects/validate.sh to work with all languages.
Supports @rule/@expect/@desc annotations across SQL/Java/Python/JS/XML.
Used to validate test case correctness during reorganization."
```

---

### Task 0.3: Create project-level CONVENTIONS.md

**Files:**
- Create: `tests/CONVENTIONS.md`

**Content:** Promote `tests/categories/sql_dialects/CONVENTIONS.md` to a project-level document, generalized for all languages.

```markdown
# Test Case Conventions

## Overview

All rule-driven test categories in `tests/categories/` follow a self-describing
pattern adapted from the GaussDB SQL dialect tests.

## Directory Structure (per category)

tests/categories/{category}/
├── rules/                          # Rule YAML files
│   ├── {concern}.yaml
│   └── {concern}.yaml
└── cases/                          # Test case source files
    └── {concern}/
        ├── {RULE_ID}_{scenario}.{ext}         # positive (should match)
        ├── {RULE_ID}_{scenario}.neg.{ext}     # negative (should NOT match)
        └── {RULE_ID}_{scenario}.{other_ext}   # multi-language variant

## Rule ID Scheme

{LANG}-{CATEGORY}-{NNN}

| LANG | Languages |
|------|-----------|
| JAVA | Java |
| JS | JavaScript/TypeScript |
| PY | Python |
| SQL | SQL (with dialect suffix) |
| BASH | Bash |
| XML | XML/HTML |
| GAUSSDB | GaussDB SQL dialect |
| POLARDB | PolarDB SQL dialect |

Examples:
- `JAVA-SQLI-001` — First SQL injection rule for Java
- `JS-XSS-003` — Third XSS rule for JavaScript
- `PY-EVAL-002` — Second eval injection rule for Python

## Test Case Annotations

Every test file MUST start with these annotations in the appropriate comment syntax:

### SQL (-- comments)
```sql
-- @rule GAUSSDB-SET-001
-- @desc 3 columns SET subquery (should trigger)
-- @expect MATCH
```

### Java / JS / C++ (// comments)
```java
// @rule JAVA-SQLI-001
// @desc PreparedStatement with string concatenation
-- @expect MATCH
```

### Python (# comments)
```python
# @rule PY-EVAL-001
# @desc eval() with user input
# @expect MATCH
```

### XML (<!-- --> comments)
```xml
<!-- @rule XML-XPATH-001 -->
<!-- @desc User input in XPath expression -->
<!-- @expect MATCH -->
```

| Annotation | Required | Values |
|-----------|----------|--------|
| `@rule` | Yes | Rule ID |
| `@expect` | Yes | `MATCH` or `NO_MATCH` |
| `@desc` | Yes | Human-readable scenario description |
| `@dialect` | No | SQL dialect override (default: inferred from path) |

## Naming Convention

{RULE_ID}_{short_description}.{ext}          ← positive case
{RULE_ID}_{short_description}.neg.{ext}      ← negative case

- `short_description`: lowercase, underscores, ≤30 chars
- `.neg.` infix marks negative cases
- Each rule should have at least 1 positive + 1 negative test case

## Validation

```bash
# Validate ALL annotated test cases
python tests/scripts/validate_annotations.py

# Validate specific category
python tests/scripts/validate_annotations.py --category sql

# Dry run (list cases without executing)
python tests/scripts/validate_annotations.py --dry-run
```

## Exemptions

The following directories intentionally use legacy semgrep-core format
(`// MATCH:` / `// ERROR:` annotations) and are NOT subject to these conventions:

- `tests/categories/patterns/` — Semgrep pattern matching compatibility tests
- `tests/categories/semgrep-core/` — Semgrep-core upstream test snapshots
- `tests/categories/semgrep-core-e2e/` — End-to-end semgrep compatibility
- `tests/categories/comparison/` — astgrep vs semgrep comparison
```

**Step 1: Write the file**

**Step 2: Verify no conflicts with existing tests/README.md**

The existing `tests/README.md` documents the semgrep-core format. The new `CONVENTIONS.md` documents the new format. Add a cross-reference in `tests/README.md`:

```markdown
> **Note:** For new rule-driven test categories, see [CONVENTIONS.md](CONVENTIONS.md).
> The `// MATCH:` / `// ERROR:` format documented below applies only to
> legacy semgrep-core compatibility tests.
```

**Step 3: Commit**

```bash
git add tests/CONVENTIONS.md tests/README.md
git commit -m "docs: add project-level test conventions

Promotes sql_dialects CONVENTIONS.md to project level, generalized for
all languages. Documents @rule/@expect/@desc annotation standard and
exemptions for legacy semgrep-core test directories."
```

---

### Task 0.4: Add validator to CI/lint workflow

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add annotation validation step to CI**

After the existing `cargo test` step, add:

```yaml
  - name: Validate test annotations
    run: |
      python tests/scripts/validate_annotations.py --dry-run
      # Full validation (may fail if rules don't exist yet — non-blocking initially)
      python tests/scripts/validate_annotations.py || echo "::warning::Annotation validation failures detected"
```

**Step 2: Test locally**

```bash
python tests/scripts/validate_annotations.py --dry-run
```

Expected: Exits 0, lists all annotated cases.

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add annotation validation step

Runs universal annotation validator in dry-run mode to catch
annotation syntax errors. Full validation is non-blocking during
migration period."
```

---

## Phase 1: Low-Risk Migrations (Tier A — "Already Close")

These categories already have `yaml+code` pairs. Migration = restructure into `rules/+cases/`, add annotations, add `.neg.` cases.

### Migration Template (reusable for all Phase 1+2 tasks)

For each category migration, follow these exact steps:

```
Step A: Inventory      — list all files, count yaml/target pairs
Step B: Design RULE_IDs — assign {LANG}-{CATEGORY}-{NNN} to each rule
Step C: Restructure    — create rules/ + cases/{concern}/ directories
Step D: Move YAMLs     — move/copy yaml files to rules/, rename if needed
Step E: Move targets   — move target files to cases/{concern}/, rename to {RULE_ID}_{desc}.{ext}
Step F: Annotate       — add @rule/@expect/@desc to each target file header
Step G: Add negatives  — for each rule, create at least 1 .neg. case if missing
Step H: Validate       — run validate_annotations.py --category {category}
Step I: Run old runner — run comprehensive_test_runner.py to ensure no regression
Step J: Commit         — git add + commit with descriptive message
```

---

### Task 1.1: Migrate `sql/` category

**Current state:** Already has `yaml+sql` pairs + README. Closest to target pattern.

**Files:**
- Read: `tests/categories/sql/` (inventory all files)
- Create: `tests/categories/sql/rules/`, `tests/categories/sql/cases/{concern}/`

**Step A: Inventory**

```bash
ls tests/categories/sql/
```

Sample expected: `sql_injection.yaml`, `sql_injection.sql`, `README.md`, etc.

**Step B: Design RULE_IDs**

Map each YAML rule to a `SQL-{CATEGORY}-{NNN}` ID:
- `sql_injection.yaml` → `SQL-SQLI-001`
- etc.

**Step C-G: Restructure + annotate + add negatives**

Follow migration template. Create:
```
tests/categories/sql/
├── rules/
│   └── injection.yaml         # (renamed from sql_injection.yaml)
├── cases/
│   └── injection/
│       ├── SQL-SQLI-001_basic_concat.sql
│       ├── SQL-SQLI-001_basic_concat.neg.sql    # NEW: negative case
│       └── ...
└── README.md                   # Update to reference CONVENTIONS.md
```

**Step H: Validate**

```bash
python tests/scripts/validate_annotations.py --category sql --verbose
```

Expected: All cases pass.

**Step I: Regression check**

```bash
python tests/scripts/utils/comprehensive_test_runner.py --suite sql
```

**Step J: Commit**

```bash
git add tests/categories/sql/
git commit -m "test: migrate sql/ category to self-describing pattern

Restructures sql/ tests into rules/+cases/ layout with @rule/@expect/@desc
annotations. Adds negative test cases for each rule.
Follows CONVENTIONS.md pattern."
```

---

### Task 1.2: Migrate `simple/` category

**Current state:** Minimal yaml+code pairs.

Apply migration template. RULE_ID scheme: `SIMPLE-{TYPE}-{NNN}`.

**Validate + commit per template.**

---

### Task 1.3: Migrate `errors/` category

**Current state:** Error-type flat YAMLs (no code files — these test rule validation errors, not matching).

**Special handling:** Error test cases test that INVALID rules are rejected. The annotation pattern needs adaptation:

```yaml
# For error test cases, annotate the YAML itself:
# @rule_errors missing_id
# @expect VALIDATION_ERROR
```

Or: keep errors/ as a special category not requiring the full pattern (it tests rule parsing, not rule matching). Document this exemption in CONVENTIONS.md.

**Decision point:** Ask user whether errors/ should adopt the pattern or be exempted.

---

### Task 1.4: Migrate `advanced_patterns/` category

**Current state:** yaml+py pairs + README. Well-organized.

Apply migration template. RULE_ID scheme: `ADV-{PATTERN}-{NNN}`.

---

### Task 1.5: Migrate `explanations/` category

**Current state:** yaml+py pairs with descriptive names.

Apply migration template. RULE_ID scheme: `EXPL-{FEATURE}-{NNN}`.

---

## Phase 2: Annotation Unification (Tier A — Mixed Categories)

These categories have good structure but use different annotation styles.

### Task 2.1: Migrate `tainting_rules/` category

**Current state:** Per-language subdirs, uses `#ruleid:` / `#OK:` annotations.

**Key transformation:** Convert `#ruleid: test-id` → `# @rule RULE_ID` + `# @expect MATCH`, and `#OK:` → `# @expect NO_MATCH`.

**Step 1: Write conversion script (one-time use)**

```python
# convert_taint_annotations.py — one-time migration script
# Converts #ruleid: → @rule/@expect, #OK: → @expect NO_MATCH
```

**Step 2: Run conversion**

**Step 3: Restructure into rules/+cases/ layout**

```
tests/categories/tainting_rules/
├── rules/
│   └── {concern}.yaml
└── cases/
    └── {lang}_{concern}/
        ├── {RULE_ID}_{scenario}.{ext}
        └── {RULE_ID}_{scenario}.neg.{ext}
```

**Step 4: Validate + commit**

---

### Task 2.2: Evaluate and merge `rules/` + `rules_v2/`

**Current state:** Both have yaml+code pairs. `rules_v2/` is newer syntax.

**Step 1: Diff the rule formats**

Determine if rules_v2 tests are supersets of rules/ or testing different features.

**Step 2: Decision**

- If rules_v2 supersedes rules/ → merge into single `rules/` with v2 syntax
- If they test different features → keep both, migrate each independently

**Step 3: Migrate per template**

---

### Task 2.3: Migrate `autofix/` category

**Current state:** Language subdirs with paired files.

**Special handling:** Autofix tests verify fix output, not just matching. May need additional annotation:

```python
# @rule AUTOFIX-PY-001
# @expect MATCH
# @fix fixed_code_here
```

Or validate fix output separately. Document approach in CONVENTIONS.md.

---

### Task 2.4: Migrate sparse categories (`naming/`, `metachecks/`, `typing/`)

**Current state:** Very few files.

**Approach:** If content is thin, either:
- Merge into an appropriate existing category
- Migrate to pattern and mark as "needs more cases" in README

---

## Phase 3: Cleanup (Tier C)

### Task 3.1: Archive `TODO/` directory

**Step 1: Inventory contents**

```bash
ls tests/categories/TODO/
```

**Step 2: For each file, decide:**
- Has a corresponding implemented test elsewhere? → Delete
- Still relevant? → Move to appropriate category
- Unknown? → Document in `tests/categories/TODO/README.md` as "needs triage"

**Step 3: Delete or move, then commit**

```bash
git commit -m "test: clean up TODO/ directory

Triage: X files deleted (duplicated), Y files moved to appropriate
categories, Z files documented for future triage."
```

---

### Task 3.2: Delete abandoned `osemgrep/` directory

```bash
ls tests/categories/osemgrep/
# If single abandoned file:
rm -rf tests/categories/osemgrep/
git commit -m "test: remove abandoned osemgrep/ directory"
```

---

### Task 3.3: Merge `parsing*/` directories

**Current state:** 5 separate dirs: `parsing/`, `parsing_errors/`, `parsing_missing/`, `parsing_patterns/`, `parsing_todo/`

**Target structure:**

```
tests/categories/parsing/
├── success/        # Files that should parse without errors (from parsing/)
├── errors/         # Files that should fail parsing (from parsing_errors/)
├── missing/        # Files with missing features (from parsing_missing/)
├── patterns/       # Parsing pattern tests (from parsing_patterns/)
└── todo/           # Future parsing tests (from parsing_todo/)
```

**Step 1: Move subdirectories**

**Step 2: Update comprehensive_test_runner.py if any of these are in test_patterns list**

**Step 3: Commit**

---

### Task 3.4: Reorganize `perf/` directory

**Current state:** Mixed formats, no clear structure.

**Target:** Either merge into newtest/ performance infrastructure or structure as:

```
tests/categories/perf/
├── benchmarks/     # Large files for throughput testing
├── timeouts/       # Files that should timeout
└── README.md       # How to run benchmarks
```

---

### Task 3.5: Triage Tier D (unknown categories)

For each of: `bash-sql`, `ci`, `cpp`, `irrelevant_rules`, `jsonnet`, `login`, `misc`, `precommit_dogfooding`, `rule_formats`, `rules_error_recovery`, `syntax_v2`, `taint_maturity`, `validation_reports`, `windows`, `xml`

**Step 1: Sample 2-3 files from each**

**Step 2: Classify:**
- Has rule-driven content → Migrate to pattern (Phase 1/2)
- Semgrep legacy → Move to Tier B
- Empty/abandoned → Delete
- Valid but different purpose → Document exemption

**Step 3: Execute per-category decisions**

---

## Phase 4: Preserve Legacy (Tier B)

### Task 4.1: Add README to `patterns/`

**File:** `tests/categories/patterns/README.md`

```markdown
# Patterns — Semgrep-Core Compatibility Tests

**⚠️ INTENTIONAL LEGACY — DO NOT REORGANIZE**

This directory contains semgrep-core upstream test cases in their original
format (`.sgrep` pattern files + target files with `// MATCH:` / `// ERROR:`
annotations). These are used for astgrep-vs-semgrep compatibility regression
testing.

**Do NOT:**
- Rename files to `{RULE_ID}_{desc}` convention
- Replace `// MATCH:` / `// ERROR:` with `@rule/@expect` annotations
- Restructure into `rules/+cases/` layout

**Format:** See [tests/README.md](../../README.md) for semgrep-core format details.

**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md#exemptions)
```

---

### Task 4.2: Add README to `semgrep-core/`

**File:** `tests/categories/semgrep-core/README.md`

```markdown
# Semgrep-Core — Upstream Test Snapshots

**⚠️ INTENTIONAL LEGACY — DO NOT REORGANIZE**

78 directories with hash-based names corresponding to upstream semgrep commits.
Used for compatibility regression: if astgrep behavior diverges from semgrep
on these cases, the hash allows tracing back to the specific semgrep test.

**Do NOT** rename hash directories or add @rule annotations.
```

---

### Task 4.3: Add README to remaining Tier B directories

For: `semgrep-core-e2e/`, `semgrep_output/`, `comparison/`

Each gets a short README explaining:
- Purpose (what compatibility aspect it tests)
- Why it's exempt from CONVENTIONS.md
- Format documentation reference

---

## Phase 5: Integration & Documentation

### Task 5.1: Update comprehensive_test_runner.py

Ensure the Python test runner can discover tests in the new `rules/+cases/` structure.

**Check:** The runner already does recursive `*.yaml` discovery, so it should work.
Verify by running:

```bash
python tests/scripts/utils/comprehensive_test_runner.py
```

**If broken:** Update `test_patterns` list or discovery logic.

---

### Task 5.2: Update AGENTS.md

**File:** `AGENTS.md`

Update the "Test cases" row in the "Where to Look" table:

```markdown
| Test cases | `tests/categories/{category}/cases/{lang}/` | `@rule`/`@expect`/`@desc` annotations; see `tests/CONVENTIONS.md` |
```

Add to Key Conventions:

```markdown
- **Test conventions**: All rule-driven test categories follow the self-describing
  pattern in `tests/CONVENTIONS.md` (`@rule/@expect/@desc` annotations + `rules/+cases/` layout).
  Legacy semgrep-core tests (`patterns/`, `semgrep-core/`) are intentionally exempt.
```

---

### Task 5.3: Create migration progress tracker

**File:** `tests/MIGRATION_STATUS.md`

```markdown
# Test Reorganization Migration Status

| Category | Tier | Status | Date | Notes |
|----------|------|--------|------|-------|
| sql_dialects/gaussdb | Reference | ✅ Done | — | Original pattern source |
| sql | A | ✅ Migrated | 2026-06-14 | |
| simple | A | ✅ Migrated | 2026-06-14 | |
| errors | A | ⏳ Exempt | — | Tests rule validation, not matching |
| advanced_patterns | A | ✅ Migrated | 2026-06-14 | |
| explanations | A | ✅ Migrated | 2026-06-14 | |
| tainting_rules | A | ✅ Migrated | 2026-06-14 | Converted from #ruleid: |
| rules + rules_v2 | A | ✅ Merged | 2026-06-14 | |
| autofix | A | ✅ Migrated | 2026-06-14 | |
| naming | A | ✅ Merged | — | Thin, merged into relevant category |
| patterns | B | 📌 Legacy | — | Semgrep compatibility |
| semgrep-core | B | 📌 Legacy | — | Semgrep snapshots |
| TODO | C | 🗑️ Cleaned | 2026-06-14 | |
| parsing* (5 dirs) | C | ✅ Merged | 2026-06-14 | Into parsing/{success,errors,...} |
| ... | | | | |
```

---

## Effort Estimates

| Phase | Tasks | Estimated Effort | Dependencies |
|-------|-------|-----------------|--------------|
| Phase 0 | 0.1-0.4 | 4-6 hours | None (prerequisite) |
| Phase 1 | 1.1-1.5 | 6-10 hours (1-2 hrs per category) | Phase 0 complete |
| Phase 2 | 2.1-2.4 | 8-12 hours (mixed categories need conversion) | Phase 0 complete |
| Phase 3 | 3.1-3.5 | 4-8 hours (mostly mechanical) | None (can parallelize) |
| Phase 4 | 4.1-4.3 | 1-2 hours (documentation only) | None |
| Phase 5 | 5.1-5.3 | 2-3 hours | Phases 1-2 complete |
| **Total** | | **25-41 hours** | |

**Parallelization:** Phase 3 (cleanup) can run in parallel with Phases 1-2. Phase 4 (legacy docs) can run anytime.

---

## Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Validator false positives/negatives | Medium | High | Phase 0 cross-checks against existing validate.sh |
| Breaking comprehensive_test_runner.py discovery | Low | Medium | Update test_patterns list if dirs are renamed |
| Losing test coverage during migration | Medium | High | Run old runner BEFORE and AFTER each migration |
| Annotation parser edge cases (comment styles) | Medium | Low | Dry-run mode catches syntax errors early |
| Resistance from semgrep compatibility tests | N/A | N/A | Tier B explicitly excluded |
| Scope creep into newtest/ | Low | Medium | Working Assumption #3 explicitly defers newtest/ |

---

## Success Criteria

- [ ] `python tests/scripts/validate_annotations.py` passes with 0 failures
- [ ] All Tier A categories have `rules/+cases/` structure
- [ ] All Tier A test files have `@rule/@expect/@desc` annotations
- [ ] All Tier A rules have at least 1 positive + 1 negative test case
- [ ] Tier B directories have READMEs explaining legacy status
- [ ] Tier C directories are cleaned/merged
- [ ] `tests/CONVENTIONS.md` exists and is referenced from AGENTS.md
- [ ] CI runs annotation validation in dry-run mode
- [ ] `tests/MIGRATION_STATUS.md` tracks all categories

---

## Execution Notes

- **Commit frequently**: One commit per task (or sub-task for large migrations)
- **Run validator after EVERY migration**: `python tests/scripts/validate_annotations.py --category {cat}`
- **Run old runner after EVERY migration**: Catch regressions immediately
- **Don't batch migrations**: Each category migration is an independent PR
- **Keep conversion scripts one-time**: Don't over-engineer; delete after use
