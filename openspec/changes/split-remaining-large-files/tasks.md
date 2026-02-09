## 1. Parser Files Refactoring

### 1.1 language_discovery.rs (1097 lines)

- [x] 1.1.1 Analyze language_discovery.rs structure and identify module boundaries (detection, extensions, discovery)
- [x] 1.1.2 Create module directory: `crates/astgrep-parser/src/language_discovery/`
- [x] 1.1.3 Create mod.rs with module declarations for detection, extensions, discovery
- [x] 1.1.4 Create detection.rs for language detection functionality
- [x] 1.1.5 Create extensions.rs for file extension mapping functionality
- [x] 1.1.6 Create discovery.rs for parser discovery functionality
- [x] 1.1.7 Move language detection code to detection.rs
- [x] 1.1.8 Move extension mapping code to extensions.rs
- [x] 1.1.9 Move parser discovery code to discovery.rs
- [x] 1.1.10 Add re-exports to mod.rs for backward compatibility
- [x] 1.1.11 Add module-level documentation to each new module
- [x] 1.1.12 Replace language_discovery.rs with mod.rs module structure
- [x] 1.1.13 Verify each module compiles: `cargo check -p astgrep-parser`
- [x] 1.1.14 Verify each module is under 400 lines
- [x] 1.1.15 Run tests: `cargo test -p astgrep-parser`

### 1.2 tree_sitter_parser.rs (1118 lines)

- [x] 1.2.1 Analyze tree_sitter_parser.rs structure and identify module boundaries (integration, traversal, ast_ops)
- [x] 1.2.2 Create module directory: `crates/astgrep-parser/src/tree_sitter_parser/`
- [x] 1.2.3 Create mod.rs with module declarations for integration, traversal, ast_ops
- [x] 1.2.4 Create integration.rs for tree-sitter integration functionality
- [x] 1.2.5 Create traversal.rs for node traversal functionality (split into conversion.rs + pattern_matching.rs)
- [x] 1.2.6 Create ast_ops.rs for AST operations functionality
- [x] 1.2.7 Move integration code to integration.rs
- [x] 1.2.8 Move traversal code to conversion.rs and pattern_matching.rs
- [x] 1.2.9 Move AST operations code to ast_ops.rs
- [x] 1.2.10 Add re-exports to mod.rs for backward compatibility
- [x] 1.2.11 Add module-level documentation to each new module
- [x] 1.2.12 Replace tree_sitter_parser.rs with mod.rs module structure
- [x] 1.2.13 Verify each module compiles: `cargo check -p astgrep-parser`
- [x] 1.2.14 Verify each module is under 400 lines
- [x] 1.2.15 Run tests: `cargo test -p astgrep-parser` (pre-existing test errors unrelated to refactoring)

### 1.3 parser.rs (1810 lines)

- [x] 1.3.1 Analyze parser.rs structure and identify module boundaries (parsing, errors, output)
- [x] 1.3.2 Create module directory: `crates/astgrep-rules/src/parser/`
- [x] 1.3.3 Create mod.rs with module declarations for parsing, errors, output
- [x] 1.3.4 Create parsing.rs for parsing logic
- [x] 1.3.5 Create errors.rs for error handling
- [x] 1.3.6 Create output.rs for output generation
- [ ] 1.3.7 Move parsing code to parsing.rs (🔄 IN PROGRESS - background task)
- [ ] 1.3.8 Move error handling code to errors.rs (🔄 IN PROGRESS - background task)
- [ ] 1.3.9 Move output generation code to output.rs (🔄 IN PROGRESS - background task)
- [ ] 1.3.10 Add re-exports to mod.rs for backward compatibility (🔄 IN PROGRESS - background task)
- [ ] 1.3.11 Add module-level documentation to each new module (🔄 IN PROGRESS - background task)
- [ ] 1.3.12 Replace parser.rs with mod.rs module structure (🔄 IN PROGRESS - background task)
- [ ] 1.3.13 Verify each module compiles: `cargo check -p astgrep-rules` (pending background task completion)
- [ ] 1.3.14 Verify each module is under 400 lines (pending background task completion)
- [ ] 1.3.15 Run tests: `cargo test -p astgrep-rules` (pending background task completion)

- [ ] 1.4 Run full workspace test suite after parser refactoring

## 2. Engine Files Refactoring

### 2.1 engine.rs (2298 lines)

