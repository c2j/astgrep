## Summary

`core.rs` (4971行) 已拆分为 `core/` 模块目录：

```
executor/core/
├── mod.rs         (852行) - 结构体定义 + 主要公共方法
├── taint.rs       (1318行) - taint 分析方法
├── conditions.rs  (820行) - 条件评估方法
├── symbolic.rs    (787行) - symbolic 执行方法
└── utils.rs       (903行) - 其他工具方法
```

**总计**: 4680 行分布在 5 个文件中，每个文件都比原始单文件更易维护。

## 1. Trait Definitions

- [x] 1.1 Create `crates/astgrep-rules/src/executor/traits/` directory
- [x] 1.2 Create `traits/mod.rs` with re-exports for all traits
- [x] 1.3 Define `TaintAnalyzer` trait in `traits/taint.rs` with method signatures
- [x] 1.4 Define `SymbolicExecutor` trait in `traits/symbolic.rs` with method signatures
- [x] 1.5 Define `ConditionEvaluator` trait in `traits/conditions.rs` with method signatures
- [x] 1.6 Update `executor/mod.rs` to expose traits module

## 2. Core Module Split

- [x] 2.1 Create `executor/core/` directory
- [x] 2.2 Split core.rs into `core/mod.rs` (main struct + public methods)
- [x] 2.3 Move taint methods to `core/taint.rs`
- [x] 2.4 Move condition methods to `core/conditions.rs`
- [x] 2.5 Move symbolic methods to `core/symbolic.rs`
- [x] 2.6 Move utility methods to `core/utils.rs`
- [x] 2.7 Fix method visibility (`pub(super) fn`)
- [x] 2.8 Verify compilation passes

## 3. Default Implementations (Stubs)

- [x] 3.1 Create `impls/mod.rs` with re-exports
- [x] 3.2 Create `impls/taint.rs` with `DefaultTaintAnalyzer` stub
- [x] 3.3 Create `impls/symbolic.rs` with `DefaultSymbolicExecutor` stub
- [x] 3.4 Create `impls/conditions.rs` with `DefaultConditionEvaluator` stub

## 4. Helper Functions

- [x] 4.1 Create `core_helpers.rs` with utility functions
- [x] 4.2 Add `infer_type_from_value`, `calculate_entropy`, `matches_charset`
- [x] 4.3 Add `build_import_map`, `resolve_type_with_imports`
- [x] 4.4 Add `extract_type_info`, `find_method_name_by_line`
- [x] 4.5 Add `extract_method_body`, `simplify_fully_qualified_pattern`

## 5. Cleanup

- [x] 5.1 Delete original `core.rs` (replaced by `core/` module)
- [x] 5.2 Update `executor/mod.rs` with new module structure
- [x] 5.3 Verify all public APIs remain unchanged
- [x] 5.4 Run `cargo build -p astgrep-rules` - passes

## 6. Final Status

- **Original**: 1 file, 4971 lines
- **After split**: 5 files, 4680 lines total
- **Largest file**: taint.rs (1318 lines)
- **Compilation**: ✓ Passes
- **API compatibility**: ✓ Maintained
