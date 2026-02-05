# Implementation Plan: Test Directory Reorganization

**Branch**: `001-test-organization` | **Date**: 2025-12-03 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-test-organization/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Reorganize ASTGreP's test directory structure from the current scattered layout to a well-organized `newtest/` directory with hierarchical categorization. The feature will create `newtest/scripts/{category}/` for test scripts and `newtest/testcases/{language}/{test-type}/` for test cases, using primary functional purpose classification while preserving all existing functionality through a gradual migration approach.

## Technical Context

**Language/Version**: Rust 1.70+
**Primary Dependencies**: tree-sitter, clap, serde, anyhow, tokio, rayon
**Storage**: File system (directories and scripts)
**Testing**: cargo test, custom shell script test suites
**Target Platform**: Cross-platform (Linux, macOS, Windows)
**Project Type**: Single project with workspace architecture
**Performance Goals**: Script execution within 10 seconds for discovery, 15% reduction in language-specific test execution time
**Constraints**: Must maintain 100% backward compatibility for CI/CD pipelines, preserve relative dependencies
**Scale/Scope**: Approximately 100+ test scripts across 20+ programming languages, 1000+ test case files

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitution Gates for ASTGreP**:
- [x] **Modular Architecture**: Feature integrates with existing workspace crates as CLI extension in astgrep-cli
- [x] **CLI Interface**: All functionality accessible via CLI with JSON and human-readable output formats
- [x] **Test-First**: Tests written first with failing tests before implementation
- [x] **Performance**: Multi-threading support for parallel file migration and profiling capabilities
- [x] **Security**: Safe for analyzing untrusted code with no code execution during migration
- [x] **Language Support**: Maintains existing language support patterns, no new languages added

## Project Structure

### Documentation (this feature)

```text
specs/001-test-organization/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
# Single project structure
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# New organized structure (target)
newtest/
├── scripts/
│   ├── validation/
│   ├── performance/
│   ├── compatibility/
│   └── benchmarking/
└── testcases/
    ├── java/
    │   ├── pattern-matching/
    │   ├── dataflow/
    │   └── security/
    ├── python/
    ├── javascript/
    └── [other languages]/
```

**Structure Decision**: Creating new `newtest/` directory while preserving existing `tests/` for gradual migration. The new structure follows hierarchical organization with clear separation between executable scripts and test case data.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None identified at this stage | All gates appear achievable with current workspace architecture | N/A |