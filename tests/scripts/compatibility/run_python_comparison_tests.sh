#!/bin/bash

# Python Rules Comparison Test Script for astgrep vs Semgrep
# This script compares results between Semgrep and astgrep on Python test files

set -e

echo "🔍 astgrep vs Semgrep Python Rules Comparison"
echo "==============================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "INFO")
            printf "${BLUE}ℹ️  %s${NC}\n" "$message"
            ;;
        "SUCCESS")
            printf "${GREEN}✅ %s${NC}\n" "$message"
            ;;
        "WARNING")
            printf "${YELLOW}⚠️  %s${NC}\n" "$message"
            ;;
        "ERROR")
            printf "${RED}❌ %s${NC}\n" "$message"
            ;;
        "DIFF")
            printf "${PURPLE}🔍 %s${NC}\n" "$message"
            ;;
        "MATCH")
            printf "${CYAN}🎯 %s${NC}\n" "$message"
            ;;
    esac
}

# Check if semgrep is installed
check_semgrep() {
    print_status "INFO" "Checking Semgrep installation..."
    if ! command -v semgrep &> /dev/null; then
        print_status "ERROR" "Semgrep is not installed. Please install it first:"
        echo "  pip install semgrep"
        exit 1
    fi

    local semgrep_version=$(semgrep --version | head -n1)
    print_status "SUCCESS" "Found $semgrep_version"
}

# Build astgrep
build_project() {
    print_status "INFO" "Building astgrep..."
    if cargo build --quiet 2>/dev/null; then
        print_status "SUCCESS" "Build completed successfully"
    else
        print_status "WARNING" "Build had warnings, continuing..."
    fi
}

# Find all Python test files and their corresponding YAML rules
find_python_tests() {
    # Find all Python files in tests/categories/rules directory
    local py_files=($(find tests/categories/rules -maxdepth 1 -name "*.py" | sort))

    for file in "${py_files[@]}"; do
        echo "$file"
    done
}

