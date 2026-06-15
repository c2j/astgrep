# Rules — Semgrep-Core Pattern Matching Tests

**INTENTIONAL LEGACY — DO NOT REORGANIZE**

This directory contains hundreds of semgrep-core upstream test cases in their
original format (YAML rule + target source file pairs). Used for astgrep-vs-semgrep
pattern matching compatibility regression testing.

**Do NOT:**
- Rename files to `{RULE_ID}_{desc}` convention
- Add `@rule/@expect/@desc` annotations
- Restructure into `rules/+cases/` layout

**Format:** See [tests/README.md](../../README.md) for semgrep-core format.
**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md)
