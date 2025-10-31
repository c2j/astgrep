# astgrep-web-server API 接入指南

本指南面向需要通过 HTTP/REST 集成 astgrep 分析能力的工程师，基于当前仓库中 astgrep-web-server 实现的真实接口撰写，涵盖端点说明、请求/响应结构、示例与最佳实践。

- 默认服务地址：`http://127.0.0.1:8080`
- API 前缀：`/api/v1`
- 在线文档与 Playground（便于调试）：`/docs`、`/docs/guide`、`/playground`（均为离线可用的内嵌页面）

## 快速开始

1) 启动服务

```bash
astgrep-web-server
# 或指定端口/绑定地址
astgrep-web-server --bind 0.0.0.0 --port 9090
# 生成默认配置文件（TOML）
astgrep-web-server --generate-config --config astgrep-web.toml
```

2) 健康检查

```bash
curl -s http://127.0.0.1:8080/api/v1/health | jq
```

3) 发送一次代码片段分析

```bash
curl -s http://127.0.0.1:8080/api/v1/analyze \
  -H 'Content-Type: application/json' \
  -d '{
        "code": "System.out.println(\"Hello\");",
        "language": "java"
      }' | jq
```

## 认证与跨域

- 认证：当前默认未启用认证（`enable_auth=false`），未来可通过配置开启 JWT；本指南仅描述“未启用认证”的默认行为。
- CORS：默认允许任意来源（Any），允许的方法：GET/POST/PUT/DELETE；允许的请求头：`Content-Type`、`Authorization`。

## 通用规范

- 请求/响应均为 JSON（除 `/api/v1/metrics` 为 Prometheus 文本格式）；需设置 `Content-Type: application/json`。
- 时间戳字段采用 ISO 8601（UTC）。
- 错误响应统一结构（状态码见下）：

```json
{
  "error": "BAD_REQUEST",
  "message": "Bad request: <detail>",
  "details": null,
  "request_id": null,
  "timestamp": "2025-01-01T12:00:00Z"
}
```

常见状态码：
- 200 OK：请求成功
- 400 Bad Request：参数/格式错误（如不支持的语言）
- 401 Unauthorized、403 Forbidden：当启用认证/授权且失败时
- 404 Not Found：资源不存在
- 422 Unprocessable Entity：分析/文件处理/规则解析失败
- 429 Too Many Requests：触发速率限制（如启用）
- 500/503：服务内部错误/不可用

## 语言支持

当前分析端点支持（严格按照后端实现）：
- `java`, `javascript`, `python`, `sql`, `bash`, `php`, `csharp`, `c`, `ruby`, `kotlin`, `swift`, `xml`

提示：若传入其它语言将返回 400（Unsupported language）。

## 端点总览

- 健康检查：GET `/api/v1/health`
- 版本信息：GET `/api/v1/version`
- Prometheus 指标：GET `/api/v1/metrics`
- 分析（代码片段）：POST `/api/v1/analyze`
- 分析（SARIF 输出）：POST `/api/v1/analyze/sarif`
- 分析（单文件 base64）：POST `/api/v1/analyze/file`
- 分析（压缩包 base64）：POST `/api/v1/analyze/archive`
- 任务列表/状态：GET `/api/v1/jobs`、GET `/api/v1/jobs/{id}`
- 规则列表/详情：GET `/api/v1/rules`、GET `/api/v1/rules/{id}`
- 规则校验：POST `/api/v1/rules/validate`

---

## 健康检查
GET /api/v1/health

