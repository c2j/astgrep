---
name: astgrep-rules
description: Write astgrep YAML rules for static analysis — pattern matching, taint tracking, SQL dialects. Use when creating, modifying, or debugging astgrep rule files.
license: MIT
compatibility: Requires astgrep rules engine. Rules validated via `astgrep validate` CLI or `validate_rules` MCP tool.
metadata:
  author: astgrep
  version: "2.0"
---

# astgrep 规则编写指南

从零编写一条静态分析规则：先定结构，再写 pattern，加约束精确匹配，配污点追踪数据流。

---

## 快速上手

最小可用的规则只有 4 个字段：

```yaml
rules:
  - id: my-first-rule
    languages: [java]
    message: "找到了潜在问题"
    severity: WARNING
    patterns:
      - pattern: '$OBJ.execute($QUERY)'
```

写完用 `validate_rules`（MCP）或 `astgrep validate`（CLI）验证，再用 `analyze_code`（MCP）或 `astgrep analyze`（CLI）在真实代码上测试效果。

---

## 1. 规则结构

### 完整字段表

```yaml
rules:
  - id: unique-rule-id          # 必选 · 唯一标识符（kebab-case）
    name: "规则名称"             # 可选 · 人类可读（缺省用 id）
    description: "详细说明"      # 可选 · 规则用途（缺省用 message，再缺省用 id）
    message: "命中时显示"        # 可选 · 短描述（已被描述字段）
    severity: ERROR             # 必选 · INFO | WARNING | ERROR | CRITICAL
    confidence: HIGH            # 可选 · LOW | MEDIUM | HIGH（缺省 MEDIUM）
    languages: [java, python]   # 必选 · 目标语言列表
    patterns:                   # 必选 · 模式匹配定义（taint 模式下可为空）
      - pattern: '$FUNC(...)'
    mode: taint                 # 可选 · search（默认）| taint
    dataflow:                   # 可选 · 污点/数据流配置（与 mode:taint 互斥，详见 §4）
      sources: [...]
      sinks: [...]
    fix: 'const $VAR = $VALUE'  # 可选 · 修复建议（元变量可引用）
    fix-regex:                  # 可选 · 正则修复（注意连字符！）
      regex: 'var\s+(\w+)'
      replacement: 'const \1'
    paths:                      # 可选 · 文件过滤
      include: ['src/**/*.java']
      exclude: ['test/**']
    dialects: [gaussdb]         # 可选 · SQL 方言限制（未设 = 适用所有方言）
    enabled: true               # 可选 · 启用/禁用（缺省 true）
    options:                    # 可选 · 高级配置
      sql_statement_boundary: true
    metadata:                   # 可选 · 附加信息（CWE、OWASP 等）
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"
```

**注意**：`message` 不是独立字段，会被解析器存入 `description`。推荐直接用 `description` 写详细说明，`message` 写短摘要。

### 严重度分级

| 等级 | 含义 | 适用场景 |
|------|------|----------|
| `CRITICAL` | 严重安全漏洞，需立即修复 | SQL 注入、RCE、认证绕过 |
| `ERROR` | 明确的安全问题 | XSS、路径遍历、敏感信息泄露 |
| `WARNING` | 潜在风险或可疑模式 | 弱加密、不安全配置 |
| `INFO` | 代码质量建议 | 风格问题、最佳实践提醒 |

---

## 2. 模式匹配（Patterns）

### 基本 Pattern

```yaml
patterns:
  - pattern: 'eval(...)'          # 精确匹配 eval() 调用
  - pattern: '$VAR = $VALUE'      # 元变量匹配任意变量赋值
```

### 元变量（Metavariables）

以 `$` 开头，大写命名，匹配任意 AST 节点：

```yaml
# $OBJ 匹配任意对象，$ARG 匹配任意参数
pattern: '$OBJ.send($ARG)'

# 同一元变量在同一条 pattern 中出现两次时，值必须相同
pattern: '$A.equals($A)'   # 只匹配 x.equals(x)，不匹配 x.equals(y)
```

