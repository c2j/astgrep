# Autofix — Code Fix Output Tests

**SPECIAL CATEGORY**

Tests astgrep's autofix output (transforming matched code to fixed code).
Each language subdirectory contains source files and expected fix output.

The `@rule/@expect/@desc` pattern applies to matching detection, but autofix
tests additionally verify the fix transformation output.

**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md)
