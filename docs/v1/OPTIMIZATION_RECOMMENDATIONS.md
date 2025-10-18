# CR-SemService 优化建议详细方案

## 1. 跨函数数据流分析 (优先级: 🔴 高)

### 当前状态
- 仅支持单函数内的污点追踪
- 无法追踪函数调用链中的数据流
- 导致漏报率高 (20-30%)

### 优化方案

#### 1.1 构建调用图 (Call Graph)

```rust
// 新增文件: crates/cr-dataflow/src/call_graph.rs

pub struct CallGraph {
    // 函数节点: 函数签名 -> 函数定义
    functions: HashMap<FunctionSignature, FunctionDef>,
    // 调用边: 调用者 -> 被调用者列表
    calls: HashMap<FunctionId, Vec<FunctionCall>>,
    // 参数映射: 调用 -> 参数映射关系
    param_mappings: HashMap<CallId, ParameterMapping>,
}

impl CallGraph {
    pub fn build(ast: &dyn AstNode) -> Result<Self> {
        // 1. 收集所有函数定义
        // 2. 收集所有函数调用
        // 3. 建立调用关系
        // 4. 分析参数映射
    }
    
    pub fn trace_taint_through_calls(
        &self,
        source: &TaintSource,
        sink: &TaintSink,
    ) -> Result<Vec<TaintPath>> {
        // 使用BFS/DFS遍历调用图
        // 追踪污点通过函数调用的传播
    }
}
```

#### 1.2 参数映射分析

```rust
pub struct ParameterMapping {
    // 调用处的实参 -> 被调用函数的形参
    arg_to_param: HashMap<usize, usize>,
    // 返回值映射
    return_mapping: Option<ReturnMapping>,
}

impl ParameterMapping {
    pub fn map_taint(&self, taint: &TaintInfo) -> Option<TaintInfo> {
        // 将污点从实参映射到形参
        // 或从返回值映射回调用处
    }
}
```

#### 1.3 集成到数据流分析

```rust
// 修改: crates/cr-dataflow/src/lib.rs

pub struct DataFlowAnalyzer {
    graph: DataFlowGraph,
    call_graph: Option<CallGraph>,  // 新增
    // ... 其他字段
}

impl DataFlowAnalyzer {
    pub fn analyze_with_call_graph(&mut self, ast: &dyn AstNode) -> Result<DataFlowAnalysis> {
        // 1. 构建调用图
        let call_graph = CallGraph::build(ast)?;
        self.call_graph = Some(call_graph);
        
        // 2. 执行跨函数污点分析
        self.analyze_cross_function_taint()?;
        
        // 3. 返回分析结果
        Ok(self.build_analysis_result())
    }
}
```

### 预期收益
- 漏报率降低: 20-30%
- 准确率提升: 15-25%
- 性能影响: +10-20% (可通过缓存优化)

### 实现时间: 1-2周

---

## 2. 符号表和类型系统 (优先级: 🔴 高)

### 当前状态
- 基础符号表实现不完整
- 无类型推断能力
- 导致误报率高 (10-15%)

### 优化方案

#### 2.1 完整符号表实现

```rust
// 新增文件: crates/cr-core/src/symbol_table.rs

pub struct SymbolTable {
    // 作用域栈
    scopes: Vec<Scope>,
    // 全局符号
    globals: HashMap<String, Symbol>,
}

pub struct Scope {
    // 本地符号
    locals: HashMap<String, Symbol>,
    // 父作用域
    parent: Option<Box<Scope>>,
}

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_info: TypeInfo,
    pub definition: Location,
    pub usages: Vec<Location>,
}

pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Enum,
    Constant,
}
```

#### 2.2 类型推断引擎

```rust
// 新增文件: crates/cr-core/src/type_inference.rs

pub struct TypeInferencer {
    symbol_table: SymbolTable,
    type_constraints: Vec<TypeConstraint>,
}

impl TypeInferencer {
    pub fn infer_types(&mut self, ast: &dyn AstNode) -> Result<()> {
        // 1. 收集类型约束
        self.collect_constraints(ast)?;
        
        // 2. 求解约束系统
        self.solve_constraints()?;
        
        // 3. 更新符号表
        self.update_symbol_types()?;
        
        Ok(())
    }
}
```

#### 2.3 集成到规则执行

```rust
// 修改: crates/cr-rules/src/engine.rs

pub struct RuleExecutionEngine {
    symbol_table: Option<SymbolTable>,
    type_inferencer: Option<TypeInferencer>,
    // ... 其他字段
}

impl RuleExecutionEngine {
    pub fn execute_rule_with_types(
        &mut self,
        rule: &Rule,
        ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        // 1. 构建符号表
        let mut symbol_table = SymbolTable::build(ast)?;
        
        // 2. 类型推断
        let mut type_inferencer = TypeInferencer::new(symbol_table);
        type_inferencer.infer_types(ast)?;
        
        // 3. 使用类型信息执行规则
        self.symbol_table = Some(type_inferencer.symbol_table);
        self.execute_rule(rule, ast)
    }
}
```