### 省略号（Ellipsis）

`...` 匹配任意数量（含零个）的节点：

```yaml
pattern: 'foo(...)'                   # 匹配 foo() 任意参数
pattern: |
  if ($COND) {                        # 匹配 if 块内任意位置
    ...
    dangerous_call()
    ...
  }
```

### Pattern 组合

| 组合方式 | 语义 | 用法 |
|----------|------|------|
| **隐式 AND** | `patterns` 数组中所有 pattern 都匹配才命中 | 默认行为 |
| `pattern-either` | 任一匹配即命中（OR） | 多条候选取其一 |
| `pattern-all` | 全部匹配才命中（显式 AND） | 显式组合 |
| `pattern-any` | 任一匹配即命中（OR，同 either） | 等价于 either |
| `pattern-not` | 排除特定模式 | 减少误报 |
| `pattern-inside` | 必须在特定上下文内 | 限定范围 |
| `pattern-not-inside` | 不得在特定上下文内 | 排除安全上下文 |
| `pattern-regex` | 正则匹配源代码文本 | 无法用 AST 表达时降级 |
| `pattern-not-regex` | 正则排除 | 配合 regex 使用 |

```yaml
# 典型组合：匹配危险调用 + 排除安全上下文 + 确保参数是拼接
patterns:
  - pattern: '$STMT.execute($QUERY)'
  - pattern-not: '$STMT.execute("...")'          # 排除字面量
  - pattern-not-inside: |                         # 排除验证后的调用
      if (validate($QUERY)) { ... }
  - metavariable-pattern:                         # 确保参数是拼接
      metavariable: '$QUERY'
      patterns:
        - pattern-either:
            - pattern: '$A + $B'
            - pattern: 'String.format(...)'
```

---

## 3. 条件约束（Conditions）

### metavariable-pattern

对元变量的值进行二次匹配：

```yaml
- metavariable-pattern:
    metavariable: '$QUERY'
    patterns:
      - pattern: '$STR + $INPUT'
```

### metavariable-regex

正则约束元变量值：

```yaml
- metavariable-regex:
    metavariable: '$VAR'
    regex: '^(password|secret|token)'
```

### metavariable-comparison

数值/字符串比较：

```yaml
# 比较操作符：==, !=, <, >, <=, >=, `in`, `not in`
- metavariable-comparison:
    metavariable: '$TIME'
    comparison: '$TIME > 5000'
```

### metavariable-analysis

高级分析（熵检测、复杂度）：

```yaml
# 检测高熵字符串（密钥、令牌）
- metavariable-analysis:
    metavariable: '$VALUE'
    analysis:
      entropy:
        min: 3.5                 # 最小熵值
        max: 6.0                 # 最大熵值（可选）
        charset: "alphanumeric"  # 字符集（可选）
```

### metavariable-name

约束元变量的**标识符名称**（非其值）：

```yaml
# 函数名以 "test" 开头
- metavariable-name:
    metavariable: '$FUNC'
    name_pattern: '^test.*'
```

### focus-metavariable

只报告特定元变量的位置（而非整个 pattern 的匹配范围）：

```yaml
# 报告 $ARG 的位置，而非整个函数调用
patterns:
  - pattern: '$FUNC($ARG1, $ARG2, $ARG3)'
  - focus-metavariable: '$ARG2'
```

---

## 4. 污点分析（Taint Analysis）

追踪数据从不可信源（source）流向敏感操作（sink）的路径。astgrep 支持**两种写法**：

### 写法一：`mode: taint`（Semgrep 兼容）

