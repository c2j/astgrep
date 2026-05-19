#!/bin/bash

# Unified Compatibility Test Runner
# Runs comparison tests for all supported languages

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_banner() {
    echo ""
    printf "${BOLD}${PURPLE}╔══════════════════════════════════════════════════════════════╗${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}                                                              ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}   █████╗ ████████╗████████╗██████╗ ${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}  ██╔══██╗╚══██╔══╝╚══██╔══╝╚════██╗${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}  ███████║   ██║      ██║    █████╔╝${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}  ██╔══██║   ██║      ██║   ██╔═══╝ ${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}  ██║  ██║   ██║      ██║   ███████╗${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${CYAN}  ╚═╝  ╚═╝   ╚═╝      ╚═╝   ╚══════╝${NC}                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}                                                              ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${GREEN}Compatibility Test Suite${NC}                                    ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}  ${YELLOW}astgrep vs Semgrep${NC}                                         ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}║${NC}                                                              ${BOLD}${PURPLE}║${NC}\n"
    printf "${BOLD}${PURPLE}╚══════════════════════════════════════════════════════════════╝${NC}\n"
    echo ""
}

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
        "RUNNING")
            printf "${CYAN}🚀 %s${NC}\n" "$message"
            ;;
    esac
}

show_help() {
    echo "Usage: $0 [OPTIONS] [LANGUAGE...]"
    echo ""
    echo "Run compatibility comparison tests between astgrep and Semgrep."
    echo ""
    echo "Languages:"
    echo "  java        Run Java compatibility tests"
    echo "  python      Run Python compatibility tests"
    echo "  javascript  Run JavaScript compatibility tests"
    echo "  bash        Run Bash compatibility tests"
    echo "  all         Run all language tests (default)"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -s, --sample        Run sample tests only (faster)"
    echo "  -r, --report        Generate reports only"
    echo "  -l, --list          List available test cases"
    echo "  -v, --verbose       Enable verbose output"
    echo ""
    echo "Examples:"
    echo "  $0                        # Run all language tests"
    echo "  $0 python javascript      # Run Python and JavaScript tests"
    echo "  $0 --sample java          # Run sample Java tests"
    echo "  $0 --report all           # Generate reports for all languages"
}

run_language_test() {
    local language=$1
    local mode=$2
    local script=""
    local report_file=""
    
    case $language in
        "java")
            script="$SCRIPT_DIR/run_java_comparison_tests.sh"
            report_file="JAVA_COMPARISON_REPORT.md"
            ;;
        "python")
            script="$SCRIPT_DIR/run_python_comparison_tests.sh"
            report_file="PYTHON_COMPARISON_REPORT.md"
            ;;
        "javascript")
            script="$SCRIPT_DIR/run_javascript_comparison_tests.sh"
            report_file="JAVASCRIPT_COMPARISON_REPORT.md"
            ;;
        "bash")
            script="$SCRIPT_DIR/run_bash_comparison_tests.sh"
            report_file="BASH_COMPARISON_REPORT.md"
            ;;
        *)
            print_status "ERROR" "Unknown language: $language"
            return 1
            ;;
    esac
    
    if [ ! -f "$script" ]; then
        print_status "ERROR" "Script not found: $script"
        return 1
    fi
    
    print_status "RUNNING" "Running $language compatibility tests..."
    echo ""
    
    local args=""
    case $mode in
        "sample")
            args="--sample"
            ;;
        "report")
            args="--report"
            ;;
        "list")
            args="--list"
            ;;
    esac
    
    if bash "$script" $args; then
        print_status "SUCCESS" "$language tests completed"
        echo ""
        echo "📄 Report: $report_file"
        echo ""
    else
        print_status "ERROR" "$language tests failed"
        return 1
    fi
}

show_summary() {
    echo ""
    printf "${BOLD}${PURPLE}════════════════════════════════════════════════════════════${NC}\n"
    printf "${BOLD}                    Summary Report${NC}\n"
    printf "${BOLD}${PURPLE}════════════════════════════════════════════════════════════${NC}\n"
    echo ""
    
    local reports=(
        "JAVA_COMPARISON_REPORT.md:Java"
        "PYTHON_COMPARISON_REPORT.md:Python"
        "JAVASCRIPT_COMPARISON_REPORT.md:JavaScript"
        "BASH_COMPARISON_REPORT.md:Bash"
    )
    
    printf "${BOLD}%-20s %-15s %-15s %-15s${NC}\n" "Report" "Matching" "Differing" "Rate"
    printf "%-20s %-15s %-15s %-15s\n" "------" "--------" "---------" "----"
    
    for item in "${reports[@]}"; do
        IFS=':' read -r file name <<< "$item"
        if [ -f "$file" ]; then
            local matching=$(grep -oP 'Matching results.*?(\d+)' "$file" 2>/dev/null | grep -oP '\d+' || echo "N/A")
            local differing=$(grep -oP 'Differing results.*?(\d+)' "$file" 2>/dev/null | grep -oP '\d+' || echo "N/A")
            local rate=$(grep -oP 'Compatibility rate.*?(\d+)%' "$file" 2>/dev/null | grep -oP '\d+' || echo "N/A")
            if [ "$rate" != "N/A" ]; then
                rate="${rate}%"
            fi
            printf "%-20s %-15s %-15s %-15s\n" "$name" "$matching" "$differing" "$rate"
        fi
    done
    
    echo ""
    printf "${BOLD}${PURPLE}════════════════════════════════════════════════════════════${NC}\n"
    echo ""
}

check_prerequisites() {
    print_status "INFO" "Checking prerequisites..."
    
    local missing=0
    
    if ! command -v semgrep &> /dev/null; then
        print_status "WARNING" "Semgrep not installed. Install with: pip install semgrep"
        missing=1
    fi
    
    if ! command -v cargo &> /dev/null; then
        print_status "WARNING" "Cargo not installed. Install Rust from: https://rustup.rs"
        missing=1
    fi
    
    if ! command -v jq &> /dev/null; then
        print_status "WARNING" "jq not installed. Install with: brew install jq"
    fi
    
    if [ $missing -eq 1 ]; then
        echo ""
        print_status "ERROR" "Missing required dependencies. Please install them first."
        exit 1
    fi
    
    print_status "SUCCESS" "All prerequisites met"
    echo ""
}

main() {
    local languages=()
    local mode="all"
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -s|--sample)
                mode="sample"
                shift
                ;;
            -r|--report)
                mode="report"
                shift
                ;;
            -l|--list)
                mode="list"
                shift
                ;;
            -v|--verbose)
                set -x
                shift
                ;;
            java|python|javascript|bash|all)
                languages+=("$1")
                shift
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    print_banner
    
    check_prerequisites
    
    if [ ${#languages[@]} -eq 0 ]; then
        languages=("java" "python" "javascript" "bash")
    fi
    
    if [[ " ${languages[@]} " =~ " all " ]]; then
        languages=("java" "python" "javascript" "bash")
    fi
    
    local failed=0
    for lang in "${languages[@]}"; do
        if ! run_language_test "$lang" "$mode"; then
            failed=1
        fi
    done
    
    if [ "$mode" != "list" ]; then
        show_summary
    fi
    
    if [ $failed -eq 1 ]; then
        print_status "WARNING" "Some tests failed. Check the reports for details."
        exit 1
    else
        print_status "SUCCESS" "All compatibility tests completed successfully!"
    fi
}

main "$@"
