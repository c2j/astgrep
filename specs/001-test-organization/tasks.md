---

description: "Task list for test directory reorganization feature implementation"
---

# Tasks: Test Directory Reorganization

**Input**: Design documents from `/specs/001-test-organization/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/
**Tests**: Integration tests included for CLI functionality and migration verification

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story

## Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/`, `crates/` at repository root
- **New test structure**: `newtest/` with hierarchical organization
- **CLI integration**: `crates/astgrep-cli/src/commands/`
- Paths shown below follow the hierarchical structure defined in the plan

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Create newtest directory structure following hierarchical organization pattern
- [x] T002 Initialize migration CLI module in crates/astgrep-cli/src/commands/migrate.rs
- [x] T003 [P] Configure Cargo workspace dependencies for migration functionality in Cargo.toml
- [x] T004 [P] Set up structured logging with tracing and configurable levels for migration operations
- [x] T005 [P] Configure error handling with anyhow and detailed context messages
- [x] T006 [P] Set up performance benchmarking framework using criterion for migration operations

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T007 Set up file system operations infrastructure with rsync-based migration support
- [x] T008 [P] Implement multi-threading support with configurable thread pools for parallel file operations
- [x] T009 [P] Create performance profiling infrastructure with --profile flag support
- [x] T010 Configure cross-platform path handling for Windows, macOS, and Linux compatibility
- [x] T011 Create backup and rollback mechanisms for failed migrations
- [x] T012 Set up migration validation framework with checksum verification
- [x] T013 Implement JSON and human-readable output formats for migration CLI commands
- [x] T014 Create progress tracking and reporting infrastructure for migration operations

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Organize Test Scripts (Priority: P1) 🎯 MVP

**Goal**: Reorganize test scripts by functional category while preserving execution capabilities

**Independent Test**: Verify that all scripts execute successfully from new locations and produce expected results

### Implementation for User Story 1

- [x] T015 [P] [US1] Create TestScript entity model in crates/astgrep-core/src/models/test_asset.rs
- [x] T016 [P] [US1] Create script discovery module in crates/astgrep-parser/src/script_discovery.rs
- [x] T017 [US1] Implement script categorization logic in crates/astgrep-matcher/src/script_classifier.rs
- [x] T018 [US1] Create migration engine for scripts in crates/astgrep-cli/src/commands/migrate_scripts.rs
- [x] T019 [US1] Implement script execution validation in crates/astgrep-cli/src/commands/validate_scripts.rs
- [x] T020 [P] [US1] Create functional category directory structure: newtest/scripts/validation/, newtest/scripts/performance/, newtest/scripts/compatibility/, newtest/scripts/benchmarking/
- [x] T021 [US1] Update validate.sh script to work with new directory structure in newtest/scripts/runners/
- [x] T022 [US1] Implement script dependency resolution in crates/astgrep-cli/src/dependencies/script_deps.rs
- [x] T023 [US1] Create script migration CLI command with dry-run support in crates/astgrep-cli/src/commands/migrate.rs
- [x] T024 [US1] Add script execution verification functionality in crates/astgrep-cli/src/verification/script_runner.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Organize Test Cases by Language and Category (Priority: P1)

**Goal**: Group test cases by programming language with consistent directory structure

**Independent Test**: Test case discovery and execution from new directory structure with all original test cases accessible

### Implementation for User Story 2

- [x] T025 [P] [US2] Create TestCase entity model in crates/astgrep-core/src/models/test_case.rs
- [ ] T026 [P] [US2] Implement language-specific test discovery in crates/astgrep-parser/src/language_discovery.rs
- [ ] T027 [US2] Create test case migration engine in crates/astgrep-cli/src/commands/migrate_test_cases.rs
- [ ] T028 [US2] Update comprehensive_test_runner.py for new directory structure in newtest/scripts/runners/
- [ ] T029 [P] [US2] Create language directory structure: newtest/testcases/{language}/{test-type}/
- [ ] T030 [US2] Implement test-case-to-rule-file pairing logic in crates/astgrep-matcher/src/test_pairing.rs
- [ ] T031 [US2] Update test discovery patterns for hierarchical structure in crates/astgrep-cli/src/discovery/patterns.rs
- [ ] T032 [US2] Create language mapping configuration in crates/astgrep-core/src/config/language_mapping.rs
- [ ] T033 [P] [US2] Implement cross-language test case handling with primary language classification
- [ ] T034 [US2] Add test case execution validation in crates/astgrep-cli/src/verification/test_case_runner.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Document Difficult-to-Organize Content (Priority: P2)