```yaml
rules:
  - id: sql-injection-taint
    mode: taint
    languages: [java]
    message: "用户输入未经净化流入 SQL 查询"
    severity: ERROR
    pattern-sources:
      - pattern: 'request.getParameter($PARAM)'
      - pattern: 'request.getHeader($HEADER)'
    pattern-sinks:
      - pattern: 'Statement.execute($QUERY)'
      - pattern: 'Statement.executeQuery($QUERY)'
    pattern-sanitizers:
      - pattern: 'sanitize($INPUT)'
      - pattern: 'escapeSql($INPUT)'
```

在此模式下还可使用：

**传播器（Propagators）** — 定义数据如何在变量间传递：

```yaml
    pattern-propagators:
      - pattern: '$A.transform($B)'
        from: '$B'
        to: '$A'
```

**标签化污点（Labeled Taint）** — 多源多条件精确追踪：

```yaml
    pattern-sources:
      - label: USER_INPUT
        pattern: 'getUserInput()'
      - label: SENSITIVE
        pattern: 'getSecret()'
    pattern-sinks:
      - requires: USER_INPUT and SENSITIVE
        pattern: 'log($DATA)'
```

**高级选项**（在 `options:` 块内）：

```yaml
    options:
      taint_assume_safe_booleans: true
      taint_assume_safe_numbers: true
      taint_assume_safe_indexes: true
      taint_assume_safe_functions: true
      taint_only_propagate_through_assignments: true
```

### 写法二：`dataflow:` 块（非 taint 模式）

在不设 `mode: taint` 的规则中，通过 `dataflow:` 块定义源和汇：

```yaml
rules:
  - id: xss-dataflow
    languages: [javascript]
    message: "用户输入可能造成 XSS"
    severity: ERROR
    patterns:                         # pattern 仍然可以定义
      - pattern: 'res.send(...)'
    dataflow:
      sources:
        - 'req.query.$PARAM'
        - 'req.body.$FIELD'
      sinks:
        - 'res.send(...)'
        - 'res.write(...)'
      sanitizers:
        - 'escapeHtml(...)'
      must_flow: true                 # 是否存在数据流（默认 true）
      max_depth: 10                   # 最大分析深度
      taint_assume_safe_booleans: true
      taint_assume_safe_numbers: true
```

**重要区别**：

| 能力 | `mode: taint` | `dataflow:` 块 |
|------|:---:|:---:|
| 基础的 source/sink/sanitizer | ✅ | ✅ |
| Propagators (`from`/`to`) | ✅ | ❌ |
| Label (`label`/`requires`) | ✅ | ❌ |
| `must_flow` / `max_depth` | ❌（用 options 替代） | ✅ |
| `taint_assume_safe_*` | ✅（在 `options:` 内） | ✅（在 `dataflow:` 内直接写） |
| source/sink 为对象（含 focus/label/exact） | ✅ | ❌（仅为字符串） |
| 同时使用 `patterns` | ❌ | ✅ |

**不存在 `taint:` 顶级 key**。两种写法对应两个互斥的解析路径，不存在第三种"新语法"。

---

## 5. SQL 方言规则

通过 `dialects` 字段限制规则在特定 SQL 方言下触发：

```yaml
rules:
  - id: gaussdb-unsupported-feature
    languages: [sql]
    dialects: [gaussdb, opengauss]
    patterns:
      - pattern: "ON CONFLICT"
    message: "GaussDB/OpenGauss 不支持 ON CONFLICT，请用 MERGE INTO"
    severity: ERROR
```

| 方言 | `dialects` 值 | 解析器 |
|------|--------------|--------|
| 标准 SQL | `standard`（默认） | tree-sitter-sequel |
| GaussDB | `gaussdb` | ogsql-parser |
| OpenGauss | `opengauss` | ogsql-parser |
| PolarDB-MySQL | `polardb-mysql` | sqlparser-rs |

不声明 `dialects` 的规则适用**所有方言**（向后兼容）。CLI 使用 `--dialect` 指定分析时的方言。

### 嵌入式 SQL 预处理器

