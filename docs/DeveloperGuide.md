# astgrep 开发者指南 (Developer Guide)

本指南面向需要通过 REST API、Rust 库或 CI/CD 集成 astgrep 分析能力的开发者。

---

## 目录

1. 架构概览
2. REST API 集成
3. Rust 库集成
4. CI/CD 集成
5. MCP 集成规划（Roadmap）
6. 最佳实践

---

## 1. 架构概览 (Architecture Overview)

### Crate 依赖图

```
astgrep-core (类型、错误、配置)
    ↑
astgrep-ast (UniversalNode、visitor)
    ↑
astgrep-parser (tree-sitter 适配器 + SQL 方言分发)
    ↑
astgrep-matcher (模式匹配引擎)
    ↑
astgrep-dataflow (污点分析、数据流)
    ↑
astgrep-rules (YAML 规则引擎)
    ↑
astgrep-cli / astgrep-web / astgrep-gui (接口层)
```

### 核心概念

**UniversalNode**: 所有语言的规范化 AST 节点类型。无论源解析器是什么（tree-sitter、ogsql-parser、sqlparser-rs），所有适配器都产出 UniversalNode，规则匹配在其上运行。

**SqlDialect 分发**: 当分析 SQL 时，`--dialect` 标志决定使用哪个解析器:

- Standard → tree-sitter-sequel
- GaussDB/OpenGauss → ogsql-parser (经 OgsqlAdapter)
- PolarDB-MySQL → sqlparser-rs (经 SqlparserAdapter)

**规则执行流程**:

```
源码 → Parser → UniversalNode → Rule Engine → Pattern Matching → Findings
                                    ↓
                            (SQL 方言分发)
                                    ↓
                            Dialect Adapter → UniversalNode
```

### 接口层

| 接口 | 二进制 | 适用场景 |
|------|--------|---------|
| CLI | astgrep | 命令行分析、CI/CD 集成 |
| REST API | astgrep-web-server | HTTP 集成、Web 服务 |
| Web Playground | astgrep-web-server (/playground) | 交互式规则调试 |
| Desktop GUI | astgrep-gui | 本地交互式分析 |

---

## 2. REST API 集成 (REST API Integration)

astgrep 提供 RESTful API，默认监听 `http://127.0.0.1:8080`，API 前缀 `/api/v1`。

### 启动服务

```bash
astgrep-web-server
# 或自定义端口
astgrep-web-server --bind 0.0.0.0 --port 9090
# 生成配置文件
astgrep-web-server --generate-config --config astgrep-web.toml
```

### 核心端点

| 方法 | 端点 | 说明 |
|------|------|------|
| GET | /api/v1/health | 健康检查 |
| GET | /api/v1/version | 版本信息 |
| GET | /api/v1/metrics | Prometheus 指标 |
| POST | /api/v1/analyze | 代码片段分析 |
| POST | /api/v1/analyze/sarif | 分析（SARIF 输出） |
| POST | /api/v1/analyze/file | 单文件分析（base64/multipart） |
| POST | /api/v1/analyze/archive | 压缩包分析 |
| GET | /api/v1/jobs | 任务列表 |
| GET | /api/v1/jobs/{id} | 任务详情 |
| GET | /api/v1/rules | 规则列表 |
| GET | /api/v1/rules/{id} | 规则详情 |
| POST | /api/v1/rules/validate | 规则校验 |

### 快速示例

```bash
# 分析代码片段
curl -s http://127.0.0.1:8080/api/v1/analyze \
  -H 'Content-Type: application/json' \
  -d '{
    "code": "Statement stmt = conn.createStatement(); stmt.execute(query);",
    "language": "java",
    "rules": "rules:\n  - id: sqli\n    languages: [java]\n    patterns:\n      - pattern: \"$STMT.execute($QUERY)\"\n    severity: ERROR\n    message: \"Potential SQL injection\""
  }' | jq

# 分析文件（multipart 上传）
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@Example.java;filename=Example.java" \
  -F "language=java" \
  -F "rules=@rules.yaml"
```

### 支持的语言

`java`, `javascript`, `python`, `sql`, `bash`, `php`, `csharp`, `c`, `ruby`, `kotlin`, `swift`, `xml`

### 认证与 CORS

- 认证: 默认未启用（`enable_auth=false`），可通过配置开启 JWT
- CORS: 默认允许任意来源，允许方法 GET/POST/PUT/DELETE

> **完整的 REST API 文档（请求/响应结构、错误码、所有端点详情）请参阅 [API-Guide.md](API-Guide.md)。**

---

## 3. Rust 库集成 (Rust Library Integration)

astgrep 的核心能力以 Rust 库形式提供，可直接在其他 Rust 项目中使用。

### 添加依赖

```toml
[dependencies]
astgrep-core = { path = "../astgrep/crates/astgrep-core" }
# 或从 git
astgrep-core = { git = "https://github.com/c2j/astgrep.git" }
```

### 基本用法

```rust
use astgrep_core::{Language, AnalysisConfig};
use astgrep_parser::ParserRegistry;
use astgrep_rules::RuleEngine;

// 1. 创建配置
let config = AnalysisConfig::default();

// 2. 获取解析器
let registry = ParserRegistry::new();
let parser = registry.get_parser(Language::Java)?;

// 3. 解析源码
let source = r#"Statement stmt = conn.createStatement();"#;
let ast = parser.parse(source, std::path::Path::new("Example.java"))?;

// 4. 加载规则并执行
let rules = RuleEngine::from_yaml_file("rules.yaml")?;
let findings = rules.analyze(&ast, &config)?;

for finding in findings {
    println!("{}: {} at line {}", finding.rule_id, finding.message, finding.location.start_line);
}
```

