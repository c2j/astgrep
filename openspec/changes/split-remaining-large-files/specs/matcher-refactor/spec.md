## ADDED Requirements

### Requirement: Advanced matcher SHALL be split into logical components
The advanced_matcher.rs file SHALL be refactored into smaller, logically-organized modules that each handle a specific aspect of advanced pattern matching (e.g., variable binding, constraint evaluation, context tracking).

#### Scenario: Module creation for variable binding
- **WHEN** analyzing variable binding logic in advanced_matcher.rs
- **THEN** a dedicated module SHALL be created for variable binding operations

#### Scenario: Module creation for constraint evaluation
- **WHEN** analyzing constraint evaluation logic in advanced_matcher.rs
- **THEN** a dedicated module SHALL be created for constraint evaluation

#### Scenario: Module creation for context tracking
- **WHEN** analyzing context tracking logic in advanced_matcher.rs
- **THEN** a dedicated module SHALL be created for context management

### Requirement: Matcher refactoring SHALL maintain backward compatibility
All public APIs for advanced pattern matching SHALL remain unchanged after refactoring to ensure no breaking changes for rule definitions.

#### Scenario: Public API preservation
- **WHEN** using advanced matcher APIs
- **THEN** all existing public types, functions, and methods SHALL remain accessible through re-exports

### Requirement: Matcher modules SHALL be under 400 lines
Each refactored matcher module SHALL contain fewer than 400 lines of code to maintain readability and maintainability.

#### Scenario: Module size validation
- **WHEN** measuring line count of each new matcher module
- **THEN** each module SHALL contain less than 400 lines

### Requirement: Matcher modules SHALL include comprehensive documentation
Each matcher module SHALL include module-level documentation explaining its purpose in the pattern matching process and key abstractions.

#### Scenario: Module documentation presence
- **WHEN** reading any matcher module file
- **THEN** module-level documentation SHALL be present at the top of the file

### Requirement: Matcher refactoring SHALL preserve test coverage
All existing tests for advanced matching functionality SHALL continue to pass after refactoring without modification.

#### Scenario: Test execution
- **WHEN** running tests for astgrep-matcher crate
- **THEN** all tests SHALL pass with 100% of previous coverage maintained
