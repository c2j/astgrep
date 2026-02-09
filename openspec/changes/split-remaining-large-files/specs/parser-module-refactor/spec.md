## ADDED Requirements

### Requirement: Parser module shall be split into focused sub-modules
The parser-related files (language_discovery.rs, tree_sitter_parser.rs, parser.rs) SHALL be refactored into smaller, logically-organized modules that each handle a specific concern.

#### Scenario: language_discovery.rs module creation
- **WHEN** analyzing the language_discovery.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., language detection, file extension mapping, parser discovery)

#### Scenario: tree_sitter_parser.rs module creation
- **WHEN** analyzing the tree_sitter_parser.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., tree-sitter integration, node traversal, AST operations)

#### Scenario: parser.rs module creation
- **WHEN** analyzing the parser.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., parsing logic, error handling, output generation)

### Requirement: Parser refactoring SHALL maintain backward compatibility
All public APIs exposed by the parser modules SHALL remain unchanged after refactoring to ensure no breaking changes for consumers.

#### Scenario: Public API preservation
- **WHEN** refactoring parser modules
- **THEN** all existing public types, functions, and methods SHALL remain accessible through re-exports

#### Scenario: No behavioral changes
- **WHEN** using parser functionality after refactoring
- **THEN** the behavior SHALL be identical to pre-refactoring behavior

### Requirement: Parser modules SHALL be under 400 lines
Each refactored parser module SHALL contain fewer than 400 lines of code to maintain readability and maintainability.

#### Scenario: Module size validation
- **WHEN** measuring line count of each new parser module
- **THEN** each module SHALL contain less than 400 lines

### Requirement: Parser modules SHALL include comprehensive documentation
Each parser module SHALL include module-level documentation explaining its purpose, responsibilities, and key abstractions.

#### Scenario: Module documentation presence
- **WHEN** reading any parser module file
- **THEN** module-level documentation SHALL be present at the top of the file

### Requirement: Parser refactoring SHALL preserve test coverage
All existing tests for parser functionality SHALL continue to pass after refactoring without modification.

#### Scenario: Test execution
- **WHEN** running tests for astgrep-parser crate
- **THEN** all tests SHALL pass with 100% of previous coverage maintained
