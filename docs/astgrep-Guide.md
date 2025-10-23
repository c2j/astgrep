# astgrep 规则编写指南

本指南详细介绍如何为 astgrep 编写静态分析规则，包括基本模式匹配、高级特性和与 Semgrep 的兼容性说明。

## 目录

- [规则基础](#规则基础)
- [模式匹配](#模式匹配)
- [元变量](#元变量)
- [污点分析](#污点分析)
- [条件约束](#条件约束)
- [高级特性](#高级特性)
- [Semgrep 兼容性](#semgrep-兼容性)
- [最佳实践](#最佳实践)

---

## 规则基础

### 规则文件结构

astgrep 规则使用 YAML 格式定义。一个基本的规则文件包含以下结构：

```yaml
rules:
  - id: unique-rule-id
    name: "规则名称"
    description: "规则描述"
    severity: ERROR
    confidence: HIGH
    languages: [java, python]
    patterns:
      - pattern: "$FUNC(...)"
    message: "发现问题的描述"
    metadata:
      cwe: "CWE-XXX"
      owasp: "A01:2021"
```

### 必需字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 规则的唯一标识符 |
| `languages` | Array | 适用的编程语言列表 |
| `message` | String | 发现问题时显示的消息 |
| `severity` | Enum | 严重程度：INFO, WARNING, ERROR, CRITICAL |

### 可选字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | String | 规则的友好名称 |
| `description` | String | 详细描述 |
| `confidence` | Enum | 置信度：LOW, MEDIUM, HIGH |
| `patterns` | Array | 模式匹配规则列表 |
| `mode` | String | 分析模式（如 `taint`） |
| `fix` | String | 修复建议 |
| `fix_regex` | Object | 基于正则的自动修复 |
| `metadata` | Object | 元数据（CWE、OWASP 等） |
| `enabled` | Boolean | 是否启用此规则（默认 true） |

---

## 模式匹配

### 基本模式

最简单的模式是直接匹配代码结构：

```yaml
rules:
  - id: hardcoded-password
    languages: [java]
    message: "发现硬编码密码"
    patterns:
      - pattern: 'String password = "..."'
    severity: WARNING
```

### 元变量（Metavariables）

元变量使用 `$` 前缀，可以匹配任意表达式：

```yaml
rules:
  - id: sql-injection
    languages: [java]
    message: "潜在的 SQL 注入"
    patterns:
      - pattern: '$STMT.execute($QUERY)'
    severity: ERROR
```

**元变量命名规则：**
- 必须以 `$` 开头
- 使用大写字母（如 `$VAR`, `$FUNC`, `$QUERY`）
- 可以包含数字（如 `$VAR1`, `$VAR2`）

### 省略号（Ellipsis）

使用 `...` 匹配任意数量的参数或语句：

```yaml
# 匹配任意参数的函数调用
pattern: 'eval(...)'

# 匹配代码块中的任意语句
pattern: |
  if ($COND) {
    ...
    dangerous_function()
    ...
  }
```

### 模式组合

#### pattern-either（或）

匹配多个模式中的任意一个：

```yaml
patterns:
  - pattern-either:
      - pattern: 'MD5.getInstance()'
      - pattern: 'SHA1.getInstance()'
      - pattern: 'DES.getInstance()'
```

#### pattern-all（与）

所有模式都必须匹配：

```yaml
patterns:
  - pattern: '$OBJ.execute($QUERY)'
  - pattern-not: '$OBJ.prepareStatement(...)'
```

#### pattern-inside

模式必须在特定上下文中：

```yaml
patterns:
  - pattern: '$VAR = $INPUT'
  - pattern-inside: |
      function handleRequest($REQ) {
        ...
      }
```

#### pattern-not

排除特定模式：

```yaml
patterns:
  - pattern: '$STMT.execute($QUERY)'
  - pattern-not: '$STMT.execute("SELECT ...")'  # 排除字面量查询
```

#### pattern-not-inside

模式不能在特定上下文中：

```yaml
patterns:
  - pattern: 'eval($INPUT)'
  - pattern-not-inside: |
      if (isSafe($INPUT)) {
        ...
      }
```

---

## 元变量

### metavariable-pattern

对元变量的值进行进一步匹配：

```yaml
patterns:
  - pattern: '$STMT.execute($QUERY)'
  - metavariable-pattern:
      metavariable: '$QUERY'
      patterns:
        - pattern: '$STR + $INPUT'  # 查询是字符串拼接
```

### metavariable-regex

使用正则表达式约束元变量：

```yaml
patterns:
  - pattern: 'String $VAR = "..."'
  - metavariable-regex:
      metavariable: '$VAR'
      regex: '^(password|passwd|pwd|secret|token)$'
```

### metavariable-comparison

比较元变量的值：

```yaml
patterns:
  - pattern: 'setTimeout($FUNC, $TIME)'
  - metavariable-comparison:
      metavariable: '$TIME'
      comparison: '$TIME > 5000'  # 超时时间大于 5 秒
```

**支持的比较操作符：**
- `==`, `!=`: 相等/不等
- `>`, `<`, `>=`, `<=`: 数值比较
- `in`, `not in`: 包含关系
- `re.match()`: 正则匹配

### metavariable-analysis

对元变量进行高级分析：

```yaml
patterns:
  - pattern: 'const $VAR = "$VALUE"'
  - metavariable-analysis:
      metavariable: '$VALUE'
      analysis:
        entropy:
          min: 3.5  # 最小熵值（检测随机字符串）
```

**支持的分析类型：**
- `entropy`: 熵分析（检测密钥、令牌）
- `type`: 类型分析
- `complexity`: 复杂度分析

---

## 污点分析

污点分析用于追踪数据从不可信源（source）流向敏感操作（sink）的路径。

### 基本污点分析（旧语法）

```yaml
rules:
  - id: user-input-to-sql
    mode: taint
    languages: [java]
    message: "用户输入流向 SQL 查询"
    pattern-sources:
      - pattern: 'request.getParameter($PARAM)'
      - pattern: 'request.getHeader($HEADER)'
    pattern-sinks:
      - pattern: 'Statement.execute($QUERY)'
      - pattern: 'Statement.executeQuery($QUERY)'
    pattern-sanitizers:
      - pattern: 'sanitize($INPUT)'
      - pattern: 'escape($INPUT)'
    severity: ERROR
```

### 新语法污点分析（推荐）

astgrep 支持更简洁的新语法：

```yaml
rules:
  - id: xss-vulnerability
    languages: [javascript]
    message: "潜在的 XSS 漏洞"
    taint:
      sources:
        - 'req.query.$PARAM'
        - 'req.body.$FIELD'
      sinks:
        - 'res.send(...)'
        - 'res.write(...)'
      sanitizers:
        - 'escape(...)'
        - 'sanitizeHtml(...)'
    severity: CRITICAL
```

### 污点传播器（Propagators）

定义数据如何在不同变量间传播：

```yaml
taint:
  sources:
    - 'getUserInput()'
  sinks:
    - 'executeCommand(...)'
  propagators:
    - pattern: '$A.transform($B)'
      from: '$B'
      to: '$A'
  sanitizers:
    - 'validate(...)'
```

### 标签化污点分析

使用标签进行更精细的污点追踪：

```yaml
rules:
  - id: labeled-taint
    languages: [python]
    message: "需要同时满足多个污点条件"
    taint:
      sources:
        - label: TAINTED
          pattern: 'user_input()'
        - label: SENSITIVE
          pattern: 'get_secret()'
      sinks:
        - requires: TAINTED and SENSITIVE
          pattern: 'log(...)'
    severity: ERROR
```

### 数据流配置

```yaml
dataflow:
  sources:
    - 'request.getParameter(...)'
  sinks:
    - 'Statement.execute(...)'
  sanitizers:
    - 'sanitize(...)'
  must_flow: true      # 必须存在数据流
  max_depth: 10        # 最大分析深度
```

---

## 条件约束

### focus-metavariable

聚焦于特定元变量的位置：

```yaml
patterns:
  - pattern: |
      $FUNC($ARG1, $ARG2, $ARG3)
  - focus-metavariable: '$ARG2'  # 只报告第二个参数的位置
```

### metavariable-name

约束元变量的名称：

```yaml
patterns:
  - pattern: 'function $FUNC(...) { ... }'
  - metavariable-name:
      metavariable: '$FUNC'
      name_pattern: '^test.*'  # 函数名必须以 test 开头
```

---

## 高级特性

### 自动修复

#### 简单修复建议

```yaml
rules:
  - id: use-const
    languages: [javascript]
    patterns:
      - pattern: 'var $VAR = $VALUE'
    message: "使用 const 或 let 代替 var"
    fix: 'const $VAR = $VALUE'
    severity: INFO
```

#### 正则修复

```yaml
fix_regex:
  regex: 'var\s+(\w+)'
  replacement: 'const \1'
  count: 1  # 替换次数
```

### 路径过滤

```yaml
paths:
  include:
    - '*.java'
    - 'src/**/*.py'
  exclude:
    - 'test/**'
    - '**/*_test.py'
```

### 元数据

```yaml
metadata:
  cwe: 'CWE-89'
  owasp: 'A03:2021 - Injection'
  category: 'security'
  subcategory: 'sql-injection'
  confidence: 'HIGH'
  likelihood: 'MEDIUM'
  impact: 'HIGH'
  references:
    - 'https://owasp.org/www-community/attacks/SQL_Injection'
```

---

## Semgrep 兼容性

astgrep 致力于与 Semgrep 保持高度兼容，但也有一些差异和扩展。

### ✅ 完全兼容的特性

| 特性 | 说明 | 示例 |
|------|------|------|
| 基本模式 | 简单的代码模式匹配 | `pattern: 'eval(...)'` |
| 元变量 | `$VAR` 语法 | `pattern: '$FUNC($ARG)'` |
| 省略号 | `...` 匹配任意内容 | `pattern: 'foo(...)'` |
| pattern-either | 或逻辑 | ✅ |
| pattern-not | 否定模式 | ✅ |
| pattern-inside | 上下文匹配 | ✅ |
| pattern-not-inside | 否定上下文 | ✅ |
| metavariable-pattern | 元变量模式 | ✅ |
| metavariable-regex | 元变量正则 | ✅ |
| metavariable-comparison | 元变量比较 | ✅ |
| 污点分析（旧语法） | `mode: taint` | ✅ |
| 污点分析（新语法） | `taint:` 块 | ✅ |
| focus-metavariable | 聚焦元变量 | ✅ |

### 🔄 部分兼容的特性

| 特性 | astgrep 支持 | 说明 |
|------|--------------|------|
| pattern-regex | ✅ | 支持基本正则匹配 |
| metavariable-analysis | ✅ | 支持熵分析，类型分析部分支持 |
| Python 表达式比较 | 🚧 | 部分支持，不支持完整 Python 语法 |
| 跨文件分析 | 🚧 | 计划中 |
| 类型推断 | 🚧 | 部分语言支持 |

### ❌ 不兼容的特性

| 特性 | 说明 | 替代方案 |
|------|------|----------|
| `pattern-where-python` | 不支持完整 Python 表达式 | 使用 `metavariable-comparison` |
| `r2c-internal-*` | Semgrep 内部特性 | 无 |
| 某些语言特定特性 | 依赖语言支持程度 | 查看语言支持文档 |

### 🆕 astgrep 扩展特性

astgrep 提供了一些 Semgrep 没有的特性：

1. **增强的污点分析配置**
   ```yaml
   dataflow:
     max_depth: 20
     field_sensitive: true
     context_sensitive: true
   ```

2. **更丰富的元数据支持**
   ```yaml
   metadata:
     confidence: HIGH
     likelihood: MEDIUM
     impact: HIGH
   ```

3. **GUI 和 Web 界面**
   - 交互式规则测试
   - 可视化数据流图

### 迁移指南

从 Semgrep 迁移到 astgrep：

1. **规则文件兼容性**
   - 大多数 Semgrep 规则可以直接使用
   - 建议使用 `astgrep validate` 验证规则

2. **语法差异**
   ```yaml
   # Semgrep
   pattern-where-python: |
     int($TIME) > 5000
   
   # astgrep（推荐）
   metavariable-comparison:
     metavariable: '$TIME'
     comparison: '$TIME > 5000'
   ```

3. **测试规则**
   ```bash
   # 验证规则语法
   astgrep validate your-rule.yaml
   
   # 测试规则
   astgrep analyze --rules your-rule.yaml test-file.java
   ```

---

## 最佳实践

### 1. 规则命名

```yaml
# ✅ 好的命名
id: java-sql-injection-prepared-statement
id: python-hardcoded-secret-detection
id: javascript-xss-dom-based

# ❌ 不好的命名
id: rule1
id: test
id: my-rule
```

### 2. 消息编写

```yaml
# ✅ 清晰的消息
message: |
  发现潜在的 SQL 注入漏洞。用户输入 '$INPUT' 未经验证直接用于 SQL 查询。
  建议使用 PreparedStatement 和参数化查询。

# ❌ 模糊的消息
message: "发现问题"
```

### 3. 严重程度分级

```yaml
# CRITICAL: 严重安全漏洞
severity: CRITICAL  # SQL 注入、RCE、认证绕过

# ERROR: 明确的安全问题
severity: ERROR     # XSS、路径遍历、敏感信息泄露

# WARNING: 潜在问题
severity: WARNING   # 弱加密、不安全配置

# INFO: 代码质量建议
severity: INFO      # 代码风格、最佳实践
```

### 4. 减少误报

```yaml
patterns:
  - pattern: '$STMT.execute($QUERY)'
  # 排除安全的情况
  - pattern-not: '$STMT.execute("...")'  # 字面量
  - pattern-not-inside: |
      if (validate($QUERY)) {
        ...
      }
  # 确保是字符串拼接
  - metavariable-pattern:
      metavariable: '$QUERY'
      patterns:
        - pattern-either:
            - pattern: '$A + $B'
            - pattern: 'String.format(...)'
```

### 5. 性能优化

```yaml
# ✅ 高效的模式
patterns:
  - pattern: 'eval($INPUT)'  # 简单直接
  
# ❌ 低效的模式
patterns:
  - pattern-regex: '.*eval.*'  # 过于宽泛
  - pattern-inside: |
      ...  # 过深的嵌套
      ...
      ...
```

### 6. 文档化

```yaml
rules:
  - id: java-xxe-vulnerability
    name: "XML 外部实体注入"
    description: |
      检测 XML 解析器配置不当导致的 XXE 漏洞。
      当 XML 解析器允许处理外部实体时，攻击者可以读取服务器文件或进行 SSRF 攻击。
    message: |
      XML 解析器未禁用外部实体处理，可能导致 XXE 漏洞。
      建议设置 setFeature(XMLConstants.FEATURE_SECURE_PROCESSING, true)
    metadata:
      cwe: "CWE-611"
      owasp: "A05:2021 - Security Misconfiguration"
      references:
        - "https://owasp.org/www-community/vulnerabilities/XML_External_Entity_(XXE)_Processing"
      remediation: |
        禁用 DTD 和外部实体：
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
```

---

## 示例规则集

### SQL 注入检测

```yaml
rules:
  - id: java-sql-injection-comprehensive
    languages: [java]
    message: "潜在的 SQL 注入漏洞"
    patterns:
      - pattern-either:
          - pattern: '$STMT.execute($QUERY)'
          - pattern: '$STMT.executeQuery($QUERY)'
          - pattern: '$STMT.executeUpdate($QUERY)'
      - pattern-not: '$STMT.execute("...")'
      - metavariable-pattern:
          metavariable: '$QUERY'
          patterns:
            - pattern-either:
                - pattern: '$A + $B'
                - pattern: 'String.format($FMT, ...)'
                - pattern: '$STR.concat($OTHER)'
    severity: CRITICAL
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"
```

### XSS 检测

```yaml
rules:
  - id: javascript-dom-xss
    languages: [javascript]
    message: "潜在的 DOM XSS 漏洞"
    taint:
      sources:
        - 'location.search'
        - 'location.hash'
        - 'document.URL'
        - 'document.referrer'
      sinks:
        - '$EL.innerHTML = ...'
        - 'document.write(...)'
        - 'eval(...)'
      sanitizers:
        - 'DOMPurify.sanitize(...)'
        - 'escapeHtml(...)'
    severity: CRITICAL
```

---

## 总结

astgrep 提供了强大而灵活的规则系统，支持从简单的模式匹配到复杂的污点分析。通过遵循本指南和最佳实践，你可以编写高质量、低误报的静态分析规则。

### 相关资源

- [astgrep GitHub 仓库](https://github.com/c2j/astgrep)
- [Semgrep 规则语法参考](https://semgrep.dev/docs/writing-rules/rule-syntax/)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE 列表](https://cwe.mitre.org/)

### 获取帮助

- 提交 Issue: https://github.com/c2j/astgrep/issues
- 查看示例规则: `tests/rules/` 目录
- 运行 `astgrep validate` 验证规则语法

