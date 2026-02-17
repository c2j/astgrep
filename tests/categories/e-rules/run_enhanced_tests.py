#!/usr/bin/env python3
"""
Test runner for enhanced rule features
"""

import subprocess
import json
import os
import sys
from pathlib import Path

def run_cr_analysis(rule_file, target_file):
    """Run CR-SemService analysis"""
    cmd = [
        "./target/debug/astgrep",
        "analyze",
        "--config", rule_file,
        target_file
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        if result.returncode != 0:
            print(f"Error running analysis: {result.stderr}")
            return None
        
        # Extract JSON from output
        stdout = result.stdout
        json_start = stdout.find('{')
        json_end = stdout.rfind('}') + 1
        
        if json_start == -1 or json_end == 0:
            print("No JSON found in output")
            return None
        
        json_str = stdout[json_start:json_end]
        return json.loads(json_str)
        
    except Exception as e:
        print(f"Exception running analysis: {e}")
        return None

def test_enhanced_features():
    """Test all enhanced features"""
    
    test_cases = [
        # Pattern-not-inside tests
        {
            "name": "Pattern-not-inside Python",
            "rule": "tests/e-rules/pattern_not_inside_test.yaml",
            "target": "tests/e-rules/pattern_not_inside_test.py",
            "expected_rules": ["function-outside-class", "eval-outside-sandbox", "sql-query-outside-transaction"]
        },
        {
            "name": "Pattern-not-inside JavaScript",
            "rule": "tests/e-rules/pattern_not_inside_test.yaml",
            "target": "tests/e-rules/pattern_not_inside_test.js",
            "expected_rules": ["unsafe-eval-outside-try-catch"]
        },
        
        # Pattern-not-regex tests
        {
            "name": "Pattern-not-regex Python",
            "rule": "tests/e-rules/pattern_not_regex_test.yaml",
            "target": "tests/e-rules/pattern_not_regex_test.py",
            "expected_rules": ["detect-only-foo-package", "simple-variable-names", "http-urls-not-https"]
        },
        {
            "name": "Pattern-not-regex JavaScript",
            "rule": "tests/e-rules/pattern_not_regex_test.yaml",
            "target": "tests/e-rules/pattern_not_regex_test.js",
            "expected_rules": ["basic-function-calls", "http-urls-not-https", "console-log-not-error"]
        },
        
        # Focus-metavariable tests
        {
            "name": "Focus-metavariable Python",
            "rule": "tests/e-rules/focus_metavariable_test.yaml",
            "target": "tests/e-rules/focus_metavariable_test.py",
            "expected_rules": ["focus-sensitive-param", "focus-dangerous-arg", "focus-sql-query", "focus-url-endpoint"]
        },
        {
            "name": "Focus-metavariable JavaScript",
            "rule": "tests/e-rules/focus_metavariable_test.yaml",
            "target": "tests/e-rules/focus_metavariable_test.js",
            "expected_rules": ["focus-multiple-vars"]
        },
        
        # Comprehensive tests
        {
            "name": "Comprehensive Enhanced Python",
            "rule": "tests/e-rules/comprehensive_enhanced_test.yaml",
            "target": "tests/e-rules/comprehensive_enhanced_test.py",
            "expected_rules": ["complex-security-check", "unsafe-http-outside-dev", "sql-injection-advanced"]
        },
        {
            "name": "Comprehensive Enhanced JavaScript",
            "rule": "tests/e-rules/comprehensive_enhanced_test.yaml",
            "target": "tests/e-rules/comprehensive_enhanced_test.js",
            "expected_rules": ["dangerous-eval-comprehensive"]
        }
    ]
    
    results = []
    
    for test_case in test_cases:
        print(f"\n🧪 Testing: {test_case['name']}")
        
        result = run_cr_analysis(test_case["rule"], test_case["target"])
        
        if result is None:
            print(f"  ❌ FAIL: Analysis failed")
            results.append((test_case["name"], False, 0, 0))
            continue
        
        findings = result.get("findings", [])
        found_rules = set(finding.get("rule_id", "") for finding in findings)
        expected_rules = set(test_case["expected_rules"])
        
        matched_rules = found_rules.intersection(expected_rules)
        missing_rules = expected_rules - found_rules
        unexpected_rules = found_rules - expected_rules
        
        success = len(missing_rules) == 0
        
        if success:
            print(f"  ✅ PASS: Found {len(findings)} findings, all expected rules matched")
        else:
            print(f"  ❌ FAIL: Found {len(findings)} findings")
            if missing_rules:
                print(f"    Missing rules: {missing_rules}")
            if unexpected_rules:
                print(f"    Unexpected rules: {unexpected_rules}")
        
        results.append((test_case["name"], success, len(findings), len(expected_rules)))
    
    return results

def print_summary(results):
    """Print test summary"""
    print("\n" + "="*60)
    print("🎯 ENHANCED FEATURES TEST SUMMARY")
    print("="*60)
    
    passed = sum(1 for _, success, _, _ in results if success)
    total = len(results)
    
    print(f"Total tests: {total}")
    print(f"Passed: {passed}")
    print(f"Failed: {total - passed}")
    print(f"Success rate: {passed/total*100:.1f}%")
    
    print("\n📊 Detailed Results:")
    for test_name, success, findings_count, expected_count in results:
        status = "✅ PASS" if success else "❌ FAIL"
        print(f"  {status}: {test_name} (Found={findings_count}, Expected={expected_count})")
    
    if passed == total:
        print("\n🎉 ALL ENHANCED FEATURES WORKING PERFECTLY!")
    else:
        print(f"\n⚠️  {total - passed} tests failed. Need further improvements.")

if __name__ == "__main__":
    print("🚀 Running enhanced features tests...")
    
    # Check if binary exists
    if not os.path.exists("./target/debug/astgrep"):
        print("❌ Binary not found. Please build the project first:")
        print("   cargo build")
        sys.exit(1)
    
    try:
        results = test_enhanced_features()
        print_summary(results)
        
        # Exit with error code if any tests failed
        failed_count = sum(1 for _, success, _, _ in results if not success)
        sys.exit(failed_count)
        
    except Exception as e:
        print(f"\n❌ Test runner failed: {e}")
        sys.exit(1)
