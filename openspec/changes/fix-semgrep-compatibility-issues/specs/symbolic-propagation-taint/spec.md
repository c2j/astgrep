## ADDED Requirements

### Requirement: Symbolic propagation in taint mode
The system SHALL support `symbolic_propagation` option in taint mode rules to track variable aliases and values through assignment statements.

#### Scenario: Simple variable assignment
- **GIVEN** a rule with `options.symbolic_propagation: true` and mode `taint`
- **AND** source pattern `DocumentBuilderFactory.newInstance()`
- **AND** sink pattern `$FACTORY.newDocumentBuilder()`
- **WHEN** code assigns `DocumentBuilderFactory.newInstance()` to variable `dbf`
- **AND** code calls `dbf.newDocumentBuilder()`
- **THEN** the system SHALL detect the taint flow from source to sink

#### Scenario: Static field initialization
- **GIVEN** a rule with `symbolic_propagation: true` in taint mode
- **WHEN** a static class field is initialized with a taint source
- **AND** that field is later used in a sink method call
- **THEN** the system SHALL track the flow through the static field reference

#### Scenario: Symbol propagation through method chains
- **GIVEN** a rule with `symbolic_propagation: true`
- **WHEN** a variable is assigned from another variable that holds a taint source
- **AND** the assigned variable is used in a sink
- **THEN** the system SHALL propagate the symbolic value through the assignment chain
