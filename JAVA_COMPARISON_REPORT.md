# Java Rules Comparison Report

Generated on: 2026年 2月 6日 星期五 09时01分53秒 CST

## Test Summary

### Overview
- **Total Java test files**: 38
- **Semgrep version**: 1.146.0
- **CR-SemService version**: 0.1.0

### Test Categories

#### Taint Analysis Tests
- **Taint analysis tests**:       15 files
- **Metavariable tests**:        7 files
- **Symbolic propagation tests**:        6 files
- **Constant propagation tests**:        4 files

### Detailed Test Results

| Test File | Rule Type | Semgrep Matches | CR-SemService Matches | Status |
|-----------|-----------|-----------------|----------------------|--------|
| cp_private_class_attr |  | 1 | 2 | ❌ DIFFER |
| cp_private_class_attr1 |  | 00 | 00 | ✅ MATCH |
| cp_private_class_attr2 |  | 1 | 00 | ❌ DIFFER |
| cp_private_class_attr3 |  | 1 | 00 | ❌ DIFFER |
| metavar_comparison_bitand |  | 1 | 00 | ❌ DIFFER |
| metavar_comparison_bitnot |  | 1 | 00 | ❌ DIFFER |
| metavar_comparison_bitor |  | 1 | 00 | ❌ DIFFER |
| metavar_comparison_bitxor |  | 1 | 00 | ❌ DIFFER |
| metavar_name_imported_entity_java |  | 1 | 00 | ❌ DIFFER |
| metavar_type_not_java |  | 1 | 00 | ❌ DIFFER |
| metavar_type_str_eq_java |  | 1 | 00 | ❌ DIFFER |
| metavariable_name_resolution |  | 1 | 00 | ❌ DIFFER |
| misc_name_and_neg |  | 00 | 00 | ✅ MATCH |
| naming_class_attribute |  | 1 | 3 | ❌ DIFFER |
| non_irrelevant_rule |  | 1 | 00 | ❌ DIFFER |
| sym_prop_class_attr | taint | 1 | 00 | ❌ DIFFER |
| sym_prop_deep |  | 1 | 00 | ❌ DIFFER |
| sym_prop_merge1 |  | 1 | 00 | ❌ DIFFER |
| sym_prop_merge2 |  | 1 | 00 | ❌ DIFFER |
| sym_prop_new |  | 1 | 00 | ❌ DIFFER |
| sym_prop_non_literal |  | 1 | 00 | ❌ DIFFER |
| taint_assume_safe_booleans1 | taint | 1 | 00 | ❌ DIFFER |
| taint_assume_safe_numbers1 | taint | 1 | 00 | ❌ DIFFER |
| taint_assume_safe_numbers3 | taint | 1 | 00 | ❌ DIFFER |
| taint_best_fit_sink5 | taint | 1 | 00 | ❌ DIFFER |
| taint_best_fit_sink6 | taint | 1 | 00 | ❌ DIFFER |
| taint_best_fit_sink9 | taint | 00 | 00 | ✅ MATCH |
| taint_final_globals | taint | 1 | 00 | ❌ DIFFER |
| taint_final_globals2 | taint | 1 | 00 | ❌ DIFFER |
| taint_foreach | taint | 1 | 00 | ❌ DIFFER |
| taint_get_set_sensitivity | taint | 1 | 00 | ❌ DIFFER |
| taint_get_set_sensitivity1 | taint | 1 | 00 | ❌ DIFFER |
| taint_lambda1 | taint | 1 | 00 | ❌ DIFFER |
| taint_propagator_lambda | taint | 1 | 00 | ❌ DIFFER |
| taint_propagator4 | taint | 1 | 00 | ❌ DIFFER |
| taint_this1 | taint | 1 | 00 | ❌ DIFFER |
| tainted-file-path | taint | 1 | 1 | ✅ MATCH |
| typed_metavar_not |  | 00 | 00 | ✅ MATCH |

### Summary Statistics

- **Matching results**: 5 tests
- **Differing results**: 33 tests
- **Missing rules**: 0 tests
- **Compatibility rate**: 13%

### Test Categories Analysis

#### Taint Analysis
Taint analysis tests focus on data flow tracking from sources to sinks.
Key patterns tested:
- Source-to-sink data flow
- Sanitizer effectiveness
- Field sensitivity
- Lambda expressions
- Global variables

#### Metavariable Comparison
Tests for metavariable constraints and comparisons.
Key patterns tested:
- Bitwise operations (AND, OR, XOR, NOT)
- Numeric comparisons
- String equality
- Type constraints

#### Symbolic Propagation
Tests for symbolic value propagation through code.
Key patterns tested:
- Class attributes
- Method chaining
- Deep propagation
- Merge scenarios

#### Constant Propagation
Tests for constant value propagation.
Key patterns tested:
- Private class attributes
- Literal values
- Expression evaluation

### Implementation Notes

#### Current Limitations
1. **Java Parser Integration**: Need to integrate Java-specific parsing
2. **Taint Analysis**: Advanced taint tracking not fully implemented
3. **Symbolic Propagation**: Complex symbolic analysis pending
4. **Metavariable Constraints**: Some constraint types need implementation

#### Next Steps
1. Implement Java AST parsing integration
2. Add taint analysis engine for Java
3. Implement symbolic propagation
4. Add metavariable constraint evaluation
5. Optimize performance for large Java codebases

---

**Report Generated**: 2026年 2月 6日 星期五 09时02分59秒 CST
**Total Tests Analyzed**: 38
**Compatibility Status**: In Development
