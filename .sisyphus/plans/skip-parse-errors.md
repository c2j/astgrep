# Plan: 解析失败时跳过单个文件而非中断全部分析

## 问题

当前 `astgrep analyze --dialect gaussdb` 在某个 SQL 文件的 ogsql-parser tokenizer
解析失败时（如 `unterminated dollar-quoted string`），`?` 操作符会将错误传播到顶层，
导致整个分析中断退出。55 个文件中只要 1 个解析失败，其余 54 个也无法分析。

用户要求：解析失败的文件跳过，继续处理其他文件，最终报告中列出失败文件及原因。

## 当前代码路径

```
analyze_enhanced/mod.rs:78
  → analyze_file_simple(file_path, config, &mut all_findings, &mut analysis_stats)?
    → analyze_with_rule_engine(...)?  [line 158]
      → dialect_parser.parse(source_code, Path::new(file_path))?  [line 260] ← 这里抛错
      → parser.parse(source_code, Path::new(file_path))?          [line 268] ← 通用路径
```

两处 `?` 都在 `analyze_with_rule_engine` 内部，该函数签名是 `Result<(Vec<Finding>, usize)>`。

## 现状：已有基础设施

`AnalysisStatistics` 已经包含 `parse_errors: usize` 字段，且所有 6 种输出格式
（Text/JSON/SARIF/HTML/Markdown/Semgrep）在 format 时都会读取 `stats.parse_errors`。
Text 格式已有输出：
```
Parse errors: N
```

**缺失的**：没有人实际去 catch 解析错误并递增 `parse_errors` 计数器。

## 改动方案

### 1. 修改 `analyze_file_simple` (mod.rs:78)

在 `run_enhanced` 的循环中将 `?` 改为 match：

```rust
// 改前 (line 78):
analyze_file_simple(&file_path, &config, &mut all_findings, &mut analysis_stats)?;

// 改后:
match analyze_file_simple(&file_path, &config, &mut all_findings, &mut analysis_stats) {
    Err(e) => {
        warn!("Skipping {}: {}", file_path.display(), e);
        analysis_stats.parse_errors += 1;
        // 继续处理下一个文件
    }
    Ok(()) => {}
}
```

但这里问题：`analyze_file_simple` 返回的错误可能是 IO 错误、规则加载错误等，
不全是"解析错误"。我们需要区分解析失败 vs 其他致命错误。

**更精确的方案**：只 catch 解析错误，其他错误仍 propagate。

### 2. 在 `analyze_with_rule_engine` 中 catch 解析错误 (mod.rs:253-269)

将解析错误从 `Result` 返回改为写入 stats：

```rust
// 改前:
let ast = if language == Language::Sql {
    match config.sql_dialect {
        Some(dialect) if dialect != astgrep_core::SqlDialect::Standard => {
            let dialect_parser = astgrep_parser::dialect::dispatch(dialect);
            dialect_parser.parse(source_code, Path::new(file_path))?
        }
        _ => {
            let default_ast = parser.parse(source_code, Path::new(file_path))?;
            try_tree_sitter_ast(source_code, language).unwrap_or(default_ast)
        }
    }
} else {
    parser.parse(source_code, Path::new(file_path))?
};

// 改后: 解析失败时返回空 Vec + 在调用处 catch
// 但 analyze_with_rule_engine 不持有 stats...
```

### 3. 推荐方案：只在调用层 catch

改动点最小、最安全 —— 只改 `run_enhanced` 循环 (line 78)：

```rust
// run_enhanced, in the for loop:
for file_path in target_files {
    // ...
    match analyze_file_simple(&file_path, &config, &mut all_findings, &mut analysis_stats) {
        Ok(()) => {}
        Err(e) => {
            let msg = e.to_string();
            // 判断是否为解析错误（DialectParseError 以 "parse failed for dialect" 开头）
            // 格式: `parse failed for dialect 'GaussDB': ...`
            if msg.starts_with("parse failed for dialect") 
               || msg.contains("parse failed") 
            {
                warn!("Skipping file {}: {}", file_path.display(), msg);
                analysis_stats.parse_errors += 1;
                continue;
            }
            // 其他错误（IO、规则加载等）仍 abort
            return Err(e);
        }
    }
    // ...
}
```

**优点**：
- 只改 6 行代码
- `parse_errors` 已存在且输出格式已支持
- 错误消息通过 `warn!` 输出到 stderr
- 非解析错误（IO、规则语法错误等）保持 abort 行为

**缺点**：
- 通过字符串匹配判断是否为解析错误，不够优雅
- 解析错误的具体信息只在 stderr 日志中，不在最终报告里

### 4. 增强：最终报告中列出失败文件

新增一个 `parse_error_files: Vec<(PathBuf, String)>` 传给输出层。
但这需要改动 `OutputFormatter::format` 签名和所有 6 种输出格式。

**阶段 1（本 PR）**：只实现方案 3 —— catch + skip + stderr 日志。
**后续**：如果需要报告里列出失败文件详情，可扩展现有 `OutputFormatter` trait。

## 受影响文件

| 文件 | 改动 |
|------|------|
| `crates/astgrep-cli/src/commands/analyze_enhanced/mod.rs` | line ~78: `?` → match/catch + continue |
| 无 | 其余无改动 |

## 验证方式

1. 用当前失败的 SQL 目录重现命令，确认：
   - `complex_clearing_pkg.sql` 被跳过（stderr 有 warning）
   - 其余 54 个文件正常分析
   - 最终输出报告包含 `Parse errors: 1`
   - 退出码为 0（除非有 findings + `--fail-on-findings`）
2. 用正常 SQL 文件测试，确认行为不变
3. 故意写入语法错误的 SQL，确认被跳过而非 abort
