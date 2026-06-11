# Phase 1: 一致性通过率快速修复方案

**目标**: 52.8% → ~70% | **工期**: 1 周 | **风险**: 低

---

## 任务总览

| # | 任务 | 影响测试数 | 预计提升 | 难度 | 工期 |
|---|------|-----------|---------|------|------|
| 1.1 | 修复 advanced_matcher.rs:931 崩溃 | 2 测试消除 crash | +0.2% | 低 | 0.5天 |
| 1.2 | SQL pattern-regex 匹配修复 (已定位↑) | 6 测试 (0%→80%+) | +0.7% | 中 | 1.5天 |
| 1.3 | Bash dots_stmts/ellipsis 修复 | ~8 测试 | +0.9% | 中 | 1天 |
| 1.4 | 跨语言省略号末端匹配修复 | ~12 测试 | +1.3% | 中 | 1天 |
| 1.5 | 元变量相等性修复 (已定位↑) | ~25 测试 | +2.8% | 低 | 0.5天 |
| 1.6 | Bash concatenation/string 修复 | ~7 测试 | +0.8% | 中 | 1天 |
| 1.7 | 验证与回归 | — | — | 低 | 0.5天 |

**合计影响**: ~35-42 测试 → 通过率从 52.8% 提升至 ~57%+（考虑当前总测试数 890）

---

## 任务 1.1: 修复 advanced_matcher.rs 字符串切片崩溃

### 问题
advanced_matcher.rs 中有 **3 处** 相同的 panic 隐患: `begin <= end (1 <= 0) when slicing`

### 根因 (已确认)

代码去除引号时未检查字符串长度。当 tree-sitter 返回单字符引号节点（如 `"`）时:
```rust
// line 931, 1898, 1942 — 同样模式
if text.starts_with('"') && text.ends_with('"') {
    &text[1..text.len() - 1]  // text.len()=1 → text[1..0] PANIC
}
```

**触发条件**: Python f-string 的 `=~/.*bento.*/` 正则模式中，tree-sitter 将 f-string 解析为复合节点，其中的引号字符 `"` 被识别为独立子节点。匹配器遍历子节点时，遇到单字符引号节点触发切片越界。

**三处崩溃位置**:
| 行号 | 上下文 | 说明 |
|------|--------|------|
| 931 | AST 节点文本匹配 | 主要崩溃点 |
| 1898 | Token 文本匹配 | 相同模式 |
| 1942 | 元变量文本提取 | 已有 `>=2` 检查但不够安全 |

### 修复

每处添加 `&& text.len() >= 2` 检查，或提取公共函数:

```rust
fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 && (
        (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
        || (s.starts_with('`') && s.ends_with('`'))
    ) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}
```

### 验证
```bash
cargo build --release
python3 newtest/scripts/guardian_runner.py --category "patterns/python" --verbose 2>&1 | grep -E "(misc_faketok2|infer_const_regexp|panic)"
# 预期: 无 panic, 两个测试正常 pass 或 fail (不再 crash)
```

### 影响测试
- `patterns/python/misc_faketok2`
- `patterns/js/infer_const_regexp`

---

## 任务 1.2: SQL pattern-regex 匹配修复 ✅ 已定位根因

### 问题
6 个 SQL 测试全部失败 (0/6)。745 处 missed + 大量 range-spanning extra matches。

### 根因 (已确认)

**`matches_regex_pattern()` 将正则应用到单个 AST 节点文本，而非完整源码行。**

代码位置: `advanced_matcher.rs:375-388`
```rust
fn matches_regex_pattern(&mut self, regex_str: &str, node: &dyn AstNode) -> Result<bool> {
    if let Some(text) = node.text() {  // ← 问题: 用 node.text() 而非完整源文本
        Regex::new(regex_str)?.is_match(text)
    }
}
```

