# Errors — Rule Validation Error Tests

**EXEMPT from self-describing pattern**

This directory tests astgrep's rule YAML validation — each file intentionally
contains a malformed rule (bad language, missing id, invalid severity, etc.)
and expects astgrep to reject it during validation.

There are no target source files; the YAMLs themselves are the test cases.
The `@rule/@expect/@desc` annotation pattern does not apply.

**Exemption:** Documented in [tests/CONVENTIONS.md](../../CONVENTIONS.md)
