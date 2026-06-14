# SQL 方言增强实施方案

**目标**: 单一通用 SQL → 多方言感知 (GaussDB / OpenGauss / PolarDB-MySQL / Standard) | **工期**: 11 周 | **风险**: 中

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

---

## 1. 背景与决策记录

### 1.1 现状

astgrep 当前 SQL 解析存在以下局限（已通过代码审计确认）：

- `crates/astgrep-core/src/types.rs` 中 `Language::Sql` 是**扁平 enum variant**，无方言子分 discriminator
- `crates/astgrep-parser/src/sql.rs` 使用 `tree-sitter-sequel = "0.3.11"`（= DerekStride/tree-sitter-sql，226 stars / MIT），并提供手工 keyword parser 作为 fallback
- 无任何 GaussDB / OpenGauss / PolarDB 特有语法识别能力
- GaussDB 特性（MERGE INTO 限制、CREATE PACKAGE、PREDICT BY、TIMECAPSULE 等）在 tree-sitter-sequel 中产生 `ERROR` 节点

### 1.2 已确认的三个核心决策

| # | 决策项 | 选择 | 理由 |
|---|---|---|---|
| D1 | GaussDB 方言底座 | **ogsql-parser**（手写递归下降，c2j/ogsql-parser） | 537 commits / 1646 单元测试 / 1409 openGauss 回归测试全过 / License: MIT OR Apache-2.0；已覆盖 MERGE INTO、CREATE PACKAGE、PREDICT BY、PL/pgSQL、所有 GaussDB 特有 DDL |
| D2 | PolarDB 首期范围 | **仅 PolarDB-MySQL** | PolarDB-PG 与 PG 兼容度 95%+，后续可低成本扩展；PolarDB-MySQL 是 MySQL 方言族，需要独立 parser |
| D3 | PolarDB-MySQL 底座 | **sqlparser-rs**（Apache DataFusion 项目） | 手写 Rust Pratt parser，已有 `MySqlDialect`；Apache 项目，被 DataFusion / Polars / ParadeDB 使用；比 tree-sitter-mysql 更易扩展 PolarDB 特有语法 |
| D4 | 依赖方式 | `ogsql-parser = { git = "https://github.com/c2j/ogsql-parser" }` | 用户自有 repo，演进可控；不支持特性提 issue 上游 |
| D5 | Standard SQL 底座 | **保留 `tree-sitter-sequel`**（不替换） | 已在用、成熟稳定、MIT；与 astgrep 其他语言（Java/JS/Python/Bash 同为 tree-sitter）一致 |
| D6 | Language enum 改动 | **不修改 Language enum**，在 `AdapterContext` 加 `dialect` 字段 | 避免破坏性变更（serde、所有 match 表达式） |

### 1.3 已否决的方案

| 方案 | 否决理由 |
|---|---|
| tree-sitter-postgres 作底座 | 仅 4 stars / 2 forks，BSD-3-Clause（非 MIT），不支持 GaussDB 扩展语法，"ERROR 节点恢复"是技术幻觉 |
| 维护独立 tree-sitter-gaussdb grammar | 长期维护成本高（需跟踪 OpenGauss 版本），且 ogsql-parser 已用 1409 测试解决该问题 |
| 跨语言 SQL 字符串提取在 astgrep 端实现 | ogsql-parser 已交付 Java/MyBatis SQL 提取，重做属重复劳动 |
| OpenGauss 独立方言 | 与 GaussDB 语法差异极小（仅 MERGE INTO 集中式/分布式子查询限制），合并到 GaussDBDialect + 模式开关即可 |

---

## 2. 总体架构

```
┌────────────────────────────────────────────────────────────────┐
│  Rule Engine (astgrep-rules)                                    │
│  Rule { ..., dialects: Option<Vec<SqlDialect>> }                │
│  规则可声明仅在特定方言生效                                      │
├────────────────────────────────────────────────────────────────┤
│  AST 层 (astgrep-ast)                                           │
│  UniversalNode (canonical)                                      │
│  + 新增 NodeType variants:                                      │
│    MergeStatement, PredictStatement, CreatePackageStatement,    │
│    ShrinkStatement, TimecapsuleStatement, PlanHint,             │
│    GlobalIndexStatement, VersionedComment, ...                  │
├────────────────────────────────────────────────────────────────┤
│  方言派发层 (astgrep-parser/src/dialect/)                       │
│  SqlDialect enum: Standard / GaussDB / OpenGauss / PolarDBMySQL │
│  Dispatcher: 根据 AdapterContext.dialect 选择 Parser            │
├────────────────────────────────────────────────────────────────┤
│  方言适配器 (astgrep-parser/src/adapter/)                       │
│  ├── OgsqlAdapter:      ogsql::Statement → UniversalNode        │
│  ├── SqlparserAdapter:  sqlparser::Statement → UniversalNode    │
│  └── (Standard 直接用 tree-sitter → UniversalNode 现有路径)     │
├────────────────────────────────────────────────────────────────┤
│  外部 Parsers                                                   │
│  ├── tree-sitter-sequel 0.3.11 (existing, for Standard)         │
│  ├── ogsql-parser (git dep, for GaussDB/OpenGauss)              │
│  └── sqlparser-rs (crates.io, for PolarDB-MySQL)                │
└────────────────────────────────────────────────────────────────┘
```

### 2.1 关键设计原则

1. **三 parser 对称集成**：ogsql-parser 与 sqlparser-rs 都是手写 Rust parser，集成模式一致；Standard 保留 tree-sitter 与其他语言一致
2. **UniversalNode 是唯一规范 AST**：所有 parser 产出必须归一化到 UniversalNode，规则引擎不感知方言内部 AST 形状
3. **显式方言指定**：CLI `--dialect` + 规则 `dialects:` 字段；不做自动推断（`.sql` 扩展名无法可靠区分方言）
4. **ogsql-parser 上游反馈循环**：发现 ogsql-parser 不支持的特性 → 提 issue → 升级版本；不在 astgrep 端做兼容补丁

---

