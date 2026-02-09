## Context

The astgrep codebase has successfully refactored `constant_propagation.rs` (1070 lines) into a modular structure with state, analysis, and utils sub-modules. This proven approach demonstrated that splitting large monolithic files improves maintainability without breaking functionality.

However, 10 additional large files remain across the codebase:
- `language_discovery.rs` (1097 lines) - Parser language detection and file extension mapping
- `tree_sitter_parser.rs` (1118 lines) - Tree-sitter integration and AST operations
- `parser.rs` (1810 lines) - Parsing logic and output generation
- `playground.rs` (1361 lines) - Web playground code execution and UI state
- `analyze.rs` (1706 lines) - Web analysis endpoint handlers
- `engine.rs` (2298 lines) - Core pattern matching and traversal
- `executor.rs` (5134 lines) - Rule execution orchestration and concurrency
- `advanced_matcher.rs` (1979 lines) - Variable binding and constraint evaluation
- `app.rs` (1625 lines) - GUI application state and event handling
- `analyze_enhanced.rs` (2461 lines) - Enhanced CLI command handler

These files violate maintainability principles, making code harder to understand, test, and modify. The constant_propagation.rs refactoring provides a proven pattern to follow.

**Constraints:**
- No breaking changes to public APIs
- All existing tests must pass without modification
- No changes to external dependencies
- Must follow Rust best practices and existing code style

## Goals / Non-Goals

**Goals:**
- Split all 10 large files into focused, maintainable modules
- Apply the proven modularization pattern from constant_propagation.rs
- Maintain 100% backward compatibility with existing public APIs
- Ensure all existing tests pass without modification
- Add comprehensive module-level documentation
- Target module sizes: <400 lines (standard) or <500 lines (complex modules)
- Improve code organization and developer velocity

**Non-Goals:**
- Re-architecting the overall system design
- Adding new functionality or features
- Changing performance characteristics
- Modifying external dependencies
- Updating test coverage (preserving existing coverage is sufficient)
- Refactoring files under 1000 lines

## Decisions

### Modularization Strategy

**Decision:** Apply the constant_propagation.rs pattern (state/analysis/utils) where applicable, but adapt module structure to each file's domain.

**Rationale:** The constant_propagation.rs refactoring successfully separated concerns into logical modules. This pattern works well for files with distinct data structures, algorithms, and helper functions. However, different domains (web handlers, CLI commands, GUI) may require different module organizations.

**Alternatives Considered:**
1. Apply identical state/analysis/utils to all files → Rejected - would be artificial for some domains
2. Create custom module structures per file → Selected - aligns with domain-specific needs
3. Use a framework/module system → Rejected - adds complexity without clear benefit

### File-by-File Module Structure

#### 1. Parser Files (language_discovery.rs, tree_sitter_parser.rs, parser.rs)

**Decision:** Create domain-specific modules based on functionality:
- `language_discovery.rs` → `detection.rs`, `extensions.rs`, `discovery.rs`
- `tree_sitter_parser.rs` → `integration.rs`, `traversal.rs`, `ast_ops.rs`
- `parser.rs` → `parsing.rs`, `errors.rs`, `output.rs`

**Rationale:** Parser files have clear functional boundaries. Splitting by functionality (detection, traversal, error handling) creates natural module boundaries. This aligns with the single responsibility principle.

**Alternatives Considered:**
1. Merge all three parser files into one module system → Rejected - increases coupling
2. Keep structure identical to constant_propagation.rs → Rejected - not a good fit for domain

#### 2. Web Handlers (playground.rs, analyze.rs)

**Decision:** Split by request/response lifecycle:
- `playground.rs` → `execution.rs`, `state.rs`, `display.rs`
- `analyze.rs` → `handlers.rs`, `orchestration.rs`, `responses.rs`

**Rationale:** Web handlers follow a clear request lifecycle. Organizing by lifecycle stage (handling, orchestration, response) improves readability and makes testing easier.

**Alternatives Considered:**
1. Organize by endpoint → Rejected - would result in tiny modules
2. State/utils/analysis pattern → Rejected - not natural for HTTP handlers

#### 3. Core Engine (engine.rs, executor.rs)

**Decision:** Focus on core pattern matching concerns:
- `engine.rs` → `context.rs`, `traversal.rs`, `results.rs`
- `executor.rs` → `loading.rs`, `execution.rs`, `concurrency.rs`