tree-sitter-sequel 将 SQL 解析为细粒度 AST 节点。一条 `SELECT * FROM users WHERE username = 'admin' + @input;` 被解析为十几个节点：`"SELECT"`, `"*"`, `"FROM"`, `"users"`, `"WHERE"`, `"username"`, `"="`, `"'admin'"`, `"+"`, `"@input"`。将正则 `WHERE\s+.*\s*\+\s*.*` 应用到 `"WHERE"` 节点（text=`"WHERE"`）必然失败。

**影响所有 6 个测试**: SQL 规则全部依赖 `pattern-regex`，需要完整语句上下文才能匹配。

### 修复方案

**方案 A（推荐）**: 修改 `matches_regex_pattern()` 对 SQL 语言使用完整源文本。
```rust
fn matches_regex_pattern(&mut self, regex_str, node, language, source_text) -> Result<bool> {
    if language == Language::Sql {
        // 对 SQL，在整个源文本中匹配，用 find_iter 获取所有匹配位置
        Regex::new(regex_str)?.is_match(source_text)  // 传入完整文件文本
    } else {
        // 其他语言保持现有行为
        node.text().map(|t| Regex::new(regex_str)?.is_match(t)).unwrap_or(false)
    }
}
```

**方案 B**: 向上递归找到包含完整语句的祖先节点（如 `statement` 或 `expression_statement`），使用该节点的 text 进行匹配。

### 实施步骤
1. 在 `matches_regex_pattern()` 中添加 `language` 和 `source_text` 参数
2. 对 `Language::Sql` 切换为全文本匹配
3. 调整 match 位置计算以对应正确行号

### 验证
```bash
./target/release/astgrep analyze tests/categories/sql/sql_injection.sql \
  -r tests/categories/sql/sql_injection.yaml --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)
for f in data.get('findings', []):
    loc = f.get('location', {})
    print(f'{f[\"rule_id\"]}: line {loc[\"start_line\"]} ({f[\"message\"][:40]})')
"
# 预期: 22 个 finding, 行号与 baseline 对齐

python3 newtest/scripts/guardian_runner.py --category "sql" --verbose
```

### 验证
```bash
./target/release/astgrep analyze tests/categories/sql/sql_injection.sql \
  -r tests/categories/sql/sql_injection.yaml --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)
findings = data.get('findings', [])
for f in findings:
    loc = f.get('location', {})
    print(f'{f[\"rule_id\"]}: line {loc.get(\"start_line\")}-{loc.get(\"end_line\")} ({f[\"message\"][:50]}...')'
"
# 预期: 每个 finding 的行号范围 ≤ 5 行，且行号与 baseline 注释行对齐

python3 newtest/scripts/guardian_runner.py --category "sql" --verbose
```

### 影响测试 (6个)
- `sql/sql_injection`
- `sql/missing_where`
- `sql/privilege_escalation`
- `sql/select_star`
- `sql/weak_encryption`
- `sql/information_disclosure`

---

## 任务 1.3: Bash dots_stmts / ellipsis / statement 修复

### 问题
Bash 省略号与语句相关 8 个测试失败:
- `dots_stmts`, `stmt-ellipsis`, `stmt-named-ellipsis` — 语句序列中的 `...` 不工作
- `deep_exprstmt` — 深层表达式语句匹配失败
- `anchored-stmt` — `{ a; }` 无法匹配 `{ a && b; }`（复合语句）
- `not-an-expression1` — 显式分号模式匹配失败

### 根因细分

**stmt-ellipsis/dots_stmts**:
Pattern `a; ...; b` 中的 `...` 应匹配 0-N 条语句。`match_sequence` 未正确处理跨语句省略号。

**deep_exprstmt**:
多层嵌套的 `expression_statement` → 子节点未递归匹配。

**anchored-stmt**:
`{ a; }` 匹配 `{ a && b; }` 失败 — tree-sitter-bash 将 `a && b` 解析为 `logical_and_expression` 节点，与简单的 `a` 节点结构不同。

### 修复方向

