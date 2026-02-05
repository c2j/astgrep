# Advanced Pattern Tests

This directory contains comprehensive tests for advanced Semgrep pattern matching features.

## Test Categories

### 1. Pattern-Either (OR Logic)
- **File**: `pattern_either_test.yaml` / `pattern_either_test.py`
- **Purpose**: Test multiple alternative patterns using OR logic
- **Features**: 
  - Multiple pattern alternatives
  - Complex OR combinations
  - Nested either patterns

### 2. Pattern-Not (Exclusion Logic)
- **File**: `pattern_not_test.yaml` / `pattern_not_test.py`
- **Purpose**: Test pattern exclusion using NOT logic
- **Features**:
  - Simple negation
  - Complex exclusion patterns
  - Combined with other patterns

### 3. Pattern-Inside (Context Matching)
- **File**: `pattern_inside_test.yaml` / `pattern_inside_test.py`
- **Purpose**: Test patterns that must occur within specific contexts
- **Features**:
  - Function context matching
  - Class context matching
  - Nested context patterns

### 4. Pattern-Regex (Regular Expression)
- **File**: `pattern_regex_test.yaml` / `pattern_regex_test.py`
- **Purpose**: Test regular expression pattern matching
- **Features**:
  - String pattern matching
  - Complex regex patterns
  - Case sensitivity options

### 5. Metavariables (Variable Binding)
- **File**: `metavariables_test.yaml` / `metavariables_test.py`
- **Purpose**: Test metavariable binding and constraints
- **Features**:
  - Variable binding
  - Regex constraints
  - Comparison constraints
  - Type constraints

## Running Tests

To run all advanced pattern tests:

```bash
# Run the enhanced test comparison
cargo run --example test_comparison

# Run specific advanced pattern tests
./run_advanced_pattern_tests.sh
```

## Expected Results

Each test should demonstrate:
1. Correct pattern matching behavior
2. Proper metavariable binding
3. Accurate constraint evaluation
4. Performance comparable to Semgrep

## Test Structure

Each test consists of:
- YAML rule file with pattern definition
- Source code file with test cases
- Expected match count and locations
- Validation of metavariable bindings
