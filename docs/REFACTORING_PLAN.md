# ASTGrep Codebase Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan for the astgrep codebase to improve maintainability, testability, and developer productivity. The focus is on the four largest files that have grown beyond sustainable limits.

## Current State Analysis

### File Size Distribution

| File | Lines | Functions/Methods | Issues |
|------|-------|------------------|---------|
| `analyze_enhanced.rs` | 2,870 | 60+ | God file with mixed responsibilities |
| `executor.rs` | 2,815 | 14+ | Single struct with too many concerns |
| `engine.rs` | 2,214 | 30+ | Mixed execution and management logic |
| `advanced_matcher.rs` | 1,967 | 25+ | Large methods, monolithic design |

### Key Problems Identified

1. **Violation of Single Responsibility Principle (SRP)**
   - Files handling file I/O, pattern matching, output formatting, and analysis in one place
   
2. **Poor Separation of Concerns**
   - Business logic mixed with infrastructure code
   - Domain-specific code mixed with generic utilities

3. **Testability Issues**
   - Large functions with multiple dependencies
   - Hard to unit test individual components

4. **Cognitive Load**
   - Developers need to understand entire file to make changes
   - High risk of unintended side effects

## Refactoring Strategy

### Guiding Principles

1. **Single Responsibility**: Each module/file should have one reason to change
2. **High Cohesion**: Related functionality grouped together
3. **Low Coupling**: Minimize dependencies between modules
4. **Incremental Changes**: Refactor in small, reviewable chunks
5. **Backward Compatibility**: Maintain public API where possible

### Architecture Target

```
┌─────────────────────────────────────────────────────────────┐
│                    astgrep-cli                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   commands   │  │   analysis   │  │    output    │       │
│  │              │  │              │  │              │       │
│  │ - analyze    │  │ - pattern    │  │ - json       │       │
│  │ - validate   │  │ - taint      │  │ - sarif      │       │
│  │ - info       │  │ - symbolic   │  │ - html       │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                  astgrep-rules                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   parsing    │  │   execution  │  │   engine     │       │
│  │              │  │              │  │              │       │
│  │ - yaml       │  │ - rules      │  │ - parallel   │       │
│  │ - patterns   │  │ - conditions │  │ - lifecycle  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                  astgrep-matcher                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   patterns   │  │   matching   │  │   bindings   │       │
│  │              │  │              │  │              │       │
│  │ - parsing    │  │ - sequence   │  │ - metavar    │       │
│  │ - tokenizing │  │ - semantic   │  │ - scope      │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

## Phase 1: Analyze Enhanced Module (Priority: High)

### Current State
**File**: `crates/astgrep-cli/src/commands/analyze_enhanced.rs`
**Size**: 2,870 lines
**Responsibilities**: File collection, rule loading, pattern matching, taint analysis, 8 output formats, SQL extraction

### Refactoring Plan

#### Step 1.1: Extract Output Formatting (Week 1)
**Target**: Lines 2461-2856 (output generation functions)

**New Structure**:
```
crates/astgrep-cli/src/
└── output/
    ├── mod.rs          # Output trait and factory
    ├── json.rs         # JSON output
    ├── sarif.rs        # SARIF output
    ├── html.rs         # HTML output
    ├── markdown.rs     # Markdown output
    ├── text.rs         # Plain text output
    └── semgrep.rs      # Semgrep-compatible output
```

**Implementation Details**:
```rust
// output/mod.rs
pub trait OutputFormatter {
    fn format(&self, findings: &[Finding], stats: &AnalysisStatistics) -> Result<String>;
    fn content_type(&self) -> &'static str;
}

