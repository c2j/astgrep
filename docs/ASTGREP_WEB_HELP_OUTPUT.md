# astgrep-web-server --help 输出

## 完整帮助信息

```
astgrep Web Server provides a REST API for static code analysis.

USAGE:
    astgrep-web-server [OPTIONS]

EXAMPLES:
    # Start with default configuration
    astgrep-web-server

    # Start with custom configuration file
    astgrep-web-server --config /etc/astgrep/config.toml

    # Override bind address and port
    astgrep-web-server --bind 0.0.0.0 --port 9090

    # Generate default configuration file
    astgrep-web-server --generate-config

    # Enable verbose logging
    astgrep-web-server --verbose

CONFIGURATION:
    Configuration can be provided via TOML file or command-line arguments.
    Command-line arguments override configuration file settings.

ENDPOINTS:
    - GET  /api/v1/health              - Health check
    - POST /api/v1/analyze             - Analyze code
    - GET  /api/v1/jobs/{id}           - Get job status
    - GET  /api/v1/jobs/{id}/result    - Get job result
    - GET  /docs                        - API documentation
    - GET  /playground                 - Interactive playground

OPTIONS:
  -c, --config <FILE>
          Path to TOML configuration file
          
          [default: astgrep-web.toml]

  -b, --bind <ADDR>
          Server bind address (overrides config file)
          
          Examples: 127.0.0.1, 0.0.0.0, 192.168.1.100

  -p, --port <PORT>
          Server port (overrides config file)
          
          Range: 1-65535
          [default: 8080]

  -r, --rules <DIR>
          Directory containing analysis rules (overrides config file)

  -v, --verbose
          Enable verbose logging output

  --generate-config
          Generate default configuration file at specified path

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

---

## 快速参考

### 查看帮助

```bash
# 完整帮助信息
astgrep-web-server --help

# 简短帮助信息
astgrep-web-server -h

# 版本信息
astgrep-web-server --version
```

### 常用命令

```bash
# 使用默认配置启动
astgrep-web-server

# 使用自定义配置文件
astgrep-web-server --config /etc/astgrep/config.toml

# 指定绑定地址和端口
astgrep-web-server --bind 0.0.0.0 --port 9090

# 启用详细日志
astgrep-web-server --verbose

# 生成默认配置文件
astgrep-web-server --generate-config

# 组合使用多个选项
astgrep-web-server \
  --config ./config.toml \
  --bind 0.0.0.0 \
  --port 9090 \
  --rules ./rules \
  --verbose
```

---

## 选项详解

### `-c, --config <FILE>`

**说明：** 指定 TOML 配置文件路径

**默认值：** `astgrep-web.toml`

**示例：**
```bash
astgrep-web-server --config /etc/astgrep/production.toml
astgrep-web-server -c ./config/dev.toml
```

### `-b, --bind <ADDR>`

**说明：** 指定服务器绑定地址

**格式：** `IP:PORT` 或仅 `IP`

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

**说明：** 指定服务器端口

**范围：** 1-65535

**默认值：** 8080

**示例：**
```bash
astgrep-web-server --port 9090
astgrep-web-server -p 3000
```

### `-r, --rules <DIR>`

**说明：** 指定规则文件目录

**示例：**
```bash
astgrep-web-server --rules ./rules
astgrep-web-server -r /opt/astgrep/rules
```

### `-v, --verbose`

**说明：** 启用详细日志输出（调试级别）

**示例：**
```bash
astgrep-web-server --verbose
astgrep-web-server -v
```

### `--generate-config`

**说明：** 生成默认配置文件并退出

**示例：**
```bash
# 生成到默认位置
astgrep-web-server --generate-config

# 生成到指定位置
astgrep-web-server --config ./my-config.toml --generate-config
```

---

## 配置优先级

命令行参数 > 环境变量 > 配置文件 > 内置默认值

**示例：**
```bash
# 配置文件中设置 bind_address = "127.0.0.1:8080"
# 命令行参数会覆盖配置文件
astgrep-web-server --config config.toml --bind 0.0.0.0 --port 9090
# 结果：服务器绑定到 0.0.0.0:9090
```

---

## 常见用法

### 开发环境

```bash
astgrep-web-server --verbose --bind 127.0.0.1 --port 8080
```

### 生产环境

```bash
astgrep-web-server --config /etc/astgrep/production.toml
```

### 高性能配置

```bash
astgrep-web-server \
  --config /opt/astgrep/high-performance.toml \
  --bind 0.0.0.0 \
  --port 8080
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

## 相关文档

- [完整使用指南](./ASTGREP_WEB_USAGE.md)
- [配置文件示例](../examples/astgrep-web-config.toml)
- [API 文档](./API.md)