响应（示例）：
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 12345,
  "system": {
    "available_memory_bytes": 1073741824,
    "cpu_usage_percent": 5.5,
    "disk_usage_percent": 50.0,
    "active_jobs": 0
  },
  "dependencies": {
    "rules_directory": {"healthy": true, "response_time_ms": 1, "error": null},
    "temp_directory": {"healthy": true, "response_time_ms": 2, "error": null}
  }
}
```

## 版本信息
GET /api/v1/version

响应（示例）：
```json
{
  "version": "0.1.0",
  "build_timestamp": "unknown",
  "git_commit": "unknown",
  "rust_version": "1.75.0",
  "features": ["json-output", "multi-language", "rest-api", "static-analysis", ...]
}
```

## 指标（Prometheus）
GET /api/v1/metrics

- Content-Type: `text/plain; version=0.0.4; charset=utf-8`
- 返回 Prometheus 兼容的指标文本，如：

```
astgrep_info{version="0.1.0"} 1
astgrep_uptime_seconds 42
astgrep_requests_total{method="POST",endpoint="/api/v1/analyze"} 3
...
```

---

## 代码分析（片段）
POST /api/v1/analyze

请求体：
```json
{
  "code": "string",
  "language": "java|javascript|python|sql|...",
  "rules": "<YAML 字符串>" | ["rule-id-1", "rule-id-2"],
  "options": {
    "min_severity": "info|warning|error|critical",
    "min_confidence": "low|medium|high",
    "max_findings": 200,
    "enable_dataflow": false,
    "enable_dataflow_analysis": false,
    "enable_security_analysis": false,
    "enable_performance_analysis": false,
    "include_metrics": false,
    "output_format": "json",
    "mode": "normal|pro|turbo",
    "sql_statement_boundary": true
  }
}
```
说明：
- `rules` 支持两种形态：
  - YAML 字符串（推荐，后端已实现加载解析）
  - 规则 ID 数组（当前“按 ID 加载规则”尚未实现，传该形态将回退到内置/默认规则）
- 若未提供 `rules`：服务会按语言尝试从 `rules_directory` 读取 `<language>.yaml` 与 `general.yaml`；若均不存在，则使用内置演示规则。

响应体（成功示例）：
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "results": {
    "findings": [
      {
        "rule_id": "java-system-out",
        "message": "Use proper logging instead of System.out",
        "severity": "warning",
        "confidence": "high",
        "location": {"file": "input", "start_line": 1, "start_column": 1, "end_line": 1, "end_column": 10},
        "fix": null,
        "metadata": null,
        "metavariable_bindings": null,
        "constraint_matches": null,
        "taint_flow": null
      }
    ],
    "summary": {
      "total_findings": 1,
      "findings_by_severity": {"warning": 1},
      "findings_by_confidence": {"high": 1},
      "files_analyzed": 1,
      "rules_executed": 1,
      "duration_ms": 12
    },
    "metrics": null,
    "dataflow_info": null
  },
  "error": null,
  "created_at": "2025-01-01T12:00:00Z",
  "completed_at": "2025-01-01T12:00:00Z"
}
```

cURL 示例（带 YAML 规则）：
```bash
curl -s http://127.0.0.1:8080/api/v1/analyze \
  -H 'Content-Type: application/json' \
  -d '{
    "code": "System.out.println(\"Hi\");",
    "language": "java",
    "rules": "rules:\n  - id: java-system-out\n    message: \"Use proper logging instead of System.out\"\n    severity: warning\n    languages: [java]\n    patterns:\n      - \"System.out.println\""
  }'
```

## 代码分析（SARIF 输出）
POST /api/v1/analyze/sarif

- 请求体与 `/api/v1/analyze` 相同
- 响应为 SARIF JSON（2.1.0）简化结构，例如：
```json
{
  "version": "2.1.0",
  "runs": [
    {
      "tool": {"driver": {"name": "astgrep", "version": "0.1.0", "information_uri": "https://github.com/c2j/astgrep"}},
      "results": [
        {
          "rule_id": "java-system-out",
          "message": {"text": "Use proper logging instead of System.out"},
          "locations": [
            {"physical_location": {"artifact_location": {"uri": "input"}, "region": {"start_line": 1}}}
          ],
          "level": "warning"
        }
      ]
    }
  ]
}
```

## 单文件分析（base64 内容）
POST /api/v1/analyze/file

请求体：
```json
{
  "filename": "Example.java",
  "content": "<base64>",
  "language": "java",
  "rules": "<YAML 字符串>",
  "options": { /* 同上 */ }
}
```
说明：
- `content` 必须为 base64 编码；若未提供 `language`，服务会根据扩展名做简易识别。


**cURL 示例：**
```bash
# 使用本地文件 Example.java，编码为 base64 并发起请求（跨平台：用 tr 去掉换行）
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -H 'Content-Type: application/json' \
  -d @- <<EOF
{
  "filename": "Example.java",
  "language": "java",
  "content": "$(base64 < Example.java | tr -d '\n')",
  "rules": "rules:\n  - id: java-system-out\n    message: \"Use proper logging instead of System.out\"\n    severity: warning\n    languages: [java]\n    patterns:\n      - \"System.out.println\""
}
EOF
```
说明：
- macOS 也可用 `base64 -b 0 Example.java` 生成不换行的 base64；Linux 可用 `base64 -w 0 Example.java`。


### 多文件/附件方式（multipart/form-data）

- 支持以附件方式上传 1 个或多个源码文件：使用多次 `-F "file=@..."`
- 可选字段：
  - `language`：全局语言（未提供时按各文件扩展名自动识别）
  - `rules`：规则，支持两种形态
    - 文本字段（YAML 字符串）
    - 规则文件上传（推荐）：`-F "rules=@rules.yaml"`
  - `rules_file`：规则文件的别名字段。若需要传多个规则文件，请重复此字段；后端会按出现顺序拼接。
  - `options`：文本字段，内容为 JSON 字符串（例如 `{"include_metrics":true}`）

示例一：单文件 + 规则文件（推荐）
```bash
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@Example.java;filename=Example.java" \
  -F "language=java" \
  -F "rules=@rules.yaml"
```

示例二：多文件 + 规则文件
```bash
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@a.java;filename=a.java" \
  -F "file=@b.java;filename=b.java" \
  -F "language=java" \
  -F "rules=@rules.yaml"
```