pub struct OutputFactory;
impl OutputFactory {
    pub fn create(format: OutputFormat) -> Box<dyn OutputFormatter> {
        match format {
            OutputFormat::Json => Box::new(JsonFormatter),
            OutputFormat::Sarif => Box::new(SarifFormatter),
            // ... etc
        }
    }
}
```

**Benefits**:
- Easy to add new output formats
- Each format can be tested independently
- Reduces `analyze_enhanced.rs` by ~400 lines

#### Step 1.2: Extract File Collection (Week 2)
**Target**: Lines 118-194 (file discovery logic)

**New Structure**:
```
crates/astgrep-cli/src/
└── file_collection/
    ├── mod.rs          # Public API
    ├── discovery.rs    # File discovery logic
    ├── filtering.rs    # Include/exclude filters
    └── glob_matching.rs # Glob pattern matching
```

**Implementation Details**:
```rust
// file_collection/mod.rs
pub struct FileCollector {
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    max_depth: Option<usize>,
}

impl FileCollector {
    pub fn collect(&self, root: &Path) -> Result<Vec<PathBuf>> {
        // Implementation
    }
}
```

**Benefits**:
- Reusable file collection logic
- Testable in isolation
- Reduces `analyze_enhanced.rs` by ~200 lines

#### Step 1.3: Extract Rule Parsing (Week 3)
**Target**: Lines 1502-1685 (rule parsing logic)

**New Structure**:
```
crates/astgrep-cli/src/
└── rule_parsing/
    ├── mod.rs
    ├── loader.rs       # Load rules from files/dirs
    ├── parser.rs       # Parse YAML rules
    └── validation.rs   # Rule validation
```

**Implementation Details**:
```rust
// rule_parsing/mod.rs
pub struct RuleLoader;

impl RuleLoader {
    pub fn load_from_path(&self, path: &Path, language: Language) -> Result<Vec<Rule>> {
        // Implementation
    }
    
    pub fn load_from_directory(&self, dir: &Path, language: Language) -> Result<Vec<Rule>> {
        // Implementation
    }
}
```

**Benefits**:
- Centralized rule loading logic
- Easier to support new rule formats
- Reduces `analyze_enhanced.rs` by ~300 lines

#### Step 1.4: Extract Analysis Core (Week 4)
**Target**: Lines 194-530 (analysis logic)

**New Structure**:
```
crates/astgrep-cli/src/
└── analysis/
    ├── mod.rs
    ├── analyzer.rs     # Main analysis orchestration
    ├── patterns.rs     # Pattern matching logic
    ├── taint.rs        # Taint analysis wrapper
    └── utils.rs        # Analysis utilities
```

**Benefits**:
- Clean separation of analysis concerns
- Easier to add new analysis types
- Reduces `analyze_enhanced.rs` by ~400 lines

### Phase 1 Result
- `analyze_enhanced.rs`: 2,870 → ~1,570 lines (-45%)
- Clear module boundaries
- Each module < 500 lines

## Phase 2: Rule Executor Module (Priority: High)

### Current State
**File**: `crates/astgrep-rules/src/executor.rs`
**Size**: 2,815 lines
**Responsibilities**: Pattern execution, condition evaluation, taint analysis, type checking, pattern conversion

### Refactoring Plan

#### Step 2.1: Extract Pattern Conversion (Week 5)
**Target**: Lines 1609-1720 (pattern conversion methods)

**New Structure**:
```
crates/astgrep-rules/src/
└── pattern/
    ├── mod.rs
    └── conversion.rs   # Convert between pattern types
```

**Benefits**:
- Isolated pattern transformation logic
- Easier to add new pattern types

#### Step 2.2: Extract Condition Evaluation (Week 6)
**Target**: Lines 1296-1600 (condition evaluation)

**New Structure**:
```
crates/astgrep-rules/src/
└── conditions/
    ├── mod.rs
    ├── evaluator.rs    # Condition evaluation engine
    ├── metavariable.rs # Metavariable conditions
    ├── regex.rs        # Regex conditions
    └── comparison.rs   # Comparison operators
```

**Implementation Details**:
```rust
// conditions/mod.rs
pub trait ConditionEvaluator {
    fn evaluate(&self, condition: &Condition, bindings: &HashMap<String, String>) -> Result<bool>;
}

