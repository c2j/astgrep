# astgrep-web 命令 --help 选项实现总结

## 📋 概述

为 `astgrep-web-server` 命令增加了详细的 `--help` 选项支持，并提供了完整的配置文件示例和使用文档。

---

## ✅ 实现内容

### 1. 增强命令行帮助信息

**文件：** `crates/astgrep-web/src/bin/cr-web-server.rs`

#### 改进内容：

- ✅ 添加详细的 `long_about` 说明
- ✅ 包含使用示例
- ✅ 列出所有 API 端点
- ✅ 每个选项都有详细的帮助文本
- ✅ 支持 `--help` 和 `-h` 查看帮助
- ✅ 支持 `--version` 查看版本

#### 命令行选项：

```bash
-c, --config <FILE>      # 配置文件路径 (默认: astgrep-web.toml)
-b, --bind <ADDR>        # 绑定地址 (例: 127.0.0.1, 0.0.0.0)
-p, --port <PORT>        # 端口号 (1-65535)
-r, --rules <DIR>        # 规则目录
-v, --verbose            # 启用详细日志
--generate-config        # 生成默认配置文件
-h, --help              # 显示帮助信息
-V, --version           # 显示版本信息
```

---

### 2. 配置文件示例

**文件：** `examples/astgrep-web-config.toml`

#### 包含内容：

- ✅ 完整的配置项说明
- ✅ 默认值和范围
- ✅ 三种配置示例：
  - 开发环境配置
  - 生产环境配置
  - 高性能配置
- ✅ 详细的注释和说明

#### 主要配置项：

```toml
# 服务器配置
bind_address = "127.0.0.1:8080"
max_upload_size = 104857600  # 100MB
max_concurrent_jobs = 10
rules_directory = "rules"
temp_directory = "/tmp/astgrep"

# 超时配置
[request_timeout]
secs = 300

# 速率限制
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10

# CORS 配置
[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]

# 日志配置
[logging]
level = "info"
log_requests = true
log_responses = false
```

---

### 3. 使用指南

**文件：** `docs/ASTGREP_WEB_USAGE.md`

#### 包含内容：

- ✅ 快速开始指南
- ✅ 所有命令行选项详解
- ✅ 配置文件说明
- ✅ 使用示例
- ✅ API 端点文档
- ✅ 常见问题解答
- ✅ 故障排除指南
- ✅ 环境变量支持

#### 快速命令：

```bash
# 查看帮助
astgrep-web-server --help

# 启动服务器
astgrep-web-server

# 使用自定义配置
astgrep-web-server --config /etc/astgrep/config.toml

# 指定绑定地址和端口
astgrep-web-server --bind 0.0.0.0 --port 9090

# 启用详细日志
astgrep-web-server --verbose

# 生成配置文件
astgrep-web-server --generate-config
```

---

### 4. 帮助输出文档

**文件：** `docs/ASTGREP_WEB_HELP_OUTPUT.md`

#### 包含内容：

- ✅ 完整的 `--help` 输出示例
- ✅ 快速参考
- ✅ 选项详解
- ✅ 常见用法
- ✅ 配置优先级说明

---

## 🎯 主要特性

### 命令行支持

```bash
# 完整帮助
astgrep-web-server --help

# 简短帮助
astgrep-web-server -h

# 版本信息
astgrep-web-server --version

# 生成配置文件
astgrep-web-server --generate-config
```

### 配置优先级

命令行参数 > 环境变量 > 配置文件 > 内置默认值

### 使用示例

#### 开发环境

```bash
astgrep-web-server --verbose --bind 127.0.0.1 --port 8080
```

#### 生产环境

```bash
astgrep-web-server --config /etc/astgrep/production.toml
```

#### 高性能配置

```bash
astgrep-web-server \
  --config /opt/astgrep/high-performance.toml \
  --bind 0.0.0.0 \
  --port 8080
```

#### 多实例部署

```bash
astgrep-web-server --port 8080 &
astgrep-web-server --port 8081 &
astgrep-web-server --port 8082 &
```

---

## 📚 文档结构

```
docs/
├── ASTGREP_WEB_USAGE.md          # 完整使用指南
├── ASTGREP_WEB_HELP_OUTPUT.md    # --help 输出示例
└── ASTGREP_WEB_HELP_SUMMARY.md   # 本文档

examples/
└── astgrep-web-config.toml       # 配置文件示例

crates/astgrep-web/src/bin/
└── cr-web-server.rs              # 命令行实现
```

---

## 🔧 API 端点

### 健康检查

```bash
GET /api/v1/health
```

### 代码分析

```bash
POST /api/v1/analyze
Content-Type: application/json

{
  "code": "...",
  "language": "java",
  "rules": []
}
```

### 任务管理

```bash
GET /api/v1/jobs/{id}           # 获取任务状态
GET /api/v1/jobs/{id}/result    # 获取任务结果
```

### 文档和工具

```bash
GET /docs                        # API 文档
GET /playground                  # 交互式游乐场
```

---

## 💡 常见用法

### 查看帮助

```bash
# 完整帮助
astgrep-web-server --help

# 简短帮助
astgrep-web-server -h
```

### 生成配置文件

```bash
# 生成到默认位置
astgrep-web-server --generate-config

# 生成到指定位置
astgrep-web-server --config ./my-config.toml --generate-config
```

### 启用身份验证

```bash
# 生成 JWT 密钥
openssl rand -base64 32

# 在配置文件中设置
enable_auth = true
jwt_secret = "your-generated-secret"
```

### 增加上传文件大小

```toml
# 500MB
max_upload_size = 524288000
```

### 启用详细日志

```bash
astgrep-web-server --verbose

# 或在配置文件中
[logging]
level = "debug"
```

---

## 🚀 快速开始

### 1. 查看帮助

```bash
astgrep-web-server --help
```

### 2. 生成配置文件

```bash
astgrep-web-server --generate-config
```

### 3. 启动服务器

```bash
astgrep-web-server
```

### 4. 访问 API

```bash
# 健康检查
curl http://localhost:8080/api/v1/health

# API 文档
open http://localhost:8080/docs

# 交互式游乐场
open http://localhost:8080/playground
```

---

## 📖 相关文档

- [完整使用指南](./ASTGREP_WEB_USAGE.md)
- [帮助输出示例](./ASTGREP_WEB_HELP_OUTPUT.md)
- [配置文件示例](../examples/astgrep-web-config.toml)
- [项目主页](https://github.com/c2j/cr-semservice)

---

## ✨ 总结

✅ 为 `astgrep-web-server` 命令增加了完整的 `--help` 选项支持

✅ 提供了详细的配置文件示例和使用文档

✅ 支持多种使用场景：开发、生产、高性能

✅ 包含完整的 API 文档和故障排除指南

✅ 所有文档都已提交到 Git 仓库

