# 如何运行 CR-Web

**项目**: CR-SemService Web 服务  
**语言**: Rust  
**框架**: Axum (异步 Web 框架)

---

## 🚀 快速开始

### 1. 构建 CR-Web

```bash
# 从项目根目录运行
cargo build -p cr-web --bin cr-web
```

**预期输出**:
```
Compiling cr-web v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in X.XXs
```

### 2. 创建规则目录

```bash
# 创建规则目录（如果不存在）
mkdir -p rules
```

### 3. 运行服务

```bash
# 方式 1: 使用 cargo run
cargo run -p cr-web --bin cr-web

# 方式 2: 直接运行编译后的二进制文件
./target/debug/cr-web
```

**预期输出**:
```
2025-10-18T13:00:00.000Z INFO  cr_web: Starting CR Web Service
2025-10-18T13:00:00.000Z INFO  cr_web: Configuration: WebConfig { ... }
2025-10-18T13:00:00.000Z INFO  cr_web: Server listening on 127.0.0.1:8080
2025-10-18T13:00:00.000Z INFO  cr_web: API documentation available at http://127.0.0.1:8080/docs
2025-10-18T13:00:00.000Z INFO  cr_web: Health check available at http://127.0.0.1:8080/health
```

---

## 🌐 访问服务

### 健康检查

```bash
curl http://127.0.0.1:8080/health
```

**响应**:
```json
{
  "status": "healthy",
  "timestamp": "2025-10-18T13:00:00Z"
}
```

### API 文档

在浏览器中打开:
```
http://127.0.0.1:8080/docs
```

### 根路由

```bash
curl http://127.0.0.1:8080/
```

---

## 📝 API 使用示例

### 1. 分析代码片段

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "function unsafe(input) { return eval(input); }",
    "language": "javascript"
  }'
```

**响应**:
```json
{
  "findings": [
    {
      "id": "JS-001",
      "rule_id": "eval-usage",
      "message": "Use of eval() is dangerous",
      "severity": "high",
      "confidence": "high",
      "location": {
        "file": "input",
        "line": 1,
        "column": 40
      }
    }
  ],
  "summary": {
    "total_findings": 1,
    "findings_by_severity": {
      "high": 1
    }
  }
}
```

### 2. 分析 SARIF 格式

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze/sarif \
  -H "Content-Type: application/json" \
  -d '{
    "code": "eval(x)",
    "language": "javascript"
  }'
```

**响应**: SARIF 2.1.0 格式结果

### 3. 上传文件分析

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@example.js" \
  -F "language=javascript"
```

### 4. 列出规则

```bash
curl http://127.0.0.1:8080/api/v1/rules
```

### 5. 获取特定规则

```bash
curl http://127.0.0.1:8080/api/v1/rules/eval-usage
```

### 6. 验证规则

```bash
curl -X POST http://127.0.0.1:8080/api/v1/rules/validate \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-rule",
    "name": "Test Rule",
    "pattern": "eval",
    "severity": "high"
  }'
```

### 7. 获取任务状态

```bash
curl http://127.0.0.1:8080/api/v1/jobs/{job_id}
```

---

## ⚙️ 配置

### 环境变量

```bash
# 服务绑定地址 (默认: 127.0.0.1:8080)
export BIND_ADDRESS=0.0.0.0:8080

# 规则目录 (默认: rules)
export RULES_DIRECTORY=/path/to/rules

# 最大上传大小 (默认: 100MB)
export MAX_UPLOAD_SIZE=104857600

# 请求超时 (默认: 300 秒)
export REQUEST_TIMEOUT=300

# 日志级别 (默认: info)
export RUST_LOG=debug

# 运行服务
cargo run -p cr-web --bin cr-web
```

### 配置文件

在项目根目录创建 `cr-web.toml`:

```toml
[server]
bind_address = "0.0.0.0:8080"
request_timeout = 300

[analysis]
rules_directory = "rules"
max_upload_size = 104857600
enable_dataflow_analysis = true

