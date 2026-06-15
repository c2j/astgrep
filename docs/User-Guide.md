# astgrep 用户指南

本指南面向使用者，帮助你快速上手 astgrep 的命令行（CLI）、Web Playground（含 astgrep-web/astgrep-web-server）与桌面 GUI（astgrep-gui），并介绍“嵌入式 SQL 预处理器”、SQL 多方言分析等关键能力。本文聚焦于当前系统已经实现并可用的功能。

---

## 1. 组件总览

- CLI（astgrep-cli）
  - 在命令行对文件/目录运行规则扫描，支持多语言语法匹配、污点分析与 SQL 多方言分析。
- Web Playground（astgrep-web / astgrep-web-server）
  - 浏览器内交互式试验规则与样例代码。
  - Docs 页签内“离线内嵌”展示《astgrep 规则编写指南》（astgrep-Guide.md），无需联网。
- 桌面 GUI（astgrep-gui）
  - 左侧规则编辑器 + 中间/右侧结果区 + Docs 文档页。
  - Docs 文档页占满可用宽高，并以 Markdown 渲染显示，可滚动查看。
  - 内置“复制预处理器示例到规则编辑器”按钮，便于快速开始。

---

## 2. 快速开始

> 前置：已安装 Rust（稳定版），并可成功运行 `cargo`。

### 2.1 构建

```bash
# 在仓库根目录构建全部组件
cargo build
```

### 2.2 运行 CLI 示例

```bash
# 验证规则语法
astgrep validate path/to/rules.yaml

# 在指定语言/文件上执行
astgrep analyze --language java --config path/to/rules.yaml path/to/File.java
astgrep analyze --language xml  --config path/to/rules.yaml path/to/Mapper.xml

# SQL 多方言分析示例
astgrep analyze --dialect gaussdb --rules gaussdb_rules/ *.sql
```

### 2.3 运行 Web Playground（离线可用文档）

```bash
# 启动内置 Web 服务器（具体二进制名称以仓库为准）
cargo run -p astgrep-web --bin astgrep-web-server
```

- 启动后终端会输出监听地址，例如：`http://127.0.0.1:8787`。
- 用浏览器访问：`/playground` 路径（例如 `http://127.0.0.1:8787/playground`）。
- 切换到 “docs” 页签，可看到“离线内嵌”的 astgrep-Guide（无需外网）。

### 2.4 运行桌面 GUI

```bash
# 启动 GUI 应用
cargo run -p astgrep-gui
```

- 右侧 “Docs” 页占满可用空间，按 Markdown 渲染《astgrep 规则编写指南》内容。
- 点击“复制‘预处理器示例’到规则编辑器”，会将示例规则追加到左侧编辑器。

---

## 3. 在 Playground / GUI 中编写与测试规则

1) 打开 Web Playground 或 GUI。
2) 在规则编辑器中粘贴或编写你的 YAML 规则。
3) 准备待测代码片段或选择目标文件。
4) 点击运行，查看右侧/下方的匹配结果与定位信息。
5) 参考 Docs 页中的语法与示例，逐步细化规则以降低误报。

Tips：
- GUI 的 Docs 页采用 Markdown 渲染、可滚动且占满可用空间，便于“边看文档边写规则”。
- Web Playground 的 Docs 页完全离线内嵌，确保在无网络环境下也能参考文档。

---

## 4. 嵌入式 SQL 预处理器（重点能力）

当 SQL 藏在 Java 源码（如注解/字符串）或 MyBatis XML 标签中时，你可以在“SQL 语义规则”里启用预处理器，让规则像在 `.sql` 文件上一样工作。

### 4.1 何时使用
- 想让现有 SQL 规则复用到 Java 注解/字符串或 MyBatis XML 里的 SQL。
- 希望保持 `languages: [sql]` 的语义匹配，不去写复杂的 Java/XML 字符串/标签模式。

### 4.2 在 YAML 中启用

