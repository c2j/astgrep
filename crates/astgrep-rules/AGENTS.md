# astgrep-rules

YAML rule parsing, validation, and execution engine. Supports pattern matching, taint rules, and symbolic analysis.

## Structure

```
src/
├── lib.rs                    # RuleEngine — load/validate/execute rules; splits into condition vs no-condition paths
├── types.rs                  # Rule, RulePattern, MetavariableConstraint, RuleContext, RuleResult, Finding
├── parser/
│   ├── mod.rs                # RuleParser — YAML → Rule structs
│   └── parsing.rs            # Detailed YAML parsing logic
├── validator.rs              # RuleValidator — validates rule structure, patterns, references
├── engine.rs                 # RuleExecutionEngine — orchestrates rule execution
├── executor/
│   ├── mod.rs                # Executor module root
│   ├── types.rs              # Internal executor types
│   ├── dependency.rs         # Rule dependency resolution
│   ├── core_helpers.rs       # Shared execution helpers
│   ├── core/                 # Core execution strategies
│   │   ├── mod.rs            # AdvancedRuleExecutor
│   │   ├── symbolic.rs       # Symbolic rule execution
│   │   ├── taint.rs          # Taint-based rule execution
│   │   ├── conditions.rs     # Condition evaluation for rules
│   │   └── utils.rs          # Execution utilities
│   ├── traits/               # Executor trait abstractions
│   │   ├── mod.rs
│   │   ├── symbolic.rs       # SymbolicExecutor trait
│   │   ├── taint.rs          # TaintExecutor trait
│   │   └── conditions.rs     # ConditionEvaluator trait
│   └── impls/                # Concrete executor implementations
│       ├── mod.rs
│       ├── symbolic.rs       # SymbolicExecutor impl
│       └── taint.rs          # TaintExecutor impl
├── integration.rs            # Integration with DataFlowAnalyzer and PatternMatcher
└── marketplace.rs            # Rule marketplace/registry functionality
```

## Where to Look

| Task | File | Notes |
|------|------|-------|
| Add new rule YAML field | `parser/parsing.rs` + `types.rs` | Parse field + add to Rule struct |
| Modify rule validation | `validator.rs` | `RuleValidator::validate_rule()`, strict mode available |
| Change execution strategy | `executor/core/` | Conditions, taint, symbolic sub-executors |
| Add executor trait method | `executor/traits/` | Define in trait, impl in `impls/` |
| Integration with dataflow | `integration.rs` | Bridges to `astgrep-dataflow` |
| Rule dependency handling | `executor/dependency.rs` | Rule ordering and dependency resolution |

## Key Types

- `RuleEngine` — main API: `load_rules_from_yaml()`, `analyze()`, `execute_rule()`
- `Rule` — parsed rule with id, patterns, languages, severity, confidence, dataflow spec
- `RuleContext` — execution context (language, file_path, constant propagation toggle)
- `RuleResult` — execution result with findings
- `AdvancedRuleExecutor` — handles rules with conditions via `execute_comprehensive_analysis()`

## Execution Flow

1. `RuleEngine::analyze()` checks `enable_constant_propagation` → runs `ConstantPropagator` if enabled
2. Splits rules into: with conditions → `AdvancedRuleExecutor`, without conditions → `RuleExecutionEngine`
3. `AdvancedRuleExecutor::execute_comprehensive_analysis()` delegates to core/{symbolic, taint, conditions}
4. All findings merged into a single `Vec<Finding>` result

## YAML Rule Format

```yaml
rules:
  - id: unique-rule-id
    name: Rule Name
    severity: ERROR | WARNING | INFO | CRITICAL
    confidence: HIGH | MEDIUM | LOW
    languages: [java, python]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern: "$STR + $INPUT"
    dataflow:
      sources: [{pattern: "request.getParameter($P)"}]
      sinks: [{pattern: "Statement.execute($Q)"}]
      sanitizers: [{pattern: "escapeSql($X)"}]
```

## Anti-Patterns

- Do NOT skip validation — always call `validate_rule()` before adding to engine
- Do NOT bypass the condition/no-condition split in `analyze()` — it's intentional for performance
- Do NOT modify `Rule` struct without updating `RuleParser` and `RuleValidator` in sync
