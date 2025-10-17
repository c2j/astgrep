#!/bin/bash

# CR-SemService Validation Entry Point
# Simple interface to run validation suite

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTS_DIR="$PROJECT_ROOT/tests"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Help function
show_help() {
    cat << EOF
${BLUE}CR-SemService Validation Suite${NC}

Usage: $0 [COMMAND] [OPTIONS]

Commands:
  quick       Run quick validation (2-5 min)
  full        Run full validation suite (10-30 min)
  analyze     Analyze existing test results
  report      Generate detailed reports
  clean       Clean validation reports
  help        Show this help message

Examples:
  $0 quick              # Quick validation
  $0 full               # Full validation with reports
  $0 analyze            # Analyze last test run
  $0 report             # Generate HTML/text reports

EOF
}

# Quick validation
run_quick() {
    echo -e "${BLUE}Running Quick Validation...${NC}\n"
    python3 "$TESTS_DIR/quick_validation.py"
}

# Full validation
run_full() {
    echo -e "${BLUE}Running Full Validation Suite...${NC}\n"
    bash "$TESTS_DIR/run_validation_suite.sh"
}

# Analyze results
run_analyze() {
    if [ ! -f "$TESTS_DIR/test_report.json" ]; then
        echo -e "${RED}Error: test_report.json not found${NC}"
        echo "Run 'validate.sh full' first to generate test results"
        exit 1
    fi
    
    echo -e "${BLUE}Analyzing Test Results...${NC}\n"
    python3 "$TESTS_DIR/test_analyzer.py"
}

# Generate reports
run_report() {
    if [ ! -f "$TESTS_DIR/test_report.json" ]; then
        echo -e "${RED}Error: test_report.json not found${NC}"
        echo "Run 'validate.sh full' first to generate test results"
        exit 1
    fi
    
    echo -e "${BLUE}Generating Detailed Reports...${NC}\n"
    python3 "$TESTS_DIR/generate_detailed_report.py"
    
    echo -e "${GREEN}✅ Reports generated:${NC}"
    echo "  - test_report.html"
    echo "  - test_report.txt"
    echo "  - test_report.md"
}

# Clean reports
run_clean() {
    echo -e "${YELLOW}Cleaning validation reports...${NC}"
    rm -rf "$TESTS_DIR/validation_reports"
    rm -f "$TESTS_DIR/test_report.json"
    rm -f "$TESTS_DIR/test_report.md"
    rm -f "$TESTS_DIR/test_report.html"
    rm -f "$TESTS_DIR/test_report.txt"
    echo -e "${GREEN}✅ Cleaned${NC}"
}

# Main
main() {
    if [ $# -eq 0 ]; then
        show_help
        exit 0
    fi
    
    case "$1" in
        quick)
            run_quick
            ;;
        full)
            run_full
            ;;
        analyze)
            run_analyze
            ;;
        report)
            run_report
            ;;
        clean)
            run_clean
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown command: $1${NC}"
            show_help
            exit 1
            ;;
    esac
}

main "$@"