```yaml
rules:
  - id: sql-avoid-select-star
    languages: [sql]
    patterns:
      - pattern-either:
          - pattern: SELECT * FROM $TABLE
          - pattern: select * from $TABLE
    message: "避免 SELECT *；应明确列名"
    severity: WARNING
    metadata:
      preprocess: embedded-sql      # 启用“嵌入式 SQL 预处理”
      preprocess.from: "java,xml"   # 指定来源宿主语言：java、xml
```

说明：
- 当目标文件是 Java 或 XML 时，系统会先“抽取与归一化 SQL”，再以 SQL 语义匹配器执行规则，并把结果回填到原文件的大致位置。
- `preprocess.from` 仅在文件语言包含其一时才生效，避免规则被误用。

### 4.3 CLI 与界面中的使用

CLI：

```bash
# 在 XML 文件上执行
astgrep analyze --language xml --config rules.yaml path/to/mapper.xml

# 在 Java 文件上执行
astgrep analyze --language java --config rules.yaml path/to/Dao.java
```

Web/GUI：
- 将上述规则写入编辑器，并在输入区提供 Java/XML 示例代码或选择对应文件。
- 运行后即可看到按 SQL 语义匹配得到的结果，且定位映射回原始 Java/XML 源。

### 4.4 已知限制（后续增强方向）
- Java 复杂字符串拼接/条件构造/方法返回等场景当前以占位符处理；后续会增强还原与数据流分析。
- MyBatis 动态 SQL（`<if>/<where>/<trim>/<foreach>/<choose>`）当前做弱归一化，适合结构匹配；会逐步扩展“骨架级展开”。
- 行列精度当前映射到片段起始附近；后续结合片段偏移提升精度。

---

## 5. SQL 方言支持

astgrep 支持多方言 SQL 分析。每种方言使用专用解析器，实现精确的 AST 构建和方言感知的规则匹配。

### 5.1 方言对照表

| 方言 | --dialect 值 | 解析器 | 适用场景 |
|------|-------------|--------|---------|
| 标准 SQL | `standard`（默认） | tree-sitter-sequel 0.3.11 | 通用 SQL (ANSI) |
| GaussDB | `gaussdb` | ogsql-parser v0.6.20 | 完整 GaussDB DML/DDL + PREDICT BY / TIMECAPSULE / SHRINK / Plan Hints |
| OpenGauss | `opengauss` | ogsql-parser v0.6.20 | 与 GaussDB 共享实现 |
| PolarDB-MySQL | `polardb-mysql` | sqlparser-rs v0.62 (MySqlDialect) | MySQL DML/DDL + PolarDB 关键字检测 |

### 5.2 CLI 用法

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

### 5.3 方言感知规则编写

规则可通过 `dialects:` 字段声明适用的 SQL 方言：

```yaml
rules:
  - id: gaussdb-no-on-conflict
    name: "GaussDB 不支持 ON CONFLICT"
    languages: [sql]
    dialects: [gaussdb, opengauss]    # 仅在这些方言下触发
    patterns:
      - pattern: "ON CONFLICT"
    message: "请使用 MERGE INTO 替代"
    severity: ERROR
```

没有 `dialects:` 字段的规则适用于所有方言（向后兼容）。

### 5.4 GaussDB 专属特性

- PREDICT BY（AI 预测）: `PREDICT BY model FEATURES (...)`
- TIMECAPSULE（闪回查询）: `TIMECAPSULE TABLE ... TO TIMESTAMP ...`
- SHRINK（空间回收）: `SHRINK TABLE/INDEX ...`
- Plan Hints（执行计划提示）: `/*+ tablescan(t1) */`
- 内置语义校验器: MERGE DELETE 不支持、ON 列不可更新、DUAL 表不支持

### 5.5 内置规则库

- GaussDB/OpenGauss: 14 条规则（类型兼容、冲突检测、存储引擎、AI 特性、安全检查、性能提示）
- PolarDB-MySQL: 6 条规则（全局索引、分片语法、版本注释、安全检查）