### 预期收益
- 误报率降低: 10-15%
- 准确率提升: 10-20%
- 性能影响: +5-10%

### 实现时间: 1-2周

---

## 3. 规则编译和索引 (优先级: 🔴 高)

### 当前状态
- 规则在运行时解析
- 无规则索引优化
- 导致性能低下 (大规则集下)

### 优化方案

#### 3.1 规则编译器

```rust
// 新增文件: crates/cr-rules/src/compiler.rs

pub struct RuleCompiler;

impl RuleCompiler {
    pub fn compile_rule(rule: &Rule) -> Result<CompiledRule> {
        // 1. 解析规则模式
        let patterns = Self::compile_patterns(&rule.patterns)?;
        
        // 2. 优化模式
        let optimized = Self::optimize_patterns(patterns)?;
        
        // 3. 生成匹配器
        let matcher = Self::generate_matcher(optimized)?;
        
        Ok(CompiledRule {
            id: rule.id.clone(),
            matcher,
            metadata: rule.metadata.clone(),
        })
    }
    
    fn optimize_patterns(patterns: Vec<Pattern>) -> Result<Vec<Pattern>> {
        // 模式优化: 消除冗余、重新排序等
        Ok(patterns)
    }
}

pub struct CompiledRule {
    pub id: String,
    pub matcher: Box<dyn Fn(&dyn AstNode) -> Result<bool>>,
    pub metadata: HashMap<String, String>,
}
```

#### 3.2 规则索引

```rust
// 新增文件: crates/cr-rules/src/index.rs

pub struct RuleIndex {
    // 按语言索引
    by_language: HashMap<Language, Vec<RuleId>>,
    // 按关键字索引
    by_keyword: HashMap<String, Vec<RuleId>>,
    // 按模式类型索引
    by_pattern_type: HashMap<PatternType, Vec<RuleId>>,
    // 布隆过滤器用于快速排除
    bloom_filter: BloomFilter<String>,
}

impl RuleIndex {
    pub fn build(rules: &[Rule]) -> Result<Self> {
        let mut index = Self::new();
        
        for rule in rules {
            // 按语言索引
            for lang in &rule.languages {
                index.by_language.entry(*lang)
                    .or_insert_with(Vec::new)
                    .push(rule.id.clone());
            }
            
            // 按关键字索引
            for keyword in Self::extract_keywords(rule) {
                index.by_keyword.entry(keyword.clone())
                    .or_insert_with(Vec::new)
                    .push(rule.id.clone());
                index.bloom_filter.insert(&keyword);
            }
        }
        
        Ok(index)
    }
    
    pub fn find_relevant_rules(
        &self,
        ast: &dyn AstNode,
        language: Language,
    ) -> Vec<RuleId> {
        // 1. 按语言过滤
        let mut candidates = self.by_language.get(&language)
            .cloned()
            .unwrap_or_default();
        
        // 2. 按关键字进一步过滤
        let keywords = Self::extract_ast_keywords(ast);
        candidates.retain(|rule_id| {
            keywords.iter().any(|kw| {
                self.bloom_filter.might_contain(kw)
            })
        });
        
        candidates
    }
}
```

#### 3.3 缓存规则编译结果

```rust
// 修改: crates/cr-rules/src/engine.rs

pub struct RuleExecutionEngine {
    compiled_rules_cache: HashMap<String, CompiledRule>,
    rule_index: Option<RuleIndex>,
    // ... 其他字段
}

impl RuleExecutionEngine {
    pub fn execute_rules_optimized(
        &mut self,
        rules: &[Rule],
        ast: &dyn AstNode,
    ) -> Result<Vec<Finding>> {
        // 1. 构建规则索引
        let index = RuleIndex::build(rules)?;
        
        // 2. 查找相关规则
        let relevant_rules = index.find_relevant_rules(
            ast,
            ast.language(),
        );
        
        // 3. 编译并缓存规则
        let compiled = relevant_rules.iter()
            .filter_map(|rule_id| {
                rules.iter()
                    .find(|r| &r.id == rule_id)
                    .and_then(|r| {
                        self.get_or_compile_rule(r).ok()
                    })
            })
            .collect::<Vec<_>>();
        
        // 4. 执行编译后的规则
        self.execute_compiled_rules(&compiled, ast)
    }
    
    fn get_or_compile_rule(&mut self, rule: &Rule) -> Result<CompiledRule> {
        if let Some(compiled) = self.compiled_rules_cache.get(&rule.id) {
            return Ok(compiled.clone());
        }
        
        let compiled = RuleCompiler::compile_rule(rule)?;
        self.compiled_rules_cache.insert(rule.id.clone(), compiled.clone());
        Ok(compiled)
    }
}
```

