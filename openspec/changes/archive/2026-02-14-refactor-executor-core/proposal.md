## Why

`executor/core.rs` has grown to 4366 lines with 94 methods in a single `impl AdvancedRuleExecutor` block. This violates the Single Responsibility Principle, makes testing difficult, and increases maintenance risk. The file needs to be decomposed into focused, modular components following Rust best practices.

## What Changes

- Extract `TaintAnalyzer` trait and default implementation (~450 lines)
- Extract `SymbolicExecutor` trait and default implementation (~450 lines)
- Extract `ConditionEvaluator` trait and default implementation (~450 lines)
- Refactor `AdvancedRuleExecutor` to use composition pattern (~300 lines)
- Create utility module for shared helpers (~200 lines)
- All public APIs remain unchanged for backward compatibility

## Capabilities

### New Capabilities
- `taint-analysis`: Taint source/sink detection and flow analysis extracted from core executor
- `symbolic-execution`: Variable type checking and symbolic propagation functionality
- `condition-evaluation`: Pattern condition and metavariable comparison evaluation

### Modified Capabilities
- (none - this is a pure refactoring with no behavior changes)

## Impact

**Files Modified:**
- `crates/astgrep-rules/src/executor/core.rs` → split into multiple modules

**Files Created:**
- `crates/astgrep-rules/src/executor/traits/mod.rs`
- `crates/astgrep-rules/src/executor/traits/taint.rs`
- `crates/astgrep-rules/src/executor/traits/symbolic.rs`
- `crates/astgrep-rules/src/executor/traits/conditions.rs`
- `crates/astgrep-rules/src/executor/impls/mod.rs`
- `crates/astgrep-rules/src/executor/impls/taint.rs`
- `crates/astgrep-rules/src/executor/impls/symbolic.rs`
- `crates/astgrep-rules/src/executor/impls/conditions.rs`
- `crates/astgrep-rules/src/executor/executor.rs`

**API Stability:**
- All public APIs on `AdvancedRuleExecutor` remain unchanged
- Internal implementation only - no downstream code changes required
