# astgrep Validation Suite Guide

## Overview

The astgrep Validation Suite provides comprehensive testing and validation of the project's functionality against real-world test cases and rules.

## Components

### 1. **quick_validation.py** - Quick Validation
Fast validation of core functionality with minimal overhead.

```bash
python3 tests/quick_validation.py
```

**Tests:**
- Simple pattern matching (function calls, strings, numbers)
- Advanced patterns (pattern-either, pattern-not, pattern-inside, metavariables)
- Language support (Python, JavaScript, Java, Ruby)

**Output:**
- Console summary with pass/fail status
- Quick feedback on core functionality

### 2. **comprehensive_test_runner.py** - Comprehensive Testing
Runs all discovered test cases and generates detailed results.

```bash
python3 tests/comprehensive_test_runner.py
```

**Features:**
- Auto-discovers test cases (YAML rules + code files)
- Runs astgrep against each test case
- Collects detailed results and metrics
- Generates JSON report

**Output:**
- `tests/test_report.json` - Raw test results
- Console progress and summary

### 3. **test_analyzer.py** - Result Analysis
Analyzes test results and generates detailed reports.

```bash
python3 tests/test_analyzer.py
```

**Analysis:**
- Overall pass rate and quality score
- Results breakdown by test suite
- Failure pattern analysis
- Performance metrics

**Output:**
- `tests/test_report.md` - Markdown report
- Console analysis summary

### 4. **generate_detailed_report.py** - Report Generation
Generates HTML and text reports from test results.

```bash
python3 tests/generate_detailed_report.py
```

**Output:**
- `tests/test_report.html` - Interactive HTML report
- `tests/test_report.txt` - Plain text report

### 5. **run_validation_suite.sh** - Complete Validation
Orchestrates all validation steps in sequence.

```bash
bash tests/run_validation_suite.sh
```

**Steps:**
1. Build the project
2. Run quick validation
3. Run comprehensive tests
4. Analyze results
5. Generate reports

**Output:**
- All reports in `tests/validation_reports/`
- Summary file with timestamp

## Quick Start

### Option 1: Quick Check (2-5 minutes)
```bash
python3 tests/quick_validation.py
```

### Option 2: Full Validation (10-30 minutes)
```bash
bash tests/run_validation_suite.sh
```

### Option 3: Step-by-Step
```bash
# Build
cargo build --release

# Quick validation
python3 tests/quick_validation.py

# Comprehensive tests
python3 tests/comprehensive_test_runner.py

# Analyze
python3 tests/test_analyzer.py

# Generate reports
python3 tests/generate_detailed_report.py
```

## Test Structure

### Test Discovery
The validation suite automatically discovers test cases by looking for:
- YAML rule files (*.yaml)
- Corresponding code files (*.py, *.js, *.java, etc.)

### Test Directories
- `tests/simple/` - Basic pattern matching tests
- `tests/advanced_patterns/` - Advanced pattern types
- `tests/comparison/` - Comparison tests
- `tests/e-rules/` - Enhanced rules tests
- `tests/rules/` - Comprehensive rule tests

### Test Format
Each test consists of:
1. **Rule File** (*.yaml) - Semgrep-compatible rule definition
2. **Code File** (*.py, *.js, etc.) - Source code to analyze

Example:
```
tests/simple/
├── function_call.yaml    # Rule definition
└── function_call.js      # Test code
```

## Reports

### JSON Report (test_report.json)
Raw test results in JSON format:
- Test counts and pass rates
- Per-suite statistics
- Individual test results
- Performance metrics

### Markdown Report (test_report.md)
Human-readable report with:
- Summary statistics
- Results by suite
- Failure analysis
- Performance metrics

### HTML Report (test_report.html)
Interactive visual report with:
- Stat cards with metrics
- Progress bars
- Results table
- Responsive design

### Text Report (test_report.txt)
Plain text summary for quick reference

## Interpreting Results

### Pass Rate
- **90-100%**: Excellent ✅
- **80-90%**: Good 👍
- **70-80%**: Fair ⚠️
- **<70%**: Needs improvement ❌

### Quality Score
Calculated as: `(100 * pass_rate) - (50 * fail_rate)`
- **80-100**: Excellent
- **60-80**: Good
- **40-60**: Fair
- **<40**: Poor

### Performance Metrics
- **Tests per second**: Higher is better
- **Average test time**: Lower is better
- **Total duration**: Reference for full suite

## Troubleshooting

### Tests Not Found
- Ensure test files exist in `tests/` directory
- Check file naming conventions (*.yaml + *.py/js/java/etc.)

### Build Failures
```bash
cargo clean
cargo build --release
```

### Permission Denied
```bash
chmod +x tests/run_validation_suite.sh
chmod +x tests/*.py
```

### Timeout Issues
- Increase timeout in test runner (default: 30s)
- Check system resources
- Run tests individually for debugging

## Advanced Usage

### Run Specific Test Suite
```bash
# Modify comprehensive_test_runner.py
# Change test_patterns list to specific directories
```

### Custom Test Cases
1. Create rule file: `tests/custom/my_rule.yaml`
2. Create code file: `tests/custom/my_code.py`
3. Run validation suite

### Debug Individual Test
```bash
cargo run --release -- analyze tests/simple/function_call.js -r tests/simple/function_call.yaml
```

## Performance Benchmarks

Expected performance (on modern hardware):
- Quick validation: 2-5 minutes
- Comprehensive tests: 10-30 minutes
- Full suite with reports: 15-40 minutes

## CI/CD Integration

### GitHub Actions Example
```yaml
- name: Run Validation Suite
  run: bash tests/run_validation_suite.sh

- name: Upload Reports
  uses: actions/upload-artifact@v2
  with:
    name: test-reports
    path: tests/validation_reports/
```

## Support

For issues or questions:
1. Check test logs in `tests/validation_reports/`
2. Review individual test output
3. Run with verbose mode (if available)
4. Check project documentation

---

**Last Updated**: 2025-10-17  
**Version**: 1.0