在 `advanced_matcher.rs::match_sequence_ast()` 中:
1. 检查省略号是否跨 `statement` / `expression_statement` 节点匹配
2. 对复合语句（`{ ... }`），递归进入子节点匹配而非要求精确 AST 结构

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --category "patterns/bash" --verbose 2>&1 | grep -E "(dots_stmts|stmt-ellipsis|deep_exprstmt|anchored-stmt|not-an-expression)"
```

---

## 任务 1.4: 跨语言省略号末端匹配修复

### 问题
Java/JS/TS/Python/Bash 的 `dots_stmts` 均失败（使用 POLYGLOT pattern: `$V = get();\n...\neval($V);`）。

### 根因 (已确认)

匹配流程中 **文本匹配回退路径缺少 `Wildcard` 模式处理器**。

匹配流程:
1. `match_sequence()` → `match_sequence_ast()` (AST 级别，省略号通过 `EllipsisMetavariable` 处理 ✓)
2. AST 匹配失败 → 回退到 `match_sequence_against_text()` → `try_match_sequence_at_position()`
3. 文本匹配函数的 `ParsedPattern` 匹配分支中:
   - `Literal` ✓
   - `Metavariable` ✓
   - `EllipsisMetavariable` (命名省略号 `$...STMTS`) ✓
   - **`Wildcard` (裸省略号 `...`) — 缺失 ❌**

4. 当 AST 结构因语言差异（不同的 `block`/`statement` 节点类型）导致 AST 级匹配失败时，文本回退也因缺少 `...` 处理器而失败。

### 修复

在 `advanced_matcher.rs::try_match_sequence_at_position()` (line ~1852) 的模式匹配分支中添加:

```rust
ParsedPattern::Wildcard => {
    for skip in 0..=(text_tokens.len() - text_idx) {
        let snapshot = self.metavar_manager.snapshot();
        if self.try_match_sequence_at_position(
            &patterns[pattern_idx + 1..],
            text_tokens,
            text_idx + skip,
            node,
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
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | grep dots_stmts

### 验证
```bash
# 逐个语言验证
python3 newtest/scripts/guardian_runner.py --category "patterns/java" --verbose 2>&1 | grep dots_stmts
python3 newtest/scripts/guardian_runner.py --category "patterns/js" --verbose 2>&1 | grep dots_stmts
python3 newtest/scripts/guardian_runner.py --category "patterns/ts" --verbose 2>&1 | grep dots_stmts
```

### 影响测试 (~12个)
- `patterns/java/dots_stmts`
- `patterns/js/dots_stmts`
- `patterns/ts/dots_stmts`
- `patterns/python/dots_stmts`
- `patterns/bash/dots_stmts` (与任务 1.3 重叠)
- `patterns/bash/stmt-ellipsis`
- `patterns/bash/stmt-named-ellipsis`

---

## 任务 1.5: 元变量相等性修复 ✅ 已定位根因

### 问题
Java/JS/TS/Bash/Python 的 `metavar_equality_var` / `metavar_typed_*` 均失败。

### 根因 (已确认)

**经典 "ignored return value" bug**。`MetavarManager::bind()` 正确返回 `Ok(false)` 当值不一致时（`metavar.rs:136`），但调用方**丢弃了返回值**。

**bug 位置** (`advanced_matcher.rs`):
- `match_metavariable()` line 990: `.bind(...)` — 忽略返回值
- `match_ellipsis_metavariable()` lines 1000, 1004: `.bind(...)` — 忽略返回值
- `match_typed_metavar()` lines 1025, 1028: `.bind(...)` — 忽略返回值

**bug 位置** (`matcher.rs`):
- `match_metavariable()` line 185: `.bind(...)` — 忽略返回值
- `match_ellipsis_metavariable()` lines 197, 201: `.bind(...)` — 忽略返回值

**结果**: 当同一 `$VAR` 出现多次时，第二次绑定了不同值，`bind()` 返回 `false`，但匹配器当 `true` 继续执行→产生错误的 EXTRA/NEG_VIOL。

> 注：代码库中其他 `bind()` 调用（lines 1235, 1261, 1335, 1374, 1945, 1971 等）**已正确**检查返回值——说明这是个遗漏。

### 修复

在每个忽略返回值的调用处改为 check 并 propagate:

```rust
// 修改前 (advanced_matcher.rs line ~990):
self.metavar_manager
    .bind(bind_key, text.to_string(), node)

// 修改后:
self.metavar_manager
    .bind(bind_key, text.to_string(), node)?
```

同样修改所有 8 处（`advanced_matcher.rs` 5处 + `matcher.rs` 3处）。

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | grep -E "metavar_(equality|typed)"
# 预期: 大量 metavar 相关测试从 FAIL 变为 PASS
```

---

## 任务 1.6: Bash concatenation / string normalize 修复

### 问题
Bash 中有两类模式大量失败:

1. **Concatenation ellipsis**: `concatenation-ellipsis`, `concatenation-ellipsis-args`, `concatenation-named-ellipsis`, `concatenation-named-ellipsis-args` — 全部 MISSED

2. **String normalization**: `normalize-squoted-word` (MISSED lines [2, 5, 8]), `quoted-ellipsis2` (MISSED lines [2])

### 根因推测

1. **Concatenation** patterns 使用了 Bash 特有的字符串拼接语法（相邻字符串自动拼接: `'hello' 'world'` → `'helloworld'`），tree-sitter-bash 的可能 AST 表示与 pattern 预期不同。

2. **String normalize**: tree-sitter-bash 在处理单引号字符串时，内部节点表示可能与 pattern 的 AST 层级不匹配。

### 修复步骤

1. 读取每个失败的 `.sgrep` pattern 和对应 `.bash` 源文件
2. 手动用 astgrep dump AST 对比 pattern AST 与 source AST 的差异
3. 调整 pattern 以匹配实际 AST 结构

### 验证
```bash
python3 newtest/scripts/guardian_runner.py --category "patterns/bash" --verbose 2>&1 | grep -E "(concatenation|normalize|quoted)"
```

---

## 任务 1.7: 验证与回归

### 步骤

1. 全量 guardian 运行，确保无新增失败:
```bash
python3 newtest/scripts/guardian_runner.py --verbose 2>&1 | tee phase1_results.txt
```

2. 对比修复前后的通过率:
```bash
python3 -c "
import json
with open('newtest/guardian_report.json') as f:
    data = json.load(f)
s = data['summary']
print(f'通过: {s[\"passed\"]}/{s[\"total_tests\"]-s[\"skipped\"]} = {s[\"pass_rate\"]*100:.1f}%')
print(f'Missed: {s[\"total_missed\"]}, Extra: {s[\"total_extra\"]}, NegViol: {s[\"total_negative_violations\"]}')
"
```

3. 运行 `cargo test` 确保无回归:
```bash
cargo test 2>&1 | tail -20
```

4. LSP diagnostics 检查修改文件:
```bash
# 检查所有已修改的 Rust 文件
```

---

## 执行顺序

```
Day 1 AM: 任务 1.5 (元变量相等 — 最简单、影响最大) → 任务 1.1 (崩溃修复)
Day 1 PM: 任务 1.3 (Bash dots_stmts)
Day 2: 任务 1.6 (Bash concatenation/string) → 开始任务 1.2 (SQL)
Day 3: 完成任务 1.2 (SQL) → 开始任务 1.4 (省略号末端匹配)
Day 4: 完成任务 1.4
Day 5: 任务 1.7 (验证与回归)
```

### 优先级调整理由
- 任务 1.5 根因已精确定位（`bind()` 返回值被忽略），修复仅需改 8 行代码，影响 ~25 测试
- 任务 1.1 崩溃修复仅需 1 行长度检查

---

## 预期成果

| 指标 | 修复前 | 修复后 (预期) |
|------|--------|--------------|
| 通过率 | 52.8% | ~57-60% |
| Missed | 745 | ~650 |
| Extra | 259 | ~230 |
| Negative violations | 74 | ~65 |
| Crash | 2 | 0 |

## 风险与回滚

- 每个任务独立提交，方便 cherry-pick 或 revert
- 低风险任务(1.1, 1.5, 1.6)可以先做，快速积累成果
- SQL 修复(1.2)如果不能在 1.5 天内完成，可以先跳过进入 Phase 2