**Goal**: Identify and document test assets that cannot be easily reorganized

**Independent Test**: Review generated documentation and verify all problematic test assets are properly catalogued

### Implementation for User Story 3

- [ ] T035 [P] [US3] Create problematic asset detection module in crates/astgrep-cli/src/analysis/problem_detector.rs
- [ ] T036 [US3] Implement dependency analysis engine in crates/astgrep-cli/src/analysis/dependency_analyzer.rs
- [ ] T037 [US3] Create documentation generation service in crates/astgrep-cli/src/docs/generator.rs
- [ ] T038 [US3] Implement asset categorization with conflict detection in crates/astgrep-cli/src/classification/asset_classifier.rs
- [ ] T039 [P] [US3] Create migration constraint analysis in crates/astgrep-cli/src/analysis/constraint_analyzer.rs
- [ ] T040 [US3] Generate organization report with prioritized action items in crates/astgrep-cli/src/reports/organization_report.rs
- [ ] T041 [US3] Create asset metadata documentation system in crates/astgrep-cli/src/docs/metadata.rs
- [ ] T042 [US3] Implement remediation suggestion engine in crates/astgrep-cli/src/suggestions/remediation.rs

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Integration and Polish

**Purpose**: Improvements that affect multiple user stories and ensure overall system coherence

- [ ] T043 [P] Create comprehensive CLI migration interface in crates/astgrep-cli/src/commands/migrate.rs
- [ ] T044 [P] Implement migration orchestration service in crates/astgrep-cli/src/services/migration_orchestrator.rs
- [ ] T045 [P] Add progress reporting and dashboard in crates/astgrep-cli/src/ui/progress.rs
- [ ] T046 [P] Create migration rollback functionality in crates/astgrep-cli/src/commands/rollback.rs
- [ ] T047 [P] Implement CI/CD integration utilities in crates/astgrep-cli/src/ci/integration.rs
- [ ] T048 [P] Add comprehensive error handling and recovery in crates/astgrep-cli/src/error/migration_error.rs
- [ ] T049 Create performance optimization for large-scale migrations in crates/astgrep-cli/src/performance/optimizer.rs
- [ ] T050 [P] Add migration verification and validation suite in crates/astgrep-cli/src/verification/migration_validator.rs
- [ ] T051 Create documentation and user guides in docs/migration_guide.md
- [ ] T052 [P] Update existing test runner scripts to support compatibility mode
- [ ] T053 Create migration status tracking and persistence in crates/astgrep-cli/src/state/migration_state.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (US1 → US2 → US3)
- **Integration (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - May share infrastructure with US1 but should be independently testable
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - May analyze results from US1/US2 but should be independently functional

### Within Each User Story

- Models and discovery components can be developed in parallel
- Migration engines depend on models and discovery
- CLI commands depend on migration engines
- Verification components depend on migration completion

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- Models within each story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all models for User Story 1 together:
Task: "Create TestScript entity model in crates/astgrep-core/src/models/test_asset.rs"
Task: "Create script discovery module in crates/astgrep-parser/src/script_discovery.rs"
Task: "Create functional category directory structure: newtest/scripts/validation/, newtest/scripts/performance/, newtest/scripts/compatibility/, newtest/scripts/benchmarking/"

# Launch all migration components for User Story 1 together:
Task: "Implement script categorization logic in crates/astgrep-matcher/src/script_classifier.rs"
Task: "Create migration engine for scripts in crates/astgrep-cli/src/commands/migrate_scripts.rs"
Task: "Implement script dependency resolution in crates/astgrep-cli/src/dependencies/script_deps.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (script organization)
   - Developer B: User Story 2 (test case organization)
   - Developer C: User Story 3 (documentation of problematic assets)
3. Stories complete and integrate independently

---

## Testing Strategy

### Integration Tests for CLI Functionality

- Test script migration with verification of execution
- Test test case migration with discovery validation
- Test problematic asset detection and documentation generation
- Test rollback functionality with data integrity verification

### Performance Tests

- Large-scale migration performance (1000+ files)
- Parallel processing efficiency
- Memory usage optimization
- Cross-platform performance consistency

### Cross-Platform Tests

- Windows path handling and permission management
- macOS file system operations and symlink handling
- Linux performance and compatibility verification

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- All tasks maintain ASTGreP constitutional compliance (CLI interface, test-first development, performance, security)
- Migration operations must be reversible with complete rollback capability
- All file operations must preserve timestamps and permissions where possible
- Cross-platform compatibility is required for all migration operations