- [x] 2.1.1 Analyze engine.rs structure and identify module boundaries (context, traversal, results)
- [x] 2.1.2 Create module directory: `crates/astgrep-rules/src/engine/`
- [x] 2.1.3 Create mod.rs with module declarations for context, traversal, results
- [x] 2.1.4 Create context.rs for pattern matching context
- [x] 2.1.5 Create traversal.rs for traversal strategies
- [x] 2.1.6 Create results.rs for result collection
- [ ] 2.1.7 Move context code to context.rs (🔄 IN PROGRESS - background task)
- [ ] 2.1.8 Move traversal code to traversal.rs (🔄 IN PROGRESS - background task)
- [ ] 2.1.9 Move results code to results.rs (🔄 IN PROGRESS - background task)
- [ ] 2.1.10 Add re-exports to mod.rs for backward compatibility (🔄 IN PROGRESS - background task)
- [ ] 2.1.11 Add module-level documentation to each new module (🔄 IN PROGRESS - background task)
- [ ] 2.1.12 Replace engine.rs with mod.rs module structure (🔄 IN PROGRESS - background task)
- [ ] 2.1.13 Verify each module compiles: `cargo check -p astgrep-rules` (pending background task completion)
- [ ] 2.1.14 Verify each module is under 500 lines (pending background task completion)
- [ ] 2.1.15 Run tests: `cargo test -p astgrep-rules` (pending background task completion)

### 2.2 executor.rs (5134 lines)

- [x] 2.2.1 Analyze executor.rs structure and identify module boundaries (loading, execution, concurrency)
- [ ] 2.2.2 Create module directory: `crates/astgrep-rules/src/executor/` (🔄 IN PROGRESS - background task)
- [ ] 2.2.3 Create mod.rs with module declarations for loading, execution, concurrency (🔄 IN PROGRESS - background task)
- [ ] 2.2.4 Create loading.rs for rule loading (🔄 IN PROGRESS - background task)
- [ ] 2.2.5 Create execution.rs for execution orchestration (🔄 IN PROGRESS - background task)
- [ ] 2.2.6 Create concurrency.rs for concurrency handling (🔄 IN PROGRESS - background task)
- [ ] 2.2.7 Move loading code to loading.rs (🔄 IN PROGRESS - background task)
- [ ] 2.2.8 Move execution orchestration code to execution.rs (🔄 IN PROGRESS - background task)
- [ ] 2.2.9 Move concurrency handling code to concurrency.rs (🔄 IN PROGRESS - background task)
- [ ] 2.2.10 Add re-exports to mod.rs for backward compatibility (🔄 IN PROGRESS - background task)
- [ ] 2.2.11 Add module-level documentation to each new module (🔄 IN PROGRESS - background task)
- [ ] 2.2.12 Replace executor.rs with mod.rs module structure (🔄 IN PROGRESS - background task)
- [ ] 2.2.13 Verify each module compiles: `cargo check -p astgrep-rules` (pending background task completion)
- [ ] 2.2.14 Verify each module is under 500 lines (pending background task completion)
- [ ] 2.2.15 Run tests: `cargo test -p astgrep-rules` (pending background task completion)

### 2.3 advanced_matcher.rs (1979 lines)

- [ ] 2.3.1 Analyze advanced_matcher.rs structure and identify module boundaries (binding, constraints, context)
- [ ] 2.3.2 Create module directory: `crates/astgrep-matcher/src/advanced_matcher/`
- [ ] 2.3.3 Create mod.rs with module declarations for binding, constraints, context
- [ ] 2.3.4 Create binding.rs for variable binding
- [ ] 2.3.5 Create constraints.rs for constraint evaluation
- [ ] 2.3.6 Create context.rs for context tracking
- [ ] 2.3.7 Move variable binding code to binding.rs
- [ ] 2.3.8 Move constraint evaluation code to constraints.rs
- [ ] 2.3.9 Move context tracking code to context.rs
- [ ] 2.3.10 Add re-exports to mod.rs for backward compatibility
- [ ] 2.3.11 Add module-level documentation to each new module
- [ ] 2.3.12 Replace advanced_matcher.rs with mod.rs module structure
- [ ] 2.3.13 Verify each module compiles: `cargo check -p astgrep-matcher`
- [ ] 2.3.14 Verify each module is under 400 lines
- [ ] 2.3.15 Run tests: `cargo test -p astgrep-matcher`

- [ ] 2.4 Run full workspace test suite after engine refactoring

## 3. Web Handler Files Refactoring

### 3.1 playground.rs (1361 lines)

- [ ] 3.1.1 Analyze playground.rs structure and identify module boundaries (execution, state, display)
- [ ] 3.1.2 Create module directory: `crates/astgrep-web/src/handlers/playground/`
- [ ] 3.1.3 Create mod.rs with module declarations for execution, state, display
- [ ] 3.1.4 Create execution.rs for code execution functionality
- [ ] 3.1.5 Create state.rs for UI state management
- [ ] 3.1.6 Create display.rs for AST display functionality
- [ ] 3.1.7 Move execution code to execution.rs
- [ ] 3.1.8 Move state management code to state.rs
- [ ] 3.1.9 Move display code to display.rs
- [ ] 3.1.10 Add re-exports to mod.rs for backward compatibility
- [ ] 3.1.11 Add module-level documentation to each new module
- [ ] 3.1.12 Replace playground.rs with mod.rs module structure
- [ ] 3.1.13 Verify each module compiles: `cargo check -p astgrep-web`
- [ ] 3.1.14 Verify each module is under 400 lines
- [ ] 3.1.15 Run tests: `cargo test -p astgrep-web`

