# 测试守护体系完善计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 建立完整的测试守护体系，确保每个公共 API 至少有一个单元测试和集成测试，并在大特性开发期间防止回归。

**Architecture:** 分四个阶段推进 — Phase 0 修地基（编译错误 + CI 失效 + clippy 债务），Phase 1 补齐所有公共 API 测试，Phase 2 建立体系化守护手段（proptest / 并发 / 回归），Phase 3 常态化基础设施（hooks / 覆盖率 / 性能守护）。

**Tech Stack:** Rust 1.93.0, cargo test, cargo clippy, cargo-deny, cargo-audit, proptest, criterion, cargo-llvm-cov, lefthook

---

## Phase 0 — 地基修复（阻塞一切后续工作）

### Task 0.1: 修复 test-utils 编译错误

**Files:**
- Modify: `crates/test-utils/src/mock_parser.rs:112-129`

**问题:** mock_parser.rs 引用了不存在的 Language::C, Language::CSharp, Language::Php。Language 枚举只有 Java, JavaScript, Python, Sql, Bash, Xml。

**Step 1: 修复 mock_parser.rs**

将 `with_default_parsers()` 中的语言列表改为仅使用实际存在的 Language 枚举变体：

```rust
// crates/test-utils/src/mock_parser.rs:112-129
pub fn with_default_parsers() -> Self {
    let mut registry = Self::new();

    for &language in &[
        Language::Java,
        Language::JavaScript,
        Language::Python,
        Language::Bash,
        Language::Sql,
        Language::Xml,
    ] {
        registry.register(language, MockParser::simple_program_parser(language));
    }

    registry
}
```

**Step 2: 验证编译**

Run: `cargo check -p test-utils`
Expected: 无错误

**Step 3: 验证测试通过**

Run: `cargo test -p test-utils`
Expected: 所有测试通过

**Step 4: Commit**

```
fix(test-utils): remove non-existent Language variants from mock_parser
```

---

### Task 0.2: 修复 CI clippy 失效

**Files:**
- Modify: `.github/workflows/ci.yml:38`

**问题:** `|| true` 使 clippy 警告被静默吞掉。

**Step 1: 移除 || true**

```yaml
# .github/workflows/ci.yml:38
# 修改前:
#   run: cargo clippy --workspace --exclude astgrep-web --exclude astgrep-gui -- -D warnings -W clippy::all 2>&1 || true
# 修改后:
- name: Clippy
  run: cargo clippy --workspace --exclude astgrep-web --exclude astgrep-gui -- -D warnings 2>&1
```

> 注意：此步骤在 Task 0.3（修复所有 clippy 警告）之后才能合入。否则 CI 会挂。

**Step 2: Commit（暂不推送，等 Task 0.3 完成后一起推送）**

```
fix(ci): remove || true from clippy to enforce lint checks
```

---

### Task 0.3: 修复所有 clippy 警告（~112 个）

**策略:** 先自动修复，再手动修复无法自动处理的。

**Step 1: 自动修复（批量处理 ~70 个低严重性警告）**

Run:
```bash
cargo clippy --fix --workspace --exclude astgrep-web --exclude astgrep-gui --allow-dirty --allow-staged
```

这会自动修复：
- 20 个 `or_insert_with(Vec::new)` → `or_default()`
- 12 个 manual string strip → `strip_prefix()`/`strip_suffix()`
- 5 个 needless borrow
- 5 个 `map_or(false, ...)` → `is_some_and(...)`
- 3 个 `len() > 0` → `!is_empty()`
- 4 个 collapsible if
- 1 个 needless return
- 1 个 map_clone → cloned()
- 1 个 useless_vec → array
- 1 个 manual_flatten
- 1 个 get_first
- 1 个 match_like_matches_macro

**Step 2: 手动修复 — never_loop（CRITICAL）**

File: `crates/astgrep-matcher/src/advanced_matcher.rs:236`

```rust
// 修改前:
while let Some(current_node) = current {
    if self.matches_pattern(inner_pattern, current_node)? {
        return Ok(true);
    }
    current = None;
}

// 修改后:
if let Some(current_node) = current {
    if self.matches_pattern(inner_pattern, current_node)? {
        return Ok(true);
    }
}
```

