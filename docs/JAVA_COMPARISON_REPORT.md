# Java Rules Comparison Report

Generated on: Thu Aug  7 08:31:14 CST 2025

## Test Summary

### Overview
- **Total Java test files**: 124
- **Semgrep version**: 1.131.0
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
| [0;34mℹ️ | N/A | N/A | N/A | ⚠️ NO RULE |
| Discovering | N/A | N/A | N/A | ⚠️ NO RULE |
| Java | N/A | N/A | N/A | ⚠️ NO RULE |
| test | N/A | N/A | N/A | ⚠️ NO RULE |
| files...[0m | N/A | N/A | N/A | ⚠️ NO RULE |
| Found | N/A | N/A | N/A | ⚠️ NO RULE |
| 38 | N/A | N/A | N/A | ⚠️ NO RULE |
| Java | N/A | N/A | N/A | ⚠️ NO RULE |
| test | N/A | N/A | N/A | ⚠️ NO RULE |
| files: | N/A | N/A | N/A | ⚠️ NO RULE |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| cp_private_class_attr |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| cp_private_class_attr1 |  | 00 | 00 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| cp_private_class_attr2 |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| cp_private_class_attr3 |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_comparison_bitand |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_comparison_bitnot |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_comparison_bitor |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_comparison_bitxor |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_name_imported_entity_java |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_type_not_java |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavar_type_str_eq_java |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| metavariable_name_resolution |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| misc_name_and_neg |  | 00 | 00 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| naming_class_attribute |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| non_irrelevant_rule |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_class_attr | taint | 1 | 2 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_deep |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_merge1 |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_merge2 |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_new |  | 1 | 1 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| sym_prop_non_literal |  | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_assume_safe_booleans1 | taint | 1 | 4 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_assume_safe_numbers1 | taint | 1 | 14 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_assume_safe_numbers3 | taint | 1 | 6 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_best_fit_sink5 | taint | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_best_fit_sink6 | taint | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_best_fit_sink9 | taint | 00 | 00 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_final_globals | taint | 1 | 2 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_final_globals2 | taint | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_foreach | taint | 1 | 1 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_get_set_sensitivity | taint | 1 | 1 | ✅ MATCH |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_get_set_sensitivity1 | taint | 1 | 2 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_lambda1 | taint | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_propagator4 | taint | 1 | 00 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_propagator_lambda | taint | 1 | 8 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| taint_this1 | taint | 1 | 3 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| tainted-file-path | taint | 1 | 2 | ❌ DIFFER |
| - | N/A | N/A | N/A | ⚠️ NO RULE |
| typed_metavar_not |  | 00 | 00 | ✅ MATCH |
| cp_private_class_attr |  | 1 | 00 | ❌ DIFFER |
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
| naming_class_attribute |  | 1 | 00 | ❌ DIFFER |
| non_irrelevant_rule |  | 1 | 00 | ❌ DIFFER |
| sym_prop_class_attr | taint | 1 | 2 | ❌ DIFFER |
| sym_prop_deep |  | 1 | 00 | ❌ DIFFER |
| sym_prop_merge1 |  | 1 | 00 | ❌ DIFFER |
| sym_prop_merge2 |  | 1 | 00 | ❌ DIFFER |
| sym_prop_new |  | 1 | 1 | ✅ MATCH |
| sym_prop_non_literal |  | 1 | 00 | ❌ DIFFER |
| taint_assume_safe_booleans1 | taint | 1 | 4 | ❌ DIFFER |
| taint_assume_safe_numbers1 | taint | 1 | 14 | ❌ DIFFER |
| taint_assume_safe_numbers3 | taint | 1 | 6 | ❌ DIFFER |
| taint_best_fit_sink5 | taint | 1 | 00 | ❌ DIFFER |
| taint_best_fit_sink6 | taint | 1 | 00 | ❌ DIFFER |
| taint_best_fit_sink9 | taint | 00 | 00 | ✅ MATCH |
| taint_final_globals | taint | 1 | 2 | ❌ DIFFER |
| taint_final_globals2 | taint | 1 | 00 | ❌ DIFFER |
| taint_foreach | taint | 1 | 1 | ✅ MATCH |
| taint_get_set_sensitivity | taint | 1 | 1 | ✅ MATCH |
| taint_get_set_sensitivity1 | taint | 1 | 2 | ❌ DIFFER |
| taint_lambda1 | taint | 1 | 00 | ❌ DIFFER |
| taint_propagator4 | taint | 1 | 00 | ❌ DIFFER |
| taint_propagator_lambda | taint | 1 | 8 | ❌ DIFFER |
| taint_this1 | taint | 1 | 3 | ❌ DIFFER |
| tainted-file-path | taint | 1 | 2 | ❌ DIFFER |
| typed_metavar_not |  | 00 | 00 | ✅ MATCH |

### Summary Statistics

- **Matching results**: 14 tests
- **Differing results**: 62 tests  
- **Missing rules**: 48 tests
- **Compatibility rate**: 18%

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

**Report Generated**: Thu Aug  7 08:35:38 CST 2025  
**Total Tests Analyzed**: 124  
**Compatibility Status**: In Development
