# astgrep-web 快速参考卡片

## 🚀 快速开始

```bash
# 查看帮助
astgrep-web-server --help

# 启动服务器（默认配置）
astgrep-web-server

# 启动服务器（自定义配置）
astgrep-web-server --config /etc/astgrep/config.toml

# 生成配置文件
astgrep-web-server --generate-config
```

---

## 📋 命令行选项

| 选项 | 说明 | 示例 |
|------|------|------|
| `-c, --config` | 配置文件路径 | `--config config.toml` |
| `-b, --bind` | 绑定地址 | `--bind 0.0.0.0` |
| `-p, --port` | 端口号 | `--port 9090` |
| `-r, --rules` | 规则目录 | `--rules ./rules` |
| `-v, --verbose` | 详细日志 | `--verbose` |
| `--generate-config` | 生成配置文件 | `--generate-config` |
| `-h, --help` | 显示帮助 | `--help` |
| `-V, --version` | 显示版本 | `--version` |

---

## 🔧 常用命令

### 开发环境

```bash
# 启用详细日志，监听本地
astgrep-web-server --verbose --bind 127.0.0.1 --port 8080
```

### 生产环境

```bash
# 使用生产配置文件
astgrep-web-server --config /etc/astgrep/production.toml
```

### 高性能配置

```bash
# 监听所有接口，增加并发数
astgrep-web-server --bind 0.0.0.0 --port 8080
```

### 多实例部署

```bash
# 实例 1
astgrep-web-server --port 8080 &

# 实例 2
astgrep-web-server --port 8081 &

# 实例 3
astgrep-web-server --port 8082 &
```

---

## ⚙️ 配置文件示例

### 最小配置

```toml
bind_address = "127.0.0.1:8080"
rules_directory = "rules"
```

### 完整配置

```toml
bind_address = "0.0.0.0:8080"
max_upload_size = 104857600
max_concurrent_jobs = 10
rules_directory = "rules"
temp_directory = "/tmp/astgrep"
enable_auth = false

[request_timeout]
secs = 300

[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10

[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]

[logging]
level = "info"
log_requests = true
log_responses = false
```

---

## 🌐 API 端点

| 方法 | 端点 | 说明 |
|------|------|------|
| GET | `/api/v1/health` | 健康检查 |
| POST | `/api/v1/analyze` | 分析代码 |
| GET | `/api/v1/jobs/{id}` | 获取任务状态 |
| GET | `/api/v1/jobs/{id}/result` | 获取任务结果 |
| GET | `/docs` | API 文档 |
| GET | `/playground` | 交互式游乐场 |

---

## 📝 API 使用示例

### 健康检查

```bash
curl http://localhost:8080/api/v1/health
```

### 分析代码

```bash
curl -X POST http://localhost:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "SELECT * FROM users WHERE id = 1 OR 1=1",
    "language": "sql",
    "rules": []
  }'
```

### 获取任务状态

```bash
curl http://localhost:8080/api/v1/jobs/job-id-123
```

### 获取任务结果

```bash
curl http://localhost:8080/api/v1/jobs/job-id-123/result
```

---

## 🔐 身份验证配置

### 生成 JWT 密钥

```bash
openssl rand -base64 32
```

### 配置文件设置

```toml
enable_auth = true
jwt_secret = "your-generated-secret-key"
```

---

## 📊 配置优先级

```
命令行参数 > 环境变量 > 配置文件 > 内置默认值
```

### 示例

```bash
# 配置文件中: bind_address = "127.0.0.1:8080"
# 命令行参数会覆盖配置文件
astgrep-web-server --config config.toml --bind 0.0.0.0 --port 9090
# 结果: 服务器绑定到 0.0.0.0:9090
```

---

## 🐛 故障排除

### 端口已被占用

```bash
# 使用不同的端口
astgrep-web-server --port 9090

# 查找占用端口的进程
lsof -i :8080
```

### 规则目录不存在

```bash
# 创建规则目录
mkdir -p rules

# 或指定现有目录
astgrep-web-server --rules /path/to/existing/rules
```

### 权限错误

```bash
# 确保有权限访问配置文件和规则目录
chmod 755 rules
chmod 644 astgrep-web.toml
```

### 查看详细日志

```bash
# 启用详细日志
astgrep-web-server --verbose

# 或在配置文件中
[logging]
level = "debug"
```

---

## 📚 默认值

| 配置项 | 默认值 |
|--------|--------|
| `bind_address` | `127.0.0.1:8080` |
| `max_upload_size` | `104857600` (100MB) |
| `max_concurrent_jobs` | `10` |
| `rules_directory` | `rules` |
| `temp_directory` | `/tmp/astgrep` |
| `request_timeout` | `300` 秒 |
| `enable_auth` | `false` |
| `log_level` | `info` |

---

## 🔗 相关文档

- [完整使用指南](./ASTGREP_WEB_USAGE.md)
- [帮助输出示例](./ASTGREP_WEB_HELP_OUTPUT.md)
- [实现总结](./ASTGREP_WEB_HELP_SUMMARY.md)
- [配置文件示例](../examples/astgrep-web-config.toml)

---

## 💡 提示

- 使用 `--generate-config` 生成配置文件模板
- 命令行参数可以覆盖配置文件设置
- 使用 `--verbose` 启用详细日志进行调试
- 访问 `/docs` 查看完整的 API 文档
- 访问 `/playground` 使用交互式分析工具

---

## 🎯 常见场景

### 场景 1：本地开发

```bash
astgrep-web-server --verbose --bind 127.0.0.1 --port 8080
```

### 场景 2：生产部署

```bash
astgrep-web-server --config /etc/astgrep/production.toml
```

### 场景 3：Docker 容器

```bash
astgrep-web-server --bind 0.0.0.0 --port 8080
```

### 场景 4：负载均衡

```bash
# 启动多个实例
for port in 8080 8081 8082; do
  astgrep-web-server --port $port &
done
```

### 场景 5：自定义规则

```bash
astgrep-web-server --rules /opt/custom-rules
```

---

## 📞 获取帮助

```bash
# 查看完整帮助
astgrep-web-server --help

# 查看简短帮助
astgrep-web-server -h

# 查看版本
astgrep-web-server --version

# 访问 API 文档
open http://localhost:8080/docs
```

