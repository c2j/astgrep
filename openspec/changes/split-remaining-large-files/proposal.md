## Why

The codebase contains 10 additional large files (1000-5000+ lines) that violate maintainability principles. Long files are harder to understand, test, and modify, increasing technical debt and slowing development velocity. This change continues the refactoring work started with constant_propagation.rs to improve codebase maintainability.

## What Changes

- Split 10 large files across multiple crates into smaller, focused modules
- Apply the same modularization pattern used for constant_propagation.rs (state/analysis/utils separation where applicable)
- Maintain all existing functionality and public APIs (no breaking changes)
- Add module-level documentation for better code organization

## Capabilities

### New Capabilities
- `parser-module-refactor`: Refactor parser-related files (language_discovery.rs, tree_sitter_parser.rs, parser.rs) into focused modules
- `web-handler-refactor`: Refactor web handler files (playground.rs, analyze.rs) into modular components
- `engine-module-refactor`: Refactor core engine files (engine.rs, executor.rs) into manageable sub-modules
- `matcher-refactor`: Refactor advanced_matcher.rs (1979 lines) into logical components
- `app-refactor`: Refactor app.rs (1625 lines) for the GUI application
- `analyze-enhanced-refactor`: Refactor analyze_enhanced.rs (2461 lines) CLI command handler

### Modified Capabilities
- (none - this is pure refactoring with no behavioral changes)

## Impact

- **Code**: 10 files across crates (astgrep-parser, astgrep-web, astgrep-rules, astgrep-gui, astgrep-cli) will be refactored
- **APIs**: No breaking changes - all public APIs remain unchanged
- **Dependencies**: No changes to external dependencies
- **Tests**: Existing tests should continue to pass without modification
- **Documentation**: Module documentation will be added to improve maintainability