pub struct CompositeEvaluator {
    evaluators: Vec<Box<dyn ConditionEvaluator>>,
}
```

**Benefits**:
- Pluggable condition system
- Easy to add new condition types
- Test each condition type independently

#### Step 2.3: Extract Taint Analysis (Week 7)
**Target**: Lines 460-760 (taint analysis execution)

**New Structure**:
```
crates/astgrep-rules/src/
└── taint/
    ├── mod.rs
    ├── analyzer.rs     # Taint analysis orchestration
    ├── sources.rs      # Source detection
    ├── sinks.rs        # Sink detection
    └── sanitizers.rs   # Sanitizer detection
```

**Benefits**:
- Specialized taint analysis module
- Easier to implement new taint rules
- Better separation from general rule execution

#### Step 2.4: Simplify AdvancedRuleExecutor (Week 8)
**Target**: Remaining methods in executor.rs

**Actions**:
- Keep only high-level orchestration methods
- Delegate to specialized modules
- Focus on `execute_comprehensive_analysis` as main entry point

### Phase 2 Result
- `executor.rs`: 2,815 → ~800 lines (-72%)
- Specialized modules for different concerns
- `AdvancedRuleExecutor` becomes a facade/coordinator

## Phase 3: Rule Engine Module (Priority: Medium)

### Current State
**File**: `crates/astgrep-rules/src/engine.rs`
**Size**: 2,214 lines
**Responsibilities**: Rule execution, parallel processing, pattern matching, rule lifecycle

### Refactoring Plan

#### Step 3.1: Extract Parallel Execution (Week 9)
**Target**: Lines 900-1200 (parallel execution logic)

**New Structure**:
```
crates/astgrep-rules/src/
└── execution/
    ├── mod.rs
    ├── parallel.rs     # Parallel rule execution
    ├── sequential.rs   # Sequential execution
    └── strategies.rs   # Execution strategies
```

**Benefits**:
- Pluggable execution strategies
- Easier to optimize parallel execution
- Test execution strategies independently

#### Step 3.2: Extract Engine Management (Week 10)
**Target**: Lines 1-400 (engine setup and configuration)

**New Structure**:
```
crates/astgrep-rules/src/
└── engine/
    ├── mod.rs
    ├── builder.rs      # Engine builder pattern
    ├── config.rs       # Engine configuration
    └── lifecycle.rs    # Engine lifecycle management
```

**Benefits**:
- Builder pattern for engine configuration
- Clear lifecycle management
- Easier to create different engine configurations

### Phase 3 Result
- `engine.rs`: 2,214 → ~900 lines (-59%)
- Clear separation between execution and management

## Phase 4: Advanced Matcher (Priority: Medium)

### Current State
**File**: `crates/astgrep-matcher/src/advanced_matcher.rs`
**Size**: 1,967 lines
**Responsibilities**: Pattern matching, tokenization, metavariable binding, sequence matching

### Refactoring Plan

#### Step 4.1: Extract Tokenization (Week 11)
**Target**: Lines 1203-1300 (tokenization logic)

**New Structure**:
```
crates/astgrep-matcher/src/
└── tokenization/
    ├── mod.rs
    └── tokenizer.rs    # Text tokenization
```

#### Step 4.2: Extract Pattern Matching Strategies (Week 12)
**Target**: Large methods like `try_match_sequence_at_position`

**New Structure**:
```
crates/astgrep-matcher/src/
└── matching/
    ├── mod.rs
    ├── sequence.rs     # Sequence matching
    ├── literal.rs      # Literal matching
    ├── metavariable.rs # Metavariable matching
    └── wildcard.rs     # Wildcard/ellipsis matching
