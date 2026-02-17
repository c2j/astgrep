## Why

The Java compatibility test suite in `tests/scripts/compatibility/run_java_comparison_tests.sh` contains 38 test cases that must produce identical results to semgrep. Currently, some test cases may have mismatches, preventing reliable compatibility verification. This change is needed to ensure astgrep's Java analysis produces consistent, semgrep-equivalent results.

## What Changes

- Analyze all 38 Java compatibility test cases to identify mismatches with semgrep output
- For each mismatched test case, investigate root cause (parsing差异、规则匹配逻辑、输出格式等)
- Fix identified issues in the astgrep codebase
- Verify all 38 test cases pass with semgrep-equivalent results

## Capabilities

### New Capabilities
- `java-compatibility-verification`: Comprehensive capability to verify and fix Java compatibility between astgrep and semgrep

### Modified Capabilities
- None - this is a new verification capability

## Impact

- Core parser: Java parsing behavior alignment
- Rule engine: Matching logic consistency
- Test infrastructure: Compatibility test validation
