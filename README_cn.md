# astgrep

一个高性能、多语言的静态代码分析工具，专注于安全漏洞和代码质量检测，使用 Rust 实现。

## 特性

- **多语言支持**: Java、JavaScript、Python、SQL、Bash、PHP、C、C#、Ruby、Kotlin、Swift
- **安全导向**: 检测注入漏洞、XSS、身份验证问题等安全问题
- **高性能**: 使用 Rust 构建，速度快且内存安全
- **灵活的规则**: 基于 YAML 的声明式规则定义
- **多种输出格式**: JSON、YAML、SARIF、文本、XML
- **并行处理**: 多线程分析，适用于大型代码库
- **可扩展**: 模块化架构，易于添加新语言和规则
- **污点分析**: 高级数据流和污点分析能力
- **图形界面**: 提供 GUI 和 Web 界面

## 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/c2j/astgrep.git
cd astgrep

# 构建项目
cargo build --release

# 安装二进制文件
cargo install --path .
```

### 基本用法

```bash
# 分析当前目录
astgrep analyze

# 分析特定文件/目录
astgrep analyze src/ tests/

# 使用特定规则
astgrep analyze --rules security-rules.yml

# 指定语言
astgrep analyze --language java --language python

# 输出到 SARIF 格式文件
astgrep analyze --format sarif --output results.sarif

