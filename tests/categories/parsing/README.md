# Parsing Tests

Tests verifying that astgrep can parse source files without errors.

## Related Directories

- `parsing/` — Files that should parse successfully (per language)
- `parsing_errors/` — Files with intentional parse errors
- `parsing_missing/` — Files with missing grammar features
- `parsing_patterns/` — Parsing-specific pattern tests
- `parsing_todo/` — Future parsing test cases

These directories test the parser, not rule matching. They do not use
`@rule/@expect/@desc` annotations.