## 3. 任务总览

| Phase | 任务 | 工期 | 难度 | 依赖 |
|---|---|---|---|---|
| **1** | **方言抽象框架** | **2 周** | 中 | 无 |
| 1.1 | SqlDialect enum + AdapterContext 扩展 | 3 天 | 中 | — |
| 1.2 | CLI `--dialect` 参数 + YAML `dialects:` 字段 | 3 天 | 中 | 1.1 |
| 1.3 | dialect dispatcher 基础设施 | 2 天 | 中 | 1.1 |
| 1.4 | 验证与回归（确保 Standard 行为不变） | 2 天 | 低 | 1.1-1.3 |
| **2** | **ogsql-parser 集成 + GaussDB 方言** | **3 周** | 中-高 | Phase 1 |
| 2.1 | 引入 ogsql-parser git 依赖 + POC | 3 天 | 中 | Phase 1 |
| 2.2 | OgsqlAdapter：核心 DML 转换（SELECT/INSERT/UPDATE/DELETE/MERGE） | 5 天 | 中-高 | 2.1 |
| 2.3 | OgsqlAdapter：核心 DDL 转换（CREATE TABLE/INDEX/VIEW/PACKAGE） | 4 天 | 中 | 2.2 |
| 2.4 | GaussDB 特有节点（PREDICT BY / TIMECAPSULE / SHRINK / PlanHint） | 3 天 | 中 | 2.3 |
| 2.5 | GaussDBDialect + OpenGaussDialect（mode flag） | 2 天 | 低 | 2.4 |
| **3** | **GaussDB 兼容性规则库** | **2 周** | 中 | Phase 2 |
| 3.1 | YAML dialects: 字段解析 + 规则过滤逻辑 | 2 天 | 中 | Phase 1 |
| 3.2 | 规则集 1：MERGE INTO 不兼容模式（多 VALUES、分布式子查询） | 3 天 | 中 | Phase 2 |
| 3.3 | 规则集 2：Plan Hint 合规（不支持的关键字） | 2 天 | 中 | Phase 2 |
| 3.4 | 规则集 3：Oracle 兼容类型使用（VARCHAR2 / NUMBER 使用建议） | 2 天 | 低 | Phase 2 |
| 3.5 | 规则集 4：存储引擎与对象（ustore/astore、CREATE PACKAGE 检查） | 3 天 | 中 | Phase 2 |
| **4** | **PolarDB-MySQL 方言** | **3 周** | 中 | Phase 1 |
| 4.1 | 引入 sqlparser-rs 依赖 + POC | 2 天 | 低 | Phase 1 |
| 4.2 | SqlparserAdapter：MySQL DML/DDL 转换 | 5 天 | 中 | 4.1 |
| 4.3 | PolarDB-MySQL 扩展（GLOBAL INDEX / versioned comment） | 4 天 | 中-高 | 4.2 |
| 4.4 | PolarDB-X 分布式语法（DBPARTITION BY） | 3 天 | 中 | 4.3 |
| 4.5 | PolarDB 规则集（5-8 条兼容性规则） | 3 天 | 中 | 4.4 |
| **5** | **验证、文档、回归** | **1 周** | 低 | All |
| 5.1 | tests/categories/sql_dialects/ 测试目录建立 | 2 天 | 低 | Phase 2-4 |
| 5.2 | 文档更新（AGENTS.md、README、CLI help） | 2 天 | 低 | All |
| 5.3 | 全量回归 + 性能基准 | 1 天 | 低 | All |

**合计**: 11 周（含 1 周验证）

---

## 4. Phase 1：方言抽象框架

### 任务 1.1：SqlDialect enum + AdapterContext 扩展

**Files:**
- Modify: `crates/astgrep-core/src/types.rs`（新增 SqlDialect enum + 在现有 `AnalysisConfig` 上加 dialect 字段；该 struct 已在此文件第 195 行）
- Test: `crates/astgrep-core/src/types.rs` 单元测试

> **注**：`AnalysisConfig` 实际定义在 `types.rs` 第 195 行（不在 `config.rs`，后者仅含 `PathHandler` / `AstGrepConfig`）。本任务所有改动集中在 `types.rs` 即可。

**Step 1: 定义 SqlDialect enum（合规：M-TYP-03 加 `#[non_exhaustive]`；M-DOC-01 文档注释）**

```rust
// crates/astgrep-core/src/types.rs

/// SQL 方言枚举。新增方言时必须保留向前兼容（已有 match 不会因新 variant 编译失败）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]  // M-TYP-03：对外 enum 强制 non_exhaustive
pub enum SqlDialect {
    /// 通用 SQL（tree-sitter-sequel）
    Standard,
    /// 华为 GaussDB 集中式
    GaussDB,
    /// 开源 OpenGauss（默认集中式，可切换分布式）
    OpenGauss,
    /// 阿里 PolarDB MySQL 兼容版
    PolarDBMySQL,
    // 未来扩展：PolarDBPG, Oracle, MySQL, ...
}

impl SqlDialect {
    /// 返回该方言使用的底层 parser 家族，用于派发器选择路径。
    pub fn parser_family(&self) -> SqlParserFamily {
        match self {
            SqlDialect::Standard => SqlParserFamily::TreeSitterSequel,
            SqlDialect::GaussDB | SqlDialect::OpenGauss => SqlParserFamily::Ogsql,
            SqlDialect::PolarDBMySQL => SqlParserFamily::Sqlparser,
        }
    }

    /// 从字符串解析方言。未知字符串返回 `None`，由调用方决定 fallback 策略。
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "standard" | "sql" => Some(Self::Standard),
            "gaussdb" | "gauss" => Some(Self::GaussDB),
            "opengauss" | "og" => Some(Self::OpenGauss),
            "polardb-mysql" | "polardb" => Some(Self::PolarDBMySQL),
            _ => None,
        }
    }
}

/// SQL parser 家族分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlParserFamily {
    TreeSitterSequel,
    Ogsql,
    Sqlparser,
}
```

