# astgrep SQL 规则分析性能提升规划报告

**日期**: 2026-07-05
**版本**: v1.0
**基于分析**: `astgrep analyze --dialect gaussdb --rules ...` 性能分析与火焰图推断

---

## 1. 当前性能现状

| 场景 | 文件 | 耗时 | 备注 |
|------|------|------|------|
| 小目录扫描 | 55 个 SQL 文件 | **87.7 秒** | debug build |
| 大文件扫描 | 1 个 4 万行 package 文件 | **~30 分钟** | debug build, 含 CREATE PACKAGE + CREATE PACKAGE BODY |

目标：小目录 < 5 秒，大文件 < 30 秒（debug build）。

---

## 2. 瓶颈排名与影响估算

### 🔴 2.1 致命：`pattern_needs_ast_matching` 对 SQL 模式过于激进

**文件**: `crates/astgrep-rules/src/engine/traversal/pattern.rs:293`

```rust
(pattern_str.contains(';') && pattern_str.contains('\n'))
```

此检查为 Java/C/JS 设计（`;\n` 表示跨语句模式），但对 SQL 而言 `;\n` 只是正常格式化。所有含换行的 SQL 模式（如 `select_lock-1.yaml`）均被错误路由到重量级 AST 匹配路径。

**严重后果**：每个模式都触发：
- 创建新 `AdvancedRuleExecutor`（包含 `DataFlowAnalyzer::new()` + `AdvancedSemgrepMatcher::new()`）
- 对整个文件运行 `ConstantPropagator::analyze_ast()` — O(文件节点数)
- 执行完整的符号 + 污点 + 条件分析

**影响估算**: 大文件耗时的主要贡献者（>70%）；小目录约 20-40%。

### 🔴 2.2 致命：RuleEngine 按文件重复创建

**文件**: `crates/astgrep-cli/src/commands/analyze_enhanced/mod.rs:252-254`

```rust
let mut engine = RuleEngine::new();                                    // 每个文件
let rules_count = load_rules_into_engine_from_paths(&config.rule_files, &mut engine)?;
```

对 55 个文件，YAML 规则被从磁盘读取、解析、验证 **55 次**。

**影响估算**: 小目录约 30-50%（~26-44 秒）。

### 🔴 2.3 致命：三角解析（GaussDB 文件被解析 3 次）

| 次数 | 位置 | 解析器 | 目的 |
|------|------|--------|------|
| 1 | `mod.rs:271` → `gaussdb.rs:87` | ogsql-parser | 构建 UniversalNode 供规则匹配 |
| 2 | `mod.rs:303-305` | tree-sitter-sequel | 常量传播（对 GaussDB 语法无效，浪费） |
| 3 | `mod.rs:187` → `validator.rs:21` | ogsql-parser（再次） | MERGE 语义验证 |

另有两处潜在额外解析：
| 4 | `gaussdb.rs:88-98` | ogsql | 首次解析失败且含 `:=` 时的重试 |
| 5 | `gaussdb.rs:105-109` | ogsql | 重新解析 `$$ body $$` 引用块 |

**影响估算**: 大文件约 15-25%；小目录约 10-20%。

### 🟡 2.4 严重：无文件级并行

**文件**: `mod.rs:71`

```rust
for file_path in target_files { /* 串行 */ }
```

`rayon` 已是 workspace 依赖，`ThreadPool` 实现也存在，但热路径未使用。

**影响估算**: 小目录可通过并行化获得 N 核加速（约 4-8x）。

### 🟡 2.5 严重：`propagate_location` O(N²) 子树遍历

**文件**: `crates/astgrep-parser/src/adapter/ogsql/mod.rs:225-232`

每个语句调用，无条件递归整个子树，即使子节点已有位置信息。

### 🟡 2.6 严重：`append_attr` O(N²) 字符串构建

**文件**: `ogsql/dml.rs:440-448`, `ogsql/ddl.rs:492-502`

每次追加列/约束属性时，克隆并重新格式化**整个已累积的属性字符串**。含 200 列的 CREATE TABLE 会产生 200 次递增字符串拷贝。

### 🟢 2.7 一般：其他开销

- `LanguageParserRegistry::new()` 按文件创建（`mod.rs:260`）
- `with_text(source.to_string())` 全量源文件克隆（`gaussdb.rs:112`）
- `byte_index_to_line_col` O(N) 按匹配位置查找行号（`matching.rs:287-305`）
- `fancy_regex` 正则编译按模式×文件重复（`matching.rs:140`）

---

## 3. 修复方案

### 阶段一：高 ROI 快速修复（预计 2-4 小时，预期提升 >50%）

#### 3.1 修复 `pattern_needs_ast_matching` 的 SQL 误判

**文件**: `crates/astgrep-rules/src/engine/traversal/pattern.rs:271-300`

**当前**:
```rust
fn pattern_needs_ast_matching(pattern_str: &str) -> bool {
    // ...
    (pattern_str.contains(';') && pattern_str.contains('\n'))
    // ...
}
```

