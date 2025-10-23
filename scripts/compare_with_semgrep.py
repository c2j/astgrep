#!/usr/bin/env python3
"""
Automated comparison script between astgrep and semgrep.
This script runs both tools on test files and compares their results.
"""

import os
import sys
import json
import subprocess
import argparse
from pathlib import Path
from typing import Dict, List, Tuple, Any
import tempfile

class ComparisonResult:
    def __init__(self, test_name: str):
        self.test_name = test_name
        self.cr_findings = []
        self.semgrep_findings = []
        self.matches = True
        self.differences = []
        
    def add_difference(self, diff: str):
        self.differences.append(diff)
        self.matches = False

def run_cr_semservice(config_path: str, target_path: str) -> Dict[str, Any]:
    """Run astgrep and return parsed JSON output."""
    try:
        cmd = ["./target/debug/astgrep", "analyze", "--config", config_path, target_path]
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        if result.returncode != 0:
            print(f"CR-SemService failed: {result.stderr}")
            return {"findings": [], "summary": {}}
            
        # Parse JSON from stdout - look for the JSON block
        output = result.stdout.strip()

        # Find JSON block between { and }
        start_idx = output.find('{\n  "findings"')
        if start_idx == -1:
            start_idx = output.find('{"findings"')

        if start_idx != -1:
            # Find the end of JSON block
            brace_count = 0
            end_idx = start_idx
            for i, char in enumerate(output[start_idx:], start_idx):
                if char == '{':
                    brace_count += 1
                elif char == '}':
                    brace_count -= 1
                    if brace_count == 0:
                        end_idx = i + 1
                        break

            json_str = output[start_idx:end_idx]
            try:
                return json.loads(json_str)
            except json.JSONDecodeError as e:
                print(f"JSON decode error: {e}")
                print(f"JSON string: {json_str[:200]}...")
                return {"findings": [], "summary": {}}
        else:
            print(f"No JSON output found in CR-SemService output")
            print(f"Output: {output[:500]}...")
            return {"findings": [], "summary": {}}
            
    except Exception as e:
        print(f"Error running CR-SemService: {e}")
        return {"findings": [], "summary": {}}

def run_semgrep(config_path: str, target_path: str) -> Dict[str, Any]:
    """Run semgrep and return parsed JSON output."""
    try:
        cmd = ["semgrep", "--config", config_path, target_path, "--json"]
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode not in [0, 1]:  # semgrep returns 1 when findings are found
            print(f"Semgrep failed: {result.stderr}")
            return {"results": []}
            
        return json.loads(result.stdout)
        
    except Exception as e:
        print(f"Error running Semgrep: {e}")
        return {"results": []}

def normalize_finding(finding: Dict[str, Any], tool: str) -> Dict[str, Any]:
    """Normalize finding format between tools."""
    if tool == "astgrep":
        return {
            "rule_id": finding.get("rule_id", ""),
            "message": finding.get("message", "").strip(),
            "severity": finding.get("severity", ""),
            "file": finding.get("location", {}).get("file", ""),
            "start_line": finding.get("location", {}).get("start_line", 0),
            "end_line": finding.get("location", {}).get("end_line", 0),
            "start_column": finding.get("location", {}).get("start_column", 0),
            "end_column": finding.get("location", {}).get("end_column", 0),
        }
    elif tool == "semgrep":
        return {
            "rule_id": finding.get("check_id", ""),
            "message": finding.get("extra", {}).get("message", "").strip(),
            "severity": finding.get("extra", {}).get("severity", ""),
            "file": finding.get("path", ""),
            "start_line": finding.get("start", {}).get("line", 0),
            "end_line": finding.get("end", {}).get("line", 0),
            "start_column": finding.get("start", {}).get("col", 0),
            "end_column": finding.get("end", {}).get("col", 0),
        }
    return {}

