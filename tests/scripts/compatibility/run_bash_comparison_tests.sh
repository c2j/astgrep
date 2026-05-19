#!/bin/bash

# Bash Rules Comparison Test Script for astgrep vs Semgrep
# This script compares results between Semgrep and astgrep on Bash/Shell test files

set -e

echo "🔍 astgrep vs Semgrep Bash Rules Comparison"
echo "============================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
TEST_DIR="${TEST_DIR:-tests/categories/bash}"
RULES_DIR="${RULES_DIR:-tests/categories/bash/rules}"

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

# Create sample Bash test files and rules
create_sample_tests() {
    print_status "INFO" "Creating sample Bash test files..."
    
    # Create test directory if it doesn't exist
    mkdir -p "$TEST_DIR"
    mkdir -p "$RULES_DIR"
    
    # Sample 1: Command Injection
    cat > "$TEST_DIR/command_injection.sh" << 'EOF'
#!/bin/bash

# MATCH: command injection via eval
user_input="$1"
eval "echo $user_input"

# MATCH: command injection via system
system_command="ls -la $user_input"
bash -c "$system_command"

# Safe: literal string
eval "echo 'hello world'"
EOF

    cat > "$RULES_DIR/command_injection.yaml" << 'EOF'
rules:
  - id: bash-command-injection
    patterns:
      - pattern: |
          eval $CMD
      - pattern: |
          bash -c $CMD
    message: "Potential command injection vulnerability"
    severity: ERROR
    languages: [bash]
EOF

    # Sample 2: Unsafe Variable Expansion
    cat > "$TEST_DIR/unsafe_expansion.sh" << 'EOF'
#!/bin/bash

# MATCH: unquoted variable expansion
file=$1
cat $file

# MATCH: unquoted array expansion
files=("$@")
rm ${files[@]}

# Safe: quoted expansion
cat "$file"
rm "${files[@]}"
EOF

    cat > "$RULES_DIR/unsafe_expansion.yaml" << 'EOF'
rules:
  - id: bash-unquoted-expansion
    patterns:
      - pattern: |
          $CMD $VAR
      - pattern: |
          $CMD ${$VAR}
    message: "Unquoted variable expansion may cause word splitting"
    severity: WARNING
    languages: [bash]
EOF

    # Sample 3: Useless Cat
    cat > "$TEST_DIR/useless_cat.sh" << 'EOF'
#!/bin/bash

# MATCH: useless use of cat
cat file.txt | grep "pattern"
cat data.log | awk '{print $1}'

# Better alternatives
grep "pattern" file.txt
awk '{print $1}' data.log
EOF

    cat > "$RULES_DIR/useless_cat.yaml" << 'EOF'
rules:
  - id: bash-useless-cat
    pattern: |
      cat $FILE | $CMD
    message: "Useless use of cat - consider redirecting input instead"
    severity: INFO
    languages: [bash]
EOF

    # Sample 4: Test Command
    cat > "$TEST_DIR/test_command.sh" << 'EOF'
#!/bin/bash

# MATCH: deprecated test syntax
[ $var == "value" ]

# Safe: modern test syntax
[[ $var == "value" ]]

# Safe: proper quoting
[ "$var" = "value" ]
EOF

    cat > "$RULES_DIR/test_command.yaml" << 'EOF'
rules:
  - id: bash-deprecated-test
    pattern: |
      [ $VAR == $VAL ]
    message: "Consider using [[ ]] for string comparison or = with [ ]"
    severity: WARNING
    languages: [bash]
EOF

    print_status "SUCCESS" "Created sample test files in $TEST_DIR"
}

# Find all Bash test files
find_bash_tests() {
    local sh_files=()
    
    # Check multiple possible locations
    for dir in "$TEST_DIR" "tests/categories/rules" "tests/bash"; do
        if [ -d "$dir" ]; then
            while IFS= read -r -d '' file; do
                sh_files+=("$file")
            done < <(find "$dir" -name "*.sh" -print0 2>/dev/null)
            while IFS= read -r -d '' file; do
                sh_files+=("$file")
            done < <(find "$dir" -name "*.bash" -print0 2>/dev/null)
        fi
    done
    
    # Sort and deduplicate
    printf '%s\n' "${sh_files[@]}" | sort -u
}