### 预期收益
- 性能提升: 30-50% (大规则集)
- 内存使用: -10-20%
- 启动时间: -20-30%

### 实现时间: 1-2周

---

## 4. 增量解析支持 (优先级: 🟡 中)

### 当前状态
- 每次都全量解析
- IDE集成性能差
- 不适合实时分析

### 优化方案

```rust
// 新增文件: crates/cr-parser/src/incremental.rs

pub struct IncrementalParser {
    previous_ast: Option<Box<dyn AstNode>>,
    tree_sitter_parser: TreeSitterParser,
}

impl IncrementalParser {
    pub fn parse_incremental(
        &mut self,
        source: &str,
        edits: &[TextEdit],
    ) -> Result<Box<dyn AstNode>> {
        // 1. 应用编辑到Tree-sitter
        for edit in edits {
            self.tree_sitter_parser.edit(edit)?;
        }
        
        // 2. 增量解析
        let tree = self.tree_sitter_parser.parse(source)?;
        
        // 3. 转换为通用AST
        let ast = self.convert_to_universal_ast(tree)?;
        
        // 4. 缓存AST
        self.previous_ast = Some(ast.clone());
        
        Ok(ast)
    }
}
```

### 预期收益
- IDE响应时间: 5-10倍提升
- 内存使用: -30-50%

### 实现时间: 1周

---

## 5. 控制流图分析 (优先级: 🟡 中)

### 当前状态
- 基础CFG实现
- 无高级分析能力
- 条件分析准确率低

### 优化方案

```rust
// 新增文件: crates/cr-dataflow/src/cfg.rs

pub struct ControlFlowGraph {
    nodes: HashMap<NodeId, CFGNode>,
    edges: Vec<CFGEdge>,
}

pub struct CFGNode {
    id: NodeId,
    ast_node: Box<dyn AstNode>,
    predecessors: Vec<NodeId>,
    successors: Vec<NodeId>,
}

pub struct CFGEdge {
    from: NodeId,
    to: NodeId,
    condition: Option<Condition>,
}

impl ControlFlowGraph {
    pub fn build(ast: &dyn AstNode) -> Result<Self> {
        // 构建完整的控制流图
    }
    
    pub fn find_paths(
        &self,
        from: NodeId,
        to: NodeId,
    ) -> Vec<Vec<NodeId>> {
        // 查找所有路径
    }
    
    pub fn analyze_reachability(&self) -> ReachabilityAnalysis {
        // 可达性分析
    }
}
```

### 预期收益
- 条件分析准确率: +15-25%
- 死代码检测: 新增功能

### 实现时间: 1-2周

---

## 6. 性能监控系统 (优先级: 🟡 中)

### 当前状态
- 基础日志
- 无性能指标收集
- 难以优化

### 优化方案

```rust
// 新增文件: crates/cr-core/src/metrics.rs

pub struct MetricsCollector {
    parse_time: Histogram,
    match_time: Histogram,
    dataflow_time: Histogram,
    files_processed: Counter,
    findings_count: Counter,
}

impl MetricsCollector {
    pub fn record_parse_time(&self, duration: Duration) {
        self.parse_time.observe(duration.as_millis() as f64);
    }
    
    pub fn get_summary(&self) -> MetricsSummary {
        // 返回性能摘要
    }
}
```

### 预期收益
- 性能可见性: 新增
- 优化方向清晰: 新增

### 实现时间: 1周

---

## 总体优化时间表

| 优化项 | 优先级 | 时间 | 收益 |
|--------|--------|------|------|
| 跨函数数据流 | 🔴 | 1-2周 | 准确率+20% |
| 符号表/类型系统 | 🔴 | 1-2周 | 误报率-15% |
| 规则编译/索引 | 🔴 | 1-2周 | 性能+40% |
| 增量解析 | 🟡 | 1周 | IDE性能+10x |
| 控制流图 | 🟡 | 1-2周 | 准确率+20% |
| 性能监控 | 🟡 | 1周 | 可观测性+100% |

**总计**: 6-10周可完成所有优化

---

## 实施建议

1. **并行实施**: 可同时进行多个优化
2. **增量集成**: 每个优化完成后立即集成测试
3. **性能基准**: 每个优化前后进行性能测试
4. **文档更新**: 及时更新设计文档

