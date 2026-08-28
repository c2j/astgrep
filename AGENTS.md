# AGENTS.md

**Updated:** 2026-07-20 | **Commit:** 1722c9b | **Branch:** main

## Overview

astgrep (a.k.a. "CR") — Rust workspace for multi-language static analysis. Pattern matching + taint tracking + data flow over tree-sitter ASTs. Targets security vulnerability detection (SQLi, XSS, etc.) across Java/JS/Python/SQL/Bash/XML.

## TDD 工作流（Red → Green → Refactor）

本仓库采用测试驱动开发。一次循环只锁定一个行为：先写会失败的测试（Red），再写最小实现让它通过（Green），最后在测试全绿的前提下重构（Refactor）。探索草稿不得直接合入，必须按本文件用 TDD 重写。

### 先读再改
1. 确认改动落在哪个 crate（本仓库是 Cargo workspace，见「仓库地图」）。
2. 只用本文件列出的 cargo 命令；不要发明裸 `cargo update`、不要擅自切换 toolchain（以 `rust-toolchain.toml` 为准）。
3. 先跑与改动相关的最小测试；提交前再跑 workspace 门禁（fmt + clippy + test）。
4. 完成一个循环后按「完成标准与汇报」汇报，不要只说「做完了」。

### Never / Ask first / Always

**Never（不必请示，直接禁止）**
- 删除、注释、跳过已有测试：`#[ignore]`、注释掉 `#[test]`、把断言改成 `is_ok()` / `unwrap()` 了事
- 修改人类已有测试的断言来迁就实现
- 先提交无测试的业务行为，再「回头补」
- 写永真测试：无断言、只检查 `is_some()`、只 verify 调用次数不查参数与状态
- 用全量端到端测试覆盖本可单测完成的改动
- 提交半成品；每次对人类可见的结果必须能构建且相关测试为绿
- 把探索草稿、临时脚本、调试 `dbg!`/`println!` 留在主代码

**Ask first**
- 改人类已有测试（含断言、fixture、snapshot）
- 新增运行时依赖、`unsafe`、新的 workspace crate、新的外部服务
- 为不可测代码做超出当前改动路径的重构
- 接受/更新 snapshot（insta / golden file）且行为含义发生变化
- 关闭 clippy lint、新增 `#[allow]`

**Always**
- 改遗留路径前：先写特征测试，锁定当前可观察行为（允许丑，必须可重复）
- 新行为：先有会失败的行为断言，再写最少实现
- 难以测试时：先造接缝，再写测试（见「遗留代码与接缝」）
- 测试名描述行为：`should_reject_negative_amount`
- 现有测试因你的改动失败：修实现，不修测试（除非人类明确要求）

测试权限：

| 测试来源 | 权限 |
|---|---|
| 人类已有测试 | 只读 |
| 本任务新建测试 | 可改，直到该行为稳定 |
| 过时或环境偶发失败 | 只报告，不擅自跳过 |

### 工作流

**Red** — 写生产行为之前先写测试；测试必须能被收集且必须失败（断言失败，或因缺失 API 导致编译失败，二者都算合法 Red）。修改已有功能先写特征测试锁定当前输出。一次只加一个行为的测试。

**Green** — 只写让当前失败测试通过的最少代码。禁止删掉/改掉失败测试、一次引入多个未验证变更、用更宽断言或 `unwrap()` 换绿。

**Refactor** — 相关测试全绿后才重构；重构后立刻跑同一组测试；范围限于当前 crate。

**探索 vs 实现** — 需求或方案不清可写草稿验证；草稿不得合并；方案确定后必须走 TDD 重写。

### 遗留代码与接缝

**特征测试** — 锁定现有行为，不是证明它正确。用固定 fixture 或 `insta` snapshot。更新 snapshot 必须在汇报里写清 diff 含义；默认不接受「看起来差不多」。

