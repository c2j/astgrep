## Context

The test suite comparison revealed 9 failing test cases when comparing our astgrep tool against Semgrep. The failures span multiple feature areas:

1. **Metavariable Type Checking** - Pattern `$WRITER.println(...)` with `metavariable-type: PrintWriter` not matching
2. **Metavariable Pattern** - `metavariable-pattern` constraint not implemented
3. **Class Attribute Matching** - `this.$X` with `pattern-inside` not properly scoped
4. **Symbolic Propagation** - Taint mode not using symbolic propagator for variable aliases
5. **Boolean Safety** - `taint_assume_safe_booleans` option not implemented

Current implementation gaps:
- Pattern matcher binds `$WRITER` to wrong value (`.` instead of `pWriter`)
- `evaluate_condition()` in executor.rs lacks `MetavariablePattern` handling
- Symbolic propagator exists but isn't enabled for taint mode
- Taint analyzer doesn't check for `taint_assume_safe_booleans` option

## Goals / Non-Goals

**Goals:**
- Achieve 100% compatibility for the 9 identified failing test cases
- Implement proper metavariable extraction from method call patterns
- Complete `metavariable-pattern` condition evaluation
- Enable symbolic propagation for taint analysis
- Support `taint_assume_safe_booleans` option

**Non-Goals:**
- No changes to rule YAML format (all changes are implementation-only)
- No breaking changes to existing APIs
- No changes to AST structure or parser behavior

## Decisions

### Decision 1: Fix Metavariable Binding in Pattern Matcher

**Approach:** Modify `advanced_matcher.rs` to correctly extract the receiver object from method calls.

When pattern is `$WRITER.println(...)` and code is `pWriter.println(request.input)`:
- Current: binds `$WRITER` to `.` or wrong value
- Fix: bind `$WRITER` to `pWriter` before the dot

**Rationale:** The pattern matcher tokenizes code and matches patterns, but the metavariable binding logic doesn't correctly identify the object name when followed by a method call chain.

### Decision 2: Implement Metavariable-Pattern Evaluation

**Approach:** Add `Condition::MetavariablePattern` handling in `executor.rs:evaluate_condition()`.

The implementation will:
1. Get the metavariable value from bindings
2. Create a sub-matcher with the constraint pattern
3. Check if the metavariable value matches the pattern
4. Support `pattern`, `patterns`, and `pattern-either` variants

**Rationale:** This is a missing feature. The parser already parses `metavariable-pattern` but the executor doesn't evaluate it.

### Decision 3: Enable Symbolic Propagation in Taint Mode

**Approach:** Modify `execute_taint_analysis()` in executor.rs to:
1. Check if any taint rule has `symbolic_propagation: true`
2. If so, run symbolic propagation analysis on the AST
3. Use propagated variable aliases when checking if a sink receives tainted data

**Rationale:** Symbolic propagation is already implemented in `astgrep-dataflow/src/symbolic_propagation.rs` but isn't integrated with taint analysis. This is a wiring issue, not a new feature.

### Decision 4: Add Boolean Safety Check to Taint Analyzer

**Approach:** When `taint_assume_safe_booleans` is true:
1. In taint tracking, treat boolean expressions as sanitized
2. Recognize patterns: `Boolean.valueOf(x)`, `Boolean.parseBoolean(x)`, boolean comparisons
3. Skip taint violation reporting for these cases

**Rationale:** This is a Semgrep-compatible security feature. Boolean operations on tainted data are generally safe as they don't propagate the tainted value.

## Risks / Trade-offs

**Risk:** Pattern matcher changes could affect existing matches
→ **Mitigation:** Run full test suite after changes, add unit tests for edge cases

**Risk:** Symbolic propagation performance impact
→ **Mitigation:** Only enable when rule explicitly sets `symbolic_propagation: true`

**Risk:** Metavariable-pattern nested matching could cause infinite recursion
→ **Mitigation:** Track recursion depth, limit to reasonable maximum (e.g., 10)

**Risk:** Boolean safety might have false negatives
→ **Mitigation:** Follow Semgrep's exact semantics, document the behavior

## Migration Plan

1. **Phase 1:** Fix metavariable binding in pattern matcher
2. **Phase 2:** Implement metavariable-pattern evaluation
3. **Phase 3:** Enable symbolic propagation for taint
4. **Phase 4:** Add taint_assume_safe_booleans support
5. **Phase 5:** Run full test suite verification

**Rollback:** All changes are additive; no migration needed. Can revert individual commits if issues arise.

## Open Questions

1. Should metavariable-pattern support regex constraints as well? (Semgrep does, our parser might not)
2. What's the exact behavior of Semgrep's symbolic propagation for complex chains (A = B = C = source)?
3. Are there any other rule options we should check for in taint mode (like `taint_assume_safe_numbers`)?
