## ADDED Requirements

### Requirement: Parse metavariable-type constraints from YAML rules
The rule parser SHALL recognize `metavariable-type` as a valid constraint in the patterns array and parse it into a MetavariableType condition.

#### Scenario: YAML rule with metavariable-type constraint
- **WHEN** a YAML rule contains a `metavariable-type` entry in the patterns array
- **THEN** the parser SHALL successfully parse the rule without errors
- **AND** the parser SHALL extract the metavariable name and type from the constraint

#### Scenario: Missing metavariable field
- **WHEN** a `metavariable-type` entry lacks the `metavariable` field
- **THEN** the parser SHALL return a parse error indicating the missing field

#### Scenario: Missing type field
- **WHEN** a `metavariable-type` entry lacks the `type` field
- **THEN** the parser SHALL return a parse error indicating the missing field

### Requirement: Apply type constraints during pattern matching
The pattern matching engine SHALL validate that metavariable matches satisfy the declared type constraint before reporting a match.

#### Scenario: Variable matches declared type
- **GIVEN** a pattern `$X.method()` with constraint `metavariable-type: {metavariable: $X, type: PrintWriter}`
- **AND** code containing `PrintWriter writer = ...; writer.method();`
- **WHEN** the pattern is matched against the code
- **THEN** the match SHALL be reported because `writer` is declared as `PrintWriter`

#### Scenario: Variable does not match declared type
- **GIVEN** a pattern `$X.method()` with constraint `metavariable-type: {metavariable: $X, type: PrintWriter}`
- **AND** code containing `String writer = ...; writer.method();`
- **WHEN** the pattern is matched against the code
- **THEN** the match SHALL NOT be reported because `writer` is not of type `PrintWriter`

#### Scenario: Cannot determine type
- **GIVEN** a pattern with a type constraint
- **AND** code where the variable's type cannot be determined
- **WHEN** the pattern is matched against the code
- **THEN** the match SHALL be reported (permissive default behavior)

### Requirement: Support language-specific type extraction
The type extraction logic SHALL support Java variable declarations and be extensible for other languages.

#### Scenario: Java variable declaration
- **GIVEN** Java code containing `TypeName variableName = expression;`
- **WHEN** extracting type information for `variableName`
- **THEN** the system SHALL identify the type as `TypeName`

#### Scenario: Multiple variable declarations
- **GIVEN** Java code with multiple variable declarations of different types
- **WHEN** extracting type information for each variable
- **THEN** the system SHALL correctly associate each variable with its declared type
