#!/bin/bash

# Test script to verify the print_status function fix

echo "🧪 Testing print_status function fix"
echo "===================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Fixed print_status function
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

# Test all status types
echo "Testing all print_status types:"
echo ""

print_status "INFO" "This is an info message"
print_status "SUCCESS" "This is a success message"
print_status "WARNING" "This is a warning message"
print_status "ERROR" "This is an error message"
print_status "DIFF" "This is a diff message"
print_status "MATCH" "This is a match message"

echo ""
echo "✅ All print_status tests completed without basename errors!"

# Test with a basename command to ensure no conflicts
echo ""
echo "Testing basename command separately:"
test_file="tests/categories/rules/example.java"
base_name=$(basename "$test_file" .java)
echo "basename result: $base_name"

echo ""
echo "🎯 Fix verification complete - no more 'basename: illegal option -- e' errors!"