```

#### Step 4.3: Extract Metavariable Management (Week 13)
**Target**: Metavariable binding and scope logic

**New Structure**:
```
crates/astgrep-matcher/src/
└── metavariable/
    ├── mod.rs
    ├── manager.rs      # Metavariable manager
    ├── binding.rs      # Binding representation
    └── scope.rs        # Scope management
```

### Phase 4 Result
- `advanced_matcher.rs`: 1,967 → ~800 lines (-59%)
- Specialized matching strategies
- Reusable tokenization logic

## Testing Strategy

### During Refactoring

1. **Characterization Tests**: Before refactoring, write tests that capture current behavior
2. **Incremental Testing**: Test each extracted module independently
3. **Integration Tests**: Ensure refactored modules work together
4. **Performance Tests**: Monitor performance to ensure no regressions

### Test Structure

```
crates/<crate>/
├── src/
│   └── ... (refactored modules)
└── tests/
    ├── integration_tests.rs
    └── <module>/
        ├── mod.rs
        └── <specific_tests>.rs
```

## Migration Plan

### Phase 1: Output Module Migration

```rust
// Before (in analyze_enhanced.rs)
fn generate_json_output(findings: &[Finding]) -> String {
    // 50 lines of JSON generation
}

fn generate_sarif_output(findings: &[Finding]) -> String {
    // 80 lines of SARIF generation
}

// After (in output/mod.rs)
use crate::output::OutputFactory;

let formatter = OutputFactory::create(OutputFormat::Json);
let output = formatter.format(findings, &stats)?;
```

### Phase 2: Module-by-Module Migration

1. Create new module file
2. Copy code to new location
3. Update imports
4. Run tests
5. Delete old code
6. Commit

### Backward Compatibility

- Keep public API signatures unchanged during refactoring
- Use deprecation warnings for changed APIs
- Provide migration guides for breaking changes

## Success Metrics

### Quantitative
- **File Size**: Target < 1,000 lines per file
- **Function Size**: Target < 50 lines per function
- **Cyclomatic Complexity**: Target < 10 per function
- **Test Coverage**: Maintain or improve current coverage

### Qualitative
- **Developer Onboarding**: New developers can understand a module in < 30 minutes
- **Change Isolation**: Changes typically affect only 1-2 files
- **Bug Localization**: Bugs can be traced to specific module quickly

## Risk Mitigation

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking changes | High | Maintain backward compatibility, gradual migration |
| Performance regression | Medium | Benchmark before/after, optimize hot paths |
| Introduced bugs | High | Comprehensive test suite, gradual rollout |
| Scope creep | Medium | Strict adherence to refactoring plan |
| Developer disruption | Low | Coordinate with team, clear communication |

### Rollback Plan

1. Each refactoring phase is a separate branch
2. Keep original code commented during migration
3. Ability to revert to previous commit if issues arise
4. Feature flags for major changes

## Timeline

### Total Duration: 13 weeks

| Phase | Duration | Focus |
|-------|----------|-------|
| Phase 1 | Weeks 1-4 | analyze_enhanced.rs |
| Phase 2 | Weeks 5-8 | executor.rs |
| Phase 3 | Weeks 9-10 | engine.rs |
| Phase 4 | Weeks 11-13 | advanced_matcher.rs |
| Buffer | Week 14 | Testing, documentation, bug fixes |

### Weekly Commitment
- 2-3 days of focused refactoring work
- Daily standups to discuss progress and blockers
- Code reviews for each extracted module

## Conclusion

This refactoring plan will transform the astgrep codebase from a set of large, monolithic files into a well-organized, modular architecture. The benefits include:

- **Improved Maintainability**: Smaller, focused modules are easier to understand and modify
- **Better Testability**: Isolated modules can be tested independently
- **Enhanced Extensibility**: New features can be added without modifying existing code
- **Reduced Cognitive Load**: Developers can focus on specific domains
- **Faster Compilation**: Incremental builds will be faster with smaller modules

The plan is designed to be implemented incrementally, with each phase delivering value independently while building toward the overall goal of a cleaner architecture.
