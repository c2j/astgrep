<!--
Sync Impact Report:
Version change: 0.0.0 → 1.0.0 (initial constitution creation)
Modified principles: None (initial creation)
Added sections: All sections (initial creation)
Removed sections: None (initial creation)
Templates requiring updates: ✅ plan-template.md (constitution check), ✅ spec-template.md (scope alignment), ✅ tasks-template.md (task categorization)
Follow-up TODOs: None
-->

# ASTGreP Constitution

## Core Principles

### I. Modular Architecture-First

Every feature MUST start as a standalone crate within the workspace architecture. Each crate MUST be self-contained with independently testable functionality and a clear, specific purpose. Organizational-only crates without distinct technical purpose are prohibited. Modular boundaries must be enforced through clear API contracts and dependency management.

### II. CLI Interface with Multiple Formats

All functionality MUST be exposed via CLI interface as the primary user interaction method. The interface MUST follow text in/out protocol: stdin/arguments → stdout, errors → stderr. Support for both JSON (programmatic) and human-readable formats MUST be provided for all commands. CLI commands MUST be discoverable, composable, and follow consistent naming conventions.

### III. Test-First Development (NON-NEGOTIABLE)

Test-Driven Development is mandatory: Tests MUST be written before implementation, user stories MUST be approved before development, tests MUST fail initially, then implementation must proceed. Red-Green-Refactor cycle is strictly enforced. Every crate MUST have comprehensive unit tests, integration tests for cross-crate functionality, and end-to-end validation tests. Performance benchmarks MUST be included for all critical paths.

### IV. Multi-Language Extensibility

New language support MUST follow established patterns: language variant added to `astgrep_core::Language` enum, parser created in `astgrep-parser/src/{language}.rs`, tree-sitter adapter implemented in `astgrep-parser::adapters`, file extension mappings added, and comprehensive tests written in `tests/{language}/`. Language parsers MUST gracefully handle unsupported constructs and provide detailed error context.

### V. Performance and Observability

Performance is a first-class concern. Multi-threaded analysis MUST be enabled by default with configurable thread counts. Performance profiling MUST be available via `--profile` flag. Structured logging with configurable levels MUST be implemented. All critical operations MUST include performance metrics and be benchmarkable via `cargo bench`. Memory usage MUST be tracked and optimized for large codebase analysis.

### VI. Security-First Design

This tool is designed for defensive security analysis only. The tool MUST be safe for analyzing untrusted code with no code execution capabilities. Rule development MUST focus on vulnerability detection and security code review. All features MUST support security workflows and compliance checking. Integration with CI/CD pipelines for automated security scanning MUST be provided.

## Quality Standards

### Performance Requirements

- Analysis of 10,000+ lines of code MUST complete within acceptable time limits
- Memory usage MUST scale linearly with codebase size
- Parallel processing efficiency MUST be demonstrated in benchmarks
- Startup time MUST be minimized for frequent CLI usage

### Output Format Standards

- JSON: Structured results for programmatic consumption with stable schema
- SARIF: Static Analysis Results Interchange Format for CI/CD integration
- Text: Human-readable format with detailed explanations and fix suggestions
- YAML: YAML-structured output for configuration and rule exchange
- XML: Legacy format compatibility for enterprise tool integration

### Rule Development Standards

Rules MUST be defined in YAML format with pattern matching, metavariables, and data flow specifications. Rule validation MUST happen in `astgrep-rules` crate. Rules MUST include test cases demonstrating effectiveness and performance characteristics. Rule metadata MUST include severity, confidence, categorization, and references to security standards (CWE, OWASP).

## Development Workflow

### Code Review Requirements

All pull requests MUST: pass all tests, include relevant documentation updates, demonstrate performance characteristics, maintain backward compatibility where applicable, and include security considerations for new features.

### Testing Gates

- Unit tests: 100% coverage for critical paths in all crates
- Integration tests: Cross-crate functionality validation
- Language-specific tests: Validation for each supported language
- Performance tests: Regression prevention and benchmarking
- Rule validation: Syntax and semantic correctness for all rule changes

### Quality Assurance

Comprehensive validation MUST be performed via: `cargo test` for unit tests, `./tests/run_validation_suite.sh` for end-to-end validation, `./tests/run_advanced_pattern_tests.sh` for pattern matching validation, and `cargo run -- validate rules/` for rule correctness verification.

## Governance

This constitution supersedes all other development practices and guidelines. Any conflicts between this constitution and other documents MUST be resolved in favor of this constitution.

### Amendment Process

- Amendments require documentation of proposed changes, approval from project maintainers, and a migration plan for affected code.
- Version numbers MUST follow semantic versioning (MAJOR.MINOR.PATCH).
- All amendments MUST update this document and propagate changes to dependent templates and guidance documents.
- Amendments MUST be ratified via pull request with clear justification for changes.

### Versioning Policy

- **MAJOR**: Backward incompatible changes, principle removals or redefinitions
- **MINOR**: New principles added, material expansion of existing guidance
- **PATCH**: Clarifications, wording improvements, non-semantic refinements

### Compliance Review

All pull requests and code changes MUST verify compliance with this constitution. Project templates (plan-template.md, spec-template.md, tasks-template.md) MUST be kept in sync with constitutional principles. Runtime development guidance in CLAUDE.md MUST reflect constitutional requirements.

**Version**: 1.0.0 | **Ratified**: 2025-12-03 | **Last Amended**: 2025-12-03