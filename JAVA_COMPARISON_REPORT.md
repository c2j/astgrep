# Java Rules Comparison Report

Generated on: 2026年 2月 8日 星期日 19时18分15秒 CST

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
| cp_private_class_attr |  | 2 | 2 | ✅ MATCH |
| cp_private_class_attr1 |  | 00 | 00 | ✅ MATCH |
| cp_private_class_attr2 |  | 1 | 1 | ✅ MATCH |
| cp_private_class_attr3 |  | 1 | 1 | ✅ MATCH |
| metavar_comparison_bitand |  | 1 | 1 | ✅ MATCH |
| metavar_comparison_bitnot |  | 1 | 1 | ✅ MATCH |
| metavar_comparison_bitor |  | 1 | 1 | ✅ MATCH |
| metavar_comparison_bitxor |  | 1 | 1 | ✅ MATCH |
| metavar_name_imported_entity_java |  | 2 | 2 | ✅ MATCH |
| metavar_type_not_java |  | 1 | 1 | ✅ MATCH |
| metavar_type_str_eq_java |  | 2 | 2 | ✅ MATCH |
| metavariable_name_resolution |  | 1 | 1 | ✅ MATCH |
| misc_name_and_neg |  | 00 | 00 | ✅ MATCH |
| naming_class_attribute |  | 2 | 2 | ✅ MATCH |
| non_irrelevant_rule |  | 1 | 1 | ✅ MATCH |
| sym_prop_class_attr | taint | 2 | 2 | ✅ MATCH |
| sym_prop_deep |  | 1 | 1 | ✅ MATCH |
| sym_prop_merge1 |  | 2 | 2 | ✅ MATCH |
| sym_prop_merge2 |  | 1 | 1 | ✅ MATCH |
| sym_prop_new |  | 1 | 1 | ✅ MATCH |
| sym_prop_non_literal |  | 1 | 1 | ✅ MATCH |
| taint_assume_safe_booleans1 | taint | 1 | 1 | ✅ MATCH |
| taint_assume_safe_numbers1 | taint | 1 | 1 | ✅ MATCH |
