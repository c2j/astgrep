# astgrep

一个高性能、多语言的静态代码分析工具，专注于安全漏洞和代码质量检测，使用 Rust 实现。

## 特性

- **多语言支持**: Java、JavaScript、Python、SQL、Bash、XML 完全支持
- **多方言 SQL 支持**: GaussDB、OpenGauss、PolarDB-MySQL、标准 SQL —— 每种方言使用专用解析器
  - GaussDB / OpenGauss: `ogsql-parser`
  - PolarDB-MySQL: `sqlparser-rs`
  - 标准 SQL: `tree-sitter-sequel`
- **嵌入式 SQL 预处理器**: 从 Java 源码（注解/字符串）和 MyBatis XML 中抽取 SQL，用 SQL 语义规则匹配，无需编写复杂的宿主语言模式
- **安全导向**: 检测注入漏洞、XSS、身份验证问题等安全问题
- **高性能**: 使用 Rust 构建，速度快且内存安全
- **灵活的规则**: 基于 YAML 的声明式规则定义，支持元变量、条件和数据流追踪
- **多种输出格式**: JSON、SARIF、Text、HTML、Markdown、Semgrep 兼容格式
- **并行处理**: 多线程分析，适用于大型代码库
- **可扩展**: 模块化架构，易于添加新语言和规则
- **污点分析**: 高级数据流和污点分析能力
- **Web Playground**: 通过 `astgrep-web` 提供基于浏览器的交互式规则测试（REST API + Playground 界面）
- **桌面 GUI**: 使用 egui 构建的跨平台桌面应用，支持交互式分析

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

# GaussDB 兼容性扫描
astgrep analyze --dialect gaussdb --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# OpenGauss 分析
astgrep analyze --dialect opengauss --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# PolarDB-MySQL 分析
astgrep analyze --dialect polardb-mysql --rules tests/categories/rules/sql_dialects/polardb_mysql/ *.sql

# 列出可用规则
astgrep list --language java --detailed

# 初始化配置文件
astgrep init --template security --output astgrep.toml

# 查看支持的语言和扩展名
astgrep info --extensions
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

### 3. Web 服务 (astgrep-web-server)
REST API（默认端口 8080）+ 浏览器 Playground（`/playground`），用于交互式规则测试。

```bash
# 启动 Web 服务（默认端口 8080）
./target/release/astgrep-web

# 使用自定义配置
./target/release/astgrep-web --config astgrep-web.toml
```

### 4. GUI 应用 (astgrep-gui)
交互式规则编辑器，内置文档页。

```bash
./target/release/astgrep-gui
```

## 架构

项目组织为 Cargo Workspace，包含 10 个 crate：

- `astgrep-core`: 核心类型、`Language` 枚举、错误处理和配置
- `astgrep-ast`: 通用 AST 定义（UniversalNode）、visitor、builder
- `astgrep-parser`: 语言解析器和适配器，以及 SQL 方言分发器
- `astgrep-matcher`: 模式匹配引擎（字面量、元变量、结构匹配）
- `astgrep-dataflow`: 数据流、污点分析、调用图和常量传播
- `astgrep-rules`: YAML 规则解析、验证和执行引擎
- `astgrep-cli`: 命令行界面，包含 14 个命令
- `astgrep-web`: Axum REST API 服务器和 Web Playground
- `astgrep-gui`: egui 桌面 Playground
- `test-utils`: 测试工具（MockAstNode、MockParser 等）

## SQL 方言支持

astgrep 支持四种 SQL 方言，通过 `--dialect` 标志选择：

| 方言 | `--dialect` 值 | 解析器 | 覆盖范围 |
|------|----------------|--------|----------|
| 标准 SQL | `standard`（默认） | tree-sitter-sequel 0.3.11 | 通用 SQL（ANSI） |
| GaussDB | `gaussdb` | ogsql-parser v0.6.20 | 完整 DML/DDL + PREDICT BY / TIMECAPSULE / SHRINK / Plan Hints |
| OpenGauss | `opengauss` | ogsql-parser v0.6.20 | 与 GaussDB 共享实现 |
| PolarDB-MySQL | `polardb-mysql` | sqlparser-rs v0.62（MySqlDialect） | MySQL DML/DDL + PolarDB 关键字检测 |

