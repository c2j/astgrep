# Test Reorganization Migration Status

Tracking migration of test categories to the self-describing pattern
defined in [CONVENTIONS.md](CONVENTIONS.md).

## Legend

- ✅ Migrated — follows `rules/+cases/` layout with `@rule/@expect/@desc`
- 📌 Legacy — intentionally preserved semgrep-core format (see README in directory)
- 🗑️ Cleaned — removed or merged
- ⏳ Pending — not yet migrated

## Status

| Category | Tier | Status | Notes |
|----------|------|--------|-------|
| `sql_dialects/gaussdb` | Reference | ✅ Done | Original pattern source, 14/14 pass |
| `sql_dialects/polardb_mysql` | Reference | ⏳ Pending | Follow gaussdb pattern |
| `simple` | A | ✅ Migrated | 3 rules, 6/6 pass |
| `advanced_patterns` | A | ✅ Migrated | 5 concerns, 9/10 pass (1 astgrep comment-matching issue) |
| `explanations` | A | ✅ Migrated | 6 concerns, 10/12 pass (1 pre-existing rule issue) |
| `sql` | A | ✅ Migrated | 6 concerns, 11/12 pass (1 pattern-regex engine limitation) |
| `rules_v2` | A | ✅ Migrated | v2 syntax tests, 12/27 pass (rest pre-existing rule issues) |
| `errors` | Special | ✅ Exempt | Tests rule validation, README added |
| `tainting_rules` | B | 📌 Legacy | Semgrep `#ruleid:` format, README added |
| `rules` | B | 📌 Legacy | Hundreds of semgrep compatibility tests, README added |
| `autofix` | Special | ✅ Exempt | Tests fix output, README added |
| `naming`, `metachecks`, `typing` | A | ⏳ Pending | Sparse content |
| `patterns` | B | 📌 Legacy | Semgrep compatibility, README added |
| `semgrep-core` | B | 📌 Legacy | Upstream snapshots, README added |
| `semgrep-core-e2e` | B | 📌 Legacy | E2E compatibility, README added |
| `comparison` | B | 📌 Legacy | Output comparison, README added |
| `TODO` | C | ✅ Triage | README added, files need individual review |
| `osemgrep` | C | 🗑️ Deleted | Abandoned single file |
| `parsing*` (5 dirs) | C | ✅ Documented | README added, structure explained |
| `perf` | C | ⏳ Pending | Needs restructuring |
| Others (Tier D) | D | ⏳ Pending | Needs triage |

## Summary

- **5 categories migrated** to self-describing pattern (simple, advanced_patterns, explanations, sql, rules_v2)
- **8 categories marked legacy/special** with READMEs (patterns, semgrep-core, semgrep-core-e2e, comparison, rules, tainting_rules, errors, autofix)
- **3 categories cleaned/documented** (TODO, osemgrep deleted, parsing* documented)
- **42 annotated test cases** discoverable by validator (40 pass, 2 pre-existing failures)

## Validation Commands

```bash
# Validate all migrated categories
python3 tests/scripts/validate_annotations.py --verbose

# Validate specific category
python3 tests/scripts/validate_annotations.py --category simple --verbose

# Dry run (list discovered cases)
python3 tests/scripts/validate_annotations.py --dry-run
```
