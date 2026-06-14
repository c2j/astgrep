# 贡献指南 (Contributing Guide)

本指南面向 astgrep 项目的合作开发者，涵盖开发环境搭建、贡献流程、项目架构、编码规范、测试约定和扩展指南。

---

## 目录

1. 开发环境搭建
2. 贡献流程
3. 项目架构
4. 编码规范（必循规则）
5. 测试约定
6. 扩展指南
7. 提交规范

---

## 1. 开发环境搭建 (Development Setup)

### 前置要求

- Rust 1.70+ (stable)
- Cargo
- Python 3.8+ (for test annotation validation)
- Git

### 克隆与构建

```bash
git clone https://github.com/c2j/astgrep.git
cd astgrep
cargo build
```

### 安装 Pre-commit Hooks

```bash
lefthook install
```

Pre-commit hooks enforce:

- 代码格式化 (rustfmt)
- Clippy 检查

Pre-push hooks run:

- 全量测试 (cargo test)
- 依赖审计

### 验证构建

```bash
cargo build          # 构建所有 crate
cargo test           # 运行所有测试
cargo clippy         # Lint 检查
cargo fmt --check    # 格式检查
```

---

## 2. 贡献流程 (Contribution Workflow)

### 提交 Pull Request

1. Fork 仓库
2. 创建特性分支: `git checkout -b feature/your-feature`
3. 编写代码并添加测试
4. 确保所有检查通过:

   ```bash
   cargo fmt
   cargo clippy
   cargo test
   python3 tests/scripts/validate_annotations.py --dry-run
   ```

5. 提交更改（遵循提交规范，见第7节）
6. 创建 Pull Request，描述变更和测试方式

### Code Review 要点

- 所有 `pub` API 必须有文档注释
- 禁止 `unwrap()` 在非测试代码中使用
- 禁止用 `as` 做不安全转换
- 新功能必须有测试
- 规则测试必须使用 @rule/@expect/@desc 注解格式

---

## 3. 项目架构 (Project Architecture)

astgrep 使用 Cargo Workspace 组织，共 10 个 crate:

| Crate | 职责 |
|-------|------|
| astgrep-core | 核心类型（Language 枚举、AstNode trait）、错误类型、配置 |
| astgrep-ast | UniversalNode 通用 AST、visitor、builder |
| astgrep-parser | Tree-sitter 语言适配器 + SQL 方言分发器 |
| astgrep-matcher | 模式匹配引擎（字面量、元变量、结构化匹配） |
| astgrep-dataflow | 污点分析、数据流、调用图、常量传播 |
| astgrep-rules | YAML 规则解析、验证、执行引擎 |
| astgrep-cli | 命令行界面（analyze, validate, list, init, info 等） |
| astgrep-web | Axum REST API 服务器 + Web Playground |
| astgrep-gui | egui 桌面应用 |
| test-utils | 测试工具（MockAstNode, MockParser） |

### 依赖方向

core <- ast <- parser <- matcher <- dataflow <- rules <- cli/web/gui

### 关键约定

- **禁止反向依赖**: core 层零外部 IO 依赖
- **workspace deps**: 依赖统一在根 Cargo.toml [workspace.dependencies] 定义，crate 内用 `workspace = true`
- **重导出**: crate 根使用 `pub use module::*` 模式，导入前检查 lib.rs
- **SQL 解析**: 使用 tree-sitter-sequel（NOT tree-sitter-sql）
- **单个 .rs 文件**: 不超过 600 行，理想 400 行以内

---

## 4. 编码规范（必循规则）(Mandatory Coding Standards)

> **底线要求。不遵守这些规则将直接影响代码安全、可维护性、团队协作效率或生产稳定性。必须在 Code Review 和 CI 中强制检查。**

---

## 1. 项目架构与模块化

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-ARCH-01** | 使用 Cargo Workspace 组织项目，按职责分层（`core` / `application` / `adapters` / `api`），禁止反向依赖。 | 工程实践 |
| **M-ARCH-02** | `core` 层必须零外部 IO 依赖，保证业务逻辑的平台无关性与可测试性。 | 工程实践 |
| **M-ARCH-03** | 单个 `.rs` 文件不得超过 **600 行**，理想控制在 **400 行以内**。超过必须拆分模块。 | 工程实践 |
| **M-ARCH-04** | 入口文件（`main.rs`、`lib.rs`）尽量不超过 **200 行**，仅做模块聚合与初始化。 | 工程实践 |
| **M-MOD-01** | 一个项目中**禁止混用**不同的模块布局风格（统一使用 `mod.rs` 或统一使用 `module.rs`）。 | G.MOD.04 |
| **M-MOD-02** | 不要在私有模块中将内部类型设为 `pub(crate)`，可见性必须逐层精确控制。 | G.MOD.05 |
| **M-MOD-03** | 作为库对外提供时，`lib.rs` 中必须重新导出对外公开的 API。 | G.MOD.02 |