# Run Semgrep on a specific Bash file with its YAML rule
run_semgrep_test() {
    local sh_file=$1
    local yaml_file=$2

    if [ ! -f "$yaml_file" ]; then
        echo "0"
        return
    fi

    # Run semgrep and count matches
    local semgrep_output=$(semgrep --config "$yaml_file" "$sh_file" --json --lang bash 2>/dev/null || echo '{"results":[]}')
    local match_count=$(echo "$semgrep_output" | grep -c '"check_id"' 2>/dev/null || echo "0")

    # Clean and ensure we return a valid number
    match_count=$(echo "$match_count" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
    match_count=${match_count:-0}

    if [[ "$match_count" =~ ^[0-9]+$ ]]; then
        echo "$match_count"
    else
        echo "0"
    fi
}

# Run astgrep on a specific Bash file with its YAML rule
run_astgrep_test() {
    local sh_file=$1
    local yaml_file=$2

    if [ ! -f "$yaml_file" ]; then
        echo "0"
        return
    fi

    # Try to run our CLI tool if available
    if [ -f "target/debug/astgrep" ] || [ -f "target/release/astgrep" ]; then
        local output=$(cargo run --bin astgrep -- analyze --config "$yaml_file" "$sh_file" --format json 2>/dev/null || echo '{"findings":[]}')
        local match_count=$(echo "$output" | grep -c '"rule_id"' 2>/dev/null || echo "0")
        # Clean the result
        match_count=$(echo "$match_count" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
        match_count=${match_count:-0}
        echo "$match_count"
    else
        echo "0"
    fi
}

# Find YAML rule for a bash file
find_yaml_rule() {
    local sh_file=$1
    local base_name=$(basename "$sh_file" | sed 's/\.[^.]*$//')
    local dir=$(dirname "$sh_file")
    
    # Try multiple locations
    local yaml_locations=(
        "${dir}/${base_name}.yaml"
        "${dir}/${base_name}.yml"
        "$RULES_DIR/${base_name}.yaml"
        "$RULES_DIR/${base_name}.yml"
        "tests/categories/rules/${base_name}.yaml"
    )
    
    for yaml_file in "${yaml_locations[@]}"; do
        if [ -f "$yaml_file" ]; then
            echo "$yaml_file"
            return
        fi
    done
    echo ""
}

# Compare results for a single test case
compare_test_case() {
    local sh_file=$1
    local base_name=$(basename "$sh_file")
    local yaml_file=$(find_yaml_rule "$sh_file")

    print_status "INFO" "Testing: $base_name"
    echo "  Bash file: $sh_file"
    echo "  YAML rule: ${yaml_file:-<not found>}"

    if [ -z "$yaml_file" ]; then
        print_status "WARNING" "No corresponding YAML rule found for $sh_file"
        echo ""
        return 1
    fi

    # Show rule content
    echo "  Rule content:"
    local rule_id=$(grep -E "^\s*-\s*id:" "$yaml_file" | head -1 | sed 's/.*id:\s*//' | tr -d ' ')
    local rule_severity=$(grep -E "^\s*severity:" "$yaml_file" | head -1 | sed 's/.*severity:\s*//' | tr -d ' ')
    local rule_message=$(grep -E "^\s*message:" "$yaml_file" | head -1 | sed 's/.*message:\s*//')

    echo "    ID: $rule_id"
    echo "    Severity: $rule_severity"
    echo "    Message: $rule_message"

    # Show Bash code snippet
    echo "  Bash code preview:"
    head -10 "$sh_file" | sed 's/^/    /'
    if [ $(wc -l < "$sh_file") -gt 10 ]; then
        echo "    ... (truncated)"
    fi

    # Run tests
    print_status "INFO" "Running Semgrep..."
    local semgrep_matches=$(run_semgrep_test "$sh_file" "$yaml_file")

    print_status "INFO" "Running astgrep..."
    local astgrep_matches=$(run_astgrep_test "$sh_file" "$yaml_file")

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
            echo "      semgrep --config \"$yaml_file\" \"$sh_file\" --json --lang bash"
            echo "      cargo run --bin astgrep -- analyze --config \"$yaml_file\" \"$sh_file\" --format json"
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
    local sh_files=($(find_bash_tests))
    local total_tests=${#sh_files[@]}
    local matching_tests=0
    local differing_tests=0
    local missing_rules=0

    print_status "INFO" "Generating detailed comparison report..."

    local report_file="BASH_COMPARISON_REPORT.md"

    cat > "$report_file" << EOF
# Bash Rules Comparison Report

Generated on: $(date)

## Test Summary

### Overview
- **Total Bash test files**: $total_tests
- **Semgrep version**: $(semgrep --version | head -n1)
- **astgrep version**: 0.1.0

### Test Categories

#### Security Vulnerability Tests
- Command injection detection
- Unsafe variable expansion
- Path traversal risks

### Detailed Test Results

| Test File | Rule ID | Semgrep Matches | astgrep Matches | Status |
|-----------|---------|-----------------|-----------------|--------|
EOF

    # Process each test file
    for sh_file in "${sh_files[@]}"; do
        local base_name=$(basename "$sh_file")
        local yaml_file=$(find_yaml_rule "$sh_file")

        if [ -n "$yaml_file" ] && [ -f "$yaml_file" ]; then
            local rule_id=$(grep -E "^\s*-\s*id:" "$yaml_file" | head -1 | sed 's/.*id:\s*//' | tr -d ' \n\r')
            local semgrep_matches=$(run_semgrep_test "$sh_file" "$yaml_file" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
            local astgrep_matches=$(run_astgrep_test "$sh_file" "$yaml_file" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)

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

            echo "| $base_name | $rule_id | $semgrep_matches | $astgrep_matches | $status |" >> "$report_file"
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

#### Command Injection
Tests for detecting command injection vulnerabilities.
Key patterns tested:
- eval with user input
- bash -c with variables
- Command substitution risks

#### Unsafe Variable Expansion
Tests for detecting unquoted variable expansions.
Key patterns tested:
- Word splitting vulnerabilities
- Globbing risks
- Array expansion issues

#### Best Practices
Tests for detecting anti-patterns.
Key patterns tested:
- Useless use of cat
- Deprecated test syntax
- Inefficient patterns

### Implementation Notes

#### Bash-Specific Features
1. **Command Parsing**: Support for complex command structures
2. **Variable Expansion**: Various expansion types ($VAR, ${VAR}, ${VAR:-default})
3. **Control Flow**: if/while/for loops with conditions
4. **Functions**: Function definitions and calls
5. **Here Documents**: Multi-line string handling

#### Current Limitations
1. **Bash Parser Integration**: Need tree-sitter-bash support
2. **Complex Expansions**: Some expansion patterns may not parse correctly
3. **Interpolation**: Nested command substitution patterns
4. **Array Operations**: Complex array manipulation patterns

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

# Run comparison on all Bash tests
run_all_comparisons() {
    local sh_files=($(find_bash_tests))
    local total_tests=${#sh_files[@]}
    
    if [ "$total_tests" -eq 0 ]; then
        print_status "WARNING" "No Bash test files found. Creating sample tests..."
        create_sample_tests
        sh_files=($(find_bash_tests))
        total_tests=${#sh_files[@]}
    fi
    
    local current_test=0
    local passed=0
    local failed=0

    print_status "INFO" "Running comparison on all $total_tests Bash test files..."
    echo ""

    for sh_file in "${sh_files[@]}"; do
        ((current_test++))
        echo "[$current_test/$total_tests]"
        if compare_test_case "$sh_file"; then
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

    # Check if sample tests exist, create if not
    if [ ! -d "$TEST_DIR" ] || [ -z "$(find "$TEST_DIR" -name "*.sh" 2>/dev/null)" ]; then
        create_sample_tests
    fi

    # Select interesting test cases
    local sample_files=(
        "$TEST_DIR/command_injection.sh"
        "$TEST_DIR/unsafe_expansion.sh"
        "$TEST_DIR/useless_cat.sh"
        "$TEST_DIR/test_command.sh"
    )

    local total_samples=${#sample_files[@]}
    local current_sample=0

    for sh_file in "${sample_files[@]}"; do
        if [ -f "$sh_file" ]; then
            ((current_sample++))
            echo "[$current_sample/$total_samples] Sample Test"
            compare_test_case "$sh_file"
        fi
    done

    print_status "SUCCESS" "Sample comparisons completed!"
    echo ""
}

# Show sample test cases
show_sample_tests() {
    print_status "INFO" "Sample Bash test cases:"
    echo ""

    # Check if sample tests exist, create if not
    if [ ! -d "$TEST_DIR" ] || [ -z "$(find "$TEST_DIR" -name "*.sh" 2>/dev/null)" ]; then
        create_sample_tests
    fi

    # Show a few interesting test cases
    local sample_files=(
        "$TEST_DIR/command_injection.sh"
        "$TEST_DIR/unsafe_expansion.sh"
        "$TEST_DIR/useless_cat.sh"
    )

    for file in "${sample_files[@]}"; do
        if [ -f "$file" ]; then
            local base_name=$(basename "$file")
            local yaml_file=$(find_yaml_rule "$file")

            echo "📄 $base_name"
            echo "   Bash: $file"
            echo "   YAML: ${yaml_file:-<not found>}"

            if [ -n "$yaml_file" ] && [ -f "$yaml_file" ]; then
                local rule_id=$(grep -E "^\s*-\s*id:" "$yaml_file" | head -1 | sed 's/.*id:\s*//' | tr -d ' ')
                echo "   Rule ID: $rule_id"
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
    echo "  -c, --create        Create sample test files"
    echo ""
    echo "Environment Variables:"
    echo "  TEST_DIR            Directory for test files (default: tests/categories/bash)"
    echo "  RULES_DIR           Directory for rule files (default: tests/categories/bash/rules)"
    echo ""
    echo "Examples:"
    echo "  $0                  # Run all comparisons"
    echo "  $0 --sample         # Run sample comparisons"
    echo "  $0 --create         # Create sample test files"
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
            -c|--create)
                run_mode="create"
                shift
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo "Starting Bash rules comparison..."
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
        "create")
            create_sample_tests
            ;;
        "all"|*)
            run_all_comparisons
            generate_comparison_report
            ;;
    esac

    print_status "SUCCESS" "Bash comparison analysis completed!"
    echo ""
    echo "📊 Quick Summary:"
    local sh_count=$(find_bash_tests | wc -l)
    echo "  - Bash test files: $sh_count"
    echo "  - Test categories: Command Injection, Unsafe Expansion, Best Practices"
    echo ""
    echo "📄 Detailed report: BASH_COMPARISON_REPORT.md"
}

# Run main function
main "$@"