**Step 3: 手动修复 — 9 个 Missing Default Impl**

为以下类型添加 `impl Default`（通过 `#[derive(Default)]` 或手动 impl）：

1. `crates/astgrep-parser/src/bash.rs` — BashAdapter
2. `crates/astgrep-parser/src/java.rs` — JavaAdapter
3. `crates/astgrep-parser/src/javascript.rs` — JavaScriptAdapter
4. `crates/astgrep-parser/src/javascript_optimizer.rs` — JavaScriptOptimizer
5. `crates/astgrep-parser/src/python.rs` — PythonAdapter
6. `crates/astgrep-parser/src/sql.rs` — SqlAdapter
7. `crates/astgrep-parser/src/tree_sitter_parser/integration.rs` — MetaVariableBindings
8. `crates/astgrep-matcher/src/advanced_matcher.rs` — AdvancedSemgrepMatcher
9. `crates/astgrep-matcher/src/precise_matcher.rs` — PreciseExpressionMatcher

模式：
```rust
// 对于有 new() -> Self 的类型:
impl Default for XxxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// 或直接 derive:
#[derive(Default)]
pub struct XxxAdapter { ... }
```

**Step 4: 手动修复 — 已知的特定警告**

| # | 文件 | 修复 |
|---|------|------|
| 1 | `astgrep-core/src/config.rs:33` | `path: &PathBuf` → `path: &Path` |
| 2 | `astgrep-core/src/error.rs:102` | `Error::new(ErrorKind::Other, msg)` → `Error::other(msg)` |
| 3 | `astgrep-core/src/models/test_asset.rs:249` | `m.len() as u64` → `m.len()` |
| 4 | `astgrep-core/src/models/test_case.rs:521` | 手动 Default impl → `#[derive(Default)]` + `#[default] Normal` |
| 5 | `astgrep-core/src/patterns.rs:295` | `fn not()` → `fn pattern_not()` |
| 6 | `astgrep-matcher/src/conditions.rs:49` | 重命名 `from_str` 或实现 `std::str::FromStr` |
| 7 | `astgrep-matcher/src/advanced_matcher.rs:2565` | `if ... { value } else { value }` → `value` |

**Step 5: 手动修复 — 5 个 Loop Issues**

1. `astgrep-dataflow/src/symbol_table.rs:203` — `loop { if let ... }` → `while let ...`
2. `astgrep-matcher/src/parser.rs:216` — `while let Some(ch) = chars.next()` → `for ch in chars.by_ref()`
3. `astgrep-matcher/src/advanced_matcher.rs:1559` — needless_range_loop → 用迭代器
4. `astgrep-matcher/src/advanced_matcher.rs:1695` — mut_range_bound
5. `astgrep-matcher/src/advanced_matcher.rs:1672` — unused_enumerate_index

**Step 6: 手动修复 — 6 个 Unused Fields**

给未使用的字段加 `#[allow(dead_code)]` 并注释说明保留原因，或删除：

1. `javascript_optimizer.rs:114` — ExportInfo: `is_default`, `source_module`
2. `javascript_optimizer.rs:122` — RequireInfo: `assigned_to`
3. `tree_sitter_parser/conversion.rs:14` — CharPosition: `line`
4. `tree_sitter_parser/conversion.rs:25` — PreciseLocation: `start_byte`, `end_byte`
5. `astgrep-matcher/src/parser.rs:61` — PatternParser: `strict_mode`
6. `astgrep-matcher/src/advanced_matcher.rs:2150` — 未读取的 `bound_metavars` 赋值

**Step 7: 验证 clippy 干净**

Run: `cargo clippy --workspace --exclude astgrep-web --exclude astgrep-gui -- -D warnings 2>&1`
Expected: 0 warnings, 0 errors

**Step 8: 验证全量测试通过**

Run: `cargo test --workspace --exclude astgrep-web --exclude astgrep-gui`
Expected: 所有测试通过

**Step 9: Commit**