**修改为**:
```rust
fn pattern_needs_ast_matching(pattern_str: &str, language: astgrep_core::Language) -> bool {
    // SQL: ; 和 \n 是正常格式，不是多语句信号
    if language == astgrep_core::Language::Sql {
        return pattern_str.contains('{')
            || pattern_str.contains("class ")
            || pattern_str.contains("function ")
            || pattern_str.contains("def ")
            || pattern_str.contains("import ")
            || pattern_str.contains("from ")
            || pattern_str.contains('@')
            || (pattern_str.contains('(') && pattern_str.contains(')') && pattern_str.contains('$'));
    }
    // 非 SQL 语言保持原逻辑
    // ...
}
```

**预期效果**: 大文件耗时减少 >70%，小目录减少 20-40%。这是最大单次收益。

**风险**: 部分复杂 SQL 模式可能需要 AST 匹配。通过测试回归验证。若出现误匹配，为特定 SQL 模式添加白名单。

#### 3.2 将 RuleEngine 提升至文件循环外部

**文件**: `crates/astgrep-cli/src/commands/analyze_enhanced/mod.rs`

**当前**（`analyze_with_rule_engine` 内部，每文件调用）:
```rust
let mut engine = RuleEngine::new();
let rules_count = load_rules_into_engine_from_paths(&config.rule_files, &mut engine)?;
```

**修改为**（在 `run_enhanced` 中创建一次）:
```rust
pub async fn run_enhanced(...) -> Result<()> {
    // ... 收集文件 ...

    // 规则引擎只创建一次
    let mut engine = RuleEngine::new();
    let rules_count = load_rules_into_engine_from_paths(&config.rule_files, &mut engine)?;

    for file_path in target_files {
        analyze_file_simple(&file_path, &config, &mut engine, ...)?;
    }
}
```

需修改 `analyze_file_simple` 和 `analyze_with_rule_engine` 接受 `&mut RuleEngine` 参数。

**预期效果**: 小目录减少 30-50%（~26-44 秒 → ~13-18 秒）。

#### 3.3 跳过非标准 SQL 方言的 tree-sitter 常量传播

**文件**: `mod.rs:296-315`

**修改为**: 增加方言门控。
```rust
if config.enable_constant_propagation
    && config.sql_dialect.map_or(true, |d| d == astgrep_core::SqlDialect::Standard)
{
    // ... 常量传播逻辑 ...
}
```

**预期效果**: 消除 GaussDB 文件的一次无效 tree-sitter 解析。

### 阶段二：中等投入优化（预计 4-8 小时，进一步 20-40% 提升）

#### 3.4 消除 GaussDB 的重复解析

**文件**: `mod.rs:180-206`、`validator.rs`

**方案 A（推荐）**: 将 `validate_gaussdb_sql` 重构为接受预解析的 `stmt_infos`：

```rust
// 在 analyze_with_rule_engine 中，dialect parse 后保存 stmt_infos
let dialect_parser = astgrep_parser::dialect::dispatch(dialect);
let parse_result = dialect_parser.parse_with_meta(source_code, ...)?;
let ast = parse_result.ast;
let stmt_infos = parse_result.meta; // 传递给 validator 重用
```

**方案 B（简化）**: 在规则引擎未加载任何 MERGE 相关规则时，跳过 validator。

```rust
if engine.has_rules_for_category("merge") {
    validate_gaussdb_sql(&source_code);
}
```

**预期效果**: 消除第 3 次解析。

#### 3.5 文件级并行化

**文件**: `mod.rs:71`

```rust
use rayon::prelude::*;

let results: Vec<_> = target_files
    .par_iter()
    .map(|file_path| {
        analyze_file_simple(file_path, &config, &mut engine_per_thread, ...)
    })
    .collect();
```

注意：需要每个线程独立持有 `RuleEngine`（或使 `RuleExecutionEngine` 实现 `Sync`）。

**预期效果**: N 核加速（小目录 4-8x）。

#### 3.6 修复 `propagate_location` 早退

**文件**: `ogsql/mod.rs:225-232`

```rust
fn propagate_location(node: &mut UniversalNode, loc: ...) {
    for child in node.children.iter_mut() {
        if child.location.is_none() {
            child.location = Some(loc);
            propagate_location(child, loc);  // 只在需要时递归
        }
    }
}
```

**预期效果**: 消除大 AST 上的 O(N²) 行为。

### 阶段三：深层优化（预计 8-16 小时，10-20% 额外提升）

#### 3.7 修复 `append_attr` 的 O(N²) 字符串构建

**文件**: `ogsql/dml.rs:440-448`, `ogsql/ddl.rs:492-502`

```rust
// 替换为
fn append_attr(node: &mut UniversalNode, key: &str, value: &str) {
    node.attributes
        .entry(key.into())
        .and_modify(|v| {
            let mut s = v.clone(); // 一次性分配
            s.push(',');
            s.push_str(value);
            *v = s;
        })
        .or_insert_with(|| value.into());
}
```

#### 3.8 正则预编译缓存

**文件**: `matching.rs:140`

