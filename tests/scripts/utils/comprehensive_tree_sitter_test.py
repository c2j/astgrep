#!/usr/bin/env python3
"""
Comprehensive test for our improved tree-sitter AST-based pattern matching
"""

import subprocess
import json
import tempfile
import os

def run_comprehensive_tests():
    """Run comprehensive tests comparing our implementation with semgrep"""
    
    test_cases = [
        # Python tests
        ("Python eval calls", "tests/comparison/test_eval_calls.yaml", "tests/comparison/simple_python_test.py"),
        ("Python string literals", "tests/comparison/test_string_literals.yaml", "tests/comparison/simple_python_test.py"),
        ("Python numeric literals", "tests/comparison/test_numeric_literals.yaml", "tests/comparison/simple_python_test.py"),
        ("Python import statements", "tests/comparison/test_python_imports.yaml", "tests/comparison/simple_import_test.py"),
        
        # Java tests
        ("Java println calls", "tests/comparison/test_java_println.yaml", "tests/comparison/simple_java_test.java"),
        ("Java string literals", "tests/comparison/test_java_string.yaml", "tests/comparison/simple_java_test.java"),
        ("Java numeric literals", "tests/comparison/test_java_number.yaml", "tests/comparison/simple_java_test.java"),
    ]
    
    results = []
    
    for test_name, rule_file, target_file in test_cases:
        print(f"\n🧪 Testing: {test_name}")
        
        try:
            # Run our implementation
            our_result = run_cr_analysis(rule_file, target_file)
            
            # Run semgrep
            semgrep_result = run_semgrep_analysis(rule_file, target_file)
            
            # Compare results
            our_count = len(our_result.get("findings", []))
            semgrep_count = len(semgrep_result.get("results", []))
            
            if our_count == semgrep_count:
                print(f"  ✅ PASS: Both found {our_count} findings")
                results.append((test_name, True, our_count, semgrep_count))
            else:
                print(f"  ❌ FAIL: CR={our_count}, Semgrep={semgrep_count}")
                results.append((test_name, False, our_count, semgrep_count))
                
        except Exception as e:
            print(f"  ❌ ERROR: {e}")
            results.append((test_name, False, 0, 0))
    
    return results

def run_cr_analysis(rule_file, target_file):
    """Run CR-SemService analysis"""
    cmd = ["./target/debug/astgrep", "analyze", "--config", rule_file, target_file]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
    
    if result.returncode != 0:
        raise Exception(f"CR analysis failed: {result.stderr}")
    
    # Extract JSON from output
    stdout = result.stdout
    json_start = stdout.find('{')
    json_end = stdout.rfind('}') + 1
    
    if json_start == -1 or json_end == 0:
        raise Exception("No JSON found in CR output")
    
    json_str = stdout[json_start:json_end]
    return json.loads(json_str)

def run_semgrep_analysis(rule_file, target_file):
    """Run semgrep analysis"""
    cmd = ["semgrep", "--config", rule_file, target_file, "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
    
    if result.returncode != 0:
        raise Exception(f"Semgrep analysis failed: {result.stderr}")
    
    return json.loads(result.stdout)

def print_summary(results):
    """Print test summary"""
    print("\n" + "="*60)
    print("🎯 COMPREHENSIVE TEST SUMMARY")
    print("="*60)
    
    passed = sum(1 for _, success, _, _ in results if success)
    total = len(results)
    
    print(f"Total tests: {total}")
    print(f"Passed: {passed}")
    print(f"Failed: {total - passed}")
    print(f"Success rate: {passed/total*100:.1f}%")
    
    print("\n📊 Detailed Results:")
    for test_name, success, our_count, semgrep_count in results:
        status = "✅ PASS" if success else "❌ FAIL"
        print(f"  {status}: {test_name} (CR={our_count}, Semgrep={semgrep_count})")
    
    if passed == total:
        print("\n🎉 ALL TESTS PASSED! Our tree-sitter implementation is working perfectly!")
    else:
        print(f"\n⚠️  {total - passed} tests failed. Need further improvements.")

if __name__ == "__main__":
    print("🚀 Running comprehensive tree-sitter pattern matching tests...")
    
    try:
        results = run_comprehensive_tests()
        print_summary(results)
        
    except Exception as e:
        print(f"\n❌ Test suite failed: {e}")
        exit(1)
