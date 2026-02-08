## Why

Our static analysis tool has compatibility gaps with Semgrep when running against Java test cases. The analysis reveals 9 failing test cases across 5 different feature areas, preventing us from achieving parity with Semgrep's behavior. Fixing these issues is critical for users migrating from Semgrep and ensures consistent security analysis results.

## What Changes

This change addresses multiple compatibility issues identified in test suite comparison:

1. **Metavariable Type Checking** (`metavariable-type`)
   - Fix pattern matcher to correctly extract and bind metavariables from method call patterns like `$WRITER.println(...)`
   - Ensure type constraints are properly evaluated during match validation

2. **Metavariable Pattern Constraints** (`metavariable-pattern`)
   - Implement missing condition evaluation for `metavariable-pattern` in `evaluate_condition()`
   - Support `pattern-either` syntax within metavariable-pattern constraints

3. **Class Attribute Matching** (`pattern-inside` with `this.$X`)
   - Fix pattern-inside context handling for class field references
   - Ensure `this.$X` properly matches class attributes bound in outer context

4. **Symbolic Propagation for Taint Analysis** (`symbolic_propagation`)
   - Enable symbolic propagation in Taint mode analysis
   - Support variable alias tracking from assignment statements through to sink detection
   - Handle static field initialization and method call chains

5. **Taint Boolean Safety** (`taint_assume_safe_booleans`)
   - Implement the `taint_assume_safe_booleans` rule option
   - Treat boolean expressions and Boolean wrapper objects as sanitized in taint analysis

## Capabilities

### New Capabilities
- `metavariable-pattern-matching`: Support for constraining metavariable values using nested patterns
- `symbolic-propagation-taint`: Variable alias and value propagation in taint analysis mode
- `taint-boolean-safety`: Boolean sanitization in taint tracking

### Modified Capabilities
- `metavariable-type-checking`: Enhanced type extraction and validation for metavariable bindings
- `pattern-inside-context`: Improved context scoping for class-level attribute references

## Impact

**Affected Code:**
- `crates/astgrep-matcher/src/advanced_matcher.rs` - Pattern matching and metavariable binding
- `crates/astgrep-rules/src/executor.rs` - Condition evaluation and rule execution
- `crates/astgrep-dataflow/src/taint.rs` - Taint analysis engine
- `crates/astgrep-rules/src/engine.rs` - Rule engine integration

**Testing:**
- 9 Java test cases in `tests/categories/rules/` will be verified
- All fixes must pass both individual tests and full test suite

**Dependencies:**
- Requires understanding of Semgrep's pattern matching semantics
- May need updates to AST traversal utilities for proper context handling

**Compatibility:**
- All changes are additive/improvements to existing functionality
- No breaking changes to existing rule formats or APIs
