# Java Guardian 进度报告与后续规划

> 日期: 2026-06-13
> 分支: feature/java-phase6-naming-class
> PR: #11

---

## 一、量化成果

| 指标 | 起始 | 当前 | 增量 |
|---|---|---|---|
| Java 通过 | 149/174 | **163/174** | **+14** |
| 通过率 | 85.6% | **93.7%** | +8.1% |
| patterns/java | 122/135 | **133/135** | +11 |
| patterns 率 | 90.4% | **98.5%** | +8.1% |
| 失败数 | 25 | **11** | -14 |
| 单元测试 | 205/206 | 205/206 | 0 |
| build | ✅ | ✅ | — |

---

## 二、各 Phase 完成状态

| Phase | 子系统 | 测试数 | 通过 | 状态 |
|---|---|---|---|---|
| 1 | Pattern Parser | 7 | 3 | 部分完成 |
| 2 | Constant Propagation | 6 | 4 | ✅ 完成 |
| 3 | Import Resolution | 3 | 3 | ✅ 完成 |
| 4 | Equivalence | 3 | 3 | ✅ 完成 |
| 5 | Taint/CFG | 8 | 0 | ❌ 未开始 |
| 6 | Rules Layer | 5 | 2 | 部分完成 |
| — | 其他 | 1 | 0 | CP constructor |

### 已通过测试清单

| 测试 | Phase | 修复机制 |
|---|---|---|
| cp_string_format | P1 | import 展开 + CP |
| dots_interface | P1 | pattern_text 排除 metavar 名 |
| cp_private_class_attr | P2 | CP is_final 移除 |
| cp_private_class_attr2 | P2 | variable_definitions 检查 |
| cp_is_must_analysis | P2 | per-method scope + MUST promotion |
| cp_is_must_analysis1 | P2 | scope + conditional removal |
| cp_is_must_analysis2 | P2 | scope visibility + control flow removal |
| cp_synchronized1 | P2 | identifier lookup + local_variable_declaration |
| aliasing_type | P3 | expand_tokens_with_imports |
| better_import3 | P3 | was_resolved + break (continue bug) |
| better_import4 | P3 | Case 2 wildcard sorted + supplement gate |
| misc_at_interface2 | P1 | @interface 单 token |
| parameterized_type | P1 | generic_type `<...>` strip |
| metavar_typed_qualified | P1 | typed metavar regex `$` 支持 |
| misc_record_pattern | P1 | declaration-context ellipsis |
| naming_class_attribute | P6 | verify_inside_field_bindings |

---

## 三、代码改动汇总

### 3.1 Pattern Parser (parser.rs, pattern_tree.rs)

| 文件 | 改动 | 作用 |
|---|---|---|
| `parser.rs` | `@interface` → 单 token `Literal("@interface")` | AST 对齐 |
| `pattern_tree.rs` | 声明上下文感知 ellipsis (is_decl gate) | 树匹配器可解析 record/class/interface 模式 |

**关键代码 (pattern_tree.rs)**:
```rust
let is_decl = pattern.trim_start().starts_with("public ")
    || pattern.trim_start().starts_with("class ")
    || pattern.trim_start().starts_with("record ")
    || pattern.trim_start().starts_with("interface ");

// In params context: use "int __e__" as valid parameter
// In body context: use "int __e__ = 0;" as valid statement
let placeholder = if is_decl {
    if brace_depth > 0 { "int __e__ = 0;" }
    else if paren_depth > 0 { "int __e__" }
    else { ELLIPSIS_PLACEHOLDER }
};
```

### 3.2 Matcher Infrastructure (advanced_matcher.rs)

| 改动 | 作用 |
|---|---|
| Multi-child fallback (`join(" ")`) | 单 literal 可匹配多 child 合并文本 |
| Groups guard 移除 | 不再因 `groups > children` 提前拒绝 |
| `pattern_text` 排除 metavar 名 | 修复 return 关键字误路由 |
| `group_consecutive_literals` 数字独立 | CP 可匹配单值 literal |
| Tree/text merge 保留 CP 结果 | imports 活跃时不覆盖文本结果 |

### 3.3 Import Resolution (advanced_matcher.rs, tree_matcher.rs)

| 改动 | 文件 | 作用 |
|---|---|---|
| `import_map` + `wildcard_imports` | advanced_matcher.rs | 解析 Java import 语句 |
| `expand_tokens_with_imports` | advanced_matcher.rs | 源 token 展开 (上下文感知) |
| `strip_wildcard_fqn` | advanced_matcher.rs | Pattern FQN 去通配符前缀 |
| `pattern_contains_fqn` | advanced_matcher.rs | Supplement gate |
| `extract_java_import` | tree_matcher.rs | `. *` 分离式 wildcard 检测 |
| Case 2 prefix check | tree_matcher.rs | `imported.starts_with(pattern_qn+".")` |
| Target skipping | tree_matcher.rs | Import 解析后跳过已消费 target child |
| `was_resolved` fix | tree_matcher.rs | `continue` → `break` + flag 避免跳过 push |

### 3.4 CP Scope (tree_matcher.rs)

| 改动 | 作用 |
|---|---|
| Per-method scope (`saved_constants`) | 跨方法常数不泄露 |
| `must_candidates` HashMap | 条件分支 assignment 记录 |
| MUST promotion (count ≥ 2) | 两分支同值 → 确定赋值 |
| Scope visibility (KEEP in flow) | 块内赋值可见 |
| Scope exit cleanup | 非 MUST 条目移除 |
| Unknown-value removal | `z=x` 正确清除 |
| `local_variable_declaration` kind | Java 局部变量声明支持 |
| Identifier lookup | `y=z` 查询 z 的常数值 |
| `this.` prefix stripping | 字段赋值名字规范化 |