[logging]
level = "info"
format = "json"
```

---

## 🧪 测试

### 运行单元测试

```bash
cargo test -p cr-web --lib
```

### 运行集成测试

```bash
cargo test -p cr-web
```

### 运行特定测试

```bash
cargo test -p cr-web analyze
```

---

## 🏗️ 生产构建

### 构建优化版本

```bash
cargo build --release -p cr-web --bin cr-web
```

**输出**: `target/release/cr-web`

### 运行生产版本

```bash
./target/release/cr-web
```

---

## 🐳 Docker 运行 (可选)

### 创建 Dockerfile

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p cr-web --bin cr-web

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/cr-web /usr/local/bin/
COPY --from=builder /app/crates/cr-web/rules /app/rules
WORKDIR /app
EXPOSE 8080
CMD ["cr-web"]
```

### 构建 Docker 镜像

```bash
docker build -t cr-web:latest .
```

### 运行 Docker 容器

```bash
docker run -p 8080:8080 \
  -v $(pwd)/rules:/app/rules \
  cr-web:latest
```

---

## 🔧 故障排除

### 问题 1: 规则目录不存在

**错误**:
```
Configuration validation failed: Rules directory does not exist
```

**解决**:
```bash
mkdir -p rules
```

### 问题 2: 端口已被占用

**错误**:
```
Error: bind failed: Address already in use
```

**解决**:
```bash
# 方式 1: 使用不同的端口
export BIND_ADDRESS=127.0.0.1:8081
cargo run -p cr-web --bin cr-web

# 方式 2: 杀死占用端口的进程
lsof -i :8080
kill -9 <PID>
```

### 问题 3: 编译错误

**解决**:
```bash
# 清理构建缓存
cargo clean

# 重新构建
cargo build -p cr-web --bin cr-web
```

### 问题 4: 日志输出不显示

**解决**:
```bash
# 设置日志级别
export RUST_LOG=debug
cargo run -p cr-web --bin cr-web
```

---

## 📊 性能优化

### 1. 使用发布版本

```bash
cargo build --release -p cr-web --bin cr-web
./target/release/cr-web
```

### 2. 增加工作线程

```bash
export TOKIO_WORKER_THREADS=8
cargo run -p cr-web --bin cr-web
```

### 3. 启用规则缓存

```bash
export ENABLE_RULE_CACHE=true
cargo run -p cr-web --bin cr-web
```

---

## 📈 监控

### 健康检查端点

```bash
curl http://127.0.0.1:8080/health
```

### Prometheus 指标

```bash
curl http://127.0.0.1:8080/metrics
```

### 日志查看

```bash
# 实时日志
cargo run -p cr-web --bin cr-web 2>&1 | grep -i "error\|warn"

# 保存日志到文件
cargo run -p cr-web --bin cr-web > cr-web.log 2>&1 &
```

---

## 🎯 常见任务

### 分析 Python 代码

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "import pickle\ndata = pickle.loads(user_input)",
    "language": "python"
  }'
```

### 分析 Java 代码

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "String query = \"SELECT * FROM users WHERE id = \" + userId;",
    "language": "java"
  }'
```

### 分析 SQL 代码

```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "SELECT * FROM users WHERE id = \" + userId + \"",
    "language": "sql"
  }'
```

---

## 📚 相关文档

- **README.md** - 项目概述
- **API 文档** - http://127.0.0.1:8080/docs
- **配置指南** - 见上面的配置部分

---

## ✨ 总结

**快速启动**:
```bash
# 1. 构建
cargo build -p cr-web --bin cr-web

# 2. 创建规则目录
mkdir -p rules

# 3. 运行
cargo run -p cr-web --bin cr-web

# 4. 访问
curl http://127.0.0.1:8080/health
```

**服务地址**: http://127.0.0.1:8080  
**API 文档**: http://127.0.0.1:8080/docs  
**健康检查**: http://127.0.0.1:8080/health

---

**准备好了吗？** 按照上述步骤运行 CR-Web 服务！