示例三：规则作为“文本字段”（YAML 字符串）
（使用 Bash 的 $'...' 字面量，便于输入换行与双引号）
```bash
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@Example.java;filename=Example.java" \
  -F "language=java" \
  -F $'rules=rules:\n  - id: java-system-out\n    message: "Use proper logging instead of System.out"\n    severity: warning\n    languages: [java]\n    patterns:\n      - "System.out.println"'
```

示例四：多个规则文件（按顺序拼接）
```bash
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@Example.java;filename=Example.java" \
  -F "rules_file=@base.yaml" \
  -F "rules_file=@project_overrides.yaml"
```

提示：
- 规则文件需为 UTF-8 文本（YAML）。
- 当同时提供“文本 rules”和“规则文件”时，服务优先使用“文本 rules”。


### 在返回结果中保留相对路径（避免同名文件混淆）

默认情况下，curl 在 multipart 上传时会仅携带文件名（basename）。服务端会将收到的 `filename` 原样用于返回 JSON 中的 `location.file`。若希望在结果里保留“相对路径/目录名”，可在 `-F "file=@..."` 后追加 `;filename=<相对路径>`：

```bash
curl -s http://127.0.0.1:8080/api/v1/analyze/file \
  -F 'file=@test/sql-1/java_sample/Sample.java;filename=test/sql-1/java_sample/Sample.java' \
  -F 'file=@test/sql-1/mybatis_sample/selects.xml;filename=test/sql-1/mybatis_sample/selects.xml' \
  -F 'rules_file=@test/sql-1/sql_rules_either_sc.yaml'
```

说明：
- 若不指定 `;filename=...`，返回会仅显示 basename（如 `Sample.java`）。
- 指定后端将不裁剪该值，`location.file` 中会包含你传入的相对路径（便于区分不同目录下的同名文件）。
- 语言识别仍按扩展名进行，不受是否包含目录的影响。

## 压缩包分析（base64 内容）
POST /api/v1/analyze/archive

请求体：
```json
{
  "archive": "<base64>",
  "format": "zip|tar|tar.gz",
  "languages": ["java", "javascript"],
  "rules": "<YAML 字符串>",
  "include_patterns": ["src/**"],
  "exclude_patterns": ["**/test/**"],
  "options": { /* 同上 */ }
}
```
说明：当前实现对归档的解包/匹配为精简示例版，主要用于演示 API 形态。

---

## 任务管理

- 列表：GET `/api/v1/jobs?status=<pending|queued|running|completed|failed|cancelled>&limit=50&offset=0`
  - 响应为分页结构：
```json
{
  "data": [ {"id":"...","status":"completed", "job_type":"code_analysis", ...} ],
  "pagination": {"page":1,"per_page":50,"total":4,"total_pages":1,"has_next":false,"has_prev":false},
  "meta": {"request_id":null,"timestamp":"...","version":"v1"}
}
```
- 详情：GET `/api/v1/jobs/{id}` → 返回单个 `Job` 对象

## 规则管理

- 列表：GET `/api/v1/rules?language=java&category=security&enabled=true&limit=100&offset=0`
  - 响应：`RuleInfo[]`（JSON 数组）
- 详情：GET `/api/v1/rules/{id}` → 单个 `RuleInfo`
- 校验：POST `/api/v1/rules/validate`
  - 请求：
```json
{ "rules": "<YAML 字符串>", "language": "java", "check_performance": true }
```
  - 响应：
```json
{ "valid": true, "errors": [], "warnings": ["..."], "rules_count": 1, "performance": {"load_time_ms": 3, "average_complexity": 1.2, "memory_usage_bytes": 4096} }
```

---

## 最佳实践与限制

- 上传大小：默认最大 `max_upload_size = 100MB`（可通过配置修改）。base64 会使体积增大约 33%。
- 语言选择：请使用上文列出的已支持值；不支持的语言将返回 400。
- 规则传递：建议使用 YAML 字符串形态；“按规则 ID 加载”暂未实现。
- 性能选项：`options.include_metrics=true` 可返回粗粒度的性能数据；其它 `enable_*_analysis` 选项为增量能力示例，默认关闭。
- 并发与速率限制：可通过配置控制最大并发与限流，若开启限流可能返回 429。

## 常见排错

- 400 Unsupported language：请检查 `language` 值是否在支持列表中。
- 422 Invalid YAML rules：请先调用 `/api/v1/rules/validate` 进行校验。
- 413 Request too large：超过 `max_upload_size`。
- 5xx：查看服务日志（启动时可加 `--verbose`），或检查 `rules_directory` / `temp_directory` 权限。

## 参考

- API 文档页：`/docs`
- 规则指南（离线内嵌渲染）：`/docs/guide`
- Playground（离线）：`/playground`（右侧 Docs 页签同样内嵌了 `docs/astgrep-Guide.md`）

