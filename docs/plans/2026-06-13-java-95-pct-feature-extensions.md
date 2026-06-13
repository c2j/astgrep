# Java 95% 通过率 — 子系统功能扩展计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 Java guardian 通过率从 82.8% (144/174) 提升到 95%，修复剩余 30 个失败测试。

**Architecture:** 30 个失败分布在 6 个子系统中，每个子系统需要功能扩展而非 bug 修复。按影响面和依赖关系排序：Pattern Parser → Constant Propagation → Import Resolution → Equivalence → Taint Analysis → Rules Layer。

**Tech Stack:** Rust, tree-sitter-java 0.23.5, regex crate

---

## 依赖关系图

```
Pattern Parser (7 tests)
    ├──→ 解锁 cp_string_format, cp_synchronized1
    └──→ 解锁 dots_interface, misc_record_pattern 等结构测试

Constant Propagation (6 tests)
    ├── 依赖: Pattern Parser (cp_string_format 等)
    └──→ 解锁 cp_is_must_analysis, cp_switch_throw, cp_try_return

Import Resolution (3 tests)
    └──→ 解锁 aliasing_type, better_import3/4

Equivalence (3 tests)
    ├── 依赖: Constant Propagation
    └──→ 解锁 equivalence_constant_propagation*

Taint Analysis (6 tests) — 独立
    └──→ 解锁 taint_lambda1, taint_if, taint_seq 等

Rules Layer (5 tests) — 独立
    └──→ 各 rule executor 独立修复
```

---

## Phase 1: Pattern Parser 增强 (7 tests → 0)

### 现状

`crates/astgrep-matcher/src/parser.rs` (741 行) 是自研 tokenizer，不支持：
- Java keyword (`record`, `@interface`)
- 复合省略号（`{ ... pattern ... }`）
- string literal 作为方法参数（`foobar("...")` 被解析为 flat literals）
- typed metavar 作为方法接收者（`(Type $VAR).method()`）

### Task 1.1: 修复 `foobar("...")` — string literal 参数

**Files:**
- Modify: `crates/astgrep-matcher/src/parser.rs:362-377`

**方案:** 当 `LeftParen` 跟随 `Literal`（方法调用模式）时，不将 `(` `)` 作为 literal 推入 patterns，而是将括号内的内容作为结构化 pattern 处理。当前行为是把 `foobar("...")` 解析为 `[Literal("foobar"), Literal("("), Literal("..."), Literal(")")]`。修复后应为 `[Literal("foobar"), <argument pattern>]`。

**具体实现:**
```rust
// parser.rs parse_sequence(), LeftParen 分支
Token::LeftParen => {
    if pos > start && matches!(tokens[pos - 1], Token::Literal(_)) {
        // 方法调用: 收集括号内参数，构建 CallPattern
        pos += 1; // skip '('
        let mut arg_patterns = Vec::new();
        let mut paren_depth = 1;
        while pos < tokens.len() && paren_depth > 0 {
            match &tokens[pos] {
                Token::RightParen => { paren_depth -= 1; if paren_depth == 0 { break; } }
                Token::LeftParen => paren_depth += 1,
                Token::Comma => { pos += 1; continue; }
                _ => {
                    let (arg, new_pos) = self.parse_atom(tokens, pos)?;
                    arg_patterns.push(arg);
                    pos = new_pos - 1;
                }
            }
            pos += 1;
        }
        pos += 1; // skip ')'
        // 将 foobar + args 组合: Literal("foobar") + args
        patterns.extend(arg_patterns);
        continue;
    } else {
        // grouping: (a | b)
        let (nested, new_pos) = self.parse_parenthesized_group(tokens, pos)?;
        patterns.push(nested);
        pos = new_pos;
    }
}
```

**验证:** `cargo test -p astgrep-matcher` 后，手动测试 `foobar("...")` 匹配 `foobar(str)`。

**影响:** 解锁 `cp_string_format` (4 missed), `cp_synchronized1` (2 missed)。

---

### Task 1.2: 修复复合省略号 — `{ ... pattern ... }`

**Files:**
- Modify: `crates/astgrep-matcher/src/advanced_matcher.rs` — `try_match_ast_at_offset`

**方案:** 当 pattern 中有 `...` 后跟具体 pattern（如 `{ ... $RETURNTYPE $METHOD(...) }`），当前匹配器不支持省略号跳过内容后继续匹配。需要在 `try_match_ast_at_offset` 中为 `ParsedPattern::Wildcard` 后面的 pattern 添加"扫描匹配"模式。