```
fix: resolve all clippy warnings across workspace

- Auto-fix: or_default, strip_prefix, needless borrow, map_or → is_some_and
- Fix never_loop in advanced_matcher.rs
- Add Default impl for 9 types
- Rename not() → pattern_not() to avoid std::ops::Not conflict
- Fix unused fields with #[allow(dead_code)] + comments
- Fix loop issues: while-let, needless range, unused enumerate
```

---

### Task 0.4: 锁定 Rust 工具链

**Files:**
- Create: `rust-toolchain.toml`

**Step 1: 创建 rust-toolchain.toml**

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.93"
components = ["clippy", "rustfmt"]
```

**Step 2: Commit**

```
chore: pin Rust toolchain to 1.93
```

---

### Task 0.5: 添加依赖安全审计

**Files:**
- Create: `deny.toml`
- Modify: `.github/workflows/ci.yml`（添加 security job）

**Step 1: 初始化 cargo-deny**

Run:
```bash
cargo install cargo-deny
cargo deny init
```

**Step 2: 配置 deny.toml**

```toml
# deny.toml — 最小安全配置
[advisories]
db-path = "~/.cargo/advisory-db"
vulnerability = "deny"
unmaintained = "warn"
unsound = "deny"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016"]
unlicensed = "deny"

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

**Step 3: 安装并运行 cargo-audit**

Run:
```bash
cargo install cargo-audit
cargo audit
```

修复发现的任何安全问题。

**Step 4: 在 CI 中添加安全审计 job**

```yaml
# 添加到 .github/workflows/ci.yml
  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Run audit
        run: cargo audit
      - name: Install cargo-deny
        run: cargo install cargo-deny
      - name: Check dependencies
        run: cargo deny check
```

**Step 5: Commit**

```
chore: add cargo-deny and cargo-audit for dependency safety
```

---

### Task 0.6: Phase 0 最终验证

**Step 1: 全量编译检查**

Run: `cargo check --workspace`
Expected: 0 errors

**Step 2: 全量 clippy 检查**

Run: `cargo clippy --workspace --exclude astgrep-web --exclude astgrep-gui -- -D warnings`
Expected: 0 warnings

**Step 3: 全量测试**

Run: `cargo test --workspace --exclude astgrep-web --exclude astgrep-gui`
Expected: 所有测试通过

**Step 4: 格式化检查**

Run: `cargo fmt --all -- --check`
Expected: 无差异

**Step 5: 合并所有 Phase 0 提交到 main**

---

## Phase 1 — 公共 API 测试补全

> 目标：每个公共 API（pub fn/struct/enum/trait）至少有一个单元测试和一个集成测试。
> 策略：按 crate 拆分为独立 Task，每个 Task 可由独立 subagent 并行执行。

### Task 1.1: astgrep-core 未覆盖 API 测试

**需要新增测试的文件：**

#### 1.1a: config.rs（零测试）

**Files:**
- Modify: `crates/astgrep-core/src/config.rs`（在文件末尾添加 `#[cfg(test)] mod tests`）

需要测试的公共 API：
- `PathHandler::new()`, `with_base_dir()`, `normalize_path()`, `make_relative()`, `join()`, `base_dir()`
- `PathHandler::default()`
- `AstGrepConfig::default()`

