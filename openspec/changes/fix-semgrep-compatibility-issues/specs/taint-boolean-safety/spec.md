## ADDED Requirements

### Requirement: Boolean sanitization in taint analysis
The system SHALL support the `taint_assume_safe_booleans` rule option to treat boolean expressions and Boolean wrapper objects as implicitly sanitized.

#### Scenario: Boolean comparison expression
- **GIVEN** a rule with `taint_assume_safe_booleans: true`
- **AND** a tainted variable `x` of type String
- **WHEN** the code uses `(x != "safe")` as part of a sink argument
- **THEN** the system SHALL NOT flag this as a taint violation

#### Scenario: Boolean.valueOf sanitization
- **GIVEN** a rule with `taint_assume_safe_booleans: true`
- **AND** a tainted variable `x` passed to `Boolean.valueOf(x)`
- **WHEN** the result is used in a sink
- **THEN** the system SHALL NOT flag this as a taint violation

#### Scenario: Boolean.parseBoolean sanitization
- **GIVEN** a rule with `taint_assume_safe_booleans: true`
- **AND** a tainted variable `x` passed to `Boolean.parseBoolean(x)`
- **WHEN** the result is used in a sink
- **THEN** the system SHALL NOT flag this as a taint violation

#### Scenario: String concatenation with boolean (not safe)
- **GIVEN** a rule with `taint_assume_safe_booleans: true`
- **AND** a tainted variable `x` used in string concatenation `"something" + x`
- **WHEN** the concatenated string is passed to a sink
- **THEN** the system SHALL flag this as a taint violation (no boolean involved)
