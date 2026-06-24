# 编写 GaussDB SQL 方言规则与正反案例指南

> 以 `tests/categories/sql_dialects/gaussdb/` 中的 `GAUSSDB-SET-001`(检测 `UPDATE SET (col1, col2, col3, ...) = (SELECT ...)` 多列子查询赋值)为贯穿示例。
>
> **适用读者**:为 astgrep 编写 SQL 方言(尤其是 GaussDB / OpenGauss)规则、并希望产出可被自动校验的正反案例的工程师。

---

## 1. 目录结构(必须遵守)

每个规则驱动的测试类别都遵循 `tests/CONVENTIONS.md` 中定义的"自描述"结构:

```
tests/categories/sql_dialects/gaussdb/
├── rules/
│   └── update_set.yaml                       ← 规则定义文件
└── cases/
    └── update_set/                           ← 与规则同名的 concern 目录
        ├── GAUSSDB-SET-001_*_3col.sql        ← 正面案例(应触发,无 .neg)
        ├── GAUSSDB-SET-001_*_2col.neg.sql    ← 反面案例(不应触发,带 .neg)
        ├── GAUSSDB-SET-001_*.java            ← 跨语言变体(嵌入 SQL)
        └── GAUSSDB-SET-001_*.xml             ← MyBatis XML 变体
```

**关键约定:**

- `rules/{concern}.yaml` 与 `cases/{concern}/` **必须同名**(此处都叫 `update_set`)
- 文件名前缀必须是 **规则 ID**(`GAUSSDB-SET-001`)
- `.neg.` 中缀标识反面案例(规则不应触发)
- 每条规则**至少需要 1 个正面 + 1 个反面**案例

---

## 2. 规则 ID 命名规则

格式: `{LANG}-{CATEGORY}-{NNN}`

| 前缀 | 适用范围 |
|------|----------|
| `GAUSSDB` | GaussDB / OpenGauss 方言 |
| `POLARDB` | PolarDB-MySQL 方言 |
| `SQL` | 标准 SQL |
| `JAVA` / `JS` / `PY` / `BASH` / `XML` | 其他语言 |

示例:`GAUSSDB-SET-001` = GaussDB 方言、SET 类别、第 1 条规则。

---

## 3. 编写规则文件(`rules/update_set.yaml`)

### 3.1 完整模板(带逐字段注释)

```yaml
rules:
  - id: GAUSSDB-SET-001                              # 1. 唯一 ID(与文件名前缀对应)
    name: "GaussDB UPDATE SET 多列子查询 (>2列)"      # 2. 简短人类可读名称
    description: >                                    # 3. 详细说明(为何要检测)
      检测 UPDATE SET (col1, col2, col3, ...) = (SELECT ...) 语句。
      当 SET 子句元组赋值超过2个字段时，GaussDB 可能存在限制或性能问题。
    languages: [sql]                                  # 4. 目标语言
    dialects: [gaussdb, opengauss]                    # 5. SQL 方言限制(可选)
    patterns:                                         # 6. 匹配逻辑(AND 关系)
      - pattern: "$SQL"                               #    先用元变量捕获整条 SQL
      - metavariable-regex:                           #    再对捕获的内容做正则约束
          metavariable: $SQL
          regex: "(?s)SET\\s*\\([^)]*,[^)]*,"
    message: "UPDATE SET 子句包含3+字段子查询赋值..."  # 7. 报告时显示给用户的提示
    severity: WARNING                                 # 8. ERROR / WARNING / INFO
    confidence: HIGH                                  # 9. HIGH / MEDIUM / LOW
    metadata:                                         # 10. 附加元数据
      category: compatibility
      cwe: "CWE-1106"
```

### 3.2 字段详解

| 字段 | 必填 | 说明 |
|------|------|------|
| `id` | ✅ | 全局唯一,大写 + 连字符 |
| `name` | ✅ | 报告标题 |
| `description` | ✅ | 多行说明,**要说清"为什么要检测"** |
| `languages` | ✅ | 这里固定 `[sql]` |
| `dialects` | ⚠️ | SQL 方言规则强烈建议填写,限定只在该方言下生效 |
| `patterns` | ✅ | 匹配器数组(见下节) |
| `message` | ✅ | 给开发者的修复指引 |
| `severity` | ✅ | `ERROR` / `WARNING` / `INFO` |
| `confidence` | ✅ | `HIGH` / `MEDIUM` / `LOW` |
| `metadata` | ❌ | `category`、`cwe`、`owasp` 等分类标签 |

