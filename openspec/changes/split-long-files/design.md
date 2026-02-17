## Context

The `constant_propagation.rs` file in `crates/astgrep-dataflow/src/` has grown to 1070 lines, violating the single responsibility principle. Analysis of the file reveals several distinct concerns:

**Current Structure:**
- Lines 1-129: Type definitions (SourceLocation, Scope, ConstantValue, ConstantPropagator struct)
- Lines 130-560: Core analysis algorithms and AST traversal logic
- Lines 561-995: Helper methods (assignment processing, context detection, extraction utilities)
- Lines 996-1070: Unit tests

**Constraints:**
- Must maintain backward compatibility - all public APIs remain unchanged
- No test modifications allowed (existing tests must pass)
- Rust module system requires careful re-export structure

## Goals / Non-Goals

**Goals:**
- Reduce `constant_propagation.rs` to under 200 lines (module declarations + re-exports + docs)
- Extract state management logic to `state.rs` (under 400 lines)
- Extract analysis algorithms to `analysis.rs` (under 400 lines)
- Extract helper utilities to `utils.rs` (under 300 lines)
- Maintain 100% backward compatibility for public APIs
- Preserve all existing functionality and test coverage

**Non-Goals:**
- Changing any algorithm behavior or logic
- Adding new features or capabilities
- Modifying existing test files
- Moving tests out of the main module (keep in `constant_propagation.rs` for simplicity)

## Decisions

### Decision: Module Structure

**Chosen:** Split into four modules:
1. `state.rs` - Data structures and state management
2. `analysis.rs` - Constant propagation algorithm and AST traversal
3. `utils.rs` - Helper functions (context detection, name extraction, value parsing)
4. `constant_propagation.rs` - Re-exports and tests (thin facade)

**Rationale:**
- Clear separation of concerns: state vs algorithms vs utilities
- Each module has a single, well-defined purpose
- Size targets align with spec requirements (<400, <400, <300, <200 lines)

**Alternatives Considered:**
- Private submodules (`mod state { ... }` inside main file) - Rejected: still bloats the main file
- More granular split (5+ modules) - Rejected: increases complexity without clear benefit
- Keep tests separate in `tests/` directory - Rejected: would require changing existing test imports

### Decision: Re-export Strategy

**Chosen:** Explicit re-exports in main module using `pub use`

```rust
pub use state::{ConstantValue, SourceLocation, Scope, VariableDefinition, ConstantPropagator};
pub use analysis::{VisitContext, /* methods moved to impl blocks */};
```

**Rationale:**
- Maintains backward compatibility - existing `use crate::constant_propagation::X;` works unchanged
- Clear public API surface area
- Rustdoc shows all public items at the module level

**Implementation Note:**
The `ConstantPropagator` struct will remain in `state.rs` but its `impl` block methods will be split across modules using Rust's inherent impl feature:

```rust
// In state.rs
pub struct ConstantPropagator { ... }

// In analysis.rs  
use crate::constant_propagation::state::ConstantPropagator;

impl ConstantPropagator {
    pub fn analyze(...) { ... }
    fn propagate_constants(...) { ... }
}
```

### Decision: Visibility Levels

**Chosen:**
- `pub` for all currently public items (backward compatibility)
- `pub(crate)` for items used across modules within the crate
- `pub(super)` or private for truly internal helpers

**Rationale:**
- Minimizes changes to existing code
- Follows Rust visibility conventions
- Allows incremental refactoring in the future

### Decision: Error Handling Preservation

**Chosen:** Keep all error types and `Result` aliases as-is

**Rationale:**
- `crate::Result` is used throughout; moving it would break compatibility
- Error handling is not the focus of this refactor

## Risks / Trade-offs

**Risk: Breaking Changes in Public API**
- *Impact:* High - would break downstream consumers
- *Mitigation:* 
  - Comprehensive compile-time check: ensure all existing public items are re-exported
  - Run full test suite before completion
  - Manual verification of key imports

**Risk: Module Dependencies Become Circular**
- *Impact:* Medium - would prevent compilation
- *Mitigation:*
  - Careful ordering of module declarations in `lib.rs`
  - `state.rs` has no dependencies on other modules
  - `utils.rs` depends only on `state.rs`
  - `analysis.rs` depends on both `state.rs` and `utils.rs`

**Risk: Lost Git History**
- *Impact:* Low - affects code archaeology
- *Mitigation:* Use `git mv` followed by edits to preserve blame history

**Risk: Complex `impl` Block Splitting**
- *Impact:* Medium - may confuse developers
- *Mitigation:* 
  - Clear module-level documentation explaining the pattern
  - Comment at the top of each `impl` block indicating where the struct is defined
  - Consider using traits for better organization (future enhancement)

**Trade-off: Code Organization vs. Discoverability**
- Splitting across files makes the codebase more modular but may make it harder to find specific methods
- *Mitigation:* Comprehensive module-level documentation and rustdoc

## Migration Plan

**Phase 1: Preparation**
1. Create new module files (`state.rs`, `analysis.rs`, `utils.rs`)
2. Add module declarations to `lib.rs` (if not using `mod.rs` pattern)

**Phase 2: Extraction (order matters)**
1. **Extract `state.rs`** (no dependencies):
   - Move: SourceLocation, Scope, VariableDefinition, ConstantValue
   - Move: ConstantPropagator struct definition
   - Move: Scope-related methods (push_scope, pop_scope, define_local_variable, lookup_variable)

2. **Extract `utils.rs`** (depends on state):
   - Move: is_static_block_context, is_constructor_declaration, is_method_declaration
   - Move: extract_variable_name_from_assignment_target, extract_constant_from_expression
   - Move: get_node_location

3. **Extract `analysis.rs`** (depends on state + utils):
   - Move: analyze, collect_constants, propagate_constants, analyze_ast
   - Move: visit_node_for_constants, visit_node_with_context
   - Move: process_assignment_expression, process_local_assignment, check_reassignment_in_method

**Phase 3: Main Module Cleanup**
1. Replace all code with re-exports
2. Keep tests in place (or move if preferred)
3. Add module-level documentation

**Phase 4: Verification**
1. Run `cargo build` - ensure compilation succeeds
2. Run `cargo test` - ensure all tests pass
3. Run `cargo doc` - verify documentation builds
4. Verify line counts meet spec requirements

**Rollback Strategy:**
If issues arise:
1. Git revert to pre-refactor state
2. Or keep backup branch and switch back

## Open Questions

1. **Should we keep tests in the main module or move to integration tests?**
   - *Current leaning:* Keep in main module to minimize changes
   - *Consideration:* Moving tests would require updating imports in test code

2. **Should we use traits to organize the split `impl` blocks?**
   - *Current leaning:* No - keep simple inherent impls for now
   - *Consideration:* Traits add complexity; can be introduced later if needed

3. **What about the debug print statements (e.g., `eprintln!("DEBUG CP: ...")`)?**
   - *Current leaning:* Keep as-is; they're in the original code
   - *Consideration:* Could clean up in separate PR

4. **Should we add `#[inline]` attributes when moving functions?**
   - *Current leaning:* No - let compiler decide
   - *Consideration:* Could affect performance but shouldn't be in scope for this refactor
