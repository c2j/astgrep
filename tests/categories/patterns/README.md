# Patterns — Semgrep-Core Compatibility Tests

**INTENTIONAL LEGACY — DO NOT REORGANIZE**

This directory contains semgrep-core upstream test cases in their original
format (`.sgrep` pattern files + target files with `// MATCH:` / `// ERROR:`
annotations). Used for astgrep-vs-semgrep compatibility regression testing.

**Do NOT:**
- Rename files to `{RULE_ID}_{desc}` convention
- Replace `// MATCH:` with `@rule/@expect` annotations
- Restructure into `rules/+cases/` layout

**Format:** See [tests/README.md](../../README.md) for semgrep-core format.
**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md)