### 3.3 匹配模式(`patterns`)的核心思路

本规则的策略采用**两段式匹配**:

```yaml
patterns:
  - pattern: "$SQL"                                  # 第一步:捕获任意 SQL 语句到 $SQL
  - metavariable-regex:                              # 第二步:对 $SQL 内容施加正则
      metavariable: $SQL
      regex: "(?s)SET\\s*\\([^)]*,[^)]*,"            # SET( 后至少出现 2 个逗号 = 3+ 列
```

**正则解析**:

- `(?s)` — `s` 标志让 `.` 匹配换行(支持多行书写,见正面案例 4)
- `SET\s*\(` — 匹配 `SET (`(允许中间空白)
- `[^)]*,[^)]*,` — 在右括号前出现 2 个逗号 → 至少 3 列

**为什么这样设计?**

SQL 方言规则常因语法特殊(如 GaussDB 的元组赋值)无法用纯结构匹配,所以采用"宽口径捕获 + 正则精筛"的组合拳。

---

## 4. 编写测试案例

### 4.1 三行注解头(强制)

**每个**测试文件**必须**以前 3 行注解开头(语法随语言变化):

| 语言 | 注释语法 |
|------|----------|
| SQL | `-- @rule ...` |
| Java / JS / Rust | `// @rule ...` |
| Python / Bash | `# @rule ...` |
| XML / HTML | `<!-- @rule ... -->` |

三个必填注解:

```
@rule    GAUSSDB-SET-001          ← 被测规则 ID
@desc    场景的人类可读描述         ← 说明这条 SQL 在测什么
@expect  MATCH | NO_MATCH         ← 期望结果
```

可选:`@dialect gaussdb`(路径已隐含方言时可省略)。

### 4.2 文件命名规范

```
{RULE_ID}_{short_description}.{ext}         ← 正面案例
{RULE_ID}_{short_description}.neg.{ext}     ← 反面案例
```

- `short_description`:小写 + 下划线,**≤ 30 字符**
- 跨语言变体共享 RULE_ID,只换扩展名

---

## 5. 正反案例对照(本规则的全部 9 个)

### ✅ 正面案例(应触发,`@expect MATCH`)

#### 案例 1:基础 3 列子查询

`GAUSSDB-SET-001_multicol_subquery_3col.sql`

```sql
-- @rule GAUSSDB-SET-001
-- @desc 3 columns SET subquery (should trigger)
-- @expect MATCH
UPDATE employees e
SET (e.salary, e.dept, e.title) = (
    SELECT s.salary, s.dept, s.title
    FROM new_data s WHERE s.id = e.id
);
```

**为什么触发**:正则匹配到 `SET (` 后有 2 个逗号(3 列)。

#### 案例 2:多行跨行书写

`GAUSSDB-SET-001_multicol_subquery_4col_multiline.sql`

```sql
-- @rule GAUSSDB-SET-001
-- @desc Multi-line SET with 4 columns across lines (should trigger)
-- @expect MATCH
UPDATE target_table t
SET (
    t.col_a,
    t.col_b,
    t.col_c,
    t.col_d
) = (
    SELECT s.col_a, s.col_b, s.col_c, s.col_d
    FROM source_table s
    WHERE s.id = t.id
);
```

**为什么触发**:`(?s)` 标志让正则跨行匹配。

#### 案例 3:5 列(边界外)

`GAUSSDB-SET-001_multicol_subquery_5col.sql` — 验证大量列同样命中。

#### 案例 4:带 WHERE 子句

`GAUSSDB-SET-001_multicol_subquery_3col_with_where.sql` — 验证外层 `WHERE` 不影响 `SET` 元组的捕获。

#### 案例 5:Java DAO 中嵌入 SQL

`GAUSSDB-SET-001_java_dao_multicol.java`

```java
// @rule GAUSSDB-SET-001
// @desc Java DAO with UPDATE SET 3+ columns from subquery
// @expect MATCH
String sql = "UPDATE employees SET (salary, dept, title) = "
        + "(SELECT salary, dept, title FROM new_data WHERE id = ?)";
```

**为什么触发**:astgrep 的嵌入式 SQL 预处理器会从 Java 字符串中抽取 SQL 再交给方言规则匹配。