将 `fancy_regex::Regex::new()` 结果缓存在 `RuleExecutionEngine` 字段中，避免按模式×文件重复编译。

#### 3.9 预计算字节偏移→行号映射

**文件**: `matching.rs:287-305`

为超过阈值的文件（如 >10000 字节）预计算 `Vec<usize>` 行偏移表，使 `byte_index_to_line_col` 从 O(N) 降为 O(log N)。

#### 3.10 `$...` 引用块解析优化

**文件**: `gaussdb.rs:51-68, 82-114`

- 若主解析成功，跳过 `extract_and_parse_dollar_body`（已捕获引用块内容时）
- 对大文件跳过 `:=` 重试（大文件不太可能是用户手写的简短 PL 块）

---

## 4. 实施路线图

### Sprint 1: 快速止血（目标：大文件 < 3 分钟，小目录 < 15 秒）

| 任务 | 文件 | 预计耗时 | 优先级 |
|------|------|----------|--------|
| 3.1 `pattern_needs_ast_matching` SQL 修复 | `pattern.rs` | 1h | P0 |
| 3.2 RuleEngine 提升至循环外 | `mod.rs` | 1.5h | P0 |
| 3.3 跳过 GaussDB tree-sitter 常量传播 | `mod.rs` | 0.5h | P0 |

**验证方式**:
```bash
# 回归测试
cargo test -p astgrep-rules
cargo test -p astgrep-cli

# 性能验证
time target/debug/astgrep analyze --dialect gaussdb \
  --rules tests/categories/sql_dialects/gaussdb/rules/select_lock-1.yaml \
  --format text /path/to/demo-project/sql/
```

**成功标准**: 小目录 < 15s，大文件 < 3min（debug build）。

### Sprint 2: 架构优化（目标：小目录 < 5 秒）

| 任务 | 文件 | 预计耗时 | 优先级 |
|------|------|----------|--------|
| 3.4 消除 GaussDB 重复解析 | `mod.rs`, `validator.rs` | 2h | P1 |
| 3.5 文件级并行 | `mod.rs` | 3h | P1 |
| 3.6 `propagate_location` 早退 | `ogsql/mod.rs` | 0.5h | P1 |

### Sprint 3: 深度优化（目标：大文件 < 30 秒）

| 任务 | 文件 | 预计耗时 | 优先级 |
|------|------|----------|--------|
| 3.7 `append_attr` O(N²) 修复 | `ogsql/dml.rs`, `ddl.rs` | 1h | P2 |
| 3.8 正则预编译缓存 | `matching.rs`, `pattern.rs` | 2h | P2 |
| 3.9 行号映射预计算 | `matching.rs` | 1.5h | P2 |
| 3.10 `$$` 引用块跳过优化 | `gaussdb.rs` | 1h | P2 |

---

## 5. 风险与回退

### 5.1 `pattern_needs_ast_matching` SQL 豁免风险

**风险**: 部分复杂 SQL 模式可能确实需要 AST 级匹配以获得正确语义。
**缓解**: 
- 运行完整 `cargo test` 回归测试
- 若有 SQL 模式出现误匹配，在 `pattern_needs_ast_matching` 中添加白名单而非完全豁免
- 可先记录命中 AST 路径的次数（通过 `tracing::debug!`），验证后再下线

### 5.2 并行化线程安全风险

**风险**: `RuleEngine` / `RuleExecutionEngine` 可能不是线程安全的。
**缓解**: 
- 初期使用 `Arc<Mutex<RuleEngine>>` 或 per-thread clone
- 若 `AdvancedRuleExecutor` 不是 `Send`，需重构内部可变状态

### 5.3 回退策略

所有修改均可通过 `git revert` 独立回退。每个 Sprint 提交一次，不混入多功能。

---

## 6. 附：火焰图验证步骤

在实施修复前，建议用 `cargo flamegraph` 确认瓶颈假设：

```bash
# 安装
cargo install flamegraph

# 大文件测试
cargo flamegraph --bin astgrep -- analyze \
  --dialect gaussdb \
  --rules tests/categories/sql_dialects/gaussdb/rules/select_lock-1.yaml \
  --format text \
  /path/to/large-package.sql

# 关键检查：
# - ogsql_parser::parser::Parser::parse 占比 → 解析本身是否为瓶颈
# - execute_advanced_pattern / execute_comprehensive_analysis 占比 → 确认 B1
# - load_rules_into_engine_from_paths 占用 → 确认 B2
```

---

## 7. 性能目标总结

| 场景 | 当前耗时 (debug) | Sprint 1 目标 | Sprint 2 目标 | Sprint 3 目标 |
|------|-------------------|---------------|---------------|---------------|
| 55 个 SQL 文件 | 87.7 秒 | < 15 秒 | < 5 秒 | < 3 秒 |
| 4 万行 package 文件 | ~30 分钟 | < 3 分钟 | < 1 分钟 | < 30 秒 |

**注**: Release build (`cargo build --release`) 在此基础上预计还有 3-5x 提速。