> **合规要求**：`parser_family()` 返回 `SqlParserFamily`（按值，因为 `Copy`，符合 R-API-03）；`from_str` 返回 `Option<Self>`（不 panic，符合 M-ERR-02）。

**Step 2: 扩展现有 AnalysisConfig 与 AdapterContext**

```rust
// crates/astgrep-core/src/types.rs (AnalysisConfig 已存在，第 195 行)
// 在现有字段基础上追加：
pub struct AnalysisConfig {
    // ... existing fields (含 sql_statement_boundary) ...
    pub sql_dialect: Option<SqlDialect>, // ← 新增
}

// crates/astgrep-parser/src/adapters.rs (AdapterContext 已存在)
pub struct AdapterContext {
    pub source_code: String,
    pub language: Language,
    pub sql_dialect: Option<SqlDialect>, // ← 新增
}
```

**Step 3: 测试**

```bash
cargo test -p astgrep-core sql_dialect
```
Expected: enum 序列化/反序列化、`from_str` 全分支覆盖、`parser_family` 映射正确

### 任务 1.2：CLI `--dialect` 参数 + YAML `dialects:` 字段

**Files:**
- Modify: `crates/astgrep-cli/src/lib.rs`（在现有 `Commands::Analyze` enum variant 内联添加 `--dialect` 参数；现有 `--sql-statement-boundary` 已在此）
- Modify: `crates/astgrep-rules/src/types.rs`（Rule struct 加 dialects 字段）
- Modify: `crates/astgrep-rules/src/parser/parsing.rs`（YAML 解析 dialects）
- Test: `crates/astgrep-rules/tests/`

> **注**：CLI 参数实际定义在 `crates/astgrep-cli/src/lib.rs` 的 `Commands::Analyze { ... }` enum variant 中（不是 `analyze.rs` 里独立的 `AnalyzeArgs` struct）。现有 `--sql-statement-boundary` 已在该 enum inline 定义，可直接参照添加 `--dialect`。

**Step 1: Rule struct 扩展**

```rust
// crates/astgrep-rules/src/types.rs
pub struct Rule {
    // ... existing fields ...
    #[serde(default)]
    pub dialects: Option<Vec<SqlDialect>>, // None = 所有 SQL 方言生效（向后兼容）
}

impl Rule {
    pub fn applies_to_dialect(&self, dialect: Option<SqlDialect>) -> bool {
        match (&self.dialects, dialect) {
            (None, _) => true,           // 规则未声明方言 → 对所有方言生效（向后兼容）
            (Some(_), None) => false,    // 规则声明了方言但当前不是 SQL → 不生效
            (Some(list), Some(d)) => list.contains(&d),
        }
    }
}
```

**Step 2: CLI 参数（内联在 Commands::Analyze）**

```rust
// crates/astgrep-cli/src/lib.rs
// 在 Commands enum 中找到 Analyze variant，参照 --sql-statement-boundary 添加：
#[derive(clap::Parser)]
pub enum Commands {
    Analyze {
        // ... existing fields (含 sql_statement_boundary: bool) ...
        #[arg(long, value_name = "DIALECT")]
        dialect: Option<SqlDialect>,
        // ...
    },
    // ...
}
```

**Step 3: YAML schema 文档与示例**

```yaml
# 示例：仅 GaussDB 生效的规则
rules:
  - id: gaussdb-merge-multi-values
    name: "GaussDB MERGE INTO 多 VALUES 不兼容"
    languages: [sql]
    dialects: [gaussdb, opengauss]   # ← 新字段
    patterns:
      - pattern: "MERGE INTO $T USING $S ON $C WHEN NOT MATCHED THEN INSERT ($COLS) VALUES ($V1), ($V2)"
    severity: ERROR
```

**Step 4: 验证**

```bash
cargo test -p astgrep-rules dialects
cargo run -- analyze --dialect gaussdb tests/categories/sql_dialects/gaussdb/
```
Expected: YAML 中带 `dialects:` 的规则只对指定方言触发；不带 `dialects:` 的规则向后兼容

### 任务 1.3：方言派发器基础设施

**Files:**
- Create: `crates/astgrep-parser/src/dialect/mod.rs`
- Create: `crates/astgrep-parser/src/dialect/standard.rs`（将现有 sql.rs 重构为 StandardDialect）
- Modify: `crates/astgrep-parser/src/lib.rs`（注册派发器）

**Step 1: 派发 trait**

```rust
// crates/astgrep-parser/src/dialect/mod.rs
pub mod standard;
// Phase 2 加：pub mod gaussdb;
// Phase 4 加：pub mod polardb_mysql;

use astgrep_core::{Language, SqlDialect};
use std::path::Path;
use crate::ast_node::AstNode;

pub trait SqlDialectParser: Send + Sync {
    fn dialect(&self) -> SqlDialect;
    fn parse(&self, source: &str, file_path: &Path) -> anyhow::Result<Box<dyn AstNode>>;
    fn supports_file(&self, file_path: &Path) -> bool;
}

pub fn dispatch(dialect: SqlDialect) -> Box<dyn SqlDialectParser> {
    match dialect {
        SqlDialect::Standard => Box::new(standard::StandardSqlDialect::new()),
        SqlDialect::GaussDB | SqlDialect::OpenGauss => {
            unimplemented!("Phase 2: ogsql-parser integration")
        }
        SqlDialect::PolarDBMySQL => {
            unimplemented!("Phase 4: sqlparser-rs integration")
        }
    }
}
```

**Step 2: 重构现有 sql.rs → standard.rs**

将 `crates/astgrep-parser/src/sql.rs` 中 `SqlParser`/`SqlAdapter` 的实现迁移到 `dialect/standard.rs`，实现 `SqlDialectParser` trait；保留 `LanguageParser` impl 委托给当前方言派发器。

**Step 3: 验证**

```bash
cargo test -p astgrep-parser
cargo run -- analyze tests/categories/patterns/sql/
```
Expected: 所有现有 SQL 测试通过；Standard 方言路径行为完全不变

### 任务 1.4：验证与回归

**验证步骤**：