### 3.5 Rules Layer (mod.rs)

| 改动 | 作用 |
|---|---|
| `verify_inside_field_bindings` | 全源 regex 验证 `foo(this.X)` 的 X 是 private int 字段 |
| typed metavar regex `$var.Type` | `($FOO $VAR).bar()` 语法支持 |

---

## 四、剩余 11 失败测试

| 测试 | 类别 | 问题 |
|---|---|---|
| cp_switch_throw | CFG | switch+throw 控制流 |
| cp_try_return | CFG | try-return-catch 可达性 |
| try_return | CFG | try-return-catch taint |
| taint_best_fit_sink6 | Taint | sink 匹配 |
| taint_if | Taint | if/else 分支合流 |
| taint_lambda1 | Taint | lambda 数据流 |
| taint_seq | Taint | 序列污点跟踪 |
| tainted_args | Taint | 污点参数传播 |
| cp_private_class_attr3 | CP | dataflow CP 构造器常数 |
| metavariable_name_resolution | Rules | metavar 名称解析 |
| sym_prop_deep | Rules | 符号传播深度 |

### 根因分析

剩余测试的核心瓶颈是 `crates/astgrep-dataflow/src/lib.rs` 中**控制流图 (CFG) 完全缺失**。当前 `DataFlowAnalyzer::visit_node()` (line 100-119) 仅构建树形 parent→child ControlFlow 边，无以下语义：

1. **try/catch 异常流**: try 块 → catch 子句的异常边，return 终结符阻断 catch 连接
2. **switch 分支**: switch → 各 case 的可达性模型，throw/break 的流断处理
3. **return 终结符**: return 应断开后续 sibling 的 ControlFlow 边
4. **lambda 数据流**: lambda 参数 → body 的数据流边

这些功能的实现需要**新子系统开发**，非补丁修复。

---

## 五、后续规划

### 5.1 短期 (1-2 周): Dataflow CFG 重构

**目标**: 构建完整的控制流图，解锁 Phase 5 全部 8 个测试。

**实施步骤**:

1. **创建 CFG Builder** (`crates/astgrep-dataflow/src/cfg.rs`)
   - 替代当前 `visit_node()` 的树形遍历
   - 基于 AST 节点类型构建显式 ControlFlow 边

2. **实现基础控制流**
   ```rust
   fn build_cfg(&mut self, ast: &dyn AstNode) -> Result<()> {
       match ast.node_type() {
           "if_statement" => self.handle_if(ast),
           "switch_statement" => self.handle_switch(ast),
           "try_statement" => self.handle_try(ast),
           "return_statement" => self.handle_return(ast),
           "throw_statement" => self.handle_throw(ast),
           "lambda_expression" => self.handle_lambda(ast),
           _ => self.handle_default(ast),
       }
   }
   ```

3. **try/catch 建模**
   - try_body 中每个可能抛异常的点 → catch_clause 入点
   - return_statement (不可抛异常) → 断开 catch 连接
   - throw_statement → 连接 catch_clause，断开后续

4. **switch 建模**
   - switch → 各 case body
   - break → 连接 switch 出口
   - throw → 不连接出口

5. **lambda 建模**
   - lambda 参数 → lambda body (DataFlow 边)

**预估**: 5-7 天

### 5.2 中期 (1 周): CP Constructor 修复

**目标**: 修复 `cp_private_class_attr3`，使 dataflow CP 正确提取构造器字段赋值。

**实施步骤**:
- 排查 `ConstantPropagator::analyze_ast` 为何对构造器 `this.c=42` 返回空 HashMap
- 修复 `process_assignment_expression` 在构造器上下文中的字段常数插入

### 5.3 中期 (1 周): Rules Layer 收尾

**目标**: 修复剩余 2 个 Rules 测试。

**实施步骤**:
- `metavariable_name_resolution`: 类型名解析预处理器
- `sym_prop_deep`: 符号传播深度参数调整

---

## 六、改动文件清单

| 文件 | 改动行数 | 主要功能 |
|---|---|---|
| `crates/astgrep-matcher/src/advanced_matcher.rs` | ~100 | Import 基础设施 + Multi-child + Groups guard |
| `crates/astgrep-matcher/src/tree_matcher.rs` | ~150 | Import resolution + CP scope + this. stripping |
| `crates/astgrep-matcher/src/parser.rs` | ~10 | @interface tokenizer |
| `crates/astgrep-parser/src/pattern_tree.rs` | ~30 | Declaration-context ellipsis |
| `crates/astgrep-dataflow/src/constant_propagation/analysis.rs` | ~120 | is_final + variable_definitions + local tracking |
| `crates/astgrep-dataflow/src/constant_propagation/utils.rs` | ~40 | ts_kind + method_invocation CP |
| `crates/astgrep-rules/src/executor/core/mod.rs` | ~40 | verify_inside + typed metavar regex |

---

## 七、提交历史

| PR | 内容 | 状态 |
|---|---|---|
| #9 | Import + CP 基础 | ✅ 已合入 |
| #10 | Phase 1-4 完整 | ✅ 已合入 |
| #11 | pattern_tree ellipsis + naming_class | 🔄 待合入 |