测试要点：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_handler_new() { ... }
    #[test]
    fn test_path_handler_normalize() { ... }  // Windows/Unix 路径
    #[test]
    fn test_path_handler_make_relative() { ... }
    #[test]
    fn test_path_handler_join() { ... }
    #[test]
    fn test_astgrep_config_default() { ... }
}
```

#### 1.1b: execution.rs（零测试）

需要测试的公共 API：
- `ExecutionConfig::default()`
- `ExecutionContext` 构造
- `ExecutionResult` 字段访问
- `ScriptExecutor::new()`

#### 1.1c: patterns.rs（零测试）

需要测试的公共 API：
- `MatchBinding::new()`, `with_location()`, Display, Deref
- `PatternType` 各变体
- `SemgrepPattern::simple()`, `either()`, `inside()`, `pattern_not()`, `regex()`, `with_condition()`, `with_metavariable_pattern()`, `with_focus()`
- `MetavariablePattern::new()`, `with_patterns()`, `with_regex()`, `with_type_constraint()`
- `EntropyAnalysis`, `TypeAnalysis`, `ComplexityAnalysis` 构造
- `Condition` 各变体
- `ComparisonOperator` 各变体
- `SemgrepMatchResult::new()`, `with_confidence()`

#### 1.1d: models/mod.rs（零测试）

需要测试的公共 API：
- `ValidationStatus` 各变体
- `ValidationResult::new()`, `with_metadata()`
- `TestAsset` 构造和字段访问

---

### Task 1.2: astgrep-parser 未覆盖 API 测试

#### 1.2a: lib.rs — LanguageParserRegistry（零测试）

需要测试的公共 API：
- `LanguageParserRegistry::new()`, `register_parser()`, `get_parser()`, `supported_languages()`, `supports_language()`
- `LanguageParserRegistry::default()`

#### 1.2b: sql.rs（零测试 — 最关键的缺口）

**Files:**
- Modify: `crates/astgrep-parser/src/sql.rs`（添加测试模块）
- 或 Create: `crates/astgrep-parser/tests/sql_tests.rs`

需要测试：
- 基本 SQL 解析：SELECT, INSERT, UPDATE, DELETE
- 复杂 SQL：JOIN, 子查询, 窗口函数
- 畸形 SQL 的错误恢复
- sql_statement_boundary 行为
- tree-sitter-sequel 特有节点类型

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_simple_select() { ... }
    #[test]
    fn test_parse_insert() { ... }
    #[test]
    fn test_parse_join() { ... }
    #[test]
    fn test_parse_subquery() { ... }
    #[test]
    fn test_parse_malformed_sql() { ... }
    #[test]
    fn test_statement_boundary() { ... }
    // ... ≥20 个测试
}
```

---

### Task 1.3: astgrep-matcher 未覆盖 API 测试

#### 1.3a: advanced_matcher.rs（有部分测试但不够）

需要补充的测试：
- 复杂嵌套模式匹配
- metavariable 约束组合
- 递归模式匹配
- 大文件性能正确性

#### 1.3b: precise_matcher.rs（零测试）

需要测试的公共 API：
- `PreciseExpressionMatcher::new()`, `default()`
- 精确表达式匹配行为
- 与 AdvancedPatternMatcher 的对比

#### 1.3c: script_classifier.rs（零测试）

需要测试的公共 API：
- `ScriptClassifier` 构造和配置
- 各 ScriptType 分类
- 置信度计算
- 边界情况（空文件、二进制文件）

---

### Task 1.4: astgrep-dataflow 未覆盖 API 测试

#### 1.4a: lib.rs — DataFlowAnalyzer（零测试）

需要测试的公共 API：
- `DataFlowAnalyzer::new()`, `analyze()`, `reset()`
- `DataFlowAnalysis::has_vulnerable_flows()`, `vulnerable_flows()`, `statistics()`
- `DataFlowStatistics` 字段访问

#### 1.4b: sources.rs, sinks.rs, sanitizers.rs（基础覆盖需增强）

需要补充：
- 真实代码中的 source/sink/sanitizer 检测
- Java request.getParameter, Statement.execute
- Python input(), os.system
- JavaScript document.write, eval

#### 1.4c: flows.rs, call_graph.rs, interprocedural.rs, constant_analysis.rs, symbol_table.rs, symbolic_propagation.rs, enhanced_taint.rs, advanced_taint.rs

各模块至少需要测试公共 API 的构造、基本行为、边界情况。

---

### Task 1.5: astgrep-rules 未覆盖 API 测试

#### 1.5a: parser/parsing.rs（2,374 行，覆盖不足）

需要测试：
- 合法 YAML 规则解析
- 畸形 YAML 错误报告
- pattern/pattern-not/pattern-either 组合
- metavariable constraints 解析
- dataflow 块解析
- 循环依赖检测
- ≥30 个测试

#### 1.5b: marketplace.rs（有部分测试）