1. `cargo build --release` 通过
2. `cargo test` 全量通过（基线 = Phase 1 开始前的通过率）
3. `cargo run -- analyze --dialect standard tests/categories/patterns/sql/` 输出与不加 `--dialect` 一致
4. LSP diagnostics 在修改的文件上无新增 error/warning
5. `cargo clippy --workspace -- -D warnings` 通过

---

## 5. Phase 2：ogsql-parser 集成 + GaussDB 方言

### 任务 2.1：引入 ogsql-parser git 依赖 + POC

**Files:**
- Modify: `Cargo.toml`（workspace.dependencies 加 ogsql-parser）
- Modify: `crates/astgrep-parser/Cargo.toml`（添加 ogsql-parser 依赖）
- Create: `crates/astgrep-parser/src/dialect/gaussdb.rs`（骨架）
- Create: `crates/astgrep-parser/src/adapter/ogsql_adapter.rs`（骨架）

**Step 1: 添加依赖（合规：M-DEP-01 禁通配符；M-SEC-01 评估供应链）**

```toml
# Cargo.toml [workspace.dependencies]
# ogsql-parser 必须锁定 rev/tag 防止上游 main 分支意外破坏构建（M-DEP-01）
# License: MIT OR Apache-2.0（与 astgrep 兼容，M-SEC-01 通过）
# 评估：用户自有 repo，537 commits / 1646 单元测试 / 1409 openGauss 回归测试全过
ogsql-parser = { git = "https://github.com/c2j/ogsql-parser", rev = "<具体 commit SHA 或 tag>" }

# sqlparser-rs（Apache DataFusion 项目，Apache-2.0 License，被 DataFusion/Polars 使用，M-SEC-01 通过）
sqlparser = "0.53"  # Phase 4 用，提前加
```

> **合规要求**：实施时必须将 `<具体 commit SHA 或 tag>` 替换为 ogsql-parser 当前 release 的 commit SHA 或 tag（如 `tag = "v0.6.20"`）。每次升级 ogsql-parser 时更新 Cargo.lock 并跑全量回归测试。

```toml
# crates/astgrep-parser/Cargo.toml
[dependencies]
# ... existing ...
ogsql-parser.workspace = true
```

**Step 2: POC — 最小转换链（合规：M-ERR-01 库禁 anyhow；用 thiserror）**

```rust
// crates/astgrep-parser/src/adapter/ogsql_adapter.rs
use ogsql_parser::{Tokenizer, Parser as OgsqlParser};
use astgrep_ast::UniversalNode;

/// ogsql → UniversalNode 转换错误类型（M-ERR-01：库代码必须 thiserror）
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OgsqlAdapterError {
    #[error("ogsql tokenizer failed: {0}")]
    Tokenize(#[from] ogsql_parser::TokenizerError),
    #[error("ogsql parser failed: {0}")]
    Parse(#[from] ogsql_parser::ParserError),
    #[error("unsupported statement variant: {0}")]
    UnsupportedStatement(&'static str),
    #[error("conversion failed for node: {0}")]
    ConversionFailed(String),
}

pub struct OgsqlAdapter;

impl OgsqlAdapter {
    /// 将 GaussDB/openGauss SQL 字符串解析为 UniversalNode 列表。
    ///
    /// # Errors
    /// 返回 `OgsqlAdapterError` 当：
    /// - 词法分析失败（`Tokenize`）
    /// - 语法分析失败（`Parse`）
    /// - 遇到未支持的 statement variant（`UnsupportedStatement`）
    pub fn parse_to_universal(sql: &str) -> Result<Vec<UniversalNode>, OgsqlAdapterError> {
        let tokens = Tokenizer::new(sql).tokenize()?;
        let statements = OgsqlParser::new(tokens).parse()?;
        statements.iter().map(Self::convert_statement).collect()
    }

    fn convert_statement(stmt: &ogsql_parser::Statement) -> Result<UniversalNode, OgsqlAdapterError> {
        // POC: 仅处理 SELECT，验证转换链
        todo!("Phase 2.2")
    }
}
```

**Step 3: POC 验证**

```bash
cargo build -p astgrep-parser
# 手动测试或 examples/poc_gaussdb.rs：
#   解析 "SELECT * FROM users" → 1 个 UniversalNode
#   解析 "PREDICT BY model FEATURES col" → 不崩溃（即使暂不识别）
```

### 任务 2.2：OgsqlAdapter 核心 DML 转换

**Files:**
- Modify: `crates/astgrep-parser/src/adapter/ogsql_adapter.rs`
- Create: `crates/astgrep-parser/src/adapter/ogsql/dml.rs`（**M-ARCH-03 拆分**：DML 转换独立文件）
- Modify: `crates/astgrep-ast/src/nodes.rs`（确认/新增 DML NodeType variants）
- Test: `crates/astgrep-parser/src/adapter/ogsql/dml.rs` 单元测试（同文件 `#[cfg(test)] mod tests`）

> **M-ARCH-03 合规**：ogsql-parser 有 180+ Statement variants，全部塞进单文件必然超 600 行。**强制拆分**为：
> - `adapter/ogsql/mod.rs` — OgsqlAdapter 主体 + dispatch
> - `adapter/ogsql/dml.rs` — SELECT/INSERT/UPDATE/DELETE/MERGE 转换
> - `adapter/ogsql/ddl.rs` — CREATE TABLE/INDEX/VIEW/PACKAGE 等
> - `adapter/ogsql/features.rs` — GaussDB 特有节点（PREDICT/TIMECAPSULE/SHRINK/PlanHint）
>
> 每个文件控制在 400 行以内（理想值）。

**实施**：

利用 ogsql-parser 已有的 `ast::visitor::Visitor` trait，实现 `UniversalNodeBuilder` visitor：