**具体实现:**
```rust
// 在 try_match_ast_at_offset 的 Wildcard 分支中
// 当前: 只尝试不同 skip 长度
// 修复: 当 remaining 非空时，对每个 skip 位置尝试匹配 remaining，
// 而不是只匹配完 Wildcard 就结束
ParsedPattern::Wildcard => {
    for skip in 0..=(children.len().saturating_sub(child_offset)) {
        if !remaining.is_empty() {
            // 尝试在 skip 位置匹配 remaining
            if self.try_match_ast_at_offset(remaining, children, child_offset + skip, parent_node, depth)? {
                return Ok(true);
            }
        } else {
            return Ok(true); // 末尾 ... 匹配所有剩余
        }
    }
    Ok(false)
}
```

**影响:** 解锁 `dots_interface` (1 missed), `misc_record_pattern` (2 missed), `misc_at_interface2` (1 missed)。

---

### Task 1.3: 支持 `record` 和 `@interface` keyword

**Files:**
- Modify: `crates/astgrep-matcher/src/parser.rs` — tokenizer

**方案:** 在 tokenizer 中添加 `record` 和 `@interface` 的识别。`record` 应作为 keyword token 而非 literal。`@interface` 应识别 `@` 后跟 `interface`。

**具体实现:**
```rust
// tokenizer 中新增 '@' 处理
'@' => {
    let mut name = String::from("@");
    while let Some(&ch) = chars.peek() {
        if ch.is_alphanumeric() || ch == '_' {
            name.push(chars.next().unwrap());
        } else { break; }
    }
    tokens.push(Token::Literal(name)); // "@interface" 作为特殊 literal
}

// parse_sequence 中识别 record 关键字
Token::Literal(s) if s == "record" => {
    // 后面的 $R(...) { ... } 按 record 声明处理
    // ...
}
```

**影响:** 解锁 `misc_record_pattern` (如果 Task 1.2 未完全修复), `misc_at_interface2`。

---

### Task 1.4: 支持 typed metavar 作为方法接收者

**Files:**
- Modify: `crates/astgrep-matcher/src/parser.rs:484-516`

**方案:** `($X.InitialDirContext $IDC).search(...)` 这种 pattern 需要 parser 识别 `(Type $VAR)` 后跟 `.method()` 的语法。当前 `parse_parenthesized_group` 能识别 typed metavar 但后续的 `.method()` 解析失败。

**具体实现:**
在 `parse_sequence` 中，当解析完 typed metavar 后，检查下一个 token 是否为 `.`：
```rust
// 在 parse_sequence 中，处理完一个 pattern 后
if pos < tokens.len() && matches!(&tokens[pos], Token::Literal(s) if s == ".") {
    // typed metavar 后跟方法调用
    pos += 1; // skip '.'
    if pos < tokens.len() {
        if let Token::Literal(method_name) = &tokens[pos] {
            // 构建 method_call(typed_metavar, method_name, args)
        }
    }
}
```

**影响:** 解锁 `metavar_typed_qualified` (1 missed), `typed_metavar_class` (2 missed)。

---

## Phase 2: Constant Propagation 增强 (6 tests → 0)

**依赖:** Phase 1 (Task 1.1 解锁 cp_string_format, cp_synchronized1)

### Task 2.1: 区分字段赋值 vs 局部变量

**Files:**
- Modify: `crates/astgrep-dataflow/src/constant_propagation/analysis.rs:250-255`

**问题:** `process_local_assignment` 在 Method context 中将字段赋值 `this.str = "hello"` 当作局部变量处理，导致条件分支中的赋值也被传播。

**方案:** 在 `process_local_assignment` 中检查变量名是否对应已知字段。如果是字段，不进行局部变量传播。

**具体实现:**
```rust
fn is_field_assignment(&self, var_name: &str, node: &dyn AstNode) -> bool {
    // 检查赋值左侧是否包含 this. 前缀
    let left = node.child(0);
    if let Some(left_node) = left {
        if left_node.node_type() == "field_access" {
            return true;
        }
        if let Some(text) = left_node.text() {
            if text.starts_with("this.") {
                return true;
            }
        }
    }
    false
}
```

**影响:** 解锁 `cp_is_must_analysis` (3 extra), `cp_is_must_analysis2` (3 extra)。

---

### Task 2.2: 控制流感知的常量传播

**Files:**
- Modify: `crates/astgrep-dataflow/src/constant_propagation/analysis.rs:243-261`