---

## 2. 代码风格与格式化（CI 强制门禁）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-FMT-01** | **强制使用 `rustfmt`** 自动格式化代码，不接受人工风格争论。 | P.FMT.01 |
| **M-FMT-02** | 缩进使用空格而非制表符。 | P.FMT.02 |
| **M-FMT-03** | `extern` 外部函数必须显式指定 `"C"` ABI（`extern "C"`）。 | P.FMT.14 |
| **M-FMT-04** | 具名结构体字段初始化时**不得省略字段名**（除非变量名与字段名完全一致）。 | P.FMT.13 |
| **M-FMT-05** | 导入模块分组必须具有良好的可读性，禁止随便使用通配符 `*`。 | P.FMT.11 / G.MOD.03 |

---

## 3. 命名规范

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-NAM-01** | 同一个 crate 中标识符命名必须使用**统一的词序**（如全用 `verb_noun` 或全用 `noun_verb`）。 | P.NAM.01 |
| **M-NAM-02** | getter 类方法**禁止使用 `get_` 前缀**（用 `name()` 而非 `get_name()`）。 | P.NAM.05 |
| **M-NAM-03** | 类型转换函数命名遵循所有权语义：`as_`（借用）、`to_`（可能分配）、`into_`（消耗所有权）。 | G.NAM.02 |
| **M-NAM-04** | 全局静态变量必须加前缀 `G_` 以便和常量区分。 | P.NAM.09 |
| **M-NAM-05** | 作用域越大命名越精确，反之应简短。 | P.NAM.04 |

---

## 4. 类型系统与数据安全

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-TYP-01** | 类型转换**禁止使用裸 `as`**，必须使用安全的转换函数（`try_from`、`into` 等）。 | G.TYP.01 |
| **M-TYP-02** | 数字字面量**必须明确标注类型**（如 `42u64`）。 | G.TYP.02 |
| **M-TYP-03** | 对外导出的公开 Struct 和 Enum **必须添加 `#[non_exhaustive]`**。 | G.TYP.SCT.01 / G.TYP.ENM.05 |
| **M-TYP-04** | 结构体中**超过 3 个布尔字段**时，必须将其独立为新的枚举类型。 | G.TYP.SCT.02 |
| **M-TYP-05** | 禁止将数字类型转换为布尔值，禁止用数字代替布尔值。 | G.TYP.BOL.03 / G.TYP.BOL.06 |
| **M-TYP-06** | 使用数组索引时必须确保不越界，禁止依赖数组边界检查来 Panic。 | G.TYP.ARR.02 / G.EXP.03 |
| **M-TYP-07** | 元组元素不宜超过 3 个，超过应使用结构体。 | G.TYP.TUP.01 |

---

## 5. 错误处理（强制底线）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-ERR-01** | **库代码（lib）禁止返回 `anyhow` 等不透明错误**，必须定义具体的错误类型（使用 `thiserror`）。 | 工程实践 |
| **M-ERR-02** | **禁止在库代码中使用 `unwrap()`**。应用代码（bin）也须极度克制。 | G.ERR.01 |
| **M-ERR-03** | 确定不可能为 `None`/`Err` 时，可使用 `expect()`，但信息必须说明"为什么不会失败"。 | P.ERR.02 |
| **M-ERR-04** | 当传入参数超出限制可能导致函数失败时，**必须使用断言（`assert!`）**。 | P.ERR.01 |
| **M-ERR-05** | 公开的返回 `Result` 的函数文档中**必须增加 Error 注释**；可能 Panic 的必须增加 Panic 注释。 | G.CMT.01 / G.CMT.02 |
| **M-ERR-06** | 实现 `From` 而非 `Into`（因为 `Into` 有默认实现）。 | G.TRA.BLN.08 |

---

## 6. 并发与异步（安全底线）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-ASY-01** | **禁止在异步块/函数中持有同步互斥锁（`MutexGuard`）跨越 `await` 点**。 | G.ASY.02 |
| **M-ASY-02** | **禁止在异步块/函数中持有 `RefCell` 引用跨越 `await` 点**。 | G.ASY.03 |
| **M-ASY-03** | 异步函数中**禁止包含阻塞操作**（文件 IO、密集计算必须使用 `spawn_blocking`）。 | G.ASY.05 |
| **M-ASY-04** | 异步运行时（tokio/async-std）一旦选定，**全局统一，禁止混用**。 | 工程实践 |
| **M-MTH-01** | 对布尔或引用的并发访问**必须使用原子类型**，禁止用互斥锁。 | G.MTH.LCK.01 |
| **M-MTH-02** | 多线程下必须识别锁争用情况，避免死锁。 | P.MTH.LCK.01 |