**接缝（优先顺序，靠后的更差）**
1. trait + 泛型或 `impl Trait`，测试用假类型
2. 用类型去掉非法状态（enum / newtype），而不是在测试里补分支
3. 时钟、ID、熵、文件系统做成可注入依赖；测试用 `tempfile` / 内存实现
4. `unsafe` 不是接缝。新增 `unsafe` 必须 Ask first，并写 `SAFETY` 注释

只给即将修改的代码路径补测试，不要一次性给整个模块「补全覆盖率」。

### 测试分层

| 层级 | 位置 | 测什么 |
|---|---|---|
| 单元 | `src` 内 `#[cfg(test)] mod tests` | 模块不变量、错误类型、状态转换 |
| 集成 | `tests/*.rs` | 公共 API；不可访问私有项 |
| 文档测试 | `///` 示例 | 公共 API 必须可运行；禁止滥用 `no_run` |
| CLI/二进制 | 项目惯用方式 | 退出码与 stdout 契约 |
| 不变量 | `proptest`（项目已用时） | 往返解析、幂等、单调性 |
| 特征/快照 | `insta` 或固定 fixture | 遗留输出；接受 snapshot 必须说明 |

不要把本该测公共契约的内容塞进 `#[cfg(test)]` 去读私有字段。

Rust 的 Red 允许是：测试引用了尚不存在的类型/函数导致编译失败。不要为了先编译而写空 `todo!()` 实现再补测试——可以留 `todo!()` 仅作为 Green 的最小占位，且下一步必须替换。

### Rust Never 补遗
- 库代码（非 main/example/测试）用 `unwrap` / `expect` / `panic!` 做控制流
- 无必要 `unsafe`；有则必须 `SAFETY` 注释
- 一次性 `cargo update` 整个 lockfile
- 用 `#[allow(...)]` 静默应修复的 lint
- 为绿而改 snapshot 却不解释行为是否应该变

### 命令

```bash
# 单测（单 crate，按测试名过滤）
cargo test -p astgrep-core <test_name>

# 单 crate
cargo test -p astgrep-core

# 全量测试
cargo test

# 提交前门禁
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test

# 规则测试注解校验（规则驱动测试必须先通过注解校验）
python3 tests/scripts/validate_annotations.py
```

> 规则驱动测试遵循 `tests/CONVENTIONS.md` 的 `@rule`/`@expect`/`@desc` 自描述约定；semgrep-core 遗留测试用 `// MATCH:` / `// ERROR:` 格式。新测试必须带注解。`cargo fmt --all -- --check` 是最常见 CI 失败点。

循环内只跑受影响 crate；提交前再 workspace。

### 完成标准与汇报

提交或交还人类前，确认：
- [ ] 新行为有失败→通过的测试
- [ ] 修改的遗留路径有特征测试
- [ ] 未删除、跳过、改写人类已有测试
- [ ] 已跑与改动匹配的门禁（fmt + clippy + test）
- [ ] `cargo fmt` 与 clippy 干净
- [ ] 没有把草稿、调试输出、无主 lockfile 大面积变更带上

每个 TDD 循环汇报：
1. 测试了什么行为（测试函数名）
2. 最小实现改了哪些文件
3. 是否重构、边界在哪
4. 实际执行的命令和结果（通过 / 失败原因；不要只写「测过了」）

### 质量判断（自我检查）
- 这条测试在实现写错时会失败吗？
- 我是否在测行为，而不是私有实现细节？
- 我是否用 skip、更宽断言、unwrap、snapshot 盲收换绿？
- 命令是否来自本文件，而不是我编的？

## Structure