需要补充：
- 规则下载和缓存
- 版本兼容性检查
- 规则元数据验证

#### 1.5c: executor/ 子模块

需要测试：
- 规则执行顺序
- 依赖解析
- 并行执行

---

### Task 1.6: astgrep-ast 未覆盖 API 测试

#### 1.6a: nodes.rs（需确认/增强覆盖）

需要测试：
- `NodeType` 各变体
- `LiteralValue` 各变体
- `BinaryOperator` / `UnaryOperator` 各变体
- `UniversalNode` 构造、child 操作、text、location

#### 1.6b: visitor.rs（需确认/增强覆盖）

需要测试：
- `AstVisitor` trait 的各方法
- `DispatchingVisitor` 分发行为
- `NodeCollector` / `NodeCounter` / `LocationFinder`

---

### Task 1.7: 集成测试新增

**Files:**
- Create: `tests/lib/taint_realworld_tests.rs`
- Create: `tests/lib/sql_parser_integration_tests.rs`
- Create: `tests/lib/rule_parser_integration_tests.rs`
- Create: `tests/lib/concurrency_tests.rs`
- Create: `tests/lib/regression_tests.rs`

每个集成测试文件覆盖跨模块协作场景：

**taint_realworld_tests.rs:**
- Java SQL injection 完整链路
- XSS 路径
- Sanitizer 正确中断
- Inter-procedural 流
- ≥15 个端到端场景

**sql_parser_integration_tests.rs:**
- 从文件解析 SQL → 构建 AST → 匹配模式 → 产生 Finding
- 端到端 SQL 注入检测

**rule_parser_integration_tests.rs:**
- 从 YAML → 加载规则 → 验证 → 执行 → 产生结果
- 畸形规则错误恢复

**concurrency_tests.rs:**
- 多线程并行分析同一文件
- 多线程并行分析不同文件（共享 RuleEngine）
- Arc<Mutex<>> 竞争条件

**regression_tests.rs:**
- 初始为空
- 约定：每个 bug fix 必须附带 `test_regression_<issue>_xxx` 测试

---

## Phase 2 — 体系化守护手段

### Task 2.1: 添加 proptest（Property-Based Testing）

**Files:**
- Modify: `Cargo.toml`（workspace.dependencies 添加 proptest）
- Create: `crates/astgrep-parser/tests/proptest_parser.rs`
- Create: `crates/astgrep-matcher/tests/proptest_matcher.rs`
- Create: `crates/astgrep-rules/tests/proptest_rules.rs`

每个核心 crate 至少 5 个 property：

**parser proptest:**
- 随机源代码片段 → 解析不 panic
- 解析 + 重新序列化 → 等价
- 空输入 → 不 panic，返回空 AST

**matcher proptest:**
- 随机模式 + 随机 AST → 匹配结果的对称性
- 空模式匹配一切
- 恒等模式匹配自身

**rules proptest:**
- 随机生成 YAML → 无效规则必报错
- 合法规则 → 可成功加载

---

### Task 2.2: 并发安全测试

使用 `std::thread::spawn` + 反复执行（1000 次）检测 race condition。

重点测试：
- `DataFlowAnalyzer` 多线程共享
- `RuleEngine` 并行 execute_rules
- `LanguageParserRegistry` 并发 get_parser
- `OperationCache` 并发读写

---

### Task 2.3: 回归测试框架

**约定（写入 AGENTS.md）：**
1. 每个 bug fix 必须附带测试
2. 命名：`test_regression_<issue_number>_<简述>`
3. 位置：`tests/lib/regression_tests.rs`

---

## Phase 3 — 持续基础设施

### Task 3.1: Pre-commit Hooks (lefthook)

**Files:**
- Create: `lefthook.yml`

```yaml
pre-commit:
  commands:
    fmt:
      run: cargo fmt --all -- --check
    clippy:
      run: cargo clippy --workspace --exclude astgrep-web --exclude astgrep-gui -- -D warnings

pre-push:
  commands:
    test:
      run: cargo test --workspace --exclude astgrep-web --exclude astgrep-gui
    audit:
      run: cargo audit
```

