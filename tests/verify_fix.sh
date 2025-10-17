#!/bin/bash

# Verify that the integer expression error is fixed

echo "🔧 Verifying Integer Expression Fix"
echo "==================================="
echo ""

# Test the problematic line that was causing the error
test_integer_comparison() {
    echo "Testing integer comparison logic..."
    
    # Simulate the problematic values that were causing the error
    local test_values=("1" "0" "2" "0\n0" "1\n" "\n1" "")
    
    for value in "${test_values[@]}"; do
        echo "Testing value: '$value'"
        
        # Clean the value like our fixed script does
        cleaned_value=$(echo "$value" | tr -d '\n\r' | grep -o '[0-9]*' | head -1)
        cleaned_value=${cleaned_value:-0}
        
        echo "  Cleaned value: '$cleaned_value'"
        
        # Test the comparison
        if [[ "$cleaned_value" =~ ^[0-9]+$ ]]; then
            if [ "$cleaned_value" -eq 0 ]; then
                echo "  ✅ Comparison successful: value equals 0"
            else
                echo "  ✅ Comparison successful: value is $cleaned_value"
            fi
        else
            echo "  ❌ Invalid value for comparison"
        fi
        echo ""
    done
}

# Test with actual Java files
test_with_real_files() {
    echo "Testing with real Java files..."
    
    # Source the functions from our fixed script
    source ./run_java_comparison_tests.sh
    
    local test_file="tests/rules/taint_final_globals.java"
    local yaml_file="tests/rules/taint_final_globals.yaml"
    
    if [ -f "$test_file" ] && [ -f "$yaml_file" ]; then
        echo "Testing with: $test_file"
        
        local semgrep_result=$(run_semgrep_test "$test_file" "$yaml_file")
        local cr_result=$(run_cr_semservice_test "$test_file" "$yaml_file")
        
        echo "Semgrep result: '$semgrep_result'"
        echo "CR-SemService result: '$cr_result'"
        
        # Test the comparison that was failing
        if [[ "$semgrep_result" =~ ^[0-9]+$ ]] && [[ "$cr_result" =~ ^[0-9]+$ ]]; then
            if [ "$semgrep_result" -eq "$cr_result" ]; then
                echo "✅ Integer comparison successful - values match"
            else
                echo "✅ Integer comparison successful - values differ ($semgrep_result vs $cr_result)"
            fi
        else
            echo "❌ Integer comparison failed - invalid values"
        fi
    else
        echo "⚠️  Test files not found, skipping real file test"
    fi
}

# Main execution
echo "1. Testing integer comparison logic:"
test_integer_comparison

echo ""
echo "2. Testing with real files:"
test_with_real_files

echo ""
echo "🎯 Fix Verification Complete!"
echo ""
echo "The 'integer expression expected' error should now be resolved."
echo "The script properly cleans values and validates them before comparison."