**Rationale:** The engine has distinct phases: establishing context, traversing AST, collecting results. Executor separates rule loading from execution orchestration. Given executor.rs is 5134 lines, we target <500 lines per module.

**Alternatives Considered:**
1. Merge engine and executor into one module system → Rejected - would create massive modules
2. More granular splitting (<300 lines) → Rejected - may over-fragment related code

#### 4. Advanced Matcher (advanced_matcher.rs)

**Decision:** Split by matching concerns:
- `advanced_matcher.rs` → `binding.rs`, `constraints.rs`, `context.rs`

**Rationale:** Advanced matching involves variable binding, constraint evaluation, and context tracking - distinct concerns that benefit from separation.

**Alternatives Considered:**
1. Keep as single module → Rejected - too large (1979 lines)
2. Merge into matcher core → Rejected - increases complexity

#### 5. GUI App (app.rs)

**Decision:** Split by UI architecture:
- `app.rs` → `state.rs`, `events.rs`, `rendering.rs`, `config.rs`

**Rationale:** GUI applications have natural separation between state management, event handling, rendering, and configuration. This aligns with common UI frameworks.

**Alternatives Considered:**
1. Organize by widget/component → Rejected - would require understanding UI structure first
2. Single module → Rejected - too large (1625 lines)

#### 6. CLI Command (analyze_enhanced.rs)

**Decision:** Split by CLI command concerns:
- `analyze_enhanced.rs` → `args.rs`, `orchestration.rs`, `output.rs`, `errors.rs`

**Rationale:** CLI commands have clear stages: argument parsing, orchestrating the analysis, formatting output, and handling errors. Given it's 2461 lines, we target <500 lines per module.

**Alternatives Considered:**
1. Merge into general CLI module → Rejected - reduces clarity
2. More granular splitting → Rejected - CLI commands naturally group functionality

### Backward Compatibility Strategy

**Decision:** Use re-exports from mod.rs to preserve all public APIs.

**Rationale:** Rust's `pub use` allows maintaining exact public API surface while reorganizing internal structure. All downstream consumers see no changes.

**Implementation:**
```rust
// astgrep-parser/src/language_discovery/mod.rs
pub mod detection;
pub mod extensions;
pub mod discovery;

// Re-export all public items for backward compatibility
pub use detection::*;
pub use extensions::*;
pub use discovery::*;
```

**Alternatives Considered:**
1. Deprecate old APIs and create new ones → Rejected - unnecessary churn
2. Update all consumers → Rejected - too much work for a refactoring

### Module Dependency Management

**Decision:** Enforce clear dependency order and avoid circular dependencies.

**Rationale:** Circular dependencies complicate compilation and testing. Establishing clear dependency order prevents issues.

**Implementation:**
- Create mod.rs first (acts as coordinator)
- Create leaf modules (no dependencies on other new modules) next
- Create intermediate modules (depend on leaf modules)
- Create top-level modules (depend on intermediate modules)

**Validation:** Run `cargo check` after each module to catch circular dependencies early.

### Documentation Strategy

**Decision:** Add comprehensive module-level docs following Rust conventions.

**Rationale:** Documentation is essential for maintainability. Module-level docs explain purpose, responsibilities, and key abstractions.

**Template:**
```rust
//! # Module Name
//!
//! Brief description of the module's purpose.
//!
//! ## Responsibilities
//! - Responsibility 1
//! - Responsibility 2
//!
//! ## Key Abstractions
//! - `StructName`: description
//! - `TraitName`: description
```

### Testing Strategy

**Decision:** Preserve all existing tests and run full test suite after each file refactoring.

**Rationale:** Tests verify backward compatibility. Running full test suite catches regressions early.

**Validation Steps:**
1. `cargo build -p <crate>` after each module creation
2. `cargo test -p <crate>` after completing file refactoring
3. `cargo test` from workspace root after all files complete

### Execution Order

**Decision:** Refactor files in dependency order (parser → engine → handlers → CLI/GUI).

**Rationale:** Parser and engine are foundational. Handlers depend on them. CLI/GUI depend on handlers. This order minimizes repeated testing.

