#!/usr/bin/env python3
"""
Universal annotation validator for astgrep test cases.

Validates that @rule/@expect/@desc annotations in test files
match actual astgrep analysis results.

Generalizes tests/categories/sql_dialects/validate.sh to work with
all languages and test categories.

Usage:
    python tests/scripts/validate_annotations.py                    # validate all
    python tests/scripts/validate_annotations.py --category gaussdb # specific category
    python tests/scripts/validate_annotations.py --dry-run          # scan only, no execution
    python tests/scripts/validate_annotations.py --verbose          # show all cases
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

PROJECT_ROOT = Path(__file__).resolve().parents[2]

# Prefer pre-built binary for speed; fall back to cargo run
_DEBUG_BINARY = PROJECT_ROOT / "target" / "debug" / "astgrep"
_RELEASE_BINARY = PROJECT_ROOT / "target" / "release" / "astgrep"


def _get_astgrep_cmd() -> list[str]:
    """Return the astgrep command, preferring pre-built binaries."""
    if _DEBUG_BINARY.exists():
        return [str(_DEBUG_BINARY)]
    if _RELEASE_BINARY.exists():
        return [str(_RELEASE_BINARY)]
    return ["cargo", "run", "--quiet", "--"]


ASTGREP_BASE_CMD = _get_astgrep_cmd()

# Regex per comment style: captures @key value
ANNOTATION_PATTERNS: dict[str, re.Pattern] = {
    "--":   re.compile(r"--\s*@(\w+)\s+(.+?)\s*$"),
    "//":   re.compile(r"//\s*@(\w+)\s+(.+?)\s*$"),
    "#":    re.compile(r"#\s*@(\w+)\s+(.+?)\s*$"),
    "<!--": re.compile(r"<!--\s*@(\w+)\s+(.+?)\s*-->"),
}

# File extension → comment style
EXTENSION_COMMENT_STYLE: dict[str, str] = {
    ".sql":   "--",
    ".lua":   "--",
    ".java":  "//",
    ".js":    "//",
    ".ts":    "//",
    ".tsx":   "//",
    ".c":     "//",
    ".cpp":   "//",
    ".h":     "//",
    ".hpp":   "//",
    ".go":    "//",
    ".rs":    "//",
    ".kt":    "//",
    ".swift": "//",
    ".cs":    "//",
    ".py":    "#",
    ".rb":    "#",
    ".sh":    "#",
    ".bash":  "#",
    ".pl":    "#",
    ".xml":   "<!--",
    ".html":  "<!--",
}

# Extension → astgrep language (for --language flag if ever needed)
EXTENSION_LANGUAGE: dict[str, str] = {
    ".sql": "sql", ".java": "java", ".js": "javascript",
    ".py": "python", ".ts": "javascript", ".tsx": "javascript",
    ".xml": "xml", ".sh": "bash", ".rb": "ruby",
    ".go": "go", ".c": "c", ".cpp": "cpp",
}

# SQL dialect detection from directory path
DIALECT_PATH_MAP: dict[str, str] = {
    "gaussdb": "gaussdb",
    "polardb_mysql": "polardb-mysql",
    "opengauss": "opengauss",
}


@dataclass
class TestCase:
    """A single test case discovered from an annotated file."""
    file_path: Path
    rule_id: str
    expect: str           # "MATCH" or "NO_MATCH"
    desc: str
    dialect: Optional[str]
    rules_dir: Path
    is_sql_dialect_case: bool = False


def _detect_comment_style(file_path: Path) -> Optional[str]:
    """Determine comment syntax from file extension."""
    return EXTENSION_COMMENT_STYLE.get(file_path.suffix)


def parse_annotations(file_path: Path) -> dict[str, str]:
    """Parse @rule, @expect, @desc from the first ~30 header lines.

    Returns a dict like {"@rule": "GAUSSDB-SET-001", "@expect": "MATCH", ...}.
    Returns empty dict if the file uses an unknown comment style or has no annotations.
    """
    style = _detect_comment_style(file_path)
    if style is None:
        return {}

    pattern = ANNOTATION_PATTERNS[style]
    annotations: dict[str, str] = {}

    try:
        with open(file_path, "r", errors="replace") as f:
            for i, line in enumerate(f):
                if i >= 30:  # annotations are always in the header
                    break
                m = pattern.match(line)
                if m:
                    key = f"@{m.group(1)}"
                    val = m.group(2).strip()
                    # For XML, trailing --> already stripped by regex
                    annotations[key] = val
    except (OSError, UnicodeDecodeError):
        return {}

    return annotations


def _detect_dialect(file_path: Path) -> Optional[str]:
    """Detect SQL dialect from directory path, if any."""
    path_str = str(file_path)
    for dir_name, dialect in DIALECT_PATH_MAP.items():
        if f"/{dir_name}/" in path_str:
            return dialect
    return None


def _find_rules_dir(file_path: Path) -> Optional[Path]:
    """Walk up from the test case to find the sibling ``rules/`` directory.

    Convention::

        cases/{concern}/{RULE_ID}_xxx.{ext}
        → cases/      = parent of {concern}/
        → root/       = parent of cases/
        → rules/      = root/rules/

    Falls back to scanning for the nearest ancestor that contains both
    ``cases/`` and ``rules/``.
    """
    current = file_path.parent
    while current != PROJECT_ROOT:
        # Case 1: current is inside cases/ (e.g. cases/update_set/)
        if current.parent.name == "cases":
            root = current.parent.parent  # dialect or category root
            rules_dir = root / "rules"
            if rules_dir.is_dir():
                return rules_dir

        # Case 2: current IS cases/
        if current.name == "cases":
            root = current.parent
            rules_dir = root / "rules"
            if rules_dir.is_dir():
                return rules_dir

        current = current.parent

    return None


def _extract_sql_from_java(java_path: Path) -> Path:
    """Extract SQL from Java string literals (concatenated with +).

    Mirrors validate.sh: grep quoted strings, strip quotes, join with spaces.
    """
    sql_parts: list[str] = []
    string_re = re.compile(r'"([^"]*)"')

    with open(java_path, "r", errors="replace") as f:
        for line in f:
            # Skip comment lines
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("/*") or stripped.startswith("*"):
                continue
            for m in string_re.finditer(line):
                content = m.group(1)
                # Skip import/class/package strings
                if any(kw in content for kw in ("java.", "com.", "org.", "http", "DTD")):
                    continue
                sql_parts.append(content)

    extracted = " ".join(sql_parts)
    tmp = tempfile.NamedTemporaryFile(mode="w", suffix=".sql", delete=False,
                                       prefix="astgrep_extract_")
    tmp.write(extracted + "\n")
    tmp.close()
    return Path(tmp.name)


def _extract_sql_from_xml(xml_path: Path) -> Path:
    """Extract SQL from iBatis/MyBatis mapper XML tags.

    Mirrors validate.sh: extract <update>/<select>/<insert>/<delete> blocks,
    strip tags, replace #{...} and ${...} with ?.
    """
    tag_re = re.compile(
        r"<(?:update|select|insert|delete)\b[^>]*>(.*?)</(?:update|select|insert|delete)>",
        re.DOTALL,
    )
    strip_tags_re = re.compile(r"<[^>]+>")
    param_hash_re = re.compile(r"#\{[^}]*\}")
    param_dollar_re = re.compile(r"\$\{[^}]*\}")

    with open(xml_path, "r", errors="replace") as f:
        content = f.read()

    blocks: list[str] = []
    for m in tag_re.finditer(content):
        block = m.group(1)
        block = strip_tags_re.sub("", block)
        block = param_hash_re.sub("?", block)
        block = param_dollar_re.sub("?", block)
        block = block.strip()
        if block:
            blocks.append(block)

    extracted = "\n".join(blocks)
    tmp = tempfile.NamedTemporaryFile(mode="w", suffix=".sql", delete=False,
                                       prefix="astgrep_extract_")
    tmp.write(extracted + "\n")
    tmp.close()
    return Path(tmp.name)


def discover_cases(
    root: Path,
    category_filter: Optional[str] = None,
) -> list[TestCase]:
    """Walk *root* recursively and return every annotated test case."""
    cases: list[TestCase] = []

    for file_path in sorted(root.rglob("*")):
        if not file_path.is_file():
            continue
        if file_path.suffix not in EXTENSION_COMMENT_STYLE:
            continue

        if category_filter and category_filter not in str(file_path):
            continue

        annotations = parse_annotations(file_path)
        rule_id = annotations.get("@rule")
        expect = annotations.get("@expect")

        if not rule_id or not expect:
            continue

        rules_dir = _find_rules_dir(file_path)
        if not rules_dir:
            continue

        dialect = _detect_dialect(file_path)
        # SQL dialect cases may need SQL extraction from Java/XML hosts
        is_sql_dialect_case = dialect is not None

        cases.append(TestCase(
            file_path=file_path,
            rule_id=rule_id,
            expect=expect,
            desc=annotations.get("@desc", ""),
            dialect=dialect,
            rules_dir=rules_dir,
            is_sql_dialect_case=is_sql_dialect_case,
        ))

    return cases


def _prepare_analysis_file(tc: TestCase) -> tuple[Path, bool]:
    """Return (file_to_analyze, is_temp) for the test case.

    For SQL-dialect cases with Java/XML host files, extract SQL to a temp
    .sql file.  For everything else, analyze the original file directly.
    """
    ext = tc.file_path.suffix

    if tc.is_sql_dialect_case and ext == ".java":
        return _extract_sql_from_java(tc.file_path), True

    if tc.is_sql_dialect_case and ext == ".xml":
        return _extract_sql_from_xml(tc.file_path), True

    return tc.file_path, False


def _resolve_rules_path(tc: TestCase) -> str:
    """Resolve the best --rules path for a test case.

    Prefers a specific YAML file matching the concern directory name
    (e.g. cases/select_for_update/ → rules/select_for_update.yaml),
    which avoids loading unrelated rules that may be expensive on
    large test files.  Falls back to the whole rules/ directory.
    """
    concern = tc.file_path.parent.name
    specific = tc.rules_dir / f"{concern}.yaml"
    if specific.is_file():
        return str(specific)
    return str(tc.rules_dir) + "/"


def run_astgrep(tc: TestCase) -> int:
    """Run astgrep for *tc* and return the finding count for its rule_id."""
    analysis_file, is_temp = _prepare_analysis_file(tc)

    try:
        cmd = list(ASTGREP_BASE_CMD) + ["analyze"]
        if tc.dialect:
            cmd += ["--dialect", tc.dialect]
        cmd += ["--rules", _resolve_rules_path(tc)]
        cmd.append(str(analysis_file))

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
            cwd=str(PROJECT_ROOT),
        )

        # Parse JSON from stdout
        try:
            data = json.loads(result.stdout)
        except json.JSONDecodeError:
            # If JSON parse fails, fall back to string matching
            return result.stdout.count(f'"rule_id": "{tc.rule_id}"')

        findings = data.get("findings", [])
        return sum(1 for f in findings if f.get("rule_id") == tc.rule_id)

    except subprocess.TimeoutExpired:
        print(f"  ERROR  {tc.rule_id}  TIMEOUT (>60s)  {tc.desc}", file=sys.stderr)
        return -1
    finally:
        if is_temp:
            try:
                analysis_file.unlink()
            except OSError:
                pass


def validate_case(tc: TestCase) -> tuple[bool, str]:
    """Validate a single test case.  Returns (passed, message)."""
    finding_count = run_astgrep(tc)

    if finding_count < 0:
        return False, f"  ERROR  {tc.rule_id}  {tc.desc}  (execution failed)"

    if tc.expect == "MATCH":
        if finding_count > 0:
            return True, f"  PASS  {tc.rule_id}  {tc.desc}"
        return False, f"  FAIL  {tc.rule_id}  {tc.desc}  (expected MATCH, got 0)"

    if tc.expect == "NO_MATCH":
        if finding_count == 0:
            return True, f"  PASS  {tc.rule_id}  {tc.desc}"
        return False, f"  FAIL  {tc.rule_id}  {tc.desc}  (expected NO_MATCH, got {finding_count})"

    return False, f"  SKIP  {tc.rule_id}  (unknown @expect: {tc.expect})"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate astgrep test case annotations.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s --dry-run                    # list all annotated cases
  %(prog)s --category gaussdb           # validate GaussDB cases only
  %(prog)s --verbose                    # show passing cases too
        """,
    )
    parser.add_argument(
        "--category", "-c",
        help="Filter to cases whose path contains this substring",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="List discovered cases without running astgrep",
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show passing cases (default: show failures only)",
    )
    args = parser.parse_args()

    categories_root = PROJECT_ROOT / "tests" / "categories"
    if not categories_root.is_dir():
        print(f"Error: {categories_root} not found", file=sys.stderr)
        return 2

    cases = discover_cases(categories_root, args.category)

    if not cases:
        print("No annotated test cases found.")
        if args.category:
            print(f"  (filtered by: {args.category})")
        return 0

    if args.dry_run:
        print(f"Found {len(cases)} annotated test case(s):\n")
        for tc in cases:
            rel = tc.file_path.relative_to(PROJECT_ROOT)
            print(f"  {tc.rule_id:<30s}  {tc.expect:<10s}  {rel}")
        return 0

    print(f"Validating {len(cases)} test case(s)...\n")

    passed = 0
    failed = 0
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
    print("=" * 50)
    print(f" Results: {passed} passed, {failed} failed")
    print("=" * 50)

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
