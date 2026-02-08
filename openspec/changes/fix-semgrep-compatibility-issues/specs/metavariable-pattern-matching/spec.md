## ADDED Requirements

### Requirement: Metavariable pattern matching
The system SHALL support constraining metavariable values using nested pattern matching via the `metavariable-pattern` YAML directive.

#### Scenario: Single pattern constraint
- **WHEN** a rule contains `metavariable-pattern` with a single `pattern` field
- **THEN** the system SHALL verify that the metavariable's bound value matches the specified pattern

#### Scenario: Multiple patterns constraint
- **WHEN** a rule contains `metavariable-pattern` with a `patterns` array
- **THEN** the system SHALL verify that the metavariable's bound value matches ALL specified patterns

#### Scenario: Pattern-either constraint
- **WHEN** a rule contains `metavariable-pattern` with a `pattern-either` field
- **THEN** the system SHALL verify that the metavariable's bound value matches ANY of the alternative patterns

#### Scenario: Metavariable-pattern with import resolution
- **GIVEN** a Java file with import statement `import org.foo.Foo`
- **AND** a rule with `metavariable-pattern` matching the imported type against `org.foo.Foo`
- **WHEN** the metavariable is bound to the imported type
- **THEN** the system SHALL resolve the type and verify the match
