## Context

`crates/astgrep-rules/src/executor/core.rs` contains 4366 lines with a single `AdvancedRuleExecutor` struct implementing 94 methods. This creates maintenance burden, testing difficulty, and violates separation of concerns.

**Current Structure:**
```
executor/
├── mod.rs
├── core.rs        (4366 lines - TOO LARGE)
└── types.rs       (already extracted)
```

**Target Structure:**
```
executor/
├── mod.rs              (~50 lines)
├── types.rs            (existing)
├── executor.rs         (~300 lines) - refactored main struct
├── traits/
│   ├── mod.rs          (~20 lines)
│   ├── taint.rs        (~50 lines)
│   ├── symbolic.rs     (~50 lines)
│   └── conditions.rs   (~50 lines)
├── impls/
│   ├── mod.rs          (~20 lines)
│   ├── taint.rs        (~450 lines)
│   ├── symbolic.rs     (~450 lines)
│   ├── conditions.rs   (~450 lines)
│   └── core.rs         (~400 lines) - remaining core logic
└── utils.rs            (~200 lines)
```

## Goals / Non-Goals

**Goals:**
- Decompose 4366-line file into modules under 500 lines each
- Enable independent testing of taint analysis, symbolic execution, and condition evaluation
- Maintain 100% backward compatibility with public APIs
- Use composition pattern over "god class" anti-pattern

**Non-Goals:**
- No behavioral changes to analysis results
- No API changes to `AdvancedRuleExecutor` public interface
- No performance optimization (maintain current performance)

## Decisions

### D1: Trait-based Composition over Inheritance

**Decision:** Extract three traits (`TaintAnalyzer`, `SymbolicExecutor`, `ConditionEvaluator`) and use composition in `AdvancedRuleExecutor`.

**Rationale:** Rust doesn't support inheritance. Traits allow:
- Independent testing/mocking of each component
- Future alternative implementations (e.g., optimized analyzers)
- Clear separation of responsibilities

**Alternatives Considered:**
- Simple file split with multiple `impl` blocks → Rejected: still couples all methods to single struct
- Module-based namespacing only → Rejected: doesn't enable mocking or independent testing

### D2: Box<dyn Trait> for Flexibility

**Decision:** Store analyzers as `Box<dyn Trait>` in `AdvancedRuleExecutor`.

**Rationale:** Allows runtime polymorphism for testing with mocks and future extensibility.

**Trade-off:** Small heap allocation overhead, but negligible for this use case (executor is long-lived).

### D3: Incremental Migration Strategy

**Decision:** Migrate in 5 phases, keeping code compilable at each step:
1. Create trait definitions (empty impls)
2. Implement `TaintAnalyzer`, update `AdvancedRuleExecutor` to delegate
3. Implement `SymbolicExecutor`, update delegation
4. Implement `ConditionEvaluator`, update delegation  
5. Final cleanup and test additions

**Rationale:** Each phase produces working code, reducing risk of large breaking changes.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Trait boundary design issues | Start with minimal trait interfaces, expand only as needed |
| Performance regression | Benchmark before/after with existing test suite |
| Circular dependencies | Keep traits in separate `traits/` module, impls depend on traits only |
| API leakage through trait visibility | Use `pub(crate)` for trait methods where appropriate |

## Migration Plan

**Phase 1: Trait Definitions** (1 day)
- Create `executor/traits/` directory
- Define `TaintAnalyzer`, `SymbolicExecutor`, `ConditionEvaluator` traits
- All methods return `todo!()` initially

**Phase 2: TaintAnalyzer Implementation** (2 days)
- Create `executor/impls/taint.rs`
- Move taint-related methods from `core.rs`
- Update `AdvancedRuleExecutor` to use `Box<dyn TaintAnalyzer>`
- Verify tests pass

**Phase 3: SymbolicExecutor Implementation** (2 days)
- Create `executor/impls/symbolic.rs`
- Move symbolic execution methods
- Update delegation in executor
- Verify tests pass

**Phase 4: ConditionEvaluator Implementation** (2 days)
- Create `executor/impls/conditions.rs`
- Move condition evaluation methods
- Update delegation
- Verify tests pass

**Phase 5: Cleanup & Testing** (2 days)
- Extract remaining utilities to `utils.rs`
- Add unit tests for each trait implementation
- Remove deprecated code from `core.rs`
- Final integration testing

**Rollback:** Each phase can be reverted independently via git. Original `core.rs` backed up before changes.

## Open Questions

1. Should `AdvancedSemgrepMatcher` also be extracted as a trait? → **Deferred**: Out of scope for this change, matcher is already well-encapsulated
2. Should we add `DefaultTaintAnalyzer::default()` for convenience? → **Yes**: Add in Phase 2