---

## 7. Unsafe Rust（绝对红线）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-UNS-01** | **禁止为了逃避编译器检查而滥用 Unsafe**。 | P.UNS.01 |
| **M-UNS-02** | **任何 `unsafe` 块之前必须加 `SAFETY` 注释**，说明为什么此处是安全的。 | P.UNS.SAS.09 |
| **M-UNS-03** | 公开的 `unsafe` 函数文档中**必须增加 Safety 注释**。 | G.UNS.SAS.01 |
| **M-UNS-04** | Unsafe 函数中校验边界条件必须使用 `assert!`，**禁止使用 `debug_assert!`**。 | G.UNS.SAS.02 |
| **M-UNS-05** | 禁止在公开 API 中暴露未初始化内存和裸指针。 | P.UNS.SAS.03 / P.UNS.SAS.06 |
| **M-UNS-06** | 禁止将不可变指针手工转换为可变指针。 | G.UNS.PTR.02 |
| **M-UNS-07** | 禁止将裸指针在多线程间共享。 | P.UNS.PTR.01 |

---

## 8. FFI（如涉及 C 互操作）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-FFI-01** | 跨越 FFI 边界的函数**必须处理 Panic**（使用 `catch_unwind`）。 | P.UNS.FFI.04 |
| **M-FFI-02** | 使用 `libc` 或标准库提供的可移植类型别名，禁止直接使用平台特定类型。 | P.UNS.FFI.05 |
| **M-FFI-03** | 禁止为传出外部的类型实现 `Drop`。 | P.UNS.FFI.07 |
| **M-FFI-04** | 依赖 C 端传入的参数时，文档中必须声明不变性，并进行合法性检查。 | P.UNS.FFI.12 / P.UNS.FFI.15 |
| **M-FFI-05** | 自定义数据类型必须保证与 C 端一致的数据布局（`#[repr(C)]`）。 | P.UNS.FFI.13 |

---

## 9. 日志与可观测性（底线）

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-LOG-01** | **统一使用 `tracing`**，禁止使用 `log`。 | 工程实践 |
| **M-LOG-02** | 生产环境日志**必须输出结构化 JSON**，禁止纯文本格式。 | 工程实践 |
| **M-LOG-03** | 日志级别语义必须统一：`ERROR`（需告警）、`WARN`（可自愈异常）、`INFO`（关键生命周期）、`DEBUG/TRACE`（开发调试）。 | 工程实践 |
| **M-LOG-04** | **严禁在日志中记录敏感信息**（密码、Token、PII），必须使用脱敏或 `[REDACTED]`。 | 工程实践 |
| **M-LOG-05** | 每个外部请求入口必须创建 Span，包含 `trace_id` / `request_id`。 | 工程实践 |
| **M-LOG-06** | `ERROR` 级别日志必须包含可行动的上下文（哪里、为什么、影响范围），禁止仅记录 `?err`。 | 工程实践 |

---

## 10. 依赖与构建

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-DEP-01** | `Cargo.toml` 中依赖版本**禁止使用通配符 `*`**。 | G.CAR.04 |
| **M-DEP-02** | 应用项目必须将 `Cargo.lock` 提交到版本控制。 | 工程实践 |
| **M-DEP-03** | 必须声明 `rust-version`（MSRV）并在 CI 中验证。 | 工程实践 |
| **M-DEP-04** | 使用 `cargo-deny` 在 CI 中检查许可证、安全漏洞（`RUSTSEC`）和禁止的依赖。 | 工程实践 |
| **M-DEP-05** | 使用 `cargo features` 进行条件编译，**禁止使用 `--cfg`**。 | P.CAR.03 |

---

## 11. 文档与注释

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-DOC-01** | 所有 `pub` API 必须有文档注释，`cargo doc` 无警告。 | 工程实践 |
| **M-DOC-02** | 文档注释中**使用空格代替 tab**。 | G.CMT.03 |
| **M-DOC-03** | 优先使用行注释 `//`，避免使用块注释 `/* */`。 | P.CMT.03 |
| **M-DOC-04** | 代码中保留的 `FIXME` / `TODO` 必须通过任务系统跟踪，禁止无跟踪长期遗留。 | P.CMT.05 |

---

## 12. 信息安全