```rust
use ogsql_parser::ast::{Statement, SelectStatement, InsertStatement, UpdateStatement, DeleteStatement, MergeStatement};

impl OgsqlAdapter {
    fn convert_statement(stmt: &Statement) -> anyhow::Result<UniversalNode> {
        match stmt {
            Statement::Select(s) => Self::convert_select(s),
            Statement::Insert(s) => Self::convert_insert(s),
            Statement::Update(s) => Self::convert_update(s),
            Statement::Delete(s) => Self::convert_delete(s),
            Statement::Merge(s) => Self::convert_merge(s),
            _ => Self::convert_generic(stmt),
        }
    }
    // convert_select / convert_insert / ... 各自把 ogsql 结构字段映射到 UniversalNode 属性
}
```

**验证**：

```bash
cargo test -p astgrep-parser gaussdb::dml
# 测试用例：覆盖 SELECT (含 JOIN/WHERE/GROUP BY/HAVING/ORDER BY/LIMIT/CTE/window)、
# INSERT (VALUES/SELECT/ON CONFLICT)、UPDATE (SET/WHERE/RETURNING)、
# DELETE (WHERE/RETURNING)、MERGE INTO (完整 WHEN MATCHED/NOT MATCHED)
```

**重要**：ogsql-parser 的 `SelectStatement` 等是 serde-serializable，可先 `serde_json::to_string` 输出 JSON 验证字段结构，再编写 UniversalNode 映射。

### 任务 2.3：OgsqlAdapter 核心 DDL 转换

覆盖：CREATE TABLE / CREATE INDEX / CREATE VIEW / CREATE PACKAGE / CREATE PACKAGE BODY / CREATE FUNCTION / CREATE PROCEDURE / DROP / ALTER TABLE

**Files:**
- 同 2.2
- Modify: `crates/astgrep-ast/src/nodes.rs`（新增 `CreatePackageStatement` 等 NodeType）

**验证**：

```bash
cargo test -p astgrep-parser gaussdb::ddl
# 至少 30 条 DDL 测试用例，覆盖 GaussDB 特有 DDL（NODE / NODE GROUP / RESOURCE POOL / WORKLOAD GROUP / MASKING POLICY 等）
```

### 任务 2.4：GaussDB 特有节点

**Files:**
- Modify: `crates/astgrep-ast/src/nodes.rs`（新增节点类型）
- Modify: `crates/astgrep-parser/src/adapter/ogsql_adapter.rs`

**新增 NodeType variants**：

| NodeType | ogsql 源类型 | 说明 |
|---|---|---|
| `PredictStatement` | `PredictByStatement` | GaussDB AI 推理 |
| `TimecapsuleStatement` | (ogsql 中对应类型) | 闪回表 |
| `ShrinkStatement` | (ogsql 中对应类型) | 表/索引压缩 |
| `CreatePackageStatement` | (ogsql 中对应类型) | Oracle 风格 PACKAGE |
| `PlanHint` | comment + `/*+ ... */` 启发式 | 优化器 hint |

**验证**：

```bash
cargo test -p astgrep-parser gaussdb::features
# 测试 PREDICT BY / TIMECAPSULE TABLE / SHRINK TABLE / CREATE PACKAGE / /*+ INDEX(...) */ 等
```

### 任务 2.5：GaussDBDialect + OpenGaussDialect

**Files:**
- Create: `crates/astgrep-parser/src/dialect/gaussdb.rs`
- Create: `crates/astgrep-parser/src/dialect/opengauss.rs`

```rust
// crates/astgrep-parser/src/dialect/gaussdb.rs
pub struct GaussDBDialect { pub mode: GaussDBMode }
pub enum GaussDBMode { Centralized, Distributed }

impl SqlDialectParser for GaussDBDialect { ... }

// opengauss.rs
pub struct OpenGaussDialect(GaussDBDialect);  // 共享实现
```

**验证**：

```bash
cargo run -- analyze --dialect gaussdb tests/categories/sql_dialects/gaussdb/
cargo run -- analyze --dialect opengauss tests/categories/sql_dialects/opengauss/
```

---

## 6. Phase 3：GaussDB 兼容性规则库

### 任务 3.1：YAML dialects 字段 + 规则过滤

详见任务 1.2（已实现），此处补全 `applies_to_dialect` 在执行器中的调用点：

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/mod.rs`

```rust
let applicable_rules: Vec<&Rule> = rules.iter()
    .filter(|r| r.applies_to(language) && r.applies_to_dialect(ctx.sql_dialect))
    .collect();
```

### 任务 3.2-3.5：规则编写

**目录**: `tests/categories/rules/sql_dialects/gaussdb/`

| 规则 ID | 类别 | 描述 |
|---|---|---|
| `GAUSSDB-MERGE-001` | MERGE | INSERT 子句多 VALUES 不兼容 |
| `GAUSSDB-MERGE-002` | MERGE | 分布式模式下 USING subquery |
| `GAUSSDB-HINT-001` | Plan Hint | 不支持的 hint 关键字 |
| `GAUSSDB-HINT-002` | Plan Hint | hint 语法错误（缺 `+` 或括号） |
| `GAUSSDB-TYPE-001` | Type | VARCHAR2 长度超限 |
| `GAUSSDB-TYPE-002` | Type | NUMBER 精度/标度配置问题 |
| `GAUSSDB-STORE-001` | Storage | ustore/astore 语法合规 |
| `GAUSSDB-PKG-001` | Package | PACKAGE 与 PACKAGE BODY 不一致（用 ogsql-parser 的 `validate_package_consistency`） |
| `GAUSSDB-PKG-002` | Package | PACKAGE 中引用未定义的存储过程 |
| `GAUSSDB-PREDICT-001` | AI | PREDICT BY 模型不存在（需 schema） |
| `GAUSSDB-TX-001` | Transaction | PL/pgSQL 块中 COMMIT/ROLLBACK 模式（用 ogsql 的 `analyze_transactions`） |
| ... | | 共 ~20 条 |

**验证**：

每条规则配 `.sgrep` + target 文件，target 含 `// MATCH:` 与 `// ERROR:` 注释：

```bash
cargo test -p astgrep-rules --test sql_dialect_rules
```

---

## 7. Phase 4：PolarDB-MySQL 方言