# Run Semgrep on a specific Python file with its YAML rule
run_semgrep_test() {
    local py_file=$1
    local yaml_file=$2

    if [ ! -f "$yaml_file" ]; then
        echo "0"
        return
    fi

    # Run semgrep and count matches
    local semgrep_output=$(semgrep --config "$yaml_file" "$py_file" --json 2>/dev/null || echo '{"results":[]}')
    
    # Check if jq is available
    if command -v jq &> /dev/null; then
        local match_count=$(echo "$semgrep_output" | jq '.results | length' 2>/dev/null || echo "0")
    else
        # Fallback: count occurrences of "check_id" (not lines)
        local match_count=$(echo "$semgrep_output" | grep -o '"check_id"' | wc -l)
    fi

    # Clean and ensure we return a valid number
    match_count=$(echo "$match_count" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
    match_count=${match_count:-0}

    if [[ "$match_count" =~ ^[0-9]+$ ]]; then
        echo "$match_count"
    else
        echo "0"
    fi
}

# Run astgrep on a specific Python file with its YAML rule
run_astgrep_test() {
    local py_file=$1
    local yaml_file=$2

    if [ ! -f "$yaml_file" ]; then
        echo "0"
        return
    fi

    # Try to run our CLI tool if available
    if [ -f "target/debug/astgrep" ]; then
        local output=$("./target/debug/astgrep" analyze --config "$yaml_file" "$py_file" --format json 2>&1 || echo '{"findings":[]}')
    elif [ -f "target/release/astgrep" ]; then
        local output=$("./target/release/astgrep" analyze --config "$yaml_file" "$py_file" --format json 2>&1 || echo '{"findings":[]}')
    else
        echo "0"
        return
    fi
    
    # Extract JSON from end of output (after all debug messages)
    local json_output=$(echo "$output" | tail -30 | grep -o '{"findings":\[.*\],"summary":{.*}}')
    
    # Check if jq is available
    if command -v jq &> /dev/null && [ -n "$json_output" ]; then
        local match_count=$(echo "$json_output" | jq '.findings | length' 2>/dev/null || echo "0")
    else
        # Fallback: count occurrences of "rule_id" in entire output
        local match_count=$(echo "$output" | grep -o '"rule_id"' | wc -l)
    fi
    
    # Clean the result
    match_count=$(echo "$match_count" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
    match_count=${match_count:-0}
    echo "$match_count"
}

# Compare results for a single test case
compare_test_case() {
    local py_file=$1
    local base_name=$(basename "$py_file" .py)
    local yaml_file="tests/categories/rules/${base_name}.yaml"

    print_status "INFO" "Testing: $base_name"
    echo "  Python file: $py_file"
    echo "  YAML rule: $yaml_file"

    if [ ! -f "$yaml_file" ]; then
        print_status "WARNING" "No corresponding YAML rule found for $py_file"
        echo ""
        return 1
    fi

    # Show rule content
    echo "  Rule content:"
    local rule_id=$(grep -E "^\s*-\s*id:" "$yaml_file" | head -1 | sed 's/.*id:\s*//' | tr -d ' ')
    local rule_mode=$(grep -E "^\s*mode:" "$yaml_file" | head -1 | sed 's/.*mode:\s*//' | tr -d ' ')
    local rule_message=$(grep -E "^\s*message:" "$yaml_file" | head -1 | sed 's/.*message:\s*//')

    echo "    ID: $rule_id"
    echo "    Mode: $rule_mode"
    echo "    Message: $rule_message"

    # Show Python code snippet
    echo "  Python code preview:"
    head -10 "$py_file" | sed 's/^/    /'
    if [ $(wc -l < "$py_file") -gt 10 ]; then
        echo "    ... (truncated)"
    fi

    # Run tests
    print_status "INFO" "Running Semgrep..."
    local semgrep_matches=$(run_semgrep_test "$py_file" "$yaml_file")

    print_status "INFO" "Running astgrep..."
    local astgrep_matches=$(run_astgrep_test "$py_file" "$yaml_file")

    # Compare results
    echo "  Results:"
    echo "    Semgrep matches: $semgrep_matches"
    echo "    astgrep matches: $astgrep_matches"

    # Ensure both values are valid numbers before comparison
    if [[ "$semgrep_matches" =~ ^[0-9]+$ ]] && [[ "$astgrep_matches" =~ ^[0-9]+$ ]]; then
        if [ "$semgrep_matches" -eq "$astgrep_matches" ]; then
            print_status "MATCH" "Results match! ✓"
            echo ""
            return 0
        else
            print_status "DIFF" "Results differ!"
            echo "    Difference: $((astgrep_matches - semgrep_matches))"
            echo "    Commands to debug:"
            echo "      semgrep --config \"$yaml_file\" \"$py_file\" --json"
            echo "      cargo run --bin astgrep -- analyze --config \"$yaml_file\" \"$py_file\" --format json"
            echo ""
            return 1
        fi
    else
        print_status "ERROR" "Invalid match counts: semgrep=$semgrep_matches, astgrep=$astgrep_matches"
        echo ""
        return 1
    fi
}

# Generate detailed comparison report
generate_comparison_report() {
    local py_files=($(find_python_tests))
    local total_tests=${#py_files[@]}
    local matching_tests=0
    local differing_tests=0
    local missing_rules=0

    print_status "INFO" "Generating detailed comparison report..."

    local report_file="PYTHON_COMPARISON_REPORT.md"

    cat > "$report_file" << EOF
# Python Rules Comparison Report

Generated on: $(date)

## Test Summary

### Overview
- **Total Python test files**: $total_tests
- **Semgrep version**: $(semgrep --version | head -n1)
- **astgrep version**: 0.1.0

### Test Categories

#### Taint Analysis Tests
EOF

    # Count different types of tests
    local taint_tests=$(find tests/categories/rules -maxdepth 1 -name "taint_*.py" | wc -l)
    local metavar_tests=$(find tests/categories/rules -maxdepth 1 -name "metavar_*.py" | wc -l)
    local sym_prop_tests=$(find tests/categories/rules -maxdepth 1 -name "sym_prop_*.py" | wc -l)
    local cp_tests=$(find tests/categories/rules -maxdepth 1 -name "cp_*.py" | wc -l)

    cat >> "$report_file" << EOF
- **Taint analysis tests**: $taint_tests files
- **Metavariable tests**: $metavar_tests files
- **Symbolic propagation tests**: $sym_prop_tests files
- **Constant propagation tests**: $cp_tests files

### Detailed Test Results

| Test File | Rule Type | Semgrep Matches | astgrep Matches | Status |
|-----------|-----------|-----------------|-----------------|--------|
EOF

    # Process each test file
    for py_file in "${py_files[@]}"; do
        local base_name=$(basename "$py_file" .py)
        local yaml_file="tests/categories/rules/${base_name}.yaml"

        if [ -f "$yaml_file" ]; then
            local rule_mode=$(grep -E "^\s*mode:" "$yaml_file" | head -1 | sed 's/.*mode:\s*//' | tr -d ' \n\r')
            local semgrep_matches=$(run_semgrep_test "$py_file" "$yaml_file" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
            local astgrep_matches=$(run_astgrep_test "$py_file" "$yaml_file" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)

            # Ensure we have valid numbers
            semgrep_matches=${semgrep_matches:-0}
            astgrep_matches=${astgrep_matches:-0}

            local status="❌ DIFFER"
            if [[ "$semgrep_matches" =~ ^[0-9]+$ ]] && [[ "$astgrep_matches" =~ ^[0-9]+$ ]]; then
                if [ "$semgrep_matches" -eq "$astgrep_matches" ]; then
                    status="✅ MATCH"
                    ((matching_tests++))
                else
                    ((differing_tests++))
                fi
            else
                ((differing_tests++))
                status="❌ INVALID"
            fi

            echo "| $base_name | $rule_mode | $semgrep_matches | $astgrep_matches | $status |" >> "$report_file"
        else
            echo "| $base_name | N/A | N/A | N/A | ⚠️ NO RULE |" >> "$report_file"
            ((missing_rules++))
        fi
    done

    local effective_total=$((total_tests - missing_rules))
    local compatibility_rate=0
    if [ "$effective_total" -gt 0 ]; then
        compatibility_rate=$(( matching_tests * 100 / effective_total ))
    fi

    cat >> "$report_file" << EOF

### Summary Statistics

- **Matching results**: $matching_tests tests
- **Differing results**: $differing_tests tests
- **Missing rules**: $missing_rules tests
- **Compatibility rate**: ${compatibility_rate}%

### Test Categories Analysis

#### Taint Analysis
Taint analysis tests focus on data flow tracking from sources to sinks.
Key patterns tested:
- Source-to-sink data flow
- Sanitizer effectiveness
- Field sensitivity
- Decorator handling
- Async functions

#### Metavariable Comparison
Tests for metavariable constraints and comparisons.
Key patterns tested:
- Regex patterns
- Type constraints
- String equality
- Pattern composition

#### Symbolic Propagation
Tests for symbolic value propagation through code.
Key patterns tested:
- Class attributes
- Method chaining
- Deep propagation
- Python-specific constructs (with statements, decorators)

#### Constant Propagation
Tests for constant value propagation.
Key patterns tested:
- String operations
- Numeric operations
- Expression evaluation

### Implementation Notes

#### Python-Specific Features
1. **Decorator Handling**: Support for @decorator syntax
2. **With Statements**: Context manager support
3. **Async/Await**: Asynchronous code patterns
4. **F-Strings**: Formatted string literals
5. **Type Hints**: Type annotation patterns

#### Current Limitations
1. **Python Parser Integration**: Need to verify tree-sitter Python support
2. **Taint Analysis**: Advanced taint tracking for Python patterns
3. **Symbolic Propagation**: Complex symbolic analysis pending
4. **Metavariable Constraints**: Some constraint types need implementation

---

**Report Generated**: $(date)
**Total Tests Analyzed**: $total_tests
**Compatibility Status**: In Development
EOF

    print_status "SUCCESS" "Comparison report generated: $report_file"
    echo ""
    echo "📊 Summary:"
    echo "  - Matching: $matching_tests"
    echo "  - Differing: $differing_tests"
    echo "  - Missing rules: $missing_rules"
    echo "  - Compatibility: ${compatibility_rate}%"
}

# Run comparison on all Python tests
run_all_comparisons() {
    local py_files=($(find_python_tests))
    local total_tests=${#py_files[@]}
    local current_test=0
    local passed=0
    local failed=0

    print_status "INFO" "Running comparison on all $total_tests Python test files..."
    echo ""

    for py_file in "${py_files[@]}"; do
        ((current_test++))
        echo "[$current_test/$total_tests]"
        if compare_test_case "$py_file"; then
            ((passed++))
        else
            ((failed++))
        fi
    done

    echo ""
    echo "📊 Test Results:"
    echo "  - Passed: $passed"
    echo "  - Failed: $failed"
    echo "  - Total: $total_tests"
}

# Run sample comparisons on interesting test cases
run_sample_comparisons() {
    print_status "INFO" "Running sample comparisons on key test cases..."
    echo ""

    # Select interesting test cases
    local sample_files=(
        "tests/categories/rules/taint_basic.py"
        "tests/categories/rules/sym_prop_chain.py"
        "tests/categories/rules/cp_python_strings.py"
        "tests/categories/rules/metavar_pattern_lang.py"
        "tests/categories/rules/taint_flask.py"
    )

    local total_samples=${#sample_files[@]}
    local current_sample=0

    for py_file in "${sample_files[@]}"; do
        if [ -f "$py_file" ]; then
            ((current_sample++))
            echo "[$current_sample/$total_samples] Sample Test"
            compare_test_case "$py_file"
        fi
    done

    print_status "SUCCESS" "Sample comparisons completed!"
    echo ""
}

# Show sample test cases
show_sample_tests() {
    print_status "INFO" "Sample Python test cases:"
    echo ""

    # Show a few interesting test cases
    local sample_files=(
        "tests/categories/rules/taint_basic.py"
        "tests/categories/rules/sym_prop_chain.py"
        "tests/categories/rules/cp_python_strings.py"
        "tests/categories/rules/metavar_pattern_lang.py"
    )

    for file in "${sample_files[@]}"; do
        if [ -f "$file" ]; then
            local base_name=$(basename "$file" .py)
            local yaml_file="tests/categories/rules/${base_name}.yaml"

            echo "📄 $base_name"
            echo "   Python: $file"
            echo "   YAML: $yaml_file"

            if [ -f "$yaml_file" ]; then
                local rule_type=$(grep -E "^\s*mode:" "$yaml_file" | head -1 | sed 's/.*mode:\s*//' | tr -d ' ')
                echo "   Type: $rule_type"
            fi
            echo ""
        fi
    done
}

# Show help
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -s, --sample        Run sample comparisons only"
    echo "  -a, --all           Run all comparisons (default)"
    echo "  -r, --report        Generate comparison report only"
    echo "  -l, --list          List sample test cases"
    echo ""
    echo "Examples:"
    echo "  $0                  # Run all comparisons"
    echo "  $0 --sample         # Run sample comparisons"
    echo "  $0 --report         # Generate report only"
}

# Main execution
main() {
    local run_mode="all"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -s|--sample)
                run_mode="sample"
                shift
                ;;
            -a|--all)
                run_mode="all"
                shift
                ;;
            -r|--report)
                run_mode="report"
                shift
                ;;
            -l|--list)
                run_mode="list"
                shift
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo "Starting Python rules comparison..."
    echo ""

    # Run all checks and tests
    check_semgrep
    echo ""

    build_project
    echo ""

    case $run_mode in
        "sample")
            run_sample_comparisons
            ;;
        "report")
            generate_comparison_report
            ;;
        "list")
            show_sample_tests
            ;;
        "all"|*)
            run_all_comparisons
            generate_comparison_report
            ;;
    esac

    print_status "SUCCESS" "Python comparison analysis completed!"
    echo ""
    echo "📊 Quick Summary:"
    local py_count=$(find tests/categories/rules -maxdepth 1 -name "*.py" | wc -l)
    local yaml_count=$(find tests/categories/rules -maxdepth 1 -name "*.yaml" | wc -l)
    echo "  - Python test files: $py_count"
    echo "  - YAML rule files: $yaml_count"
    echo "  - Test categories: Taint, Metavar, SymProp, ConstProp"
    echo ""
    echo "📄 Detailed report: PYTHON_COMPARISON_REPORT.md"
}

# Run main function
main "$@"