# 验证规则文件
astgrep validate rules/*.yml

# 列出支持的语言
astgrep languages

# 初始化配置文件
astgrep init --output astgrep.toml

# 查看语言信息
astgrep info --language java
```

## 可用工具

astgrep 提供了多个工具来满足不同的使用场景：

### 1. 主程序 (astgrep)
主要的命令行工具，提供完整的静态分析功能。

```bash
./target/release/astgrep --help
```

### 2. CLI 工具 (astgrep-cli)
专门的命令行界面，提供更多高级功能。

```bash
./target/release/astgrep-cli --version
```

### 3. Web 服务 (astgrep-web)
提供 RESTful API 接口，可以集成到 CI/CD 流程中。

```bash
# 启动 Web 服务（默认端口 8080）
./target/release/astgrep-web

# 使用自定义配置
./target/release/astgrep-web --config astgrep-web.toml
```

### 4. GUI 应用 (astgrep-gui)
图形化界面，提供交互式的代码分析体验。

```bash
./target/release/astgrep-gui
```

## 架构

项目组织为多个 crate：

- `astgrep-core`: 核心类型、trait 和错误处理
- `astgrep-ast`: 通用 AST 定义和操作
- `astgrep-rules`: 规则解析、验证和执行引擎
- `astgrep-parser`: 语言解析器和适配器
- `astgrep-matcher`: 模式匹配引擎
- `astgrep-dataflow`: 数据流和污点分析
- `astgrep-cli`: 命令行界面
- `astgrep-web`: Web 服务接口
- `astgrep-gui`: 图形用户界面

## 开发

### 前置要求

- Rust 1.70+ 
- Cargo

### 构建

```bash
# 构建所有 crate
cargo build

# 构建 release 版本
cargo build --release

# 构建特定的二进制文件
cargo build --release -p astgrep-cli
cargo build --release -p astgrep-web
cargo build --release -p astgrep-gui

# 运行测试
cargo test

# 带日志运行
RUST_LOG=debug cargo run -- analyze

# 运行基准测试
cargo bench
```

### 测试

每个 crate 都有完整的单元测试。运行测试：

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p astgrep-core

# 运行测试并显示输出
cargo test -- --nocapture

# 运行库测试
cargo test --lib

# 运行所有目标的测试
cargo test --all-targets
```

## 规则格式

规则使用 YAML 格式定义。astgrep 支持类似 Semgrep 的规则语法，同时也有自己的扩展。

### 基本规则示例

```yaml
rules:
  - id: java-sql-injection
    name: "SQL 注入检测"
    description: "检测潜在的 SQL 注入漏洞"
    severity: ERROR
    confidence: HIGH
    languages: [java]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern: "$STR + $INPUT"
    fix: "使用 PreparedStatement 和参数化查询"
    metadata:
      cwe: "CWE-89"
      owasp: "A03:2021 - 注入"
```

### 污点分析规则

```yaml
rules:
  - id: user-input-to-sql
    name: "用户输入流向 SQL 查询"
    languages: [java]
    mode: taint
    pattern-sources:
      - pattern: "request.getParameter($PARAM)"
    pattern-sinks:
      - pattern: "Statement.execute($QUERY)"
    pattern-sanitizers:
      - pattern: "sanitize($INPUT)"
    severity: ERROR
    message: "用户输入未经验证直接用于 SQL 查询"
```

### 新语法（v2）

astgrep 还支持更简洁的新语法：

```yaml
rules:
  - id: taint-example
    languages: [python]
    message: "发现不安全的数据流"
    taint:
      sources:
        - "user_input()"
      sinks:
        - "eval(...)"
      sanitizers:
        - "sanitize(...)"
    severity: ERROR
```

详细的规则编写指南请参考 [astgrep 规则编写指南](docs/astgrep-Guide.md)。

## 配置文件

astgrep 使用 TOML 格式的配置文件：

```toml
# astgrep.toml

[general]
verbose = false
threads = 0  # 0 表示自动检测
profile = false

[analysis]
languages = ["java", "javascript", "python", "sql", "bash"]
output_format = "json"
include_metrics = true
enable_dataflow = true
max_findings = 0  # 0 表示无限制
fail_on_findings = false

[filtering]
min_severity = "info"
min_confidence = "low"
exclude_patterns = [
    "*.test.java",
    "*.spec.js",
    "**/test/**",
    "**/tests/**",
    "**/node_modules/**",
    "**/target/**",
    "**/build/**",
    "**/.git/**"
]

[rules]
rules_directory = "rules"
rule_files = []
enabled_categories = ["security", "best-practice", "performance"]
disabled_categories = ["style", "experimental"]
```

使用 `astgrep init` 命令可以生成配置文件模板。

## 支持的语言

| 语言 | 扩展名 | AST 支持 | 污点分析 |
|------|--------|----------|----------|
| Java | .java | ✅ | ✅ |
| JavaScript | .js, .jsx | ✅ | ✅ |
| Python | .py | ✅ | ✅ |
| SQL | .sql | ✅ | ✅ |
| Bash | .sh | ✅ | ✅ |
| PHP | .php | ✅ | ✅ |
| C | .c, .h | ✅ | ✅ |
| C# | .cs | ✅ | ✅ |
| Ruby | .rb | 🚧 | 🚧 |
| Kotlin | .kt | 🚧 | 🚧 |
| Swift | .swift | 🚧 | 🚧 |

## 输出格式

astgrep 支持多种输出格式：

- **JSON**: 结构化的 JSON 输出
- **YAML**: 人类可读的 YAML 格式
- **SARIF**: 静态分析结果交换格式（SARIF 2.1.0）
- **Text**: 简洁的文本格式
- **XML**: XML 格式输出

## 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 编写代码并添加测试
4. 确保所有测试通过 (`cargo test`)
5. 提交更改 (`git commit -m 'Add amazing feature'`)
6. 推送到分支 (`git push origin feature/amazing-feature`)
7. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 路线图

- [x] 多语言 AST 实现
- [x] 基础模式匹配
- [x] 数据流和污点分析
- [x] GUI 界面
- [x] Web 服务接口
- [ ] 高级模式匹配（元变量）
- [ ] IDE 集成（VS Code、IntelliJ）
- [ ] CI/CD 流水线集成
- [ ] 自定义规则开发工具
- [ ] 性能优化和缓存

## 支持

如有问题、建议或想要贡献，请访问我们的 [GitHub 仓库](https://github.com/c2j/astgrep)。

## 相关资源

- [规则编写指南](docs/astgrep-Guide.md)
- [项目状态](docs/v1/PROJECT_STATUS.md)
- [快速参考](docs/v1/QUICK_REFERENCE.md)
- [Semgrep 兼容性](docs/v1/SEMGREP_COMPATIBILITY_ASSESSMENT.md)