### SQL 方言分析

```rust
use astgrep_core::{Language, SqlDialect};
use astgrep_parser::dialect;

let dialect = SqlDialect::GaussDB;
let dialect_parser = dialect::dispatch(dialect);
let ast = dialect_parser.parse(source, path)?;
```

---

## 4. CI/CD 集成 (CI/CD Integration)

### GitHub Actions

```yaml
name: Security Scan
on: [push, pull_request]
jobs:
  astgrep:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install astgrep
        run: |
          git clone https://github.com/c2j/astgrep.git
          cd astgrep && cargo build --release
          echo "$(pwd)/target/release" >> $GITHUB_PATH
      - name: Run analysis
        run: |
          astgrep analyze --rules security-rules/ \
            --format sarif --output results.sarif \
            --fail-on-findings src/
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

### GitLab CI

```yaml
astgrep_scan:
  image: rust:latest
  script:
    - git clone https://github.com/c2j/astgrep.git
    - cd astgrep && cargo build --release
    - ./target/release/astgrep analyze --rules rules/ --format json --output results.json src/
  artifacts:
    reports:
      dotenv: results.json
```

### Pre-commit Hook

```yaml
# .pre-commit-config.yaml
- repo: local
  hooks:
    - id: astgrep
      name: astgrep security scan
      entry: astgrep analyze --fail-on-findings
      language: system
      files: \.(java|js|py|sql|sh|xml)$
```

### Docker

```dockerfile
FROM rust:latest as builder
RUN git clone https://github.com/c2j/astgrep.git /app
WORKDIR /app
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/astgrep /usr/local/bin/
COPY --from=builder /app/target/release/astgrep-web-server /usr/local/bin/
ENTRYPOINT ["astgrep"]
```

---

## 5. MCP 集成规划 (MCP Integration — Roadmap)

> **⚠️ 注意：MCP (Model Context Protocol) 集成尚未实现。本节描述未来的设计规划，供感兴趣的开发者参考和讨论。**

### 背景

[MCP (Model Context Protocol)](https://modelcontextprotocol.io/) 是一种开放协议，用于让 AI 模型（如 Claude）与外部工具和数据源交互。将 astgrep 的静态分析能力暴露为 MCP 工具，可使 AI 编程助手直接调用代码安全扫描。

### 规划的 MCP 工具 (Planned MCP Tools)

| 工具名 | 参数 | 说明 |
|--------|------|------|
| `astgrep_analyze` | code, language, rules? | 分析代码片段，返回 findings |
| `astgrep_analyze_file` | path, language?, rules? | 分析本地文件 |
| `astgrep_validate_rules` | rules_yaml | 校验规则 YAML 语法 |
| `astgrep_list_rules` | language?, category? | 列出可用规则 |
| `astgrep_list_languages` | — | 列出支持的语言 |

### 设计考量

**传输方式**: stdio（本地）或 HTTP+SSE（远程）

**架构方案**:

```
MCP Client (Claude/IDE)
    ↕ (JSON-RPC)
MCP Server (new crate: astgrep-mcp)
    ↕
astgrep-core / astgrep-rules / astgrep-parser (复用现有库)
```

**实现路径**:

1. 新建 `crates/astgrep-mcp/` crate
2. 依赖 `rmcp` 或 `mcp-server` crate（Rust MCP SDK）
3. 将现有 REST API 的分析逻辑包装为 MCP 工具
4. 提供 stdio 传输（本地 IDE 集成）和 HTTP 传输（远程服务）
5. 在 Claude Desktop / VS Code MCP 配置中注册

**配置示例（未来）**:

```json
{
  "mcpServers": {
    "astgrep": {
      "command": "astgrep-mcp",
      "args": ["--stdio"]
    }
  }
}
```

### 当前替代方案

在 MCP 实现之前，可通过 REST API 实现类似集成:

- 使用 astgrep-web-server 提供 HTTP 端点
- 在 AI 工具中通过 HTTP 调用 `/api/v1/analyze`
- 参考 [API-Guide.md](API-Guide.md) 了解 REST API 详情

---

## 6. 最佳实践 (Best Practices)

### 规则管理

- 将自定义规则放在独立目录，与内置规则分离
- 使用 `astgrep validate` 在提交前校验规则
- 为每条规则编写 @rule/@expect/@desc 测试用例

### 性能优化

- 大型代码库: 使用 `--no-parallel false`（默认并行）和 `--threads 0`（自动检测）
- 精确过滤: 使用 `--language` 和 `--severity` 减少不必要的分析
- 规则优化: 避免过于宽泛的模式，使用 `pattern-not` 减少误报

### 安全集成

- SARIF 输出可与 GitHub Code Scanning、Azure DevOps 集成
- 使用 `--fail-on-findings` 在 CI 中阻断有问题的提交
- 定期更新规则库: `astgrep update`

### 错误处理

- API 返回 422 表示规则解析失败，先用 `/api/v1/rules/validate` 校验
- API 返回 400 表示不支持的语言，检查 language 参数
- CLI 退出码: 0=成功无发现，1=有发现（--fail-on-findings），2=错误

---

## 参考

- [REST API 完整文档](API-Guide.md)
- [规则编写指南](astgrep-Guide.md)
- [SQL 方言支持](sql-dialects.md)
- [用户指南](User-Guide.md)
- [贡献指南](CONTRIBUTING.md)
- [MCP 协议规范](https://modelcontextprotocol.io/)