### 任务 4.1：引入 sqlparser-rs + POC

**Files:**
- Modify: `Cargo.toml`（sqlparser 已在 Phase 2.1 加入）
- Modify: `crates/astgrep-parser/Cargo.toml`（启用 sqlparser）
- Create: `crates/astgrep-parser/src/dialect/polardb_mysql.rs`（骨架）
- Create: `crates/astgrep-parser/src/adapter/sqlparser_adapter.rs`（骨架）

**POC**：

```rust
use sqlparser::{parser::Parser as SqlparserParser, dialect::MySqlDialect};

pub struct PolarDBMySQLAdapter;

impl PolarDBMySQLAdapter {
    pub fn parse_to_universal(sql: &str) -> anyhow::Result<Vec<UniversalNode>> {
        let dialect = &PolarDBMyDialect;  // 继承 MySqlDialect
        let ast = SqlparserParser::parse_sql(dialect, sql)?;
        ast.iter().map(Self::convert_statement).collect()
    }
}

// PolarDB 扩展 dialect
struct PolarDBMyDialect;
impl sqlparser::dialect::Dialect for PolarDBMyDialect {
    // 继承 MySql 行为 + PolarDB 特有关键字
    // GLOBAL / DBPARTITION / TBPARTITION 等在 4.3 实现
}
```

**验证**：

```bash
cargo build -p astgrep-parser
# POC: 解析 "SELECT * FROM t" 不崩溃
```

### 任务 4.2：SqlparserAdapter MySQL DML/DDL 转换

覆盖标准 MySQL DML/DDL，参考 sqlparser-rs 的 AST 文档（https://apache.github.io/arrow-datafusion-sqlparser-rs/sqlparser_ast/）。

**验证**：

```bash
cargo test -p astgrep-parser polardb::mysql
```

### 任务 4.3：PolarDB-MySQL 扩展语法

| 语法 | 实现方式 |
|---|---|
| `CREATE GLOBAL INDEX` | 在 `PolarDBMyDialect` 中识别 `GLOBAL` 关键字；扩展 sqlparser 的 `CREATE INDEX` 产生式（fork 或 wrapper） |
| `/*!99990 ... */` versioned comment | 自定义 tokenizer 预处理：把 versioned comment 转成普通 comment 节点 |
| `COVERING` (GSI) | 同上 |
| `DBPARTITION BY` / `TBPARTITION BY` | 在方言层识别（详见 4.4） |

**关键策略**：sqlparser-rs 是 Apache 项目，扩展的最佳方式是**自定义 Dialect trait impl + 必要时 fork 上游**。如果 PolarDB 特有语法无法通过 Dialect 表达，考虑：
1. 在 `PolarDBMyDialect` 中重写关键 method
2. 严重情况下 fork sqlparser-rs（短期方案）
3. 上游 PR（长期方案）

### 任务 4.4：PolarDB-X 分布式语法

`DBPARTITION BY hash(col) TBPARTITION BY hash(col) TBPARTITIONS 4` 在标准 MySQL parser 中无法识别。

**实现方案**：

1. 在 `PolarDBMyDialect` 中前置识别这些 token
2. 自定义 AST 节点 `DistributeClause`（参考 ogsql-parser 中同名类型）
3. 通过预处理：把 `DBPARTITION BY ...` 段提取为独立节点，剩余 SQL 交给 sqlparser

### 任务 4.5：PolarDB 规则集

**目录**: `tests/categories/rules/sql_dialects/polardb_mysql/`

| 规则 ID | 描述 |
|---|---|
| `POLARDB-MYSQL-GSI-001` | GLOBAL INDEX 必须指定 DBPARTITION |
| `POLARDB-MYSQL-VERCOMMENT-001` | 版本注释 `/*!99990 */` 使用建议 |
| `POLARDB-MYSQL-SHARD-001` | DBPARTITION hash 列选择建议 |
| `POLARDB-MYSQL-INCEPT-001` | 检测无 WHERE 的 UPDATE/DELETE（参考 polar_sql_inception） |
| `POLARDB-MYSQL-INCEPT-002` | 检测 SELECT * |
| `POLARDB-MYSQL-INCEPT-003` | UPDATE/DELETE 影响行数上限检查 |
| ... | 共 5-8 条 |

---

## 8. Phase 5：验证、文档、回归

### 任务 5.1：测试目录建立

**目录结构**：

```
tests/categories/sql_dialects/
├── gaussdb/
│   ├── dml/               # MERGE / PREDICT / TIMECAPSULE 等
│   ├── ddl/               # CREATE PACKAGE / NODE / SHRINK 等
│   ├── plpgsql/           # 存储过程
│   └── hints/             # Plan hints
├── opengauss/
│   └── merge_centralized/ # 集中式/分布式差异
├── polardb_mysql/
│   ├── gsi/               # GLOBAL INDEX
│   ├── versioned_comments/
│   └── shard/             # DBPARTITION
└── standard/              # 现有 SQL 测试迁入（验证向后兼容）
```

每个测试用例：`.sgrep` pattern + `.sql` target + `// MATCH:` / `// ERROR:` 注释。

### 任务 5.2：文档更新

**Files:**
- Modify: `AGENTS.md`（在 "Where to Look" 表加 SQL 方言章节）
- Modify: `README.md`（Quick Start 加 `--dialect` 示例）
- Modify: `docs/ROADMAP.md`（更新 SQL 方言进度）
- Create: `docs/sql-dialects.md`（用户文档：方言列表、规则 schema、CLI 用法）

### 任务 5.3：全量回归 + 性能基准

```bash
# 全量测试
cargo test --workspace

# 性能基准（对比 Phase 1 前后）
cargo bench --bench parser_benchmarks

# 与基线对比通过率
python3 newtest/scripts/guardian_runner.py --category "patterns/sql" --verbose

# 强制规范检查（M-FMT-01 / M-DOC-01）
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps 2>&1 | grep -c "^warning" | grep -q "^0$"
```

