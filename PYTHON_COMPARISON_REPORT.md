# Python Rules Comparison Report

Generated on: 2026年 4月 4日 星期六 06时53分30秒 CST

## Test Summary

### Overview
- **Total Python test files**: 128
- **Semgrep version**: 1.146.0
- **astgrep version**: 0.1.0

### Test Categories

#### Taint Analysis Tests
- **Taint analysis tests**:       48 files
- **Metavariable tests**:       15 files
- **Symbolic propagation tests**:       13 files
- **Constant propagation tests**:        3 files

### Detailed Test Results

| Test File | Rule Type | Semgrep Matches | astgrep Matches | Status |
|-----------|-----------|-----------------|-----------------|--------|
| anonymous_metavar |  | 0 | 0 | ✅ MATCH |
| anywhere_global |  | 00 | 0 | ✅ MATCH |
| anywhere_metavar |  | 00 | 0 | ✅ MATCH |
| as_metavariable |  | 1 | 0 | ❌ DIFFER |
| as_metavariable2 |  | 1 | 0 | ❌ DIFFER |
| capture_group_unification |  | 2 | 2 | ✅ MATCH |
| cp_mults |  | 5 | 0 | ❌ DIFFER |
| cp_python_and_or |  | 1 | 0 | ❌ DIFFER |
| cp_python_strings |  | 0 | 0 | ✅ MATCH |
| date_comparison |  | 1 | 0 | ❌ DIFFER |
| decorated_match |  | 1 | 0 | ❌ DIFFER |
| defer-persistent-binding |  | 1 | 0 | ❌ DIFFER |
| different_binding_locations |  | 2 | 0 | ❌ DIFFER |
| ellipsis_metavar_extended_match |  | 1 | 0 | ❌ DIFFER |
| ellipsis_stmts_deep |  | 0 | 0 | ✅ MATCH |
| entropy_python |  | 1 | 1 | ✅ MATCH |
| eval_not_in |  | 1 | 1 | ✅ MATCH |
| focus_metavariable |  | 1 | 1 | ✅ MATCH |
| focus_metavariable1 |  | 1 | 2 | ❌ DIFFER |
| focus_metavariable2 |  | 2 | 0 | ❌ DIFFER |
| labeled_propagators | taint | 3 | 0 | ❌ DIFFER |
| metavar_comparison_constness1 |  | 1 | 0 | ❌ DIFFER |
| metavar_pattern_dots_mvar |  | 2 | 0 | ❌ DIFFER |
| metavar_pattern_lang |  | 1 | 0 | ❌ DIFFER |
| metavar_pattern_lang1 |  | 1 | 0 | ❌ DIFFER |
| metavar_pattern_nested |  | 1 | 0 | ❌ DIFFER |
| metavar_pattern_nested1 |  | 2 | 0 | ❌ DIFFER |
| metavar_pattern_not |  | 1 | 0 | ❌ DIFFER |
| metavar_pattern_open_redirect |  | 4 | 0 | ❌ DIFFER |
| metavar_regex_capture |  | 2 | 0 | ❌ DIFFER |
| metavar_regex_scope |  | 2 | 2 | ✅ MATCH |
