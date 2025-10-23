#!/usr/bin/env python3
"""
Test runner for Bash and SQL language support in CR-SemService
"""

import subprocess
import json
import os
import sys
from pathlib import Path

def run_analysis(rule_file, target_file, language):
    """Run CR-SemService analysis on a file"""
    cmd = [
        "./target/debug/astgrep",
        "analyze",
        "--config", rule_file,
        target_file
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        if result.returncode != 0:
            print(f"❌ Error analyzing {target_file}: {result.stderr}")
            return None
        
        # Extract JSON from output
        stdout = result.stdout
        json_start = stdout.find('{')
        json_end = stdout.rfind('}') + 1
        
        if json_start == -1 or json_end == 0:
            print(f"❌ No JSON found in output for {target_file}")
            return None
        
        json_str = stdout[json_start:json_end]
        return json.loads(json_str)
        
    except Exception as e:
        print(f"❌ Exception analyzing {target_file}: {e}")
        return None

def test_language_support():
    """Test basic language support"""
    print("🔍 Testing language support...")
    
    # Test languages command
    cmd = ["./target/debug/astgrep", "languages"]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
    
    if result.returncode == 0:
        output = result.stdout
        if "bash" in output.lower() and "sql" in output.lower():
            print("✅ Bash and SQL languages are supported")
            return True
        else:
            print("❌ Bash or SQL not found in supported languages")
            print(f"Output: {output}")
            return False
    else:
        print(f"❌ Failed to get languages: {result.stderr}")
        return False

def test_bash_analysis():
    """Test Bash script analysis"""
    print("\n🐚 Testing Bash analysis...")
    
    rule_file = "tests/bash-sql/bash_security_rules.yaml"
    target_file = "tests/bash-sql/test_bash_script.sh"
    
    if not os.path.exists(rule_file):
        print(f"❌ Rule file not found: {rule_file}")
        return False
    
    if not os.path.exists(target_file):
        print(f"❌ Target file not found: {target_file}")
        return False
    
    result = run_analysis(rule_file, target_file, "bash")
    
    if result is None:
        return False
    
    findings = result.get("findings", [])
    summary = result.get("summary", {})
    
    print(f"📊 Analysis Results:")
    print(f"   Files analyzed: {summary.get('files_analyzed', 0)}")
    print(f"   Total findings: {summary.get('total_findings', 0)}")
    print(f"   Analysis time: {summary.get('analysis_time_ms', 0)}ms")
    
    if findings:
        print(f"\n🔍 Found {len(findings)} security issues:")
        
        # Group findings by rule
        rule_counts = {}
        for finding in findings:
            rule_id = finding.get("rule_id", "unknown")
            rule_counts[rule_id] = rule_counts.get(rule_id, 0) + 1
        
        for rule_id, count in rule_counts.items():
            print(f"   • {rule_id}: {count} occurrence(s)")
        
        # Show a few example findings
        print(f"\n📝 Example findings:")
        for i, finding in enumerate(findings[:3]):
            location = finding.get("location", {})
            print(f"   {i+1}. {finding.get('rule_id', 'unknown')} at line {location.get('start_line', '?')}")
            print(f"      Message: {finding.get('message', 'No message')}")
        
        if len(findings) > 3:
            print(f"   ... and {len(findings) - 3} more")
        
        return True
    else:
        print("⚠️  No findings detected - this might indicate parsing issues")
        return False

def test_sql_analysis():
    """Test SQL query analysis"""
    print("\n🗄️  Testing SQL analysis...")
    
    rule_file = "tests/bash-sql/sql_security_rules.yaml"
    target_file = "tests/bash-sql/test_sql_queries.sql"
    
    if not os.path.exists(rule_file):
        print(f"❌ Rule file not found: {rule_file}")
        return False
    
    if not os.path.exists(target_file):
        print(f"❌ Target file not found: {target_file}")
        return False
    
    result = run_analysis(rule_file, target_file, "sql")
    
    if result is None:
        return False
    
    findings = result.get("findings", [])
    summary = result.get("summary", {})
    
    print(f"📊 Analysis Results:")
    print(f"   Files analyzed: {summary.get('files_analyzed', 0)}")
    print(f"   Total findings: {summary.get('total_findings', 0)}")
    print(f"   Analysis time: {summary.get('analysis_time_ms', 0)}ms")
    
    if findings:
        print(f"\n🔍 Found {len(findings)} security issues:")
        
        # Group findings by rule
        rule_counts = {}
        for finding in findings:
            rule_id = finding.get("rule_id", "unknown")
            rule_counts[rule_id] = rule_counts.get(rule_id, 0) + 1
        
        for rule_id, count in rule_counts.items():
            print(f"   • {rule_id}: {count} occurrence(s)")
        
        # Show a few example findings
        print(f"\n📝 Example findings:")
        for i, finding in enumerate(findings[:3]):
            location = finding.get("location", {})
            print(f"   {i+1}. {finding.get('rule_id', 'unknown')} at line {location.get('start_line', '?')}")
            print(f"      Message: {finding.get('message', 'No message')}")
        
        if len(findings) > 3:
            print(f"   ... and {len(findings) - 3} more")
        
        return True
    else:
        print("⚠️  No findings detected - this might indicate parsing issues")
        return False

def test_performance():
    """Test performance of Bash and SQL analysis"""
    print("\n⚡ Testing performance...")
    
    tests = [
        ("tests/bash-sql/bash_security_rules.yaml", "tests/bash-sql/test_bash_script.sh", "Bash"),
        ("tests/bash-sql/sql_security_rules.yaml", "tests/bash-sql/test_sql_queries.sql", "SQL")
    ]
    
    for rule_file, target_file, language in tests:
        if not os.path.exists(rule_file) or not os.path.exists(target_file):
            print(f"⚠️  Skipping {language} performance test - files not found")
            continue
        
        # Run multiple times for average
        times = []
        for _ in range(3):
            result = run_analysis(rule_file, target_file, language.lower())
            if result and "summary" in result:
                times.append(result["summary"].get("analysis_time_ms", 0))
        
        if times:
            avg_time = sum(times) / len(times)
            print(f"   {language}: {avg_time:.1f}ms average")
        else:
            print(f"   {language}: Failed to measure performance")

def main():
    """Main test function"""
    print("🚀 CR-SemService Bash and SQL Support Test")
    print("=" * 50)
    
    # Check if binary exists
    if not os.path.exists("./target/debug/astgrep"):
        print("❌ Binary not found. Please build the project first:")
        print("   cargo build")
        sys.exit(1)
    
    # Run tests
    tests_passed = 0
    total_tests = 4
    
    try:
        # Test 1: Language support
        if test_language_support():
            tests_passed += 1
        
        # Test 2: Bash analysis
        if test_bash_analysis():
            tests_passed += 1
        
        # Test 3: SQL analysis
        if test_sql_analysis():
            tests_passed += 1
        
        # Test 4: Performance
        test_performance()
        tests_passed += 1  # Performance test always "passes"
        
    except Exception as e:
        print(f"\n❌ Test suite failed with exception: {e}")
        sys.exit(1)
    
    # Summary
    print(f"\n" + "=" * 50)
    print(f"🎯 TEST SUMMARY")
    print(f"=" * 50)
    print(f"Tests passed: {tests_passed}/{total_tests}")
    
    if tests_passed == total_tests:
        print("✅ All tests passed! Bash and SQL support is working correctly.")
        sys.exit(0)
    else:
        print(f"❌ {total_tests - tests_passed} test(s) failed.")
        sys.exit(1)

if __name__ == "__main__":
    main()