详细文档请参考 [SQL 方言支持](sql-dialects.md)。

---

## 6. CLI 命令参考

### 6.1 全局选项

| 选项 | 简写 | 说明 |
|------|------|------|
| --verbose | -v | 启用详细日志 |
| --quiet | -q | 安静模式（仅显示错误） |
| --config | -c | 配置文件路径 |
| --threads | -j | 并行线程数（0=自动） |
| --profile | | 启用性能分析 |

### 6.2 主要命令

1. `analyze`：分析源码
   - 主要标志: --rules/-r, --language/-l, --dialect, --format/-f, --output/-o, --severity/-S, --confidence/-C, --dataflow, --metrics, --max-findings, --fail-on-findings, --no-parallel, --sql-statement-boundary, --constant-propagation, --compatible
   - 示例：
   ```bash
   astgrep analyze --rules security.yml --format sarif --output results.sarif src/
   astgrep analyze --dialect gaussdb --rules gaussdb_rules/ *.sql
   astgrep analyze --language java --dataflow --metrics src/
   ```

2. `validate`：验证规则文件
   ```bash
   astgrep validate rules/*.yml
   astgrep validate --performance rules/  # 含性能检查
   ```

3. `list`：列出可用规则
   ```bash
   astgrep list --language java --detailed
   astgrep list --category security
   ```

4. `init`：初始化配置文件
   ```bash
   astgrep init --template security --output astgrep.toml
   ```
   模板: default, minimal, comprehensive, security, performance

5. `info`：查看语言和特性信息
   ```bash
   astgrep info --extensions    # 支持的文件扩展名
   astgrep info --categories    # 规则类别
   astgrep info --language java # 特定语言详情
   ```

6. `update`：从远程仓库更新规则
   ```bash
   astgrep update --repository https://github.com/astgrep/rules.git
   ```

7. `migrate`：测试目录结构重组（子命令: analyze, validate, migrate, rollback, status, test）

### 6.3 输出格式

| 格式 | --format 值 | 说明 |
|------|-----------|------|
| JSON | json | 结构化 JSON（默认） |
| SARIF | sarif | 静态分析结果交换格式 2.1.0 |
| Text | text | 人类可读文本 |
| HTML | html | HTML 报告 |
| Markdown | markdown | Markdown 格式 |
| Semgrep | semgrep | Semgrep 兼容格式 |

---

## 7. 文档（离线可用）

- 《astgrep 规则编写指南》（docs/astgrep-Guide.md）已被：
  - Web Playground 的“docs”页签内嵌并渲染（无需跳转 GitHub、无需外网）。
  - GUI 的“Docs”页以 Markdown 渲染，并占满可用空间（更易读、更易复制示例）。
- 在 GUI 中，你可一键将示例规则复制到规则编辑器，快速开始试验。

---

## 8. 常见任务示例

### 8.1 避免 SELECT *（SQL）

```yaml
rules:
  - id: sql-avoid-select-star
    languages: [sql]
    patterns:
      - pattern-either:
          - pattern: SELECT * FROM $TABLE
          - pattern: select * from $TABLE
    message: "避免 SELECT *；应明确列名"
    severity: WARNING
    metadata:
      preprocess: embedded-sql
      preprocess.from: "java,xml"
```

### 8.2 WHERE EXISTS/IN + ORDER BY 性能隐患（SQL）

```yaml
rules:
  - id: sql-performance-issue-where-exists-in-with-orderby
    languages: [sql]
    patterns:
      - pattern-either:
          - pattern: |
              SELECT $... FROM $T1 WHERE  EXISTS ($SUBQUERY) $... ORDER BY $...;
          - pattern: |
              SELECT $... FROM $T1 WHERE  $COL IN ($SUBQUERY) $... ORDER BY $...;
    message: "WHERE EXISTS/IN + ORDER BY 可能导致迁移后退化"
    severity: WARNING
    metadata:
      preprocess: embedded-sql
      preprocess.from: "java,xml"
```