Expected:
- 所有现有测试通过率不下降
- Standard 方言路径性能不退化（< 5%）
- GaussDB/PolarDB 方言路径有合理性能（无硬性指标，POC 阶段记录基线）
- `cargo fmt --check` 无 diff
- `cargo clippy` 无 warning
- `cargo doc` 无 doc warning（M-DOC-01）

---

## 9. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| ogsql-parser 0.6.x API 演进破坏兼容 | 中 | 中 | 锁定 Cargo.lock；语义化版本；上游提 issue 反馈 |
| ogsql-parser 不支持某些 GaussDB 特性 | 高 | 中 | 提 issue 上游；短期在 adapter 层 try/catch 降级为 `UniversalNode::Unknown` |
| sqlparser-rs 不支持 PolarDB 特有语法 | 高 | 中 | 自定义 Dialect impl；必要时 fork；上游 PR |
| UniversalNode 映射工作量大（180+ ogsql variants） | 中 | 中 | MVP 仅覆盖核心 ~30 variants，剩余降级为通用节点 |
| 三 parser 路径导致包体积/编译时间增加 | 低 | 低 | ogsql-parser 与 sqlparser-rs 都是纯 Rust，无 C 依赖；可 feature gate |
| tree-sitter-sequel 与 ogsql-parser 对同一 SQL 解析结果不一致 | 中 | 中 | 测试用 `dialect: standard` 与 `dialect: gaussdb` 对照；规则按方言生效，不会同时触发 |

---

## 10. 成功标准

完成以下全部条件即视为方案交付：

- [ ] `cargo run -- analyze --dialect gaussdb tests/categories/sql_dialects/gaussdb/` 正确识别 MERGE/PREDICT/TIMECAPSULE/SHRINK/CREATE PACKAGE 等特有语法
- [ ] `cargo run -- analyze --dialect opengauss ...` 在集中式/分布式模式下行为区分正确
- [ ] `cargo run -- analyze --dialect polardb-mysql tests/categories/sql_dialects/polardb_mysql/` 正确识别 GLOBAL INDEX / versioned comment / DBPARTITION
- [ ] `cargo run -- analyze --dialect standard tests/categories/patterns/sql/` 行为与不加 `--dialect` 一致（向后兼容）
- [ ] 20+ GaussDB 兼容性规则 + 5+ PolarDB-MySQL 规则通过测试
- [ ] `cargo test --workspace` 全量通过
- [ ] `cargo fmt --all -- --check` 无 diff（M-FMT-01）
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` 通过
- [ ] `cargo doc --workspace --no-deps` 无 warning（M-DOC-01）
- [ ] 所有新增 .rs 文件 ≤ 600 行，理想 ≤ 400 行（M-ARCH-03）
- [ ] 所有新增公开 enum/struct 加 `#[non_exhaustive]`（M-TYP-03）
- [ ] 库代码无 `anyhow::Result`，统一 thiserror 错误类型（M-ERR-01）
- [ ] AGENTS.md / README.md / docs/sql-dialects.md 更新完成

---

## 11. 编码规范合规性（CI 强制门禁）

本方案严格遵循 `docs/CONTRIBUTING.md`（必须遵循）与 `docs/BEST-PRATICE.md`（推荐遵循）。下表列出关键合规点：

### 11.1 必须遵循（CONTRIBUTING.md）

| 规则 ID | 要求 | 在本方案中的应用 |
|---|---|---|
| **M-ARCH-01** | Cargo Workspace 分层，禁反向依赖 | 新模块全部加在 `astgrep-parser`（adapter 层），不污染 `astgrep-core` |
| **M-ARCH-02** | core 层零 IO | `SqlDialect` enum 仅在 `astgrep-core/types.rs` 定义，无 IO |
| **M-ARCH-03** | 单文件 ≤ 600 行（理想 ≤ 400） | OgsqlAdapter **强制拆分**为 `ogsql/{mod,dml,ddl,features}.rs`，每文件 ≤ 400 行 |
| **M-ARCH-04** | 入口文件 ≤ 200 行 | 不修改入口；新增 trait 在独立文件 |
| **M-MOD-01** | 统一模块布局风格 | 项目已用 `mod.rs` 子目录（`language_discovery/`、`tree_sitter_parser/`），新增 `dialect/`、`adapter/` 保持一致 |
| **M-MOD-03** | lib.rs 重导出公开 API | 在 `astgrep-parser/src/lib.rs` 加 `pub use dialect::{SqlDialectParser, dispatch};` 等 |
| **M-FMT-01-05** | rustfmt 强制 | 任务 5.3 验证步骤包含 `cargo fmt --check` |
| **M-NAM-01-05** | 命名规范 | `parse_to_universal`（verb_noun）、`parser_family()`（无 get_ 前缀）、`as_str()`（借用语义） |
| **M-TYP-01** | 禁止裸 `as` | 字符串处理用 `to_ascii_lowercase()` 等方法，不用 `as` 强转 |
| **M-TYP-03** | 公开类型加 `#[non_exhaustive]` | `SqlDialect`、`SqlParserFamily`、`OgsqlAdapterError` 全部加上 |
| **M-ERR-01** | 库禁 anyhow，用 thiserror | `OgsqlAdapterError`、`SqlparserAdapterError` 用 thiserror 定义 |
| **M-ERR-02** | 库禁 unwrap | POC 用 `todo!()`，正式实现用 `?` + Result 传播 |
| **M-ERR-05** | pub Result 函数加 # Errors 文档 | 见 OgsqlAdapter::parse_to_universal 文档注释 |
| **M-ERR-06** | impl From 而非 Into | 错误转换用 `#[from]` 自动生成 From |
| **M-LOG-01** | tracing 而非 log | 新增日志统一 `use tracing::{info, warn, error, debug};`，禁止 `println!`、`eprintln!` 进入生产代码 |
| **M-DEP-01** | 禁通配符 | ogsql-parser 必须指定 `rev` 或 `tag`；sqlparser-rs 用具体版本 `"0.53"` |
| **M-DEP-04** | cargo-deny CI 检查 | ogsql-parser (MIT/Apache) + sqlparser (Apache-2.0) 均通过 license gate |
| **M-DEP-05** | 用 features 不用 --cfg | 暂未引入新 feature flag；后续如方言独立 feature 化，遵循此规则 |
| **M-DOC-01** | pub API 加文档注释 | 所有 pub fn/struct/enum/trait 加 `///` 文档；任务 5.3 验证 `cargo doc` 无 warning |
| **M-DOC-04** | FIXME/TODO 任务系统跟踪 | `todo!()` 仅 POC 用，正式代码不允许遗留；任何必要的 TODO 转 GitHub issue |
| **M-SEC-01** | 评估依赖供应链 | ogsql-parser（用户自有，可控）+ sqlparser-rs（Apache 项目，被 DataFusion/Polars/ParadeDB 使用）均通过 |

