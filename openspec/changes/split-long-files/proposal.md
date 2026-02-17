## Why

The codebase contains a large file (`constant_propagation.rs` with 1070 lines) that violates the single responsibility principle. Long files are harder to maintain, test, and understand. This change improves code maintainability by refactoring the monolithic constant propagation module into smaller, focused components.

## What Changes

- Split `constant_propagation.rs` (1070 lines) into multiple smaller modules
- Extract state management, analysis algorithms, and helper functions into separate files
- Maintain all existing functionality and public APIs
- Add module-level documentation for better code organization

## Capabilities

### New Capabilities
- `constant-prop-refactor`: Split the monolithic constant propagation module into focused sub-modules (state, analysis, utils)

### Modified Capabilities
- (none - this is a pure refactoring with no behavioral changes)

## Impact

- **Code**: `crates/astgrep-dataflow/src/constant_propagation.rs` will be refactored
- **APIs**: No breaking changes - all public APIs remain unchanged
- **Dependencies**: No changes to external dependencies
- **Tests**: Existing tests should continue to pass without modification
- **Documentation**: Module documentation will be added to improve maintainability
