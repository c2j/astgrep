# Phase 2: Engine 层修复方案

**目标**: 53.0% → ~70% | **工期**: 3-4 周 | **风险**: 中

> 基于 Phase 1 发现：pattern 层修复已达天花板，需进入 matching engine 和 taint engine 层。

---

## 任务总览

| # | 任务 | 影响测试 | 预计提升 | 难度 | 工期 |
|---|------|---------|---------|------|------|
| 2.1 | TaintEnv 核心状态机 | ~25 | +2.8% | 中 | 4天 |
| 2.2 | TaintEnv 集成到 taint.rs | ~10 | +1.1% | 中 | 2天 |
| 2.3 | 跨语言省略号序列匹配修复 | ~12 | +1.3% | 高 | 4天 |
| 2.4 | 元变量跨行状态传递 | ~8 | +0.9% | 中 | 2天 |
| 2.5 | Rules engine: sym_prop / metavar_pattern 修复 | ~15 | +1.7% | 中 | 4天 |
| 2.6 | 验证与回归 | — | — | 低 | 1天 |

**合计影响**: ~50-70 测试 → 通过率从 53.0% 提升至 ~65-70%

---

## 任务 2.1: TaintEnv 核心状态机 (来自 taint-dataflow-engine.md Phase 1)

### 背景

当前 taint 引擎使用启发式 source×sink 配对，缺少：
- 变量 taint 状态跟踪
- 赋值传播 (`a = tainted; b = a; sink(b)` 失败)
- 控制流感知

### 实施 (按已有 spec: docs/plans/2026-05-23-taint-dataflow-engine.md Task 1-3)

**Task 1: 定义 TaintEnv 类型** (新建文件)
- 文件: `crates/astgrep-rules/src/executor/core/taint_env.rs`
- 实现: `TaintState` (source_lines, tainted, sanitized_by, origin_patterns)
- 实现: `TaintEnv` (per-scope HashMap, push_scope/pop_scope)
- 操作: `taint()`, `is_tainted()`, `propagate()`, `sanitize()`, `untaint()`

**Task 2: 构建语句遍历器**
- 添加 `process_line()` 方法：逐行解析赋值、方法调用、sink 匹配
- 添加 `extract_function_name()` 辅助函数

**Task 3: 集成到 detect_taint_flows**
- 在 `taint.rs` 中添加 `detect_taint_flows_with_env()` 
- 采用 env-based 为主、heuristic 为 fallback 的双轨策略
- 在 `execute_taint_analysis` 中布线

### 验证
```bash
cargo build --release
python3 -c "
import json
with open('newtest/guardian_report.json') as f:
    data = json.load(f)
taint = [r for r in data['test_results'] if 'taint' in r.get('category','')]
passed = sum(1 for t in taint if t.get('passed'))
print(f'Taint: {passed}/{len(taint)}')
"
```

### 影响
- `taint_maturity/*`: 0% → 50%+ (4 测试)
- `tainting_rules/*`: 30% → 60%+ (8+ 测试)
- `rules` 中的 taint 子集: ~10 测试改进

---

## 任务 2.2: TaintEnv 精细化 (Control Flow + Sanitizers)

### 实施

**Task 4: Control flow 分支合并** (来自 taint spec Phase 2)
- `fork()` / `merge()` 方法：分支 taint 并集语义
- 在 `detect_taint_flows_with_env` 中检测 if/else、try/catch

