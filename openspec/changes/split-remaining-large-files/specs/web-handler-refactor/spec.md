## ADDED Requirements

### Requirement: Web handler modules SHALL be split into focused components
The web handler files (playground.rs, analyze.rs) SHALL be refactored into smaller, logically-organized modules that each handle a specific HTTP endpoint or related functionality.

#### Scenario: playground.rs module creation
- **WHEN** analyzing the playground.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., code execution, AST display, UI state management)

#### Scenario: analyze.rs module creation
- **WHEN** analyzing the analyze.rs file structure
- **THEN** modules SHALL be created for distinct functionality (e.g., request handling, analysis orchestration, response formatting)

### Requirement: Web handler refactoring SHALL maintain backward compatibility
All HTTP endpoints and their request/response formats SHALL remain unchanged after refactoring to ensure no breaking changes for API consumers.

#### Scenario: Endpoint preservation
- **WHEN** making HTTP requests to web endpoints after refactoring
- **THEN** all existing endpoints SHALL respond with identical status codes and response formats

### Requirement: Web handler modules SHALL be under 400 lines
Each refactored web handler module SHALL contain fewer than 400 lines of code to maintain readability and maintainability.

#### Scenario: Module size validation
- **WHEN** measuring line count of each new web handler module
- **THEN** each module SHALL contain less than 400 lines

### Requirement: Web handler modules SHALL include comprehensive documentation
Each web handler module SHALL include module-level documentation explaining its purpose, endpoints handled, and request/response contracts.

#### Scenario: Module documentation presence
- **WHEN** reading any web handler module file
- **THEN** module-level documentation SHALL be present at the top of the file

### Requirement: Web handler refactoring SHALL preserve test coverage
All existing tests for web handler functionality SHALL continue to pass after refactoring without modification.

#### Scenario: Test execution
- **WHEN** running tests for astgrep-web crate
- **THEN** all tests SHALL pass with 100% of previous coverage maintained
