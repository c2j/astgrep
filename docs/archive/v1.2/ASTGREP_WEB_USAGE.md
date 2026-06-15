# astgrep-web 命令使用指南

## 概述

`astgrep-web-server` 是 astgrep 的 REST API 服务器，提供了基于 HTTP 的代码分析接口。

## 快速开始

### 查看帮助信息

```bash
# 显示完整的帮助信息
astgrep-web-server --help

# 显示版本信息
astgrep-web-server --version
```

### 启动服务器

```bash
# 使用默认配置启动
astgrep-web-server

# 使用自定义配置文件启动
astgrep-web-server --config /etc/astgrep/config.toml

# 启用详细日志
astgrep-web-server --verbose

# 覆盖配置文件中的绑定地址和端口
astgrep-web-server --bind 0.0.0.0 --port 9090

# 覆盖规则目录
astgrep-web-server --rules /opt/astgrep/rules
```

---

## 命令行选项

### `-c, --config <FILE>`

指定配置文件路径。

**默认值：** `astgrep-web.toml`

**示例：**
```bash
astgrep-web-server --config ./config/production.toml
astgrep-web-server -c /etc/astgrep/astgrep-web.toml
```

### `-b, --bind <ADDR>`

指定服务器绑定地址。

**格式：** `IP:PORT` 或仅 `IP`（使用默认端口）

**示例：**
```bash
# 仅在本地监听
astgrep-web-server --bind 127.0.0.1

# 在所有接口上监听
astgrep-web-server --bind 0.0.0.0

# 指定特定 IP 和端口
astgrep-web-server --bind 192.168.1.100:9090
```

### `-p, --port <PORT>`

指定服务器端口（1-65535）。

**默认值：** 8080

**示例：**
```bash
astgrep-web-server --port 9090
astgrep-web-server -p 3000
```

### `-r, --rules <DIR>`

指定规则目录路径。

**示例：**
```bash
astgrep-web-server --rules ./rules
astgrep-web-server -r /opt/astgrep/rules
```

### `-v, --verbose`

启用详细日志输出（调试级别）。

**示例：**
```bash
astgrep-web-server --verbose
astgrep-web-server -v
```

### `--generate-config`

生成默认配置文件并退出。

**示例：**
```bash
# 生成默认配置文件
astgrep-web-server --generate-config

# 生成到指定位置
astgrep-web-server --config ./my-config.toml --generate-config
```

---

## 配置文件

### 配置文件格式

配置文件使用 TOML 格式。详见 `examples/astgrep-web-config.toml`。

### 配置优先级（从高到低）

1. **命令行参数** - 最高优先级
2. **环境变量** - `ASTGREP_*` 前缀
3. **配置文件** - TOML 文件中的设置
4. **内置默认值** - 最低优先级

### 主要配置项

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `bind_address` | `127.0.0.1:8080` | 服务器绑定地址 |
| `max_upload_size` | `104857600` (100MB) | 最大上传文件大小 |
| `max_concurrent_jobs` | `10` | 最大并发分析任务数 |
| `rules_directory` | `rules` | 规则文件目录 |
| `temp_directory` | `/tmp/astgrep` | 临时文件目录 |
| `enable_auth` | `false` | 是否启用身份验证 |

---

## 使用示例

### 示例 1：开发环境

```bash
# 启动开发服务器，启用详细日志
astgrep-web-server --verbose --bind 127.0.0.1 --port 8080
```

**配置文件 (dev.toml)：**
```toml
bind_address = "127.0.0.1:8080"
max_concurrent_jobs = 5
rules_directory = "./rules"

[logging]
level = "debug"
log_requests = true
log_responses = true
```

### 示例 2：生产环境

```bash
# 启动生产服务器
astgrep-web-server --config /etc/astgrep/production.toml
```

**配置文件 (production.toml)：**
```toml
bind_address = "0.0.0.0:8080"
max_upload_size = 209715200  # 200MB
max_concurrent_jobs = 20
rules_directory = "/etc/astgrep/rules"
temp_directory = "/var/tmp/astgrep"
enable_auth = true
jwt_secret = "your-production-secret"

[logging]
level = "warn"
log_file = "/var/log/astgrep/server.log"

[rate_limit]
enabled = true
requests_per_minute = 100
burst_size = 20

[cors]
allowed_origins = ["https://example.com"]
```

### 示例 3：高性能配置

```bash
# 启动高性能服务器
astgrep-web-server --config /opt/astgrep/high-performance.toml
```

**配置文件 (high-performance.toml)：**
```toml
bind_address = "0.0.0.0:8080"
max_upload_size = 524288000  # 500MB
max_concurrent_jobs = 50
rules_directory = "/opt/astgrep/rules"
temp_directory = "/mnt/fast-storage/astgrep"

[request_timeout]
secs = 600  # 10 minutes

[rate_limit]
enabled = true
requests_per_minute = 200
burst_size = 50
```

---

## API 端点

### 健康检查

```bash
GET /api/v1/health
```

**示例：**
```bash
curl http://localhost:8080/api/v1/health
```

### 代码分析

```bash
POST /api/v1/analyze
Content-Type: application/json

{
  "code": "function example() { return eval(userInput); }",
  "language": "javascript",
  "rules": []
}
```

**示例：**
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
GET /api/v1/jobs/{id}
```

### 获取任务结果

```bash
GET /api/v1/jobs/{id}/result
```

### API 文档

```
GET /docs
```

在浏览器中访问 `http://localhost:8080/docs` 查看完整的 API 文档。

### 交互式游乐场

```
GET /playground
```

在浏览器中访问 `http://localhost:8080/playground` 使用交互式分析工具。

---

## 常见问题

### Q: 如何生成配置文件？

```bash
astgrep-web-server --generate-config
```

这会在当前目录生成 `astgrep-web.toml` 文件。

### Q: 如何在不同的端口上运行多个实例？

```bash
# 实例 1
astgrep-web-server --port 8080

# 实例 2
astgrep-web-server --port 8081

# 实例 3
astgrep-web-server --port 8082
```

### Q: 如何启用身份验证？

1. 生成 JWT 密钥：
```bash
openssl rand -base64 32
```

2. 在配置文件中设置：
```toml
enable_auth = true
jwt_secret = "your-generated-secret"
```

3. 重启服务器

### Q: 如何增加最大上传文件大小？

在配置文件中修改 `max_upload_size`：
```toml
# 500MB
max_upload_size = 524288000
```

### Q: 如何查看详细日志？

```bash
# 启用详细日志
astgrep-web-server --verbose

# 或在配置文件中设置
[logging]
level = "debug"
```

---

## 环境变量

支持以下环境变量（可选）：

- `ASTGREP_CONFIG` - 配置文件路径
- `ASTGREP_BIND` - 绑定地址
- `ASTGREP_PORT` - 端口号
- `ASTGREP_RULES` - 规则目录
- `ASTGREP_VERBOSE` - 启用详细日志

**示例：**
```bash
export ASTGREP_BIND=0.0.0.0
export ASTGREP_PORT=9090
astgrep-web-server
```

---

## 故障排除

### 端口已被占用

```bash
# 使用不同的端口
astgrep-web-server --port 9090

# 或查找占用端口的进程
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

---

## 更多信息

- 完整配置示例：`examples/astgrep-web-config.toml`
- API 文档：访问 `http://localhost:8080/docs`
- 项目主页：https://github.com/c2j/cr-semservice

