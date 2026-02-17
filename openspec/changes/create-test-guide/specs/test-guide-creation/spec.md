## ADDED Requirements

### Requirement: Test guide document exists
A comprehensive TEST_GUIDE.md SHALL exist at the tests directory root.

#### Scenario: Guide is accessible
- **WHEN** a user navigates to the tests directory
- **THEN** they SHALL find a TEST_GUIDE.md file at the root level
- **AND** the guide SHALL provide instructions on running tests

### Requirement: Guide documents all test scripts
The guide SHALL document every test script in the scripts/ directory.

#### Scenario: Script documentation is complete
- **WHEN** a user reads the TEST_GUIDE.md
- **THEN** they SHALL find documentation for each script including:
  - Script name and location
- **AND** purpose and functionality
- **AND** how to run the script
- **AND** expected output or results

### Requirement: Guide explains directory structure
The guide SHALL explain the hierarchical directory structure.

#### Scenario: Directory structure is documented
- **WHEN** a user reads the TEST_GUIDE.md
- **THEN** they SHALL understand the purpose of each subdirectory:
  - scripts/ - for executable test runners
  - cases/ - for test case files
  - config/ - for test configurations
  - reports/ - for generated reports
  - lib/ - for Rust test modules

### Requirement: Guide includes quick start section
The guide SHALL include a quick start section for new users.

#### Scenario: Quick start is available
- **WHEN** a new user opens TEST_GUIDE.md
- **THEN** they SHALL find a "Quick Start" section within the first 50 lines
- **AND** it SHALL provide the most common command to run tests
