## Context

The Java compatibility test suite (`tests/scripts/compatibility/run_java_comparison_tests.sh`) compares astgrep results against semgrep for 38 Java test cases. Each test case consists of:
- A Java source file in `tests/categories/rules/`
- A corresponding YAML rule file with the same base name

The test script runs both semgrep and astgrep on each Java file, counting matches and comparing results. Currently, some test cases may produce different match counts between the two tools.

## Goals / Non-Goals

**Goals:**
- Run all 38 Java compatibility test cases and identify mismatches
- Analyze root cause for each mismatch (parsing, rule matching, output format)
- Fix identified issues in astgrep codebase
- Verify 100% compatibility (all 38 test cases match semgrep)

**Non-Goals:**
- Not adding new Java language features
- Not improving performance beyond what's needed for compatibility
- Not modifying the test script itself (unless required for correct comparison)

## Decisions

### 1. Test Execution Strategy
- Run the test script as-is to get baseline results
- For each mismatch, run both tools manually to understand the difference
- Analyze the rule pattern, Java code, and matching logic

### 2. Root Cause Categories
- **Parsing differences**: Java AST structure differences between tree-sitter and semgrep
- **Pattern matching**: Differences in how patterns are interpreted (e.g., metavariables, ellipses)
- **Taint analysis**: Advanced taint tracking features not yet implemented
- **Symbolic propagation**: Complex symbolic value propagation
- **Output format**: Differences in how results are counted/reported

### 3. Fix Priority
- Simple pattern matching issues first
- Metavariable comparison issues
- Taint analysis issues (may require significant work)
- Edge cases and complex scenarios

## Risks / Trade-offs

- **[Risk]** Taint analysis may require significant engine work
  - → Mitigation: Document limitations, may need to skip some taint tests if too complex

- **[Risk]** Some semgrep patterns may use advanced features not in astgrep
  - → Mitigation: Analyze feature gaps, potentially add missing features

- **[Risk]** Test environment differences (semgrep version, configuration)
  - → Mitigation: Use consistent semgrep version, verify with `--version`

## Open Questions

- Which specific test cases currently fail?
- Are there any semgrep-specific features used in the rules that astgrep doesn't support?
- Should we modify the test script for better debugging output?