**问题:** `cp_switch_throw` 和 `cp_try_return` 需要常量传播理解 switch/throw 和 try/return 的控制流。

**方案:** 在 `visit_node_with_context` 中，跟踪当前是否在条件分支内。如果是，标记赋值为 "可能" 而非 "确定"。

**具体实现:**
```rust
// 在 ConstantPropagator 中添加字段
conditional_depth: usize,

// 在 visit_node_with_context 中
if node.node_type() == "if_statement" 
    || node.node_type() == "switch_statement" 
    || node.node_type() == "try_statement" 
{
    self.conditional_depth += 1;
}

// 处理子节点后
if is_conditional_node {
    self.conditional_depth -= 1;
}

// 在 process_local_assignment 中
if self.conditional_depth > 0 {
    // 条件赋值: 不传播为确定常量
    return Ok(());
}
```

**影响:** 解锁 `cp_switch_throw` (1 missed + 1 extra), `cp_try_return` (1 missed)。

---

## Phase 3: Import Resolution (3 tests → 0)

**依赖:** 无

### Task 3.1: 构建 import map

**Files:**
- Create: `crates/astgrep-matcher/src/import_resolver.rs`
- Modify: `crates/astgrep-matcher/src/advanced_matcher.rs`

**方案:** 在匹配前解析 Java 源文件的 import 语句，构建简单名 → 全限定名的映射表。匹配时将全限定名与模式中的全限定名比较。

**具体实现:**
```rust
pub struct ImportResolver {
    imports: HashMap<String, String>, // simple_name -> fully_qualified_name
    star_imports: Vec<String>,        // "java.util.*" style imports
    same_package: String,
}

impl ImportResolver {
    pub fn from_source(source: &str, file_path: &Path) -> Self {
        let imports = HashMap::new();
        let re = Regex::new(r"import\s+(static\s+)?([\w.]+)(\.\*)?;").unwrap();
        for cap in re.captures_iter(source) {
            let full = &cap[2];
            let is_star = cap.get(3).is_some();
            if is_star {
                star_imports.push(full.to_string());
            } else {
                let simple = full.split('.').last().unwrap();
                imports.insert(simple.to_string(), full.to_string());
            }
        }
        // ...
    }
    
    pub fn resolve(&self, simple_name: &str) -> Option<String> {
        if let Some(full) = self.imports.get(simple_name) {
            return Some(full.clone());
        }
        // Try star imports
        for star in &self.star_imports {
            // In practice, we can't resolve star imports without classpath
            // Accept match if the simple name could come from a star import
        }
        None
    }
}
```

**验证:** 在 `advanced_matcher.rs` 的 `find_matches` 中集成，匹配前调用 `resolver.resolve()` 比较全限定名。

**影响:** 解锁 `aliasing_type` (11 missed), `better_import3` (7 missed), `better_import4` (6 missed + 12 extra)。

---

## Phase 4: Equivalence 增强 (3 tests → 0)

**依赖:** Phase 2 (Constant Propagation)

### Task 4.1: 常量 + 等价性组合

**Files:**
- Modify: `crates/astgrep-matcher/src/advanced_matcher.rs` — `match_metavariable`

**方案:** Phase 2 使常量传播正确工作后，`match_metavariable` 中的常量值绑定（已在本次 PR 中添加）将自动处理 `$X == $X` 对 `foo == null` 的等价性匹配。但还需处理模式 `foo("password")` 中的 string literal 参数问题（依赖 Task 1.1）。

**额外修复:** 在 `match_metavariable` 中，当绑定常量值时，同时记录原始文本用于报告。

**影响:** 解锁 `equivalence_constant_propagation` (1 missed), `equivalence_constant_propagation2` (2 missed), `equivalence_constant_propagation_field` (2 missed)。

---

## Phase 5: Taint Analysis 增强 (6 tests → 0)

**依赖:** 无（独立子系统）

### Task 5.1: Lambda 表达式中的污点传播

**Files:**
- Modify: `crates/astgrep-dataflow/src/taint.rs`
- Modify: `crates/astgrep-dataflow/src/call_graph.rs`

**方案:** 当前 taint tracker 的 call graph 不包含 lambda 表达式作为调用边。需要识别 `request -> lambda param -> lambda body -> sink` 的路径。