#### 案例 6 & 7:MyBatis XML

`GAUSSDB-SET-001_ibatis_mapper_multicol.xml` 与 `GAUSSDB-SET-001_ibatis_dynamic_3col_aggregate.xml` — 验证 `<update>` 标签内的 SQL(含 `<if>` 动态片段)同样被抽取并匹配。

---

### ❌ 反面案例(不应触发,`@expect NO_MATCH`)

#### 反例 1:刚好 2 列(阈值之内)

`GAUSSDB-SET-001_multicol_subquery_2col.neg.sql`

```sql
-- @rule GAUSSDB-SET-001
-- @desc 2 columns SET subquery (within limit, should NOT trigger)
-- @expect NO_MATCH
UPDATE employees e
SET (e.salary, e.dept) = (
    SELECT s.salary, s.dept
    FROM new_data s WHERE s.id = e.id
);
```

**为什么不触发**:`SET (` 后只有 1 个逗号,正则 `[^)]*,[^)]*,` 要求至少 2 个。

#### 反例 2:普通 SET col=val(非元组)

`GAUSSDB-SET-001_normal_set_colval.neg.sql`

```sql
-- @rule GAUSSDB-SET-001
-- @desc Normal SET col=val (no tuple, should NOT trigger)
-- @expect NO_MATCH
UPDATE employees SET salary = 100, dept = 'IT', title = 'Engineer' WHERE id = 1;
```

**为什么不触发**:虽然 `SET` 后有多个逗号,但**没有紧跟 `(`,正则中 `SET\s*\(` 这一段不成立。

> 📌 **反例设计的核心原则**:每个反例都应针对正则/模式中的**某一个具体子条件**构造"差一点就匹配"的场景,以证明边界正确。本例中:
>
> - 反例 1 验证**列数阈值**(2 vs 3)
> - 反例 2 验证**语法形态**(元组赋值 vs 普通赋值)

---

## 6. 从零编写一条新规则的完整流程

1. **确定检测目标** — 写下"什么样的 SQL/代码应该被报告"以及"为什么",填入 `description`。
2. **申请规则 ID** — 按 `{LANG}-{CATEGORY}-{NNN}` 格式分配。
3. **设计匹配策略**:
   - 优先用结构化 `pattern`(如 `$STMT.execute($QUERY)`)
   - SQL 方言语法特殊时,采用 `$SQL + metavariable-regex` 组合
4. **建立目录**:

   ```
   rules/{concern}.yaml
   cases/{concern}/
   ```

5. **先写 1 正 1 反**最简案例,跑通规则。
6. **补充边界案例**:多列、单行/多行、带 WHERE、嵌入 Java、嵌入 XML。
7. **补充反例**:针对每个匹配条件各写一个"差一点"的反例。
8. **校验**:

   ```bash
   # 校验注解格式
   python3 tests/scripts/validate_annotations.py --category gaussdb

   # 真正跑规则
   cargo run -- analyze \
       --dialect gaussdb \
       --rules tests/categories/sql_dialects/gaussdb/rules/update_set.yaml \
       tests/categories/sql_dialects/gaussdb/cases/update_set/
   ```

---

## 7. 常见陷阱清单

| 陷阱 | 后果 | 解决 |
|------|------|------|
| 忘记 `(?s)` 标志 | 多行 SQL 漏报 | 正则前加 `(?s)` |
| 正则未转义 `\(` | 匹配失败或 panic | YAML 中用 `\\(` |
| `dialects` 字段缺失 | 规则可能误报其他方言的同类语法 | SQL 方言规则必填 |
| 反例与正例"差太多" | 边界覆盖不充分 | 反例应只改动**一个**条件 |
| 文件名 RULE_ID 与 `id` 不一致 | 校验脚本无法关联 | 严格保持一致 |
| 跨语言变体忘记改注释语法 | 注解无法被识别 | SQL 用 `--`,Java 用 `//`,XML 用 `<!-- -->` |

---

## 8. 参考资源

- 命名与注解规范:[`tests/CONVENTIONS.md`](../tests/CONVENTIONS.md)
- 通用规则格式:[`astgrep-Guide.md`](./astgrep-Guide.md)
- SQL 方言架构:[`sql-dialects.md`](./sql-dialects.md)
- 同类参考规则:同目录下其他 `*.yaml` 与 `cases/` 子目录(如 `plpgsql/`)
