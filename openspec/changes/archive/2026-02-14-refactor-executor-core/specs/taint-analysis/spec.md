## ADDED Requirements

### Requirement: TaintAnalyzer trait definition
The system SHALL provide a `TaintAnalyzer` trait in `executor/traits/taint.rs` with the following methods:
- `execute_taint_analysis`: Execute taint analysis for a rule
- `find_taint_sources`: Find taint sources in the AST
- `find_taint_sinks`: Find taint sinks in the AST
- `detect_taint_flows`: Detect flows between sources and sinks

#### Scenario: Trait is publicly accessible
- **WHEN** other modules import `executor::traits::TaintAnalyzer`
- **THEN** the trait and all its methods are accessible

#### Scenario: Trait methods have correct signatures
- **WHEN** implementing `TaintAnalyzer` trait
- **THEN** all four methods MUST match the signatures defined in the trait

### Requirement: DefaultTaintAnalyzer implementation
The system SHALL provide a `DefaultTaintAnalyzer` struct in `executor/impls/taint.rs` that implements `TaintAnalyzer`.

#### Scenario: Implementation compiles
- **WHEN** `DefaultTaintAnalyzer` is instantiated
- **THEN** all trait methods are implemented without `todo!()` panics

#### Scenario: Implementation produces same results as original
- **WHEN** taint analysis is executed via `DefaultTaintAnalyzer`
- **THEN** results SHALL be identical to the original `AdvancedRuleExecutor` implementation

### Requirement: TaintAnalyzer composition in executor
The system SHALL store a `Box<dyn TaintAnalyzer>` in `AdvancedRuleExecutor` and delegate taint-related operations.

#### Scenario: Executor delegates taint analysis
- **WHEN** `execute_taint_analysis` is called on `AdvancedRuleExecutor`
- **THEN** the call is delegated to the internal `TaintAnalyzer` instance

#### Scenario: Custom TaintAnalyzer can be injected
- **WHEN** constructing `AdvancedRuleExecutor` with a custom analyzer
- **THEN** the custom analyzer is used for all taint operations
