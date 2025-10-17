#!/bin/bash

# CR-SemService Comprehensive Validation Suite
# Runs all tests and generates detailed reports

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTS_DIR="$PROJECT_ROOT/tests"
REPORTS_DIR="$TESTS_DIR/validation_reports"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create reports directory
mkdir -p "$REPORTS_DIR"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  CR-SemService Comprehensive Validation Suite              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Build the project
echo -e "${YELLOW}[1/4] Building CR-SemService...${NC}"
cd "$PROJECT_ROOT"
if cargo build --release 2>&1 | tail -5; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi
echo ""

# Step 2: Run quick validation
echo -e "${YELLOW}[2/4] Running Quick Validation...${NC}"
if python3 "$TESTS_DIR/quick_validation.py"; then
    echo -e "${GREEN}✅ Quick validation passed${NC}"
else
    echo -e "${YELLOW}⚠️  Quick validation had issues${NC}"
fi
echo ""

# Step 3: Run comprehensive tests
echo -e "${YELLOW}[3/4] Running Comprehensive Tests...${NC}"
if python3 "$TESTS_DIR/comprehensive_test_runner.py"; then
    echo -e "${GREEN}✅ Comprehensive tests completed${NC}"
else
    echo -e "${YELLOW}⚠️  Some tests failed${NC}"
fi
echo ""

# Step 4: Analyze results and generate reports
echo -e "${YELLOW}[4/4] Analyzing Results and Generating Reports...${NC}"
if python3 "$TESTS_DIR/test_analyzer.py"; then
    echo -e "${GREEN}✅ Analysis complete${NC}"
else
    echo -e "${RED}❌ Analysis failed${NC}"
fi
echo ""

# Copy reports to reports directory
cp "$TESTS_DIR/test_report.json" "$REPORTS_DIR/" 2>/dev/null || true
cp "$TESTS_DIR/test_report.md" "$REPORTS_DIR/" 2>/dev/null || true

# Generate final summary
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")
REPORT_FILE="$REPORTS_DIR/validation_summary_$(date +%Y%m%d_%H%M%S).txt"

cat > "$REPORT_FILE" << EOF
CR-SemService Validation Report
Generated: $TIMESTAMP
Project: $PROJECT_ROOT

Test Results:
- Quick Validation: See test_report.md
- Comprehensive Tests: See test_report.json
- Detailed Analysis: See test_report.md

Reports Location: $REPORTS_DIR

For more details, run:
  python3 $TESTS_DIR/test_analyzer.py
EOF

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Validation Complete                                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}📊 Reports saved to: $REPORTS_DIR${NC}"
echo -e "${GREEN}📄 Summary: $REPORT_FILE${NC}"
echo ""
echo "Next steps:"
echo "  1. Review test_report.md for detailed results"
echo "  2. Check test_report.json for raw data"
echo "  3. Run individual tests for debugging"
echo ""