**具体实现:**
```rust
// 在 call_graph.rs 中，为 lambda 表达式添加调用边
if node.kind() == "lambda_expression" {
    // lambda 参数 → lambda body 是数据流边
    for param in lambda_params {
        for usage in body_usages {
            graph.add_edge(param, usage, EdgeType::DataFlow);
        }
    }
}
```

**影响:** 解锁 `taint_lambda1` (4 missed + 4 extra)。

---

### Task 5.2: if/switch 分支中的污点传播

**Files:**
- Modify: `crates/astgrep-dataflow/src/enhanced_taint.rs`

**方案:** 在 if/else 分支中，两个分支的赋值应该都传播到汇聚点。当前只追踪单一路径。

**具体实现:**
```rust
// 在 enhanced_taint.rs 中，处理 if_statement 时
fn track_through_if(&mut self, if_node: &Node) {
    let mut then_state = self.state.clone();
    let mut else_state = self.state.clone();
    self.track_block(&mut then_state, then_body);
    self.track_block(&mut else_state, else_body);
    // 汇聚: 合并两个分支的状态
    self.state = then_state.merge(&else_state);
}
```

**影响:** 解锁 `taint_if` (1 missed + 1 extra), `taint_seq` (1 missed + 1 extra)。

---

### Task 5.3: try/return 中的污点传播

**Files:**
- Modify: `crates/astgrep-dataflow/src/enhanced_taint.rs`

**方案:** try 块中的 return 不应阻止 catch 块的污点传播。当前 return 可能使 catch 块被标记为不可达。

**影响:** 解锁 `try_return` (1 extra), `taint_best_fit_sink6` (1 missed + 1 extra), `tainted_args` (1 missed + 1 extra)。

---

## Phase 6: Rules Layer (5 tests → 0)

**依赖:** 无（各自独立）

### Task 6.1: 私有属性命名检查

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/conditions.rs`

**方案:** `cp_private_class_attr2/3` 和 `naming_class_attribute` 测试私有属性的命名规范。当前检查过于宽松（对所有 private 字段发出警告）。

**具体实现:** 检查具体的命名模式（如 `$X` 必须匹配特定正则），而非仅检查修饰符。

**影响:** 解锁 `cp_private_class_attr2` (1 extra), `cp_private_class_attr3` (3 extra), `naming_class_attribute` (1 extra)。

---

### Task 6.2: Metavar 名称解析

**Files:**
- Modify: `crates/astgrep-rules/src/executor/core/mod.rs` — `preprocess_typed_metavariables`

**方案:** `metavariable_name_resolution` 测试 `($FOO $VAR).bar()` 中 `$FOO` 应解析为 `org.foo.Foo`。当前预处理器的正则不处理这种嵌套 typed metavar 作为接收者的情况。

**影响:** 解锁 `metavariable_name_resolution` (1 missed)。

---

### Task 6.3: 深度符号传播

**Files:**
- Modify: `crates/astgrep-dataflow/src/symbolic_propagation.rs`

**方案:** `sym_prop_deep` 测试深度符号传播（通过多层方法调用跟踪类型）。当前传播深度不足。

**具体实现:** 增加传播迭代次数或深度限制。

**影响:** 解锁 `sym_prop_deep` (1 missed + 2 extra)。

---

## 工作量估算

| Phase | 子系统 | 测试数 | 预估时间 | 难度 |
|---|---|---|---|---|
| 1 | Pattern Parser | 7 | 3-5 天 | ★★★★ |
| 2 | Constant Propagation | 6 | 2-3 天 | ★★★ |
| 3 | Import Resolution | 3 | 1-2 天 | ★★★ |
| 4 | Equivalence | 3 | 0.5 天 | ★★ (依赖 Phase 2) |
| 5 | Taint Analysis | 6 | 3-5 天 | ★★★★★ |
| 6 | Rules Layer | 5 | 1-2 天 | ★★ |
| **总计** | | **30** | **10-17 天** | |

## 建议执行顺序

1. **Phase 1 Tasks 1.1 + 1.2** — 最高 ROI，解锁最多测试
2. **Phase 2** — 依赖 Phase 1.1，随后立即执行
3. **Phase 4** — 依赖 Phase 2，自动化通过
4. **Phase 3** — 独立，可与 Phase 1-2 并行
5. **Phase 6** — 独立，可随时穿插
6. **Phase 5** — 最复杂，最后做

## 验证标准

每个 Phase 完成后运行:
```bash
cargo build --release && python3 newtest/scripts/guardian_runner.py --language java
```

目标: Java 通过率 ≥ 95% (≥165/174)。