**Task 5: Sanitizer 模式匹配**
- 检测 `x = sanitize(y)` 模式，标记 x 为 sanitized
- 防止 sanitized 变量产生 false positive

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --category "tainting_rules" --verbose
```

---

## 任务 2.3: 跨语言省略号序列匹配修复

### 问题

`dots_stmts` (POLYGLOT: `$V = get();\n...\neval($V);`) 在所有语言中失败。Phase 1 已确认 Wildcard handler 存在于文本回退路径 (line 2222)，问题在 AST 级 `match_sequence_ast()`。

### 根因

`match_sequence_ast()` (line ~1102) 对省略号的处理依赖 `EllipsisMetavariable` 绑定机制，但存在以下问题：
1. 各语言的 statement 节点类型不一致（Java: `expression_statement`, Python: `expression_statement`, Bash: 顶层命令）
2. `match_sequence_ast()` 中的 child filtering 过于激进，可能过滤掉能匹配省略号的关键节点

### 修复方向

在 `match_sequence_ast()` 中：

```rust
ParsedPattern::Wildcard => {
    // Skip consecutive Wildcards
    while pattern_idx + 1 < patterns.len() 
        && matches!(patterns[pattern_idx + 1], ParsedPattern::Wildcard) 
    {
        pattern_idx += 1;
    }

    if pattern_idx + 1 >= patterns.len() {
        return Ok(true); // ... at end matches everything
    }

    // Try matching remaining patterns from each child position
    for skip in 0..=(children.len().saturating_sub(child_offset)) {
        let snapshot = self.metavar_manager.snapshot();
        if self.match_sequence_ast(
            &patterns[pattern_idx + 1..],
            children,
            child_offset + skip,
            depth + 1,
        )? {
            return Ok(true);
        }
        self.metavar_manager.restore(snapshot);
    }
    return Ok(false);
}
```

### 验证
```bash
# 逐个语言
for lang in java js ts python bash; do
    python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | grep "patterns/$lang" | grep dots_stmts
done
```

### 影响 (~12 测试)
- `patterns/java/dots_stmts`
- `patterns/js/dots_stmts`
- `patterns/ts/dots_stmts`
- `patterns/python/dots_stmts`
- `patterns/bash/dots_stmts`
- `patterns/bash/stmt-ellipsis`
- `patterns/bash/stmt-named-ellipsis`

---

## 任务 2.4: 元变量跨行状态传递

### 问题

`metavar_equality_var` 跨语言失败。Phase 1 确认 `bind()` 返回值在调用点正确传播，但多行模式中元变量绑定在序列匹配器子调用间丢失。

### 根因

`match_sequence_ast()` 中，第一个子模式匹配后绑定 `$FILE=out`，但后续子模式 `touch "${$FILE}"` 使用全新的匹配器上下文，无法访问先前的绑定。

### 修复

在 `match_sequence_ast()` 中确保 snapshot/restore 正确工作：
1. 每个省略号 backtracking 分支恢复正确的 snapshot
2. 确保子模式匹配不重置 MetavarManager（仅在完整匹配成功时 commit）

```rust
// 在 match_sequence_ast 中
// 每个子模式匹配成功后，绑定应持久化到下一个子模式
// snapshot 捕获的是"当前子模式匹配前"的状态
// 如果后续子模式失败，restore 回到该状态
// 如果所有子模式成功，保留所有绑定
```

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | grep metavar_equality_var
```

### 影响 (~8 测试)
- `patterns/bash/metavar_equality_var`
- `patterns/java/metavar_equality_var`
- `patterns/js/metavar_equality_var`
- `patterns/ts/metavar_equality_var`
- `patterns/js/metavar_equality_vardef_vs_use`
- `patterns/python/metavar_equality_var` (如果失败)
- 相关 metavar_func_def 测试

---

## 任务 2.5: Rules Engine 定向修复

### 问题

`rules` 类别 119/265 (44.9%)，133 失败。主要失败子类：

| 子类 | 失败数 | 优先级 |
|------|--------|--------|
| `sym_prop_*` (符号传播) | ~20 | 高 |
| `metavar_pattern_*` (元变量模式) | ~20 | 高 |
| `taint_*` (与 taint engine 重叠) | ~35 | 中 (任务 2.1/2.2 覆盖) |
| 其他 (cp_*, metavar_regex_*, etc.) | ~58 | 低 |

### sym_prop 系列修复