| 规则 | 要求 | 来源/依据 |
|------|------|----------|
| **M-SEC-01** | 引入第三方库前必须评估维护活跃度、下载量、依赖树深度，防范供应链攻击。 | P.SEC.01 |
| **M-SEC-02** | 代码中禁止出现非法 Unicode 字符（如双向覆盖字符）。 | G.SEC.01 |

---

## 使用建议

1. **文档一（必须遵循）**应直接写入团队的 `CONTRIBUTING.md`，并在 CI 中配置对应的检查工具（`rustfmt`、`clippy`、`cargo-deny`、`cargo-semver-checks` 等）。
2. 文档应**每半年评审一次**，根据项目演进和 Rust 生态发展进行更新。

---

## 5. 测试约定 (Testing Conventions)

### 自描述测试用例 (Self-Describing Test Cases)

测试用例位于 `tests/categories/{category}/`，使用注解格式:

#### 必需注解（文件头前 30 行）:

- `@rule`: 规则 ID，如 `JAVA-SQLI-001`
- `@expect`: 期望结果 `MATCH` 或 `NO_MATCH`
- `@desc`: 人类可读的场景描述
- `@dialect` (可选): SQL 方言覆盖

#### 注释语法（因语言而异）:

- SQL: `-- @rule GAUSSDB-001`
- Java/JS/C++/Rust: `// @rule JAVA-SQLI-001`
- Python/Ruby/Bash: `# @rule PY-EVAL-001`
- XML/HTML: `<!-- @rule XML-XPATH-001 -->`

#### 规则 ID 命名:

`{LANG}-{CATEGORY}-{NNN}`，如 `JAVA-SQLI-001`, `JS-XSS-003`, `GAUSSDB-TYPE-001`

#### 目录结构:

```
tests/categories/{category}/
├── rules/           # 规则 YAML 文件
└── cases/{concern}/ # 测试用例源文件
    ├── {RULE_ID}_{scenario}.{ext}      # 正例 (MATCH)
    └── {RULE_ID}_{scenario}.neg.{ext}  # 反例 (NO_MATCH)
```

#### 验证注解:

```bash
python3 tests/scripts/validate_annotations.py                 # 验证全部
python3 tests/scripts/validate_annotations.py --category gaussdb  # 按类别
python3 tests/scripts/validate_annotations.py --dry-run       # 仅列出用例
```

### 运行测试

```bash
cargo test                          # 所有测试
cargo test -p astgrep-core          # 单个 crate
cargo test test_name -- --nocapture # 单个测试带输出
```

### 旧格式豁免

以下目录使用旧格式（`// MATCH:` / `// ERROR:`），不强制 @rule/@expect/@desc:

- tests/categories/patterns/
- tests/categories/semgrep-core/
- tests/categories/comparison/
- tests/categories/semgrep-core-e2e/

---

## 6. 扩展指南 (Extension Guide)

### 添加新语言

1. 在 `crates/astgrep-core/src/types.rs` 添加 Language 枚举变体
2. 在 `crates/astgrep-parser/src/{lang}.rs` 创建解析器
3. 更新 `Language::extensions()` 和 `Language::from_str()`
4. 更新解析器注册表
5. 添加测试用例

### 添加新 SQL 方言

1. 在 `crates/astgrep-core/src/types.rs` 添加 SqlDialect 枚举变体
2. 在 `crates/astgrep-parser/src/adapter/{name}/mod.rs` 创建适配器
3. 在 `crates/astgrep-parser/src/dialect/{name}.rs` 创建方言解析器
4. 在 `crates/astgrep-parser/src/dialect/mod.rs` 的 `dispatch()` 中注册
5. 在方言 `parse()` 中调用 `.with_text(source)` 启用元变量支持
6. 编写带 `dialects: [{name}]` 的规则

### 添加新规则

在 `tests/categories/rules/{category}/` 下创建 YAML 文件:

```yaml
rules:
  - id: {LANG}-{CATEGORY}-{NUM}
    name: "规则名称"
    languages: [java]
    patterns:
      - pattern: "..."
    message: "问题描述"
    severity: WARNING
    confidence: HIGH
```

然后在 `cases/{concern}/` 下添加带 @rule/@expect/@desc 注解的测试用例。

---

## 7. 提交规范 (Commit Conventions)

使用约定式提交:

```
<type>(<scope>): <description>

[optional body]
```

类型:

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档变更
- `test`: 测试相关
- `refactor`: 重构
- `chore`: 构建/工具变更
- `ci`: CI 配置

示例:

```
feat(sql): add PolarDB-MySQL dialect via sqlparser-rs
fix(matcher): enable metavariable patterns for ogsql dialect
docs(sql): Phase 5 dialect documentation
```