### 使用 `--dialect` 标志

```bash
# GaussDB 兼容性扫描
astgrep analyze --dialect gaussdb --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# OpenGauss
astgrep analyze --dialect opengauss --rules tests/categories/rules/sql_dialects/gaussdb/ *.sql

# PolarDB-MySQL
astgrep analyze --dialect polardb-mysql --rules tests/categories/rules/sql_dialects/polardb_mysql/ *.sql

# 标准 SQL（默认，向后兼容）
astgrep analyze *.sql
```

### 方言感知规则

规则可以通过 `dialects:` 字段声明适用的方言。未声明 `dialects:` 的规则适用于所有方言（向后兼容）。

```yaml
rules:
  - id: gaussdb-no-on-conflict
    name: "GaussDB 不支持 ON CONFLICT"
    languages: [sql]
    dialects: [gaussdb, opengauss]
    patterns:
      - pattern: "ON CONFLICT"
    message: "请使用 MERGE INTO 替代"
    severity: ERROR
```

### 内置规则库

- **GaussDB / OpenGauss**: 14 条专用规则（11 条 YAML 规则 + 3 条内置 MERGE 语义校验器），覆盖类型兼容、存储、AI 特性、安全、性能等场景
- **PolarDB-MySQL**: 6 条专用规则，覆盖兼容性和安全场景
- **GaussDB 专有特性**: PREDICT BY、TIMECAPSULE、SHRINK、Plan Hints

更多细节请参考 [docs/sql-dialects.md](docs/sql-dialects.md)。

## 开发

### 前置要求

- Rust 1.70+
- Cargo

克隆后安装 pre-commit 钩子：

```bash
lefthook install
```

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

# 验证测试注解
python3 tests/scripts/validate_annotations.py
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
| Java | .java | 完全支持 | 完全支持 |
| JavaScript | .js, .jsx | 完全支持 | 完全支持 |
| Python | .py | 完全支持 | 完全支持 |
| SQL | .sql | 完全支持 | 完全支持 |
| Bash | .sh, .bash | 完全支持 | 完全支持 |
| XML | .xml, .xsd, .xsl | 完全支持 | — |

此外，PHP、C、C#、Ruby、Kotlin、Swift 的解析器适配器已经存在，但这些语言尚未完全集成到 `Language` 枚举中。

## 输出格式

astgrep 支持多种输出格式：

- **JSON**: 结构化的 JSON 输出
- **SARIF**: 静态分析结果交换格式（SARIF 2.1.0）
- **Text**: 简洁的文本格式
- **HTML**: HTML 格式报告
- **Markdown**: Markdown 格式报告
- **Semgrep 兼容格式**: 与 Semgrep 兼容的输出

## 贡献

详细贡献指南请参考 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)。

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 路线图

详细路线图请参考 [docs/ROADMAP.md](docs/ROADMAP.md)。

### 当前重点

1. **代码库健康** — 修复编译错误，减少警告，添加 CI/CD
2. **架构重构** — 拆分过大的文件
3. **Semgrep 兼容性** — 完成剩余兼容性修复
4. **测试基础设施** — 完成测试目录重组

### 里程碑

- [x] 多语言 AST 实现
- [x] 基础模式匹配
- [x] 数据流和污点分析
- [x] GUI 界面
- [x] Web 服务接口
- [x] 多方言 SQL 支持（GaussDB/OpenGauss/PolarDB-MySQL）
- [x] 嵌入式 SQL 预处理器
- [ ] 高级模式匹配（元变量）
- [ ] IDE 集成（VS Code、IntelliJ）
- [ ] CI/CD 流水线集成
- [ ] 自定义规则开发工具
- [ ] 性能优化和缓存

## 支持

如有问题、建议或想要贡献，请访问我们的 [GitHub 仓库](https://github.com/c2j/astgrep)。

## 相关资源

- [用户指南](docs/User-Guide.md)
- [开发者指南](docs/DeveloperGuide.md)
- [规则编写指南](docs/astgrep-Guide.md)
- [SQL 方言支持](docs/sql-dialects.md)
- [贡献指南](docs/CONTRIBUTING.md)
- [路线图](docs/ROADMAP.md)
- [历史文档归档](docs/archive/) — v1/v1.1/v1.2/v1.3 版本的实现报告与设计记录
