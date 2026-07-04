# SQL SELECT FOR UPDATE 跨语句 AST 匹配方案

## 用户需求

匹配 GaussDB PL/pgSQL 中的三语句并发控制模式：

```sql
SELECT c1 INTO var1 FROM TABX WHERE ... FOR UPDATE;
var1 := var1 + 1;
UPDATE TABX SET col = var1 WHERE ...;
```

**核心要求**：尽可能基于 AST 实现匹配（而非纯正则），且需要精确的变量名/表名统一。

## 当前状态

- 8 条 regex 规则已通过（`select_lock.yaml`，22/22 tests）
- 但只覆盖了单语句"零件"（FOR UPDATE 检测、SELECT INTO 检测）
- 跨语句变量/表名统一完全未实现
- DO 块和 `:=` 赋值无法解析（`AnonyBlock`/`Do` → `UnsupportedStatement`）

## 关键发现

`TreeMatcher`（`crates/astgrep-matcher/src/tree_matcher.rs`）是通用 AST 结构匹配器，对所有语言（包括 SQL）工作，工作在 `UniversalNode` 层。但 SQL 的 pattern 由 tree-sitter-sequel 解析，target 由 ogsql-parser 解析——两者产生的 `UniversalNode` 结构**可能不兼容**。

## 方案：五阶段

### Phase A: 适配器 metadata 映射（风险：低）

**文件**: `crates/astgrep-parser/src/adapter/ogsql/dml.rs`

在 `convert_select()` 中新增 3 块 metadata 映射：

1. `select.into_targets` → metadata `has_into=true`, `into_vars="v1,v2"`, 并添加 `identifier` 子节点（role=into_target）
2. `select.lock_clause` → metadata `has_lock=true`, `lock_type="Update"|"Share"|...`, `lock_nowait`, `lock_skip_locked`
3. `select.bulk_collect` → metadata `bulk_collect=true`

**产出**: SELECT 节点携带结构化属性和子节点，规则可精确匹配。

### Phase B: PL/pgSQL 块支持（风险：中）

**新文件**: `crates/astgrep-parser/src/adapter/ogsql/pl.rs`

转换 PL/pgSQL AST → UniversalNode：
- `convert_anony_block()` — 匿名块 `DECLARE...BEGIN...END`
- `convert_do_block()` — `DO $$...END$$`
- `convert_pl_block()` — `PlBlock` 核心转换（声明 + 语句体）
- `convert_pl_statement()` — `PlStatement::Assignment`（`var := expr`）、`Perform`、`Execute` 等
- `convert_pl_declaration()` — 变量/游标声明

**文件**: `crates/astgrep-parser/src/adapter/ogsql/mod.rs`
- 添加 `mod pl;`
- 在 `convert_statement()` 中挂接 `AnonyBlock` 和 `Do`

**产出**: DO 块被解析为带子节点的 `do_block` AST，内部每条语句作为独立子节点，`:=` 赋值变为 `assignment_statement` 节点。

### Phase C1: TreeMatcher 可行性验证（风险：高）

写最小测试规则验证 AST 匹配是否工作：

```yaml
rules:
  - id: GAUSSDB-AST-TEST
    languages: [sql]
    dialects: [gaussdb]
    patterns:
      - pattern: "select_statement"
```

运行 `cargo run -- analyze --dialect gaussdb --rules ... test.sql`，检查能否返回匹配。

**如果 TreeMatcher 不工作**（pattern 和 target AST 结构不兼容），回退到 C2 方案。

### Phase C2: 增强版跨语句 regex（风险：低，fallback）

```yaml
- id: GAUSSDB-LOCK-009
  options:
    sql_statement_boundary: false
  patterns:
    - pattern-regex: "(?s)SELECT\\s+\\w+\\s+INTO\\s+(\\w+)\\s+FROM\\s+(\\w+)\\s+.*FOR\\s+UPDATE.*\\1\\s*:=\\s*\\1.*UPDATE\\s+\\2\\s+SET.*\\1"
```

利用 P1 让整个 DO 块文本可见，用单一正则跨分号匹配三语句 + 捕获组做变量/表名统一。

### Phase C3: AST 优先规则（如果 C1 通过）

```yaml
- id: GAUSSDB-LOCK-010
  patterns:
    - pattern: "do_block"
    - pattern-inside: "select_statement"  # 利用 metadata 过滤
    - pattern-inside: "assignment_statement"
    - pattern-inside: "update_statement"
```

### Phase D: 测试用例（风险：低）

```
cases/select_lock/
  ├── GAUSSDB-LOCK-009_do_block_full_flow.sql       MATCH
  ├── GAUSSDB-LOCK-009_anon_block_full_flow.sql     MATCH
  ├── GAUSSDB-LOCK-009_do_block_no_lock.neg.sql     NO_MATCH
  ├── GAUSSDB-LOCK-009_do_block_diff_table.neg.sql  NO_MATCH
  ├── GAUSSDB-LOCK-009_procedure_full_flow.sql      MATCH
  ├── GAUSSDB-LOCK-009_function_full_flow.sql       MATCH
```

## 实施顺序

```
Phase A (30min) → Phase B (1.5h) → Phase C1 (30min) → C2/C3 (30min) → Phase D (30min)
```

## 风险矩阵

| 风险 | 影响 | 缓解 |
|---|---|---|
| TreeMatcher tree-sitter-sequel vs ogsql-parser AST 不兼容 | C3 不可行 | 回退到 C2 regex，仍实现跨语句匹配 |
| PL/pgSQL 语句变体多，pl.rs 遗漏 | 部分 PL 语法无法解析 | 对用户目标模式（:= 赋值 + SELECT/UPDATE）优先覆盖 |
| metadata 映射遗漏字段 | 部分属性不可查询 | 单元测试覆盖 |
