# Test Guide

This guide explains how to run tests and validate the astgrep project.

## Quick Start

Run the validation suite:
```bash
./scripts/validation/validate.sh quick    # Quick validation (2-5 min)
./scripts/validation/validate.sh full     # Full validation suite (10-30 min)
```

Run compatibility tests:
```bash
./scripts/compatibility/run_compatibility_tests.sh
```

## Directory Structure

```
tests/
├── TEST_GUIDE.md              # This file
├── scripts/                   # Test runner scripts
│   ├── validation/           # Validation suite scripts
│   ├── compatibility/        # Compatibility test scripts
│   ├── performance/          # Performance test scripts
│   └── utils/                # Utility and debug scripts
├── cases/                     # Test case files organized by language
│   ├── java/
│   ├── js/
│   └── sql/
├── config/                    # Test configuration files (.yaml)
├── reports/                   # Generated test reports
└── lib/                       # Rust test modules (.rs files)
```

## Validation Scripts

### validate.sh
Main validation entry point.

**Usage:**
```bash
./scripts/validation/validate.sh [command]
```

**Commands:**
- `quick` - Run quick validation (2-5 minutes)
- `full` - Run full validation suite (10-30 minutes)
- `analyze` - Analyze existing test results
- `report` - Generate detailed reports
- `clean` - Clean validation reports

**Examples:**
```bash
./scripts/validation/validate.sh quick
./scripts/validation/validate.sh full
./scripts/validation/validate.sh analyze
./scripts/validation/validate.sh report
```

### run_validation_suite.sh
Runs the comprehensive validation suite.

**Usage:**
```bash
./scripts/validation/run_validation_suite.sh
```

## Compatibility Scripts

### run_compatibility_tests.sh
Runs compatibility tests against Semgrep.

**Usage:**
```bash
./scripts/compatibility/run_compatibility_tests.sh
```

**Requirements:**
- Semgrep must be installed (`pip install semgrep`)

### demo_java_comparison.sh
Demonstrates Java comparison tests.

**Usage:**
```bash
./scripts/compatibility/demo_java_comparison.sh
```

## Performance Scripts

### run_advanced_pattern_tests.sh
Runs advanced pattern matching tests.

**Usage:**
```bash
./scripts/performance/run_advanced_pattern_tests.sh
```

## Utility Scripts

### analyze_tests.py
Analyzes test results and generates statistics.

**Usage:**
```bash
python3 scripts/utils/analyze_tests.py
```

### generate_detailed_report.py
Generates detailed HTML/text reports from test results.

**Usage:**
```bash
python3 scripts/utils/generate_detailed_report.py
```

### quick_validation.py
Runs a quick validation check.

**Usage:**
```bash
python3 scripts/utils/quick_validation.py
```

### Debug Scripts

- **debug_metavar.py** - Debug metavariable matching
- **debug_precision.py** - Debug precision/recall metrics
- **debug_tree_sitter.py** - Debug tree-sitter integration

## Test Cases

Test cases are organized by language in the `cases/` directory:

```
cases/
├── java/          # Java test cases
│   ├── parsing/
│   ├── patterns/
│   ├── rules/
│   ├── taint/
│   ├── autofix/
│   └── security/
├── js/            # JavaScript test cases
│   ├── parsing/
│   ├── patterns/
│   ├── rules/
│   ├── taint/
│   ├── autofix/
│   └── security/
└── sql/           # SQL test cases
    ├── parsing/
    ├── patterns/
    ├── rules/
    ├── taint/
    ├── autofix/
    └── security/
```

## Configuration Files

Test configurations are stored in `config/`:

- `test_rule.yaml` - Basic test rule configuration
- `test_taint_rule.yaml` - Taint analysis rule configuration
- `debug_*.yaml` - Debug configurations

## Common Commands

### Run all tests
```bash
cargo test
```

### Run specific test
```bash
cargo test test_name
```

### Run validation
```bash
./scripts/validation/validate.sh full
```

### Check compatibility
```bash
./scripts/compatibility/run_compatibility_tests.sh
```

### Generate reports
```bash
./scripts/validation/validate.sh report
```

### Clean test artifacts
```bash
./scripts/validation/validate.sh clean
```

## Rust Tests

Rust test files are located in `lib/` and can be run with:

```bash
cargo test --test <test_name>
```

Available test files:
- `integration_tests.rs`
- `performance_tests.rs`
- `semgrep_compatibility_tests.rs`
- `advanced_taint_tests.rs`
- And more...

## Troubleshooting

### Test failures
1. Check that all dependencies are installed
2. Run `./scripts/validation/validate.sh clean` to clear old reports
3. Try running tests individually to isolate issues

### Path issues
If scripts report path errors, ensure you're running them from the `tests/` directory or use the full path.

### Semgrep compatibility
Make sure Semgrep is installed and in your PATH:
```bash
pip install semgrep
semgrep --version
```

## Migration from Old Structure

If you have scripts referencing old paths, update them to use the new structure:

- Old: `./validate.sh` → New: `./scripts/validation/validate.sh`
- Old: `./analyze_tests.py` → New: `./scripts/utils/analyze_tests.py`
- Old: `tests/*.yaml` → New: `tests/config/*.yaml`

See `cases/MIGRATION.md` for test case reorganization details.