def compare_findings(cr_findings: List[Dict], semgrep_findings: List[Dict]) -> ComparisonResult:
    """Compare findings between the two tools."""
    result = ComparisonResult("test")
    
    # Normalize findings
    cr_normalized = [normalize_finding(f, "astgrep") for f in cr_findings]
    semgrep_normalized = [normalize_finding(f, "semgrep") for f in semgrep_findings]
    
    result.cr_findings = cr_normalized
    result.semgrep_findings = semgrep_normalized
    
    # Sort findings by line number for comparison
    cr_sorted = sorted(cr_normalized, key=lambda x: (x["file"], x["start_line"], x["start_column"]))
    semgrep_sorted = sorted(semgrep_normalized, key=lambda x: (x["file"], x["start_line"], x["start_column"]))
    
    # Compare counts
    if len(cr_sorted) != len(semgrep_sorted):
        result.add_difference(f"Finding count mismatch: CR={len(cr_sorted)}, Semgrep={len(semgrep_sorted)}")
    
    # Compare individual findings
    for i, (cr_finding, semgrep_finding) in enumerate(zip(cr_sorted, semgrep_sorted)):
        if cr_finding["start_line"] != semgrep_finding["start_line"]:
            result.add_difference(f"Finding {i+1}: Line mismatch CR={cr_finding['start_line']}, Semgrep={semgrep_finding['start_line']}")
        
        if cr_finding["rule_id"] != semgrep_finding["rule_id"]:
            result.add_difference(f"Finding {i+1}: Rule ID mismatch CR={cr_finding['rule_id']}, Semgrep={semgrep_finding['rule_id']}")
    
    return result

def find_test_pairs(test_dir: str) -> List[Tuple[str, str]]:
    """Find all test file pairs (yaml config + target file)."""
    test_pairs = []
    test_path = Path(test_dir)
    
    for yaml_file in test_path.rglob("*.yaml"):
        # Find corresponding target file
        base_name = yaml_file.stem
        parent_dir = yaml_file.parent
        
        # Look for files with same base name but different extensions
        for ext in [".java", ".py", ".c", ".cs", ".php", ".js", ".ts"]:
            target_file = parent_dir / f"{base_name}{ext}"
            if target_file.exists():
                test_pairs.append((str(yaml_file), str(target_file)))
                break
    
    return test_pairs

def main():
    parser = argparse.ArgumentParser(description="Compare astgrep with semgrep")
    parser.add_argument("--test-dir", default="tests/taint_maturity", 
                       help="Directory containing test files")
    parser.add_argument("--specific-test", help="Run specific test (config,target)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    if args.specific_test:
        config_path, target_path = args.specific_test.split(",")
        test_pairs = [(config_path, target_path)]
    else:
        test_pairs = find_test_pairs(args.test_dir)
    
    print(f"Found {len(test_pairs)} test pairs")
    
    total_tests = 0
    passed_tests = 0
    failed_tests = []
    
    for config_path, target_path in test_pairs:
        total_tests += 1
        test_name = f"{Path(config_path).parent.name}/{Path(config_path).stem}"
        
        print(f"\n{'='*60}")
        print(f"Testing: {test_name}")
        print(f"Config: {config_path}")
        print(f"Target: {target_path}")
        print(f"{'='*60}")
        
        # Run both tools
        cr_result = run_cr_semservice(config_path, target_path)
        semgrep_result = run_semgrep(config_path, target_path)
        
        # Compare results
        comparison = compare_findings(
            cr_result.get("findings", []),
            semgrep_result.get("results", [])
        )
        comparison.test_name = test_name
        
        if comparison.matches:
            print("✅ PASS: Results match!")
            passed_tests += 1
        else:
            print("❌ FAIL: Results differ!")
            failed_tests.append(comparison)
            
            for diff in comparison.differences:
                print(f"  - {diff}")
                
            if args.verbose:
                print(f"\nCR-SemService findings ({len(comparison.cr_findings)}):")
                for i, finding in enumerate(comparison.cr_findings):
                    print(f"  {i+1}. Line {finding['start_line']}: {finding['rule_id']}")
                    
                print(f"\nSemgrep findings ({len(comparison.semgrep_findings)}):")
                for i, finding in enumerate(comparison.semgrep_findings):
                    print(f"  {i+1}. Line {finding['start_line']}: {finding['rule_id']}")
    
    # Summary
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Total tests: {total_tests}")
    print(f"Passed: {passed_tests}")
    print(f"Failed: {len(failed_tests)}")
    print(f"Success rate: {passed_tests/total_tests*100:.1f}%")
    
    if failed_tests:
        print(f"\nFailed tests:")
        for test in failed_tests:
            print(f"  - {test.test_name}")
    
    return len(failed_tests) == 0

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
