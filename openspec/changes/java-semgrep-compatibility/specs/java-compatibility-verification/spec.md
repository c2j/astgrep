## ADDED Requirements

### Requirement: Java compatibility test runner
The system SHALL provide a test runner that compares astgrep Java analysis results against semgrep results for 38 test cases and reports compatibility status.

#### Scenario: Run all Java compatibility tests
- **WHEN** the user runs `tests/scripts/compatibility/run_java_comparison_tests.sh`
- **THEN** the script executes all 38 Java test cases and reports match counts for both astgrep and semgrep

#### Scenario: Test case has matching results
- **WHEN** astgrep and semgrep report the same number of matches for a test case
- **THEN** the test case is marked as MATCH (✅)

#### Scenario: Test case has differing results
- **WHEN** astgrep and semgrep report different match counts for a test case
- **THEN** the test case is marked as DIFFER (❌) with the difference displayed

### Requirement: Root cause analysis
For each mismatched test case, the system SHALL identify the root cause category: parsing difference, pattern matching difference, taint analysis gap, or output format difference.

#### Scenario: Identify parsing difference
- **WHEN** the Java AST structure differs between tools causing mismatch
- **THEN** the issue is categorized as "parsing" and documented

#### Scenario: Identify pattern matching difference
- **WHEN** pattern interpretation differs (metavariables, ellipses, etc.)
- **THEN** the issue is categorized as "pattern matching" and documented

#### Scenario: Identify taint analysis gap
- **WHEN** advanced taint tracking features are missing
- **THEN** the issue is categorized as "taint analysis" and documented

#### Scenario: Identify output format difference
- **WHEN** result counting or reporting differs
- **THEN** the issue is categorized as "output format" and documented

### Requirement: Compatibility fix verification
After fixing identified issues, the system SHALL verify that all 38 test cases produce matching results.

#### Scenario: All tests pass
- **WHEN** all 38 test cases match between astgrep and semgrep
- **THEN** the compatibility rate is 100% and all tests pass

#### Scenario: Partial tests pass
- **WHEN** some test cases still differ after fixes
- **THEN** remaining issues are documented with rationale for deferral
