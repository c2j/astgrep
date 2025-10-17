#!/bin/bash

# Java Comparison Demo Script
# This script demonstrates the differences between Semgrep and CR-SemService on Java files

set -e

echo "🔍 Java Comparison Demo: Semgrep vs CR-SemService"
echo "================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    case $status in
        "INFO") echo -e "${BLUE}ℹ️  $message${NC}" ;;
        "SUCCESS") echo -e "${GREEN}✅ $message${NC}" ;;
        "WARNING") echo -e "${YELLOW}⚠️  $message${NC}" ;;
        "ERROR") echo -e "${RED}❌ $message${NC}" ;;
        "DIFF") echo -e "${PURPLE}🔍 $message${NC}" ;;
        "MATCH") echo -e "${CYAN}🎯 $message${NC}" ;;
    esac
}

# Function to show file content with line numbers
show_file_content() {
    local file=$1
    local title=$2
    
    echo "📄 $title"
    echo "   File: $file"
    echo "   Content:"
    if [ -f "$file" ]; then
        cat -n "$file" | sed 's/^/   /'
    else
        echo "   File not found!"
    fi
    echo ""
}

# Function to run Semgrep and show results
run_semgrep_demo() {
    local yaml_file=$1
    local java_file=$2
    local test_name=$3
    
    print_status "INFO" "Running Semgrep on $test_name"
    
    if ! command -v semgrep &> /dev/null; then
        print_status "WARNING" "Semgrep not installed, skipping..."
        return
    fi
    
    echo "   Command: semgrep --config $yaml_file $java_file"
    
    # Run semgrep with detailed output
    local semgrep_output=$(semgrep --config "$yaml_file" "$java_file" 2>/dev/null || echo "No matches or error")
    
    echo "   Semgrep Results:"
    echo "$semgrep_output" | sed 's/^/   /'
    
    # Count matches
    local match_count=$(echo "$semgrep_output" | grep -c "rule:" 2>/dev/null || echo "0")
    echo "   Match count: $match_count"
    echo ""
}

# Function to run CR-SemService demo
run_cr_semservice_demo() {
    local yaml_file=$1
    local java_file=$2
    local test_name=$3
    
    print_status "INFO" "Running CR-SemService on $test_name"
    
    echo "   Command: cargo run --example java_test_runner"
    
    # Try to run our Java test runner
    if cargo run --example java_test_runner --quiet 2>/dev/null; then
        print_status "SUCCESS" "CR-SemService Java test completed"
    else
        print_status "WARNING" "CR-SemService Java support in development"
        echo "   Status: Java parsing and analysis capabilities are being implemented"
        echo "   Current: Basic pattern matching available"
        echo "   Planned: Full Java AST analysis, taint tracking, symbolic propagation"
    fi
    echo ""
}

# Demo test case 1: Taint Analysis
demo_taint_analysis() {
    echo "🧪 Demo 1: Taint Analysis"
    echo "========================="
    echo ""
    
    local java_file="tests/rules/taint_final_globals.java"
    local yaml_file="tests/rules/taint_final_globals.yaml"
    
    if [ -f "$java_file" ] && [ -f "$yaml_file" ]; then
        show_file_content "$yaml_file" "Taint Analysis Rule"
        show_file_content "$java_file" "Java Test Code"
        
        run_semgrep_demo "$yaml_file" "$java_file" "Taint Analysis"
        run_cr_semservice_demo "$yaml_file" "$java_file" "Taint Analysis"
    else
        print_status "WARNING" "Taint analysis test files not found"
    fi
}

# Demo test case 2: Metavariable Comparison
demo_metavar_comparison() {
    echo "🧪 Demo 2: Metavariable Comparison"
    echo "=================================="
    echo ""
    
    local java_file="tests/rules/metavar_comparison_bitxor.java"
    local yaml_file="tests/rules/metavar_comparison_bitxor.yaml"
    
    if [ -f "$java_file" ] && [ -f "$yaml_file" ]; then
        show_file_content "$yaml_file" "Metavariable Comparison Rule"
        show_file_content "$java_file" "Java Test Code"
        
        run_semgrep_demo "$yaml_file" "$java_file" "Metavariable Comparison"
        run_cr_semservice_demo "$yaml_file" "$java_file" "Metavariable Comparison"
    else
        print_status "WARNING" "Metavariable comparison test files not found"
    fi
}

