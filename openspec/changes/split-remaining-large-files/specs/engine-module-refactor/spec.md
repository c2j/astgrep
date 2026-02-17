## ADDED Requirements

### Requirement: Engine modules SHALL be split into manageable sub-modules
The core engine files (engine.rs, executor.rs) SHALL be refactored into smaller, logically-organized modules that each handle a specific aspect of pattern matching and rule execution.

#### Scenario: engine.rs module creation
- **WHEN** analyzing the engine.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., pattern matching context, traversal strategies, result collection)

#### Scenario: executor.rs module creation
- **WHEN** analyzing the executor.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., rule loading, execution orchestration, concurrency handling)

### Requirement: Engine refactoring SHALL maintain backward compatibility
All public APIs used by other crates SHALL remain unchanged after refactoring to ensure no breaking changes.

#### Scenario: Public API preservation
- **WHEN** using engine APIs from other crates
- **THEN** all existing public types, functions, and methods SHALL remain accessible through re-exports

### Requirement: Engine modules SHALL be under 500 lines
Each refactored engine module SHALL contain fewer than 500 lines of code to maintain readability and maintainability, acknowledging the complexity of executor.rs (5134 lines).

#### Scenario: Module size validation
- **WHEN** measuring line count of each new engine module
- **THEN** each module SHALL contain less than 500 lines

### Requirement: Engine modules SHALL include comprehensive documentation
Each engine module SHALL include module-level documentation explaining its purpose, responsibilities, and key abstractions for pattern matching.

#### Scenario: Module documentation presence
- **WHEN** reading any engine module file
- **THEN** module-level documentation SHALL be present at the top of the file

### Requirement: Engine refactoring SHALL preserve test coverage
All existing tests for engine functionality SHALL continue to pass after refactoring without modification.

#### Scenario: Test execution
- **WHEN** running tests for astgrep-rules crate
- **THEN** all tests SHALL pass with 100% of previous coverage maintained