让 SQL 规则直接作用于 Java/MyBatis XML 中的嵌入式 SQL：

```yaml
rules:
  - id: avoid-select-star
    languages: [sql]                         # 规则语言仍为 sql
    patterns:
      - pattern: SELECT * FROM $TABLE
    message: "避免 SELECT *"
    severity: WARNING
    metadata:
      preprocess: embedded-sql               # 启用嵌入式 SQL 提取
      preprocess.from: "java,xml"            # 来源宿主语言
```

启用后，分析引擎从 Java 注解（`@Select("...")`）/ JDBC 调用 / MyBatis XML 标签中提取 SQL，归一化后执行 SQL 规则，命中回填到原文件位置。

**当前支持**：Java 注解、JDBC 常见调用、MyBatis `<select>` 标签。  
**限制**：复杂拼接（StringBuilder/format/条件拼接）以占位符处理；行列精度为片段起始行。

---

## 6. 最佳实践

### 命名

```yaml
# ✅ 好的：含义明确、层级清晰
id: java-sql-injection-prepared-statement
id: python-hardcoded-secret-detection
id: javascript-xss-dom-based

# ❌ 不好
id: rule1
id: test
```

### 减少误报

三条原则：排除安全模式 → 限制上下文 → 精确匹配参数。

```yaml
patterns:
  - pattern: '$OBJ.dangerous($ARG)'
  # 1. 排除已经安全的写法
  - pattern-not: '$OBJ.safeWrapper($ARG)'
  # 2. 排除安全上下文（如在 try-catch 内、在验证后）
  - pattern-not-inside: |
      try { ... } catch (...) { ... }
  # 3. 对参数做进一步约束
  - metavariable-pattern:
      metavariable: '$ARG'
      patterns:
        - pattern: '$USER + $DATA'       # 仅匹配拼接的参数
```

### 性能

- 优先用简单 pattern（AST 匹配快于正则匹配快于深度污点分析）
- `pattern-regex` 应作为最后手段，尽量用结构化 pattern 替代
- 嵌套过深（> 3 层 `pattern-inside`）会显著影响性能
- 通过 `paths.include` 限制扫描范围

### 验证规则

写完规则后：

```bash
# CLI 验证
astgrep validate my-rule.yaml

# CLI 测试
astgrep analyze --rules my-rule.yaml test-file.java

# MCP 验证
validate_rules(rule_content="...")

# MCP 测试
analyze_code(code="...", language="java")
```

### 文档化

为每条规则配上完整元数据：

```yaml
metadata:
  cwe: "CWE-89"
  owasp: "A03:2021 - Injection"
  references:
    - "https://owasp.org/www-community/attacks/SQL_Injection"
  remediation: "使用 PreparedStatement 和参数化查询替代字符串拼接"
```

---

## Semgrep 兼容性参考

| 特性 | 状态 |
|------|:----:|
| `pattern`, `pattern-either`, `pattern-not` | ✅ |
| `pattern-inside`, `pattern-not-inside` | ✅ |
| `pattern-regex`, `pattern-not-regex` | ✅ |
| `pattern-all`, `pattern-any` | ✅ |
| `metavariable-pattern`, `metavariable-regex` | ✅ |
| `metavariable-comparison` | ✅ |
| `metavariable-analysis`（entropy） | ✅ |
| `metavariable-name` | ✅ |
| `focus-metavariable` | ✅ |
| `mode: taint` + `pattern-sources`/`pattern-sinks` | ✅ |
| `pattern-propagators` | ✅（仅 taint 模式） |
| labeled taint (`label`/`requires`) | ✅（仅 taint 模式） |
| `fix`, `fix-regex` | ✅ |
| `paths` 过滤 | ✅ |
| `pattern-where-python` | ❌ 不支持（用 `metavariable-comparison` 替代） |
| `r2c-internal-*` | ❌ Semgrep 内部特性 |
| 跨文件分析 | 🚧 计划中 |
