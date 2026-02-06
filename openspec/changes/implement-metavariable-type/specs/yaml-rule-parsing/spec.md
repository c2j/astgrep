## ADDED Requirements

### Requirement: Recognize metavariable-type as valid pattern constraint
The YAML rule parser SHALL accept `metavariable-type` entries in the patterns array alongside existing pattern types and metavariable constraints.

#### Scenario: Patterns array with mixed entries
- **GIVEN** a patterns array containing `pattern`, `pattern-not`, and `metavariable-type` entries
- **WHEN** the rule is parsed
- **THEN** all entries SHALL be processed successfully
- **AND** the `metavariable-type` constraint SHALL be attached to the preceding pattern

#### Scenario: Metavariable-type without preceding pattern
- **GIVEN** a patterns array where `metavariable-type` is the first entry
- **WHEN** the rule is parsed
- **THEN** the parser SHALL return an error indicating a pattern must precede metavariable constraints

#### Scenario: Multiple metavariable constraints on same pattern
- **GIVEN** a patterns array with a pattern followed by multiple `metavariable-type` constraints
- **WHEN** the rule is parsed
- **THEN** all constraints SHALL be attached to the pattern
- **AND** all constraints SHALL be evaluated during matching
