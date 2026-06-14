# AGENTS.md

**Updated:** 2026-05-19 | **Commit:** 935b443 | **Branch:** main

## Overview

astgrep (a.k.a. "CR") — Rust workspace for multi-language static analysis. Pattern matching + taint tracking + data flow over tree-sitter ASTs. Targets security vulnerability detection (SQLi, XSS, etc.) across Java/JS/Python/SQL/Bash/XML.

## Structure

```
.
├── src/main.rs              # Binary entry → delegates to astgrep_cli::run()
├── src/lib.rs               # Re-exports all workspace crates
├── crates/
│   ├── astgrep-core/        # Language enum, AstNode trait, Error types, config
│   ├── astgrep-ast/         # UniversalNode, visitor, builder
│   ├── astgrep-parser/      # Tree-sitter adapters per language + registry  [→ AGENTS.md]
│   ├── astgrep-matcher/     # Pattern matching, metavariables, conditions
│   ├── astgrep-dataflow/    # Taint/flow/call-graph/constant analysis       [→ AGENTS.md]
│   ├── astgrep-rules/       # YAML rule parsing, validation, execution      [→ AGENTS.md]
│   ├── astgrep-cli/         # CLI commands (analyze/validate/info/...) + output formats
│   ├── astgrep-web/         # Axum REST API server (handlers/{analyze,rules,auth,...})
│   ├── astgrep-gui/         # egui desktop playground
│   └── test-utils/          # MockAstNode, MockParser, MockRules
├── tests/categories/        # 43 test categories (patterns/rules/parsing/tainting/...)
├── newtest/                 # New test infrastructure (testcases/{lang}/pattern-matching/)
├── docs/                    # Design docs (v1..v1.3), API/User/Refactoring guides
└── scripts/                 # build-uos.sh, compare_with_semgrep.py
```

## Where to Look

| Task | Location | Notes |
|------|----------|-------|
| Add new language | `crates/astgrep-core/src/types.rs` Language enum + `crates/astgrep-parser/src/{lang}.rs` | Also update `Language::extensions()` and registry |
| Add/modify rules | `crates/astgrep-rules/src/parser/` + `tests/categories/rules/` | YAML format with patterns/metavariables/dataflow |
| Pattern matching | `crates/astgrep-matcher/src/matcher.rs` → `advanced_matcher.rs` | `PreciseExpressionMatcher` for exact matching |
| Taint analysis | `crates/astgrep-dataflow/src/taint.rs` → `enhanced_taint.rs` | Source→Sink flow with sanitizer support |
| CLI commands | `crates/astgrep-cli/src/commands/*.rs` | 14 commands including migrate, validate_enhanced |
| Output formats | `crates/astgrep-cli/src/output/analysis/{sarif,json,text,html,markdown,semgrep}.rs` | 6 output formats |
| SQL parsing | `crates/astgrep-parser/src/sql.rs` | Uses tree-sitter-sequel, NOT tree-sitter-sql |
| SQL dialects | `crates/astgrep-parser/src/dialect/` | SqlDialect enum + dispatcher; GaussDB→ogsql-parser, PolarDB→sqlparser-rs |
| SQL dialect adapters | `crates/astgrep-parser/src/adapter/{ogsql,sqlparser}/` | Convert parser-specific AST → UniversalNode |
| SQL dialect rules | `tests/categories/rules/sql_dialects/{gaussdb,polardb_mysql}/` | Dialect-specific YAML rules |
| Error types | `crates/astgrep-core/src/error.rs` | `AnalysisError` enum with thiserror |
| Config | `crates/astgrep-core/src/config.rs` | AnalysisConfig, SQL statement boundary toggle |
| Test cases | `tests/categories/{category}/cases/{concern}/` | `@rule`/`@expect`/`@desc` annotations; see `tests/CONVENTIONS.md`. Legacy semgrep-core tests in `patterns/` use `// MATCH:` format |

## Commands

```bash
cargo build                              # Build all workspace crates
cargo build --release                    # LTO + codegen-units=1 + panic=abort
cargo test                               # All tests
cargo test -p astgrep-core               # Single crate
cargo test -p astgrep-parser             # Parser crate
cargo test -p astgrep-rules              # Rules crate
cargo test test_name -- --nocapture      # Single test with output
RUST_LOG=debug cargo run -- analyze      # Debug logging
cargo run -- analyze                     # Analyze current dir
cargo run -- validate rules/*.yml        # Validate rule YAML files
cargo run -- info --extensions           # List supported languages + extensions
cargo run -- analyze --format sarif -o results.sarif  # SARIF output
```

## Key Conventions

- **Language enum** (`astgrep_core::Language`) has 6 variants: Java, JavaScript, Python, Sql, Bash, Xml — but parser crate has modules for C/C#/Kotlin/Ruby/Swift/PHP too (tree-sitter adapters without full Language enum support yet)
- **Workspace deps** defined in root `Cargo.toml [workspace.dependencies]`, referenced via `workspace = true`
- **Re-exports** at crate root: `pub use module::*` pattern used heavily — check `lib.rs` before importing from submodules
- **Tree-sitter version**: 0.25 for most grammars, 0.23.5 for Java, 0.3.11 for sequel (SQL)
- **Release profile**: LTO=true, codegen-units=1, panic=abort — benchmark before merging perf-sensitive changes
- **Test infrastructure**: Rule-driven categories follow self-describing pattern in `tests/CONVENTIONS.md` (`@rule`/`@expect`/`@desc` annotations + `rules/`+`cases/` layout). Validate with `python3 tests/scripts/validate_annotations.py`. Legacy semgrep-core tests in `patterns/`, `semgrep-core/` use `// MATCH:` / `// ERROR:` format — see `tests/README.md`.
- **SQL statement boundary**: Configurable via CLI `--sql-statement-boundary` flag or YAML `options.sql_statement_boundary`
- **Pre-commit hooks**: Run `lefthook install` after cloning. Hooks enforce fmt + clippy on commit, full test + audit on push.
- **No CI/CD** pipeline yet — manual validation via `cargo test`

## Error Handling

- `anyhow::Result` for application error propagation
- `thiserror` for typed errors in `astgrep-core::error::AnalysisError`
- Use `.context("...")` when wrapping; use `?` operator throughout
- Never use `unwrap()` in production code; `expect("reason")` acceptable in tests only

## Anti-Patterns (THIS PROJECT)

- Do NOT use `tree-sitter-sql` — use `tree-sitter-sequel` for SQL parsing
- Do NOT add dependencies directly to crate Cargo.toml — add to workspace deps first, then `workspace = true`
- Do NOT modify `Language` enum without also updating parser registry + `Language::extensions()` + `Language::from_str()`
- Do NOT write tests without annotations — use `@rule`/`@expect`/`@desc` for rule-driven tests (see `tests/CONVENTIONS.md`), or `// MATCH:` / `// ERROR:` for semgrep-core legacy tests
- Do NOT use `as any`, `unwrap()` in non-test code
- Do NOT suppress warnings with `#[allow(...)]` without a comment explaining why

## Notes

- Root `src/lib.rs` re-exports all workspace crates with aliases — `astgrep_core` available as `crates::core`
- `crates/astgrep-cli/src/commands/` has both `_enhanced` and base versions of analyze/validate — enhanced versions are the newer API
- `newtest/` directory contains an alternative test infrastructure alongside `tests/categories/`
- Various stray files at root (test_*.rs, *.sh, *.py) are ad-hoc scripts, not part of the build
- `crates/astgrep-gui` and `crates/astgrep-web` are secondary interfaces — CLI is primary