**Sequence:**
1. Parser files (language_discovery.rs, tree_sitter_parser.rs, parser.rs)
2. Engine files (engine.rs, executor.rs, advanced_matcher.rs)
3. Web handlers (playground.rs, analyze.rs)
4. CLI/GUI (analyze_enhanced.rs, app.rs)

## Risks / Trade-offs

### Risk 1: Over-fragmentation of related code
[Risk] Splitting files too aggressively may scatter related logic across many small modules, making the code harder to follow.

**Mitigation:** Use domain-driven module boundaries, not arbitrary line counts. Review module cohesion after each refactoring. If modules feel too fragmented, consolidate them.

### Risk 2: Circular dependencies
[Risk] Complex refactoring may introduce circular dependencies between new modules.

**Mitigation:** Enforce clear dependency order (leaf → intermediate → top). Run `cargo check` after each module to catch circular dependencies immediately. Use mod.rs to coordinate if needed.

### Risk 3: Test failures due to reorganization
[Risk] Even with re-exports, internal reorganization may break tests that depend on internal details.

**Mitigation:** Review all test imports before refactoring. Update test paths if tests import internal modules. Run full test suite after each file completion.

### Risk 4: Increased compilation time
[Risk]** More modules and files may increase compilation time due to additional metadata and dependency tracking.

**Mitigation:** Accept as temporary trade-off for maintainability. Future optimizations (workspace caching, incremental compilation) will mitigate.

### Risk 5: Inconsistent module structures across files
[Risk]** Different files may end up with different module organization patterns, confusing developers.

**Mitigation:** Document the rationale for each module structure. Create a "refactoring guide" in AGENTS.md showing examples for each domain type.

### Trade-off: Line count vs. logical coherence
[Trade-off] Enforcing strict line count limits (<400-500 lines) may force artificial splits that break logical coherence.

**Decision:** Prioritize logical coherence over strict line counts. Use line counts as guidance, not hard rules. If a module is 450 lines but logically coherent, accept it.

### Trade-off: One-time effort vs. long-term maintainability
[Trade-off] This refactoring requires significant effort but provides long-term maintainability benefits.

**Decision:** Invest the effort now. The constant_propagation.rs refactoring demonstrated the value. Future changes will be easier with modular code.

## Open Questions

1. **Should executor.rs (5134 lines) be split differently?** It's the largest file. Consider if a more aggressive split (3-4 modules) vs. 3 modules is better.
   - **Resolution:** Start with 3 modules. If analysis shows natural 4-module split, adjust.

2. **Should web handlers share common infrastructure?** playground.rs and analyze.rs may share request/response handling code.
   - **Resolution:** Investigate during refactoring. If >100 lines of shared code, extract to common module.

3. **Should we create a common refactoring guide?** documenting the patterns used would help future refactoring efforts.
   - **Resolution:** Update AGENTS.md with refactoring examples from this work.

## Migration Plan

This is a pure refactoring with no data migration or deployment concerns. The migration plan focuses on safe, incremental implementation.

### Phase 1: Parser Files (Week 1)
1. Refactor `language_discovery.rs` → validate with `cargo test -p astgrep-parser`
2. Refactor `tree_sitter_parser.rs` → validate with `cargo test -p astgrep-parser`
3. Refactor `parser.rs` → validate with `cargo test -p astgrep-parser`
4. Run full workspace test suite

### Phase 2: Engine Files (Week 2)
1. Refactor `engine.rs` → validate with `cargo test -p astgrep-rules`
2. Refactor `executor.rs` → validate with `cargo test -p astgrep-rules`
3. Refactor `advanced_matcher.rs` → validate with `cargo test -p astgrep-matcher`
4. Run full workspace test suite

### Phase 3: Web Handlers (Week 3)
1. Refactor `playground.rs` → validate with `cargo test -p astgrep-web`
2. Refactor `analyze.rs` → validate with `cargo test -p astgrep-web`
3. Run full workspace test suite

### Phase 4: CLI/GUI (Week 4)
1. Refactor `analyze_enhanced.rs` → validate with `cargo test -p astgrep-cli`
2. Refactor `app.rs` → validate with `cargo test -p astgrep-gui`
3. Run full workspace test suite

### Rollback Strategy
Each file refactoring is atomic and can be reverted independently using git:
```bash
git revert <commit>  # Revert specific file refactoring
```

Since refactoring is done incrementally with testing after each file, issues are caught early. No database or state changes exist, so rollback is straightforward.