### 11.2 推荐遵循（BEST-PRATICE.md）

| 规则 ID | 建议 | 在本方案中的应用 |
|---|---|---|
| **R-ARCH-03** | Sealed Trait | `SqlDialectParser` trait 可考虑 sealed（防止外部实现），但允许下游扩展方言时不适用 — **视实现决定** |
| **R-ERR-02** | anyhow 适合 bin 层 | CLI 命令层（`astgrep-cli`）可用 anyhow，库层（adapter）禁用 |
| **R-TST-02** | proptest 解析器 | OgsqlAdapter 可加 proptest：随机生成 SQL → 解析 → 验证 invariant（建议但非强制） |
| **R-TST-05** | cargo-udeps 检测未用依赖 | 任务 5.3 可选添加 |
| **R-DOC-01** | ADR | 本方案文档即 ADR |
| **R-DOC-04** | 代码自注释，文档干练 | 注释只在不能从代码自明的"为什么"上加，不解释"是什么" |

### 11.3 实施时合规检查清单（每个 PR 必跑）

```bash
# 提交前本地必跑
cargo fmt --all -- --check                                # M-FMT-01
cargo clippy --workspace --all-targets -- -D warnings     # 综合
cargo test --workspace                                    # M-ERR-02, M-TYP-06
cargo doc --workspace --no-deps 2>&1 | grep "^warning"    # M-DOC-01（输出应为空）

# 文件大小检查（M-ARCH-03）
find crates -name "*.rs" -exec wc -l {} \; | sort -rn | head -20
# 任何 > 600 行的文件需在 PR 描述中说明或拆分
```

---

## 12. 后续可扩展项（非本期范围）

| 扩展 | 触发条件 | 实施路径 |
|---|---|---|
| PolarDB-PG 方言 | PolarDB-PG 业务需求 | 复用 GaussDBDialect 路径 + polar_* GUC 识别（ogsql-parser 95% 兼容） |
| PL/pgSQL 深度分析 | warpdriver 集成需求 | 利用 ogsql-parser 的 analyzer 模块（validate_pl_variables / analyze_transactions / compute_query_fingerprints） |
| 跨语言 SQL 字符串提取（Java/Python） | 真实场景验证需要 | 直接调用 ogsql-parser 已有的 `parse-java` / `parse-xml` 功能，不在 astgrep 端重做 |
| Oracle 方言 | 业务迁移需求 | 评估是否可用 ogsql-parser 扩展（ogsql 已有 Oracle 风格 PACKAGE） |
| 方言自动推断 | 大量 SQL 文件批处理需求 | 关键字启发式（CREATE PACKAGE → GaussDB；DBPARTITION → PolarDB-X） |

---

## 13. 参考资料

- **ogsql-parser**: https://github.com/c2j/ogsql-parser
- **ogsql-parser crate docs**: https://github.com/c2j/ogsql-parser/tree/main/docs
- **sqlparser-rs**: https://github.com/apache/datafusion-sqlparser-rs
- **sqlparser-rs AST docs**: https://apache.github.io/arrow-datafusion-sqlparser-rs/sqlparser_ast/
- **tree-sitter-sql (DerekStride)**: https://github.com/DerekStride/tree-sitter-sql
- **GaussDB 文档**: https://support.huaweicloud.com/
- **OpenGauss 文档**: https://docs.opengauss.org/
- **PolarDB 文档**: https://help.aliyun.com/product/58609.html
- **PolarDB-X 文档**: https://help.aliyun.com/zh/polardb/polardb-x/
- **现有一致性方案**: `.sisyphus/plans/phase1-consistency-quick-wins.md`
- **现有引擎方案**: `.sisyphus/plans/phase2-engine-fixes.md`

---

## 附录 A：ogsql-parser 集成 API 速查

```rust
// 基础解析
use ogsql_parser::{Tokenizer, Parser};
let tokens = Tokenizer::new(sql).tokenize()?;
let statements = Parser::new(tokens).parse()?;

// AST 访问
use ogsql_parser::ast::visitor::{Visitor, VisitorResult, walk_statement};
struct MyVisitor;
impl Visitor for MyVisitor {
    fn visit_expr(&mut self, expr: &Expr) -> VisitorResult { ... }
}

// 语义分析
use ogsql_parser::{
    validate_merge_semantics, validate_pl_variables,
    validate_package_consistency, compute_query_fingerprints,
    analyze_transactions,
};

// Schema-aware 校验
use ogsql_parser::analyzer::schema::load_full_schema;
let schema = load_full_schema("schema.json")?;
```

## 附录 B：sqlparser-rs 集成 API 速查

```rust
use sqlparser::{parser::Parser, dialect::MySqlDialect};

let dialect = &MySqlDialect {};
let ast = Parser::parse_sql(dialect, "SELECT * FROM users")?;

// 自定义方言（PolarDB 扩展）
struct PolarDBMyDialect;
impl sqlparser::dialect::Dialect for PolarDBMyDialect {
    fn is_identifier_start(&self, ch: char) -> bool { MySqlDialect.is_identifier_start(ch) }
    fn is_identifier_part(&self, ch: char) -> bool { MySqlDialect.is_identifier_part(ch) }
    // ... override 关键 method 添加 PolarDB 关键字识别
}
```
