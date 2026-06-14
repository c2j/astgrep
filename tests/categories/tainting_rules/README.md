# Tainting Rules — Semgrep Taint Analysis Tests

**INTENTIONAL LEGACY — DO NOT REORGANIZE**

Multi-language taint analysis test cases in semgrep format (`#ruleid:` /
`#OK:` annotations). 8 language subdirectories (go, python, js, java, php,
scala, ts, ruby).

Used for astgrep-vs-semgrep taint analysis compatibility regression testing.

**Do NOT** convert `#ruleid:` to `@rule/@expect` annotations or restructure
into `rules/+cases/` layout.

**Format:** See [tests/README.md](../../README.md) for semgrep annotation format.
**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md)
