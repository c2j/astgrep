# astgrep Unified Roadmap

**Last Updated**: 2026-05-19
**Source**: Merged from design-v1.md, design-v1.1.md, REFACTORING_PLAN.md, enhance-rules.md, and specs/001-test-organization/

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Complete |
| 🚧 | In Progress |
| 📋 | Planned (next up) |
| 📦 | Backlog |
| ❌ | Dropped / Deprecated |

---

## Phase 0: Codebase Health (Current)

**Goal**: Make the project robust enough for sustainable development.

| ID | Task | Status | Source |
|----|------|--------|--------|
| H-01 | Fix all compilation errors | ✅ | Build was broken |
| H-02 | Fix pre-existing test compilation errors in astgrep-cli | 📋 | Missing Language variants (Php/CSharp/C/Swift/Ruby/Kotlin) |
| H-03 | Reduce compiler warnings (parser: 48, dataflow: 32, cli: 100) | 📋 | cargo fix |
| H-04 | Add GitHub Actions CI/CD | 📋 | No automated validation |
| H-05 | Remove debug `eprintln!` statements in production code | 📋 | Performance/noise |

---

## Phase 1: Architecture Refactoring

**Goal**: Break oversized files into maintainable modules.
**Source**: REFACTORING_PLAN.md (527 lines, 4 phases, 13 weeks)

| ID | Task | Target File | Lines | Status | Source |
|----|------|-------------|-------|--------|--------|
| R-01 | Extract output formatting (JSON/SARIF/HTML/Markdown/Text/Semgrep) | `analyze_enhanced.rs` | ~2,870 → ~1,570 | 📋 | Refactoring Plan §1.1 |
| R-02 | Extract file collection/discovery | `analyze_enhanced.rs` | -200 | 📋 | Refactoring Plan §1.2 |
| R-03 | Extract rule parsing/loader | `analyze_enhanced.rs` | -300 | 📋 | Refactoring Plan §1.3 |
| R-04 | Extract analysis core (pattern/taint/utils) | `analyze_enhanced.rs` | -400 | 📋 | Refactoring Plan §1.4 |
| R-05 | Extract condition evaluation from executor | `executor.rs` | ~2,815 → ~800 | 📋 | Refactoring Plan §2.2 |
| R-06 | Extract taint analysis from executor | `executor.rs` | -300 | 📋 | Refactoring Plan §2.3 |
| R-07 | Extract parallel execution strategies from engine | `engine.rs` | ~2,214 → ~900 | 📋 | Refactoring Plan §3.1 |
| R-08 | Extract tokenization from matcher | `advanced_matcher.rs` | ~1,967 → ~800 | 📋 | Refactoring Plan §4.1 |
| R-09 | Extract matching strategies (sequence/literal/wildcard) | `advanced_matcher.rs` | -600 | 📋 | Refactoring Plan §4.2 |

**Success Metrics**: All files < 1,000 lines. Functions < 50 lines. All tests pass.

---

## Phase 2: Semgrep Compatibility

**Goal**: Pass all 38 semgrep-core compatibility tests.
**Source**: enhance-rules.md (615 lines), recent commits fixing 22/38

| ID | Task | Status | Source |
|----|------|--------|--------|
| S-01 | pattern-not-inside support | ✅ | Commit 53e2302 |
| S-02 | pattern-not-regex support | ✅ | Commit 53e2302 |
| S-03 | focus-metavariable support | ✅ | Commit 53e2302 |
| S-04 | metavariable-regex support | ✅ | enhance-rules.md Phase 1 |
| S-05 | metavariable-comparison support | ✅ | enhance-rules.md Phase 1 |
| S-06 | metavariable-name support | ✅ | enhance-rules.md Phase 1 |
| S-07 | metavariable-pattern support | 🚧 | Partially implemented |
| S-08 | Remaining 16/38 compatibility fixes | 📋 | enhance-rules.md Phase 2-3 |
| S-09 | Rule options support (paths, version constraints) | 📦 | enhance-rules.md Phase 2 |
| S-10 | Cross-file analysis support | 📦 | enhance-rules.md Phase 3 |

---

## Phase 3: Test Infrastructure

**Goal**: Complete test directory reorganization.
**Source**: specs/001-test-organization/ (53 tasks, 25 complete)

| ID | Task | Status | Source |
|----|------|--------|--------|
| T-01 | Phase 1: Setup infrastructure (T001-T006) | ✅ | Spec 001 |
| T-02 | Phase 2: Foundation (T007-T014) | ✅ | Spec 001 |
| T-03 | Phase 3: US1 Script organization (T015-T024) | ✅ | Spec 001 |
| T-04 | Phase 4: US2 Test case organization (T025-T034) | 🚧 | T025 done, T026-T034 pending |
| T-05 | Phase 5: US3 Document difficult content (T035-T042) | 📋 | Spec 001 |
| T-06 | Phase 6: Integration and polish (T043-T053) | 📋 | Spec 001 |

---

## Phase 4: Feature Enhancements

**Goal**: Expand language support and analysis capabilities.

| ID | Task | Status | Source |
|----|------|--------|--------|
| F-01 | Add C/C++ language support (tree-sitter adapters exist) | 📦 | AGENTS.md |
| F-02 | Add C# language support | 📦 | AGENTS.md |
| F-03 | Add Kotlin language support | 📦 | AGENTS.md |
| F-04 | Add Ruby language support | 📦 | AGENTS.md |
| F-05 | Add Swift language support | 📦 | AGENTS.md |
| F-06 | Add PHP language support | 📦 | AGENTS.md |
| F-07 | Type inference across all languages | 📦 | design-v1.md |
| F-08 | Cross-file taint analysis | 📦 | design-v1.md |

---

## Phase 5: Integrations & Ecosystem

**Goal**: Make astgrep part of the development workflow.

| ID | Task | Status | Source |
|----|------|--------|--------|
| E-01 | VS Code extension | 📦 | design-v1.1.md §9.2 |
| E-02 | IDE integrations (IntelliJ) | 📦 | design-v1.md §2.1 |
| E-03 | CI/CD pipeline templates (GitHub/GitLab) | 📦 | design-v1.1.md §9.2 |
| E-04 | Rule marketplace / registry | 📦 | design-v1.1.md §9.2 |
| E-05 | SARIF integration with GitHub Code Scanning | 📦 | — |

---

## Completed Milestones

| Version | Description | Date |
|---------|-------------|------|
| v1.0 | Core analysis engine, CLI, multi-language support | 2025-10 |
| v1.1 | Web Playground, pattern matching enhancements | 2025-10 |
| v1.2 | Web enhancements, cross-compile, XML support, SQL rules | 2025-11 |
| v1.3 | GUI desktop playground (egui) | 2025-12 |

---

## Deprecation Notes

- **`tree-sitter-sql`**: Do NOT use. Use `tree-sitter-sequel` for SQL parsing.
- **`pattern-where-python`**: Not supported. Use `metavariable-comparison` instead.