安装：`cargo install lefthook && lefthook install`

---

### Task 3.2: 代码覆盖率

**CI 中添加（不卡合并，只报告）：**

```yaml
- name: Coverage
  run: |
    cargo install cargo-llvm-cov
    cargo llvm-cov --workspace --exclude astgrep-web --exclude astgrep-gui --summary-only 2>&1 | tee coverage.txt
```

目标：
1. 先建立基线数字
2. 后续 PR 要求"不降低覆盖率"
3. 长期目标：核心 crate ≥80%

---

### Task 3.3: 性能回归守护

已有：`crates/astgrep-core/benches/performance.rs`（Criterion）

增强：
- CI 中运行 `cargo bench`
- 结果持久化（Criterion 自带 html_reports）
- 设定退化阈值：任何核心操作 >10% 变慢则 CI 失败
- 关键指标：AST 构建、模式匹配、规则执行、端到端分析

---

### Task 3.4: 集成测试真实性提升

当前：8-10 行 toy 文件

目标：
1. 收集 10-20 个真实开源项目片段（各语言）
2. 放入 `tests/fixtures/<lang>/` 目录
3. 集成测试用这些文件验证端到端结果
4. 建立 "expected findings" 基线文件

---

## 总览

```
Phase 0（~1-2 天）          Phase 1（~1 周）            Phase 2（~1-2 周）          Phase 3（持续）
───────────────────         ───────────────────         ───────────────────         ───────────────────
Task 0.1: test-utils        Task 1.1: astgrep-core      Task 2.1: proptest          Task 3.1: lefthook
Task 0.2: CI clippy         Task 1.2: astgrep-parser    Task 2.2: 并发安全          Task 3.2: 覆盖率
Task 0.3: 112 clippy fix    Task 1.3: astgrep-matcher   Task 2.3: 回归框架          Task 3.3: 性能守护
Task 0.4: rust-toolchain    Task 1.4: astgrep-dataflow                              Task 3.4: 真实测试数据
Task 0.5: deny/audit        Task 1.5: astgrep-rules
Task 0.6: 最终验证          Task 1.6: astgrep-ast
                           Task 1.7: 集成测试

⬇ 必须先完成                ⬇ 每个公共 API ≥1 测试      ⬇ 体系化                   ⬇ 常态化
```

## 依赖关系

```
Task 0.1 → Task 0.6（test-utils 必须先编译通过）
Task 0.3 → Task 0.2（clippy 警告修完才能移除 || true）
Task 0.6 → Task 1.x（Phase 0 全部完成后才能开始 Phase 1）
Task 1.x 可并行（每个 crate 独立）
Task 1.7 可与 Task 1.1-1.6 并行
Task 2.x 依赖 Task 1.x（需要基本测试存在后才能加 property 测试）
Task 3.x 可与 Task 2.x 并行
```

## 执行优先级

| 优先级 | Task | 原因 |
|--------|------|------|
| P0-立即 | 0.1 test-utils | 编译都过不了 |
| P0-立即 | 0.3.2 never_loop | clippy error 级别 |
| P1-当天 | 0.3 clippy fix | CI 恢复的前提 |
| P1-当天 | 0.2 CI clippy | CI 恢复 |
| P1-当天 | 0.4 toolchain | 防止工具链漂移 |
| P1-当天 | 0.5 deny/audit | 依赖安全 |
| P2-本周 | 1.2 SQL parser | 零测试 + 非标准语法 |
| P2-本周 | 1.5 Rule parser | 输入边界无守护 |
| P2-本周 | 1.4 Dataflow | 安全关键 |
| P2-本周 | 1.1 Core patterns | 基础类型无测试 |
| P3-下周 | 1.3 Matcher 补全 | 有部分覆盖 |
| P3-下周 | 1.6 AST 补全 | 有部分覆盖 |
| P3-下周 | 1.7 集成测试 | 跨模块验证 |
| P4-后续 | 2.x 体系化 | 基本覆盖完成后 |
| P4-后续 | 3.x 基础设施 | 长期维护 |
