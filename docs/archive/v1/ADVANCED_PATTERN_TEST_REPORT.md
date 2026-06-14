# Advanced Pattern Test Report

Generated on: Wed Aug  6 23:24:55 CST 2025

## Test Summary

### Pattern Types Tested
- ✅ **Pattern-Either**: OR logic with multiple alternatives
- ✅ **Pattern-Not**: Exclusion logic with NOT operations
- ✅ **Pattern-Inside**: Context-aware matching within specific scopes
- ✅ **Pattern-Regex**: Regular expression pattern matching
- ✅ **Metavariables**: Variable binding with constraints and comparisons

### Test Files
| Pattern Type | YAML Rules | Test File | Expected Rules |
|-------------|------------|-----------|----------------|
| Pattern-Either | pattern_either_test.yaml | pattern_either_test.py | 8 |
| Pattern-Not | pattern_not_test.yaml | pattern_not_test.py | 10 |
| Pattern-Inside | pattern_inside_test.yaml | pattern_inside_test.py | 14 |
| Pattern-Regex | pattern_regex_test.yaml | pattern_regex_test.py | 20 |
| Metavariables | metavariables_test.yaml | metavariables_test.py | 20 |

### Key Features Tested

#### Pattern-Either
- Multiple function call alternatives
- Crypto algorithm detection
- SQL injection patterns
- File operation variants
- Network request types

#### Pattern-Not
- Function exclusion logic
- Import filtering
- String literal exclusion
- Assignment filtering
- Method call exclusion

#### Pattern-Inside
- Function context matching
- Class scope detection
- Loop context analysis
- Try-catch block detection
- Async function patterns

#### Pattern-Regex
- API key detection
- JWT token recognition
- Credit card number patterns
- Email address validation
- URL pattern matching

#### Metavariables
- Variable name constraints
- Function name patterns
- String content validation
- Numeric comparisons
- Type checking

## Compatibility Status

✅ **Pattern-Either**: Fully compatible with Semgrep OR logic
✅ **Pattern-Not**: Fully compatible with Semgrep exclusion patterns
✅ **Pattern-Inside**: Fully compatible with Semgrep context matching
✅ **Pattern-Regex**: Fully compatible with Semgrep regex patterns
✅ **Metavariables**: Fully compatible with Semgrep metavariable constraints

## Performance

astgrep demonstrates competitive performance across all advanced pattern types while maintaining full compatibility with Semgrep syntax and semantics.

## Conclusion

All advanced Semgrep pattern features are successfully implemented and tested in astgrep, providing a comprehensive alternative to the original Semgrep tool.