### 8.3 GaussDB 不支持 ON CONFLICT（SQL 方言规则）

```yaml
rules:
  - id: gaussdb-no-on-conflict
    name: "GaussDB 不支持 ON CONFLICT"
    languages: [sql]
    dialects: [gaussdb, opengauss]
    patterns:
      - pattern: "ON CONFLICT"
    message: "GaussDB/OpenGauss 不支持 ON CONFLICT，请使用 MERGE INTO 替代"
    severity: ERROR
```

### 8.4 PolarDB-MySQL 全局索引关键字检查（SQL 方言规则）

```yaml
rules:
  - id: polardb-mysql-global-index
    name: "PolarDB-MySQL 全局索引语法"
    languages: [sql]
    dialects: [polardb-mysql]
    patterns:
      - pattern: "GLOBAL INDEX"
    message: "检测到全局索引语法，请确认分片场景下索引行为符合预期"
    severity: WARNING
```

---

## 9. 故障排除

- 构建失败 / 依赖问题
  - 请确认已安装稳定版 Rust，运行 `rustup update stable` 后重试。
  - 尝试在仓库根目录运行 `cargo clean && cargo build`。

- Playground 页面无法显示文档
  - 确认访问的是 `/playground`，并切换到 “docs” 页签。
  - 确认已使用带有内嵌文档的版本（本仓库近期版本已默认内嵌）。

- GUI 文档区域显示不全
  - 当前实现已让 Docs 页占满可用空间并可滚动；如仍异常，请反馈屏幕分辨率与系统信息。

- 规则不生效 / 无结果
  - 检查 `languages` 与目标文件语言是否匹配。
  - 若使用了预处理器，确认 `metadata.preprocess.from` 覆盖了实际宿主语言（如 `java`、`xml`）。
  - 若使用了 `dialects` 字段，确认 `--dialect` 值在规则声明的方言列表中。
  - 先在最小样例上调试规则（Playground/GUI），再扩展到实际项目。

- SQL 方言解析报错
  - 确认 `--dialect` 值拼写正确，可选值为 `standard`、`gaussdb`、`opengauss`、`polardb-mysql`。
  - 对于 GaussDB/OpenGauss 专有语法，必须使用 `--dialect gaussdb` 或 `--dialect opengauss`，标准 SQL 解析器会报错。
  - 检查 SQL 文件编码是否为 UTF-8，避免特殊字符导致解析失败。

---

## 10. FAQ

- 是否兼容 Semgrep 语法？
  - 兼容大多数常用语法（pattern/ellipsis/metavariable 等），部分高级特性仍在演进；细节见《astgrep 规则编写指南》“Semgrep 兼容性”章节。

- 如何编写污点分析规则？
  - 支持旧语法（`mode: taint`）与新语法（`taint:` 块），推荐新语法；参见指南对应章节与示例。

- 是否支持目录扫描与路径过滤？
  - CLI 支持在目录上运行并配置 `paths.include/exclude`；Web/GUI 适合对单文件/片段做交互式调试。

- 如何选择 SQL 方言？
  - 使用 `--dialect` 标志。标准 SQL 无需指定；GaussDB/OpenGauss 用 `--dialect gaussdb`/`--dialect opengauss`；PolarDB-MySQL 用 `--dialect polardb-mysql`。详见 SQL 方言支持章节和 sql-dialects.md。

- 嵌入式 SQL 预处理器支持哪些宿主语言？
  - 当前支持 `java` 与 `xml`，未来会扩展到更多宿主语言。

---

## 11. 参考

- 本地《astgrep 规则编写指南》：docs/astgrep-Guide.md（Web/GUI 均已内嵌渲染）
- SQL 方言详细文档：docs/sql-dialects.md
- 项目仓库与 Issue 反馈：见仓库 README 与 Issues 列表
- OWASP/CWE 等安全参考：可在指南“元数据/参考资料”章节找到链接