### 3.2 analyze.rs (1706 lines)

- [ ] 3.2.1 Analyze analyze.rs structure and identify module boundaries (handlers, orchestration, responses)
- [ ] 3.2.2 Create module directory: `crates/astgrep-web/src/handlers/analyze/`
- [ ] 3.2.3 Create mod.rs with module declarations for handlers, orchestration, responses
- [ ] 3.2.4 Create handlers.rs for request handling
- [ ] 3.2.5 Create orchestration.rs for analysis orchestration
- [ ] 3.2.6 Create responses.rs for response formatting
- [ ] 3.2.7 Move handler code to handlers.rs
- [ ] 3.2.8 Move orchestration code to orchestration.rs
- [ ] 3.2.9 Move response formatting code to responses.rs
- [ ] 3.2.10 Add re-exports to mod.rs for backward compatibility
- [ ] 3.2.11 Add module-level documentation to each new module
- [ ] 3.2.12 Replace analyze.rs with mod.rs module structure
- [ ] 3.2.13 Verify each module compiles: `cargo check -p astgrep-web`
- [ ] 3.2.14 Verify each module is under 400 lines
- [ ] 3.2.15 Run tests: `cargo test -p astgrep-web`

- [ ] 3.3 Run full workspace test suite after web handlers refactoring

## 4. CLI/GUI Files Refactoring

### 4.1 analyze_enhanced.rs (2461 lines)

- [ ] 4.1.1 Analyze analyze_enhanced.rs structure and identify module boundaries (args, orchestration, output, errors)
- [ ] 4.1.2 Create module directory: `crates/astgrep-cli/src/commands/analyze_enhanced/`
- [ ] 4.1.3 Create mod.rs with module declarations for args, orchestration, output, errors
- [ ] 4.1.4 Create args.rs for CLI argument parsing
- [ ] 4.1.5 Create orchestration.rs for analysis orchestration
- [ ] 4.1.6 Create output.rs for output formatting
- [ ] 4.1.7 Create errors.rs for error handling
- [ ] 4.1.8 Move argument parsing code to args.rs
- [ ] 4.1.9 Move orchestration code to orchestration.rs
- [ ] 4.1.10 Move output formatting code to output.rs
- [ ] 4.1.11 Move error handling code to errors.rs
- [ ] 4.1.12 Add re-exports to mod.rs for backward compatibility
- [ ] 4.1.13 Add module-level documentation to each new module
- [ ] 4.1.14 Replace analyze_enhanced.rs with mod.rs module structure
- [ ] 4.1.15 Verify each module compiles: `cargo check -p astgrep-cli`
- [ ] 4.1.16 Verify each module is under 500 lines
- [ ] 4.1.17 Run tests: `cargo test -p astgrep-cli`

### 4.2 app.rs (1625 lines)

- [ ] 4.2.1 Analyze app.rs structure and identify module boundaries (state, events, rendering, config)
- [ ] 4.2.2 Create module directory: `crates/astgrep-gui/src/app/`
- [ ] 4.2.3 Create mod.rs with module declarations for state, events, rendering, config
- [ ] 4.2.4 Create state.rs for application state
- [ ] 4.2.5 Create events.rs for event handling
- [ ] 4.2.6 Create rendering.rs for UI component rendering
- [ ] 4.2.7 Create config.rs for configuration
- [ ] 4.2.8 Move state management code to state.rs
- [ ] 4.2.9 Move event handling code to events.rs
- [ ] 4.2.10 Move rendering code to rendering.rs
- [ ] 4.2.11 Move configuration code to config.rs
- [ ] 4.2.12 Add re-exports to mod.rs for backward compatibility
- [ ] 4.2.13 Add module-level documentation to each new module
- [ ] 4.2.14 Replace app.rs with mod.rs module structure
- [ ] 4.2.15 Verify each module compiles: `cargo check -p astgrep-gui`
- [ ] 4.2.16 Verify each module is under 400 lines
- [ ] 4.2.17 Run tests: `cargo test -p astgrep-gui`

- [ ] 4.3 Run full workspace test suite after CLI/GUI refactoring

## 5. Final Verification

- [ ] 5.1 Verify all 10 files have been refactored into modules
- [ ] 5.2 Verify all modules are under line count limits (<400 or <500 lines)
- [ ] 5.3 Verify all modules have comprehensive documentation
- [ ] 5.4 Verify all public APIs are preserved via re-exports
- [ ] 5.5 Run full workspace test suite: `cargo test`
- [ ] 5.6 Verify documentation builds: `cargo doc`
- [ ] 5.7 Verify workspace builds without warnings: `cargo build`
- [ ] 5.8 Create summary of refactoring results (files before/after line counts)