符号传播 (`SymbolicPropagator`) 负责跟踪别名关系。常见失败模式：
- `sym_prop_chain`: 链式赋值 `a = b; c = a; sink(c)` — 传播链断裂
- `sym_prop_lambda`: lambda 闭包中的变量传播
- `sym_prop_record`: record/object 字段传播

修复方向：
1. 检查 `crates/astgrep-dataflow/src/symbolic_propagation.rs` (908 行) 的传播深度和递归逻辑
2. 对 lambda 闭包添加 scope-aware 跟踪
3. 对 record 添加字段级别跟踪

### metavar_pattern 系列修复

元变量模式 (metavariable-pattern) 是嵌套匹配：在已绑定 `$VAR` 上再应用子 pattern。

修复方向：
1. 检查 `executor/core/conditions.rs` 中的 `MetavariablePattern` 条件评估
2. 验证子 pattern 能访问外层 metavar 的绑定值
3. 检查 `pattern-either` / `pattern-generic` 等嵌套模式的组合语义

### 实施策略

不需要做全部 133 个测试 — 聚焦高影响子类：
1. 修复 sym_prop_chain (影响 ~8 测试)
2. 修复 3-5 个最常见的 metavar_pattern 失败 (影响 ~12 测试)

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --category "rules" --verbose 2>&1 | grep -E "(sym_prop|metavar_pattern)" | grep -c FAIL
# 预期: FAIL 数字减少
```

---

## 任务 2.6: 验证与回归

### 步骤

```bash
# 1. 全量 guardian
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | tee phase2_results.txt

# 2. taint 专项统计
python3 -c "
import json
with open('newtest/guardian_report.json') as f:
    data = json.load(f)
taint = [r for r in data['test_results'] if 'taint' in r.get('category','')]
passed = sum(1 for t in taint if t.get('passed'))
total = len(taint)
print(f'Taint: {passed}/{total} = {passed/total*100:.1f}%')
"

# 3. 所有 matcher 单元测试
cargo test -p astgrep-matcher -- --nocapture 2>&1 | tail -5

# 4. 检查无新增 crash
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | grep "panic" && echo "CRASH DETECTED" || echo "No crashes"
```

---

## 执行顺序 (依赖图)

```
Week 1: 
  2.1 Task 1-2 (TaintEnv 类型 + 语句遍历器)
  → 2.1 Task 3 (集成)
  → 2.4 (元变量跨行 — 可与 2.1 并行)

Week 2:
  2.2 (TaintEnv 精细化)
  → 2.3 (省略号序列匹配 — 最复杂的 engine 改动)

Week 3:
  2.5 Part 1 (sym_prop 修复)
  → 2.5 Part 2 (metavar_pattern 修复)
  → 2.6 (验证)
```

### 依赖关系
- 2.1 → 2.2: 2.2 依赖 2.1 的 TaintEnv 基础
- 2.3 独立：可与其他任务并行
- 2.4 可能与 2.3 共享代码改动（都在 match_sequence_ast）
- 2.5 部分独立 (sym_prop)，部分依赖 2.1/2.2 (taint 相关)

---

## 预期成果

| 指标 | 修复前 | 修复后 (预期) |
|------|--------|--------------|
| 通过率 | 53.0% | ~65-70% |
| Taint 专项 | ~30% | ~70% |
| dots_stmts | 0/5 语言 | 3-4/5 语言 |
| metavar_equality | 0/5 语言 | 3-4/5 语言 |
| sym_prop | ~15 失败 | ~5 失败 |

## 风险

| 风险 | 缓解 |
|------|------|
| TaintEnv 引入 regression | 双轨策略：新方法为主，旧方法为 fallback |
| match_sequence_ast 改动影响面广 | 仅修改省略号处理分支，不改其他逻辑 |
| 元变量 snapshot/restore 改动可能引入新 bug | 每次改动后跑 guardian metavar 专项 |
| 时间超出 | 任务按优先级排列，低优先级任务可延期到 Phase 3 |
