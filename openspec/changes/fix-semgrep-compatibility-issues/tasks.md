## 1. Fix Metavariable Binding in Pattern Matcher

- [ ] 1.1 Analyze `advanced_matcher.rs` to understand how `$WRITER.println(...)` pattern is tokenized
- [ ] 1.2 Identify why `$WRITER` is bound to `.` instead of `pWriter`
- [ ] 1.3 Fix the binding extraction logic to correctly identify object name before method call
- [ ] 1.4 Add unit test for metavariable binding in method call patterns
- [ ] 1.5 Run `metavar_type_not_java` test and verify it passes

## 2. Implement Metavariable-Pattern Evaluation

- [ ] 2.1 Add `Condition::MetavariablePattern` variant handling in `executor.rs:evaluate_condition()`
- [ ] 2.2 Implement pattern matching logic for single `pattern` constraint
- [ ] 2.3 Implement pattern matching for `patterns` array (AND logic)
- [ ] 2.4 Implement pattern matching for `pattern-either` (OR logic)
- [ ] 2.5 Add recursion depth protection to prevent infinite loops
- [ ] 2.6 Run `metavariable_name_resolution` test and verify it passes

## 3. Fix Class Attribute Matching with pattern-inside

- [ ] 3.1 Analyze how `pattern-inside` context is passed to nested patterns
- [ ] 3.2 Fix `this.$X` matching to use class field binding from outer context
- [ ] 3.3 Ensure `$X` in `foo(this.$X)` correctly references class attribute `private int x`
- [ ] 3.4 Run `naming_class_attribute` test and verify it passes

## 4. Enable Symbolic Propagation for Taint Analysis

- [ ] 4.1 Modify `execute_taint_analysis()` to check for `symbolic_propagation` option
- [ ] 4.2 Enable symbolic propagator when option is true
- [ ] 4.3 Use symbolic aliases when checking taint flows to sinks
- [ ] 4.4 Test with `sym_prop_class_attr.java` (static field case)
- [ ] 4.5 Test with `sym_prop_merge1.java` (simple assignment chain)
- [ ] 4.6 Test with `sym_prop_merge2.java` (conditional assignment)
- [ ] 4.7 Test with `sym_prop_new.java` (object creation chain)
- [ ] 4.8 Test with `sym_prop_non_literal.java` (method chain propagation)

## 5. Implement Taint Boolean Safety Option

- [ ] 5.1 Add check for `taint_assume_safe_booleans` in taint analyzer
- [ ] 5.2 Detect boolean comparison expressions and treat as sanitized
- [ ] 5.3 Detect `Boolean.valueOf()` calls and treat as sanitized
- [ ] 5.4 Detect `Boolean.parseBoolean()` calls and treat as sanitized
- [ ] 5.5 Run `taint_assume_safe_booleans1` test and verify it passes

## 6. Final Verification

- [ ] 6.1 Run all 9 failing test cases and verify they pass
- [ ] 6.2 Run full test suite to ensure no regressions
- [ ] 6.3 Compare results with Semgrep to confirm parity
- [ ] 6.4 Update documentation if needed
- [ ] 6.5 Archive the change

## 7. Optional: Additional Improvements

- [ ] 7.1 Add debug logging for metavariable binding to help future debugging
- [ ] 7.2 Add performance benchmarks for symbolic propagation
- [ ] 7.3 Consider caching resolved types for repeated metavariable-type checks