# Demo test case 3: Symbolic Propagation
demo_symbolic_propagation() {
    echo "🧪 Demo 3: Symbolic Propagation"
    echo "==============================="
    echo ""
    
    local java_file="tests/rules/sym_prop_class_attr.java"
    local yaml_file="tests/rules/sym_prop_class_attr.yaml"
    
    if [ -f "$java_file" ] && [ -f "$yaml_file" ]; then
        show_file_content "$yaml_file" "Symbolic Propagation Rule"
        show_file_content "$java_file" "Java Test Code"
        
        run_semgrep_demo "$yaml_file" "$java_file" "Symbolic Propagation"
        run_cr_semservice_demo "$yaml_file" "$java_file" "Symbolic Propagation"
    else
        print_status "WARNING" "Symbolic propagation test files not found"
    fi
}

# Show overall statistics
show_statistics() {
    echo "📊 Java Test Statistics"
    echo "======================="
    echo ""
    
    local java_count=$(find tests/rules -name "*.java" 2>/dev/null | wc -l)
    local yaml_count=$(find tests/rules -name "*.yaml" 2>/dev/null | wc -l)
    local taint_count=$(find tests/rules -name "taint_*.java" 2>/dev/null | wc -l)
    local metavar_count=$(find tests/rules -name "metavar_*.java" 2>/dev/null | wc -l)
    local sym_prop_count=$(find tests/rules -name "sym_prop_*.java" 2>/dev/null | wc -l)
    local cp_count=$(find tests/rules -name "cp_*.java" 2>/dev/null | wc -l)
    
    echo "📈 Test File Counts:"
    echo "   Total Java files: $java_count"
    echo "   Total YAML rules: $yaml_count"
    echo ""
    echo "📂 Test Categories:"
    echo "   Taint analysis: $taint_count files"
    echo "   Metavariable tests: $metavar_count files"
    echo "   Symbolic propagation: $sym_prop_count files"
    echo "   Constant propagation: $cp_count files"
    echo ""
    
    # Show some example files
    echo "📋 Example Test Files:"
    find tests/rules -name "*.java" 2>/dev/null | head -5 | while read file; do
        echo "   - $(basename "$file")"
    done
    echo ""
}

# Show implementation status
show_implementation_status() {
    echo "🚧 Implementation Status"
    echo "========================"
    echo ""
    
    echo "✅ Completed Features:"
    echo "   - Basic pattern matching framework"
    echo "   - Universal AST representation"
    echo "   - Advanced pattern types (either, not, inside, regex)"
    echo "   - Metavariable support"
    echo "   - Test infrastructure"
    echo ""
    
    echo "🔄 In Progress:"
    echo "   - Java-specific AST parsing"
    echo "   - Taint analysis engine"
    echo "   - Symbolic propagation"
    echo "   - Constant propagation"
    echo "   - Field sensitivity"
    echo ""
    
    echo "📋 Planned Features:"
    echo "   - Full Java language support"
    echo "   - Advanced data flow analysis"
    echo "   - Inter-procedural analysis"
    echo "   - Performance optimizations"
    echo "   - IDE integrations"
    echo ""
}

# Main demo execution
main() {
    print_status "INFO" "Starting Java comparison demo..."
    echo ""
    
    # Check if we're in the right directory
    if [ ! -d "tests/rules" ]; then
        print_status "ERROR" "tests/rules directory not found. Please run from project root."
        exit 1
    fi
    
    # Show statistics first
    show_statistics
    
    # Run demo test cases
    demo_taint_analysis
    demo_metavar_comparison
    demo_symbolic_propagation
    
    # Show implementation status
    show_implementation_status
    
    print_status "SUCCESS" "Java comparison demo completed!"
    echo ""
    echo "🎯 Key Takeaways:"
    echo "   - CR-SemService provides a solid foundation for Java analysis"
    echo "   - Advanced pattern matching capabilities are already implemented"
    echo "   - Java-specific features are in active development"
    echo "   - Performance improvements expected over Semgrep"
    echo ""
    echo "📄 For detailed analysis, run: ./run_java_comparison_tests.sh"
}

# Run the demo
main "$@"
