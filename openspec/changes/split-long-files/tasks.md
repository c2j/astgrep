## 1. Preparation

- [ ] 1.1 Create new module files: `state.rs`, `analysis.rs`, `utils.rs` in `crates/astgrep-dataflow/src/`
- [ ] 1.2 Add module declarations to `crates/astgrep-dataflow/src/lib.rs` for the new modules
- [ ] 1.3 Verify the module dependency order is correct (state → utils → analysis)

## 2. Extract state.rs Module

- [ ] 2.1 Move `SourceLocation` struct and its `impl` block to `state.rs`
- [ ] 2.2 Move `ConstantValue` enum and its `impl` block to `state.rs`
- [ ] 2.3 Move `Scope` struct and its `impl` block to `state.rs`
- [ ] 2.4 Move `VariableDefinition` struct to `state.rs`
- [ ] 2.5 Move `ConstantPropagator` struct definition to `state.rs`
- [ ] 2.6 Move `VisitContext` enum to `state.rs`
- [ ] 2.7 Move scope-related methods from `ConstantPropagator`: `push_scope`, `pop_scope`, `define_local_variable`, `lookup_variable`, `update_scope_location`
- [ ] 2.8 Move accessor methods from `ConstantPropagator`: `get_constant`, `get_node_constant`, `is_constant`, `get_all_constants`, `get_all_node_constants`, `get_location_based_constants`, `get_variable_definitions`, `get_all_constants_with_locations`
- [ ] 2.9 Move state management methods: `mark_reassigned`, `Default` impl
- [ ] 2.10 Add proper imports to `state.rs` (HashMap, HashSet, serde, etc.)
- [ ] 2.11 Verify `state.rs` compiles independently: `cargo check -p astgrep-dataflow`
- [ ] 2.12 Verify `state.rs` is under 400 lines

## 3. Extract utils.rs Module

- [ ] 3.1 Move `get_node_location` helper function to `utils.rs`
- [ ] 3.2 Move `is_static_block_context` function to `utils.rs`
- [ ] 3.3 Move `is_constructor_declaration` function to `utils.rs`
- [ ] 3.4 Move `is_method_declaration` function to `utils.rs`
- [ ] 3.5 Move `extract_variable_name_from_assignment_target` function to `utils.rs`
- [ ] 3.6 Move `extract_constant_from_expression` function to `utils.rs`
- [ ] 3.7 Add proper imports to `utils.rs` and import from `state.rs`
- [ ] 3.8 Verify `utils.rs` compiles: `cargo check -p astgrep-dataflow`
- [ ] 3.9 Verify `utils.rs` is under 300 lines

## 4. Extract analysis.rs Module

- [ ] 4.1 Move `analyze` method from `ConstantPropagator` to `analysis.rs`
- [ ] 4.2 Move `collect_constants` method to `analysis.rs`
- [ ] 4.3 Move `propagate_constants` method to `analysis.rs`
- [ ] 4.4 Move `analyze_ast` method to `analysis.rs`
- [ ] 4.5 Move `visit_node_for_constants` method to `analysis.rs`
- [ ] 4.6 Move `visit_node_with_context` method to `analysis.rs`
- [ ] 4.7 Move `process_assignment_expression` method to `analysis.rs`
- [ ] 4.8 Move `process_local_assignment` method to `analysis.rs`
- [ ] 4.9 Move `check_reassignment_in_method` method to `analysis.rs`
- [ ] 4.10 Move `get_variable_value_at_location` method to `analysis.rs`
- [ ] 4.11 Add proper imports to `analysis.rs` (import from both `state` and `utils`)
- [ ] 4.12 Add module-level doc comment explaining this module contains the core analysis algorithms
- [ ] 4.13 Verify `analysis.rs` compiles: `cargo check -p astgrep-dataflow`
- [ ] 4.14 Verify `analysis.rs` is under 400 lines

## 5. Main Module Cleanup

- [ ] 5.1 Replace all code in `constant_propagation.rs` with module declarations and re-exports
- [ ] 5.2 Add `pub use state::*;` to re-export all public items from state
- [ ] 5.3 Add explicit re-exports for items from `analysis` and `utils` if they need to be public
- [ ] 5.4 Keep the test module (`#[cfg(test)] mod tests`) in `constant_propagation.rs`
- [ ] 5.5 Add comprehensive module-level documentation explaining the module structure
- [ ] 5.6 Document which sub-module is responsible for what in a comment block
- [ ] 5.7 Verify `constant_propagation.rs` is under 200 lines
- [ ] 5.8 Verify all public items from original file are still publicly accessible

## 6. Verification

- [ ] 6.1 Run `cargo build -p astgrep-dataflow` and ensure no compilation errors
- [ ] 6.2 Run `cargo test -p astgrep-dataflow` and ensure all existing tests pass
- [ ] 6.3 Run `cargo doc -p astgrep-dataflow` and verify documentation builds without warnings
- [ ] 6.4 Verify line count requirements:
  - [ ] 6.4.1 `state.rs` < 400 lines
  - [ ] 6.4.2 `analysis.rs` < 400 lines
  - [ ] 6.4.3 `utils.rs` < 300 lines
  - [ ] 6.4.4 `constant_propagation.rs` < 200 lines
- [ ] 6.5 Verify backward compatibility by checking no public API changes
- [ ] 6.6 Run full test suite: `cargo test` from workspace root
- [ ] 6.7 Verify test coverage is maintained (no significant decrease)
