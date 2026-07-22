---
name: astgrep
description: Use astgrep for static code analysis — scan code for security vulnerabilities and quality issues via MCP or CLI. Triggered by requests like "scan this code", "find vulnerabilities", "validate this rule", or "what languages do you support".
license: MIT
compatibility: Requires astgrep CLI with MCP support (rmcp 0.2 SDK, stdio transport).
metadata:
  author: astgrep
  version: "2.0"
---

# astgrep 使用指南

通过 MCP 使用 astgrep 进行代码分析。支持 Java / JavaScript / Python / SQL / Bash / XML 的漏洞检测和代码质量检查。

## 流程概览

```
1. 启动         2. 了解规则        3. 分析代码         4. 解读结果
astgrep mcp → list_rules     → analyze_code    → findings[]
              list_languages                        stats{}
```

---

## 1. 启动服务

```bash
# 使用内置规则
astgrep mcp

# 指定自定义规则目录
astgrep mcp --rules-dir path/to/rules/
```

### 客户端配置（Claude Desktop 示例）

```json
{
  "mcpServers": {
    "astgrep": {
      "command": "astgrep",
      "args": ["mcp", "--rules-dir", "/absolute/path/to/rules"]
    }
  }
}
```

服务通过 stdio 通信，无 HTTP 端口，无需防火墙配置。

---

## 2. 四项工具速查

| 工具 | 作用 | 需要参数 | 什么时候用 |
|------|------|----------|-----------|
| `list_languages` | 列出支持的编程语言及扩展名 | 无 | 用户问"支持什么语言？"/ "TypeScript 能扫吗？" |
| `list_rules` | 列出已加载的所有规则 | 无 | 用户问"有什么规则？"/ "有 Java 相关的规则吗？" |
| `analyze_code` | 对代码执行分析 | `code` + `language` [+ `target_path`] | 用户给出代码片段 / 文件，要求"扫描漏洞" |
| `validate_rules` | 验证 YAML 规则语法和语义 | `rule_content` | 用户写了新规则，要求"检查规则是否正确" |

---

## 3. 分析代码 — `analyze_code`

### 调用方式

```json
{
  "code": "stmt.execute(\"SELECT * FROM users WHERE id = \" + userId);",
  "language": "java",
  "target_path": "/path/to/project"   // 可选，用于项目上下文分析
}
```

### 参数说明

| 参数 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `code` | string | **是** | 源代码内容。直接传入原始代码，无需转义（JSON 字符串编码即可） |
| `language` | string | **是** | 语言名。建议先调用 `list_languages` 获取准确名称。接受别名：`js`/`typescript`/`ts` → `javascript`，`py` → `python`，`shell`/`sh` → `bash`，`txt`/`plaintext` → `text` |
| `target_path` | string | 否 | 项目根目录路径。提供后，分析范围包含该目录下的文件（而非仅传入的代码片段），可用于跨文件规则 |

### 返回值

```json
{
  "findings": [
    {
      "rule_id": "java-sql-injection",
      "severity": "ERROR",
      "message": "Potential SQL injection: string concatenation in execute()",
      "line": 42,
      "column": 12,
      "file_path": "/tmp/.tmpXXXX/source.java",
      "snippet": "Potential SQL injection: string concatenation in execute()"
    }
  ],
  "stats": {
    "files_analyzed": 1,
    "rules_executed": 45,
    "parse_errors": 0,
    "analysis_errors": 0,
    "dataflow_analyses": 0
  },
  "elapsed_ms": 120
}
```

### 解读 Finding 字段

| 字段 | 含义 |
|------|------|
| `rule_id` | 匹配的规则 ID。结合 `list_rules` 结果可查看规则详情 |
| `severity` | 严重度：`ERROR`（明确漏洞）> `WARNING`（潜在风险）> `INFO`（代码建议）> `NOTE` |
| `message` | 人类可读的问题描述 |
| `line` / `column` | 问题位置（行号从 1 开始）。注意：仅包含**起始**行列，不含结束位置 |
| `file_path` | 命中文件路径（MCP 传代码片段时是临时文件路径，不代表用户文件） |
| `snippet` | 消息的前 200 字符截断（**不是**源代码片段！完整消息看 `message`） |

### 解读 Stats 字段

| 字段 | 关注点 |
|------|--------|
| `parse_errors > 0` | 代码中部分语法未被解析器支持，可能漏报。告知用户 |
| `analysis_errors > 0` | 规则执行时报错，可能有规则不兼容。建议用 `validate_rules` 检查规则 |
| `dataflow_analyses` | 当前**总是 0**，因为 MCP 默认关闭污点分析（`enable_dataflow: false`） |
| `rules_executed` | 实际运行的规则数。如果为 0，检查 `rules_dir` 是否配置正确 |

### 使用流程

1. **先探**：调用 `list_languages` 确认语言名；调用 `list_rules` 了解可用规则
2. **再扫**：传入代码 + 语言，拿到 findings
3. **报告**：只展示与用户问题相关的 findings，不要逐条罗列。`parse_errors > 0` 时提醒用户语法可能未被完整解析
4. **写规则（可选）**：如果现有规则不够，引导用户到 [规则编写指南](rules.md) 编写新规则，再用 `validate_rules` 验证

### 限制

- 污点分析（dataflow）默认关闭，无法通过 MCP 参数开启
- SQL 方言不可配置（使用默认标准 SQL 解析）
- `finding` 不含 `confidence`、`end_line`、`fix` 字段（仅 CLI/Web 接口返回完整字段）
- 每次调用独立执行，无缓存或状态保持

---

## 4. 验证规则 — `validate_rules`

### 调用方式

```json
{
  "rule_content": "rules:\n  - id: my-rule\n    languages: [java]\n    ..."
}
```

### 返回值

```json
[
  {
    "file": "/tmp/.tmpXXXX/rule.yml",
    "valid": true,
    "errors": [],
    "warnings": [],
    "rules_loaded": 1
  }
]
```

- `valid: true` + `errors: []` → 规则语法和语义均正确
- `valid: false` → `errors` 数组列出阻塞问题，`warnings` 列出建议
- 规则格式参照 [rules.md](rules.md)

---

## 5. 查询信息 — `list_rules` / `list_languages`

### `list_rules`（无参数）

返回已加载规则的元数据：`id`, `name`, `severity`, `languages`, `description`。

**注意**：如果 `rules_dir` 未配置，返回空数组 `[]`。仅扫描 `.yaml`/`.yml` 文件。

### `list_languages`（无参数）

返回 7 种语言及其扩展名。注意：`javascript` 覆盖 `.ts`/`.tsx`（TypeScript 作为 JavaScript 解析）。C/C#/Ruby/Kotlin/Swift/PHP 有 parser 适配器但**尚未**完全集成。

---

## 架构说明

- **传输**：rmcp 0.2 SDK，纯 stdio（无网络端口）
- **复用的核心函数**：`analyze_code` → `analyze_collect()`，`validate_rules` → `validate_collect()`（与 CLI 共享同一引擎）
- **循环依赖处理**：`astgrep-mcp` 依赖 `astgrep-cli`，但 `mcp` 子命令在 `src/main.rs` 中被拦截在 CLI parser 之前，避免循环依赖
- **性能**：单线程执行（`parallel: false`），临时文件用后即删，无状态保持