```
.
├── src/main.rs              # Binary entry → delegates to astgrep_cli::run()
│                               Also intercepts `mcp` subcommand before CLI parser
│                               (to avoid circular dep between astgrep-cli and astgrep-mcp)
├── src/lib.rs               # Re-exports all workspace crates
├── crates/
│   ├── astgrep-core/        # Language enum, AstNode trait, Error types, config
│   ├── astgrep-ast/         # UniversalNode, visitor, builder
│   ├── astgrep-parser/      # Tree-sitter adapters per language + registry  [→ AGENTS.md]
│   ├── astgrep-matcher/     # Pattern matching, metavariables, conditions
│   ├── astgrep-dataflow/    # Taint/flow/call-graph/constant analysis       [→ AGENTS.md]
│   ├── astgrep-rules/       # YAML rule parsing, validation, execution      [→ AGENTS.md]
│   ├── astgrep-cli/         # CLI commands (analyze/validate/info/...) + output formats
│   ├── astgrep-mcp/         # MCP (Model Context Protocol) stdio server for AI assistants
│   │                           Exposes 4 tools: analyze_code / validate_rules / list_rules / list_languages
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
| MCP tools | `crates/astgrep-mcp/src/tools/{analyze,validate,query}.rs` | 4 tools over stdio; calls analyze_collect / validate_collect |
| MCP server | `crates/astgrep-mcp/src/server.rs` | AstgrepServer with rmcp 0.2 SDK, stdio transport |
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
cargo run -- mcp                         # Start MCP server over stdio (for AI assistants)
cargo run -- mcp --rules-dir tests/categories/rules/  # MCP server with custom rules dir
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
- **Release workflow**: Tags (vX.Y.Z) must be on the `main` branch. The release workflow aborts if the tag is on a feature branch.
- **MCP tools implement `analyze_collect` / `validate_collect`**: These are core reusable functions extracted from `astgrep-cli` commands. When adding new MCP capabilities, call them rather than duplicating CLI logic.
- **MCP circular dependency**: `astgrep-mcp` depends on `astgrep-cli` (for `EnhancedAnalysisConfig`, `analyze_collect`, etc.). To avoid a cycle, the `mcp` subcommand is intercepted in `src/main.rs` before `astgrep_cli::run()`. The `Commands::Mcp` variant exists in the CLI enum for `--help` documentation but hits `unreachable!()` if reached.

## Error Handling

- `anyhow::Result` for application error propagation
- `thiserror` for typed errors in `astgrep-core::error::AnalysisError`
- Use `.context("...")` when wrapping; use `?` operator throughout
- Never use `unwrap()` in production code; `expect("reason")` acceptable in tests only

## Anti-Patterns (THIS PROJECT)

- Do NOT use `tree-sitter-sql` — use `tree-sitter-sequel` for SQL parsing
- Do NOT add dependencies directly to crate Cargo.toml — add to workspace deps first, then `workspace = true`
- EXCEPTION: non-crates.io external deps (git sources like `ogsql-parser`) MUST be declared explicitly in the consuming crate, NOT via `workspace = true` — otherwise the crate cannot be consumed as a path dependency from another workspace (see issue #21)
- Do NOT modify `Language` enum without also updating parser registry + `Language::extensions()` + `Language::from_str()`
- Do NOT write tests without annotations — use `@rule`/`@expect`/`@desc` for rule-driven tests (see `tests/CONVENTIONS.md`), or `// MATCH:` / `// ERROR:` for semgrep-core legacy tests
- Do NOT use `as any`, `unwrap()` in non-test code
- Do NOT suppress warnings with `#[allow(...)]` without a comment explaining why
- Do NOT add a direct dependency from `astgrep-cli` to `astgrep-mcp` — this creates a circular dep. Handle MCP routing at `src/main.rs` level.

## Notes

- Root `src/lib.rs` re-exports all workspace crates with aliases — `astgrep_core` available as `crates::core`
- `crates/astgrep-cli/src/commands/` has both `_enhanced` and base versions of analyze/validate — enhanced versions are the newer API
- `newtest/` directory contains an alternative test infrastructure alongside `tests/categories/`
- Various stray files at root (test_*.rs, *.sh, *.py) are ad-hoc scripts, not part of the build
- `crates/astgrep-gui` and `crates/astgrep-web` are secondary interfaces — CLI is primary
- `crates/astgrep-mcp` is a third interface that exposes analysis via MCP protocol for AI assistants (Claude Desktop, Copilot, etc.)
- MCP server uses `rmcp` 0.2 SDK with `#[tool]` / `#[tool_router]` macros. New tools are added as methods on `AstgrepServer`.
