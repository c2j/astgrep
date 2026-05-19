#!/bin/bash

# Quick test to see what happens with as_metavariable test
cd /Users/c2j/Projects/Desktop_Projects/CR/astgrep

echo "=== Testing as_metavariable ==="
echo "Running: ./target/release/astgrep analyze tests/categories/rules/as_metavariable.py --rules tests/categories/rules/as_metavariable.yaml"
./target/release/astgrep analyze tests/categories/rules/as_metavariable.py --rules tests/categories/rules/as_metavariable.yaml 2>&1

echo ""
echo "=== For comparison, semgrep output ==="
semgrep --config tests/categories/rules/as_metavariable.yaml tests/categories/rules/as_metavariable.py 2>&1 | head -20