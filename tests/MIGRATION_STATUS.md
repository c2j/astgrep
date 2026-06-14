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
| `sql` | A | ⏳ Pending | 6 yaml+sql pairs, complex multi-rule files |
| `errors` | A | ⏳ Exempt | Tests rule validation, not pattern matching |
| `tainting_rules` | A | ⏳ Pending | Uses `#ruleid:` format, needs annotation conversion |
| `rules` + `rules_v2` | A | ⏳ Pending | Evaluate merge potential |
| `autofix` | A | ⏳ Pending | Language subdirs, tests fix output |
| `naming`, `metachecks`, `typing` | A | ⏳ Pending | Sparse content |
| `patterns` | B | 📌 Legacy | Semgrep compatibility, README added |
| `semgrep-core` | B | 📌 Legacy | Upstream snapshots, README added |
| `semgrep-core-e2e` | B | 📌 Legacy | E2E compatibility, README added |
| `comparison` | B | 📌 Legacy | Output comparison, README added |
| `TODO` | C | ⏳ Pending | Needs cleanup/archive |
| `osemgrep` | C | ⏳ Pending | Abandoned, single file |
| `parsing*` (5 dirs) | C | ⏳ Pending | Merge into `parsing/{success,errors,...}` |
| `perf` | C | ⏳ Pending | Needs restructuring |
| Others (Tier D) | D | ⏳ Pending | Needs triage |

## Validation Commands

```bash
# Validate all migrated categories
python3 tests/scripts/validate_annotations.py --verbose

# Validate specific category
python3 tests/scripts/validate_annotations.py --category simple --verbose

# Dry run (list discovered cases)
python3 tests/scripts/validate_annotations.py --dry-run
```
