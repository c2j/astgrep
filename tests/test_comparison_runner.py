#!/usr/bin/env python3
"""
Comprehensive test runner to compare astgrep with semgrep.
This script discovers test rule files and their corresponding code files,
runs both tools, and compares their results.
"""

import os
import sys
import json
import subprocess
import argparse
from pathlib import Path
from typing import Dict, List, Tuple, Any, Set
import tempfile
import time

class TestResult:
    def __init__(self, test_name: str, rule_file: str, code_file: str):
        self.test_name = test_name
        self.rule_file = rule_file
        self.code_file = code_file
        self.cr_findings = []
        self.semgrep_findings = []
        self.matches = True
        self.differences = []
        self.cr_error = None
        self.semgrep_error = None
        self.language = self._detect_language()
        
    def _detect_language(self) -> str:
        """Detect language from file extension."""
        ext = Path(self.code_file).suffix.lower()
        lang_map = {
            '.java': 'java',
            '.js': 'javascript', 
            '.jsx': 'javascript',
            '.ts': 'javascript',
            '.tsx': 'javascript',
            '.py': 'python',
            '.pyw': 'python',
            '.php': 'php',
            '.phtml': 'php',
            '.sql': 'sql',
            '.sh': 'bash',
            '.bash': 'bash',
            '.c': 'c',
            '.cpp': 'cpp',
            '.cs': 'csharp',
            '.go': 'go',
            '.rb': 'ruby',
            '.rs': 'rust',
            '.scala': 'scala',
            '.kt': 'kotlin',
            '.swift': 'swift',
            '.html': 'html',
            '.xml': 'xml',
            '.yaml': 'yaml',
            '.yml': 'yaml',
            '.json': 'json',
            '.tf': 'terraform',
            '.dockerfile': 'dockerfile'
        }
        return lang_map.get(ext, 'unknown')
        
    def add_difference(self, diff: str):
        self.differences.append(diff)
        self.matches = False

def run_cr_semservice(rule_file: str, code_file: str) -> Tuple[Dict[str, Any], str]:
    """Run astgrep and return parsed JSON output and any error."""
    try:
        cmd = ["./target/debug/astgrep", "analyze", "--config", rule_file, code_file, "--format", "json"]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode != 0:
            error_msg = f"CR-SemService failed (exit code {result.returncode}): {result.stderr}"
            return {"findings": [], "summary": {}}, error_msg
            
        # Parse JSON from stdout
        output = result.stdout.strip()
        if not output:
            return {"findings": [], "summary": {}}, None

        # Try to find and parse JSON
        try:
            # Look for JSON block
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
                return json.loads(json_str), None
            else:
                # Try parsing entire output as JSON
                return json.loads(output), None
                
        except json.JSONDecodeError as e:
            return {"findings": [], "summary": {}}, f"JSON decode error: {e}"
            
    except subprocess.TimeoutExpired:
        return {"findings": [], "summary": {}}, "CR-SemService timeout"
    except Exception as e:
        return {"findings": [], "summary": {}}, f"Error running CR-SemService: {e}"

def run_semgrep(rule_file: str, code_file: str) -> Tuple[Dict[str, Any], str]:
    """Run semgrep and return parsed JSON output and any error."""
    try:
        cmd = ["semgrep", "--config", rule_file, code_file, "--json", "--no-git-ignore"]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        # semgrep returns 1 when findings are found, 0 when no findings, >1 for errors
        if result.returncode > 1:
            error_msg = f"Semgrep failed (exit code {result.returncode}): {result.stderr}"
            return {"results": []}, error_msg
            
        if not result.stdout.strip():
            return {"results": []}, None
            
        return json.loads(result.stdout), None
        
    except subprocess.TimeoutExpired:
        return {"results": []}, "Semgrep timeout"
    except json.JSONDecodeError as e:
        return {"results": []}, f"Semgrep JSON decode error: {e}"
    except Exception as e:
        return {"results": []}, f"Error running Semgrep: {e}"

def normalize_finding(finding: Dict[str, Any], tool: str) -> Dict[str, Any]:
    """Normalize finding format between tools."""
    if tool == "astgrep":
        location = finding.get("location", {})
        return {
            "rule_id": finding.get("rule_id", ""),
            "message": finding.get("message", "").strip(),
            "severity": finding.get("severity", ""),
            "file": location.get("file", ""),
            "start_line": location.get("start_line", 0),
            "end_line": location.get("end_line", 0),
            "start_column": location.get("start_column", 0),
            "end_column": location.get("end_column", 0),
        }
    elif tool == "semgrep":
        extra = finding.get("extra", {})
        start = finding.get("start", {})
        end = finding.get("end", {})
        return {
            "rule_id": finding.get("check_id", ""),
            "message": extra.get("message", "").strip(),
            "severity": extra.get("severity", ""),
            "file": finding.get("path", ""),
            "start_line": start.get("line", 0),
            "end_line": end.get("line", 0),
            "start_column": start.get("col", 0),
            "end_column": end.get("col", 0),
        }
    return {}

def compare_findings(test_result: TestResult) -> None:
    """Compare findings between the two tools and update test_result."""
    # Normalize findings
    cr_normalized = [normalize_finding(f, "astgrep") for f in test_result.cr_findings]
    semgrep_normalized = [normalize_finding(f, "semgrep") for f in test_result.semgrep_findings]
    
    # Sort findings by line number for comparison
    cr_sorted = sorted(cr_normalized, key=lambda x: (x["file"], x["start_line"], x["start_column"]))
    semgrep_sorted = sorted(semgrep_normalized, key=lambda x: (x["file"], x["start_line"], x["start_column"]))
    
    # Compare counts
    if len(cr_sorted) != len(semgrep_sorted):
        test_result.add_difference(f"Finding count mismatch: CR={len(cr_sorted)}, Semgrep={len(semgrep_sorted)}")
    
    # Compare individual findings (up to the minimum count)
    min_count = min(len(cr_sorted), len(semgrep_sorted))
    for i in range(min_count):
        cr_finding = cr_sorted[i]
        semgrep_finding = semgrep_sorted[i]
        
        if cr_finding["start_line"] != semgrep_finding["start_line"]:
            test_result.add_difference(f"Finding {i+1}: Line mismatch CR={cr_finding['start_line']}, Semgrep={semgrep_finding['start_line']}")
        
        if cr_finding["rule_id"] != semgrep_finding["rule_id"]:
            test_result.add_difference(f"Finding {i+1}: Rule ID mismatch CR='{cr_finding['rule_id']}', Semgrep='{semgrep_finding['rule_id']}'")
    
    # Store normalized findings for later analysis
    test_result.cr_findings = cr_normalized
    test_result.semgrep_findings = semgrep_normalized

def find_test_pairs(test_dirs: List[str], supported_languages: Set[str]) -> List[Tuple[str, str]]:
    """Find all test file pairs (yaml config + target file) for supported languages."""
    test_pairs = []
    
    # Language extension mapping
    lang_extensions = {
        'java': ['.java'],
        'javascript': ['.js', '.jsx', '.ts', '.tsx'],
        'python': ['.py', '.pyw'],
        'php': ['.php', '.phtml'],
        'sql': ['.sql'],
        'bash': ['.sh', '.bash']
    }
    
    # Get all extensions for supported languages
    supported_extensions = set()
    for lang in supported_languages:
        if lang in lang_extensions:
            supported_extensions.update(lang_extensions[lang])
    
    for test_dir in test_dirs:
        test_path = Path(test_dir)
        if not test_path.exists():
            continue
            
        for yaml_file in test_path.rglob("*.yaml"):
            # Find corresponding target file
            base_name = yaml_file.stem
            parent_dir = yaml_file.parent
            
            # Look for files with same base name but different extensions
            for ext in supported_extensions:
                target_file = parent_dir / f"{base_name}{ext}"
                if target_file.exists():
                    test_pairs.append((str(yaml_file), str(target_file)))
                    break
    
    return test_pairs

def main():
    parser = argparse.ArgumentParser(description="Compare astgrep with semgrep on test files")
    parser.add_argument("--test-dirs", nargs="+", 
                       default=["tests/simple", "tests/rules", "tests/comparison"],
                       help="Directories containing test files")
    parser.add_argument("--languages", nargs="+",
                       default=["java", "javascript", "python", "php", "sql", "bash"],
                       help="Languages to test")
    parser.add_argument("--specific-test", help="Run specific test (rule_file,code_file)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument("--output", "-o", help="Output results to JSON file")
    parser.add_argument("--max-tests", type=int, help="Maximum number of tests to run")
    
    args = parser.parse_args()
    
    supported_languages = set(args.languages)
    
    if args.specific_test:
        rule_file, code_file = args.specific_test.split(",")
        test_pairs = [(rule_file, code_file)]
    else:
        test_pairs = find_test_pairs(args.test_dirs, supported_languages)
    
    if args.max_tests:
        test_pairs = test_pairs[:args.max_tests]
    
    print(f"Found {len(test_pairs)} test pairs for languages: {', '.join(sorted(supported_languages))}")
    
    results = []
    total_tests = 0
    passed_tests = 0
    failed_tests = []
    language_stats = {}
    
    for rule_file, code_file in test_pairs:
        total_tests += 1
        test_name = f"{Path(rule_file).parent.name}/{Path(rule_file).stem}"
        
        test_result = TestResult(test_name, rule_file, code_file)
        
        # Skip unsupported languages
        if test_result.language not in supported_languages and test_result.language != 'unknown':
            continue
            
        # Update language stats
        if test_result.language not in language_stats:
            language_stats[test_result.language] = {'total': 0, 'passed': 0, 'failed': 0}
        language_stats[test_result.language]['total'] += 1
        
        print(f"\n{'='*80}")
        print(f"Testing: {test_name} ({test_result.language})")
        print(f"Rule: {rule_file}")
        print(f"Code: {code_file}")
        print(f"{'='*80}")
        
        # Run both tools
        cr_result, cr_error = run_cr_semservice(rule_file, code_file)
        semgrep_result, semgrep_error = run_semgrep(rule_file, code_file)
        
        test_result.cr_findings = cr_result.get("findings", [])
        test_result.semgrep_findings = semgrep_result.get("results", [])
        test_result.cr_error = cr_error
        test_result.semgrep_error = semgrep_error
        
        # Compare results
        compare_findings(test_result)
        
        # Check for errors
        if cr_error or semgrep_error:
            test_result.add_difference(f"Tool errors - CR: {cr_error}, Semgrep: {semgrep_error}")
        
        if test_result.matches:
            print("✅ PASS: Results match!")
            passed_tests += 1
            language_stats[test_result.language]['passed'] += 1
        else:
            print("❌ FAIL: Results differ!")
            failed_tests.append(test_result)
            language_stats[test_result.language]['failed'] += 1
            
            for diff in test_result.differences:
                print(f"  - {diff}")
                
            if args.verbose:
                print(f"\nCR-SemService findings ({len(test_result.cr_findings)}):")
                for i, finding in enumerate(test_result.cr_findings):
                    print(f"  {i+1}. Line {finding['start_line']}: {finding['rule_id']}")
                    
                print(f"\nSemgrep findings ({len(test_result.semgrep_findings)}):")
                for i, finding in enumerate(test_result.semgrep_findings):
                    print(f"  {i+1}. Line {finding['start_line']}: {finding['rule_id']}")
        
        results.append(test_result)
    
    # Summary
    print(f"\n{'='*80}")
    print(f"SUMMARY")
    print(f"{'='*80}")
    print(f"Total tests: {total_tests}")
    print(f"Passed: {passed_tests}")
    print(f"Failed: {len(failed_tests)}")
    print(f"Success rate: {passed_tests/total_tests*100:.1f}%" if total_tests > 0 else "No tests run")
    
    # Language breakdown
    print(f"\nLanguage breakdown:")
    for lang, stats in sorted(language_stats.items()):
        success_rate = stats['passed'] / stats['total'] * 100 if stats['total'] > 0 else 0
        print(f"  {lang}: {stats['passed']}/{stats['total']} ({success_rate:.1f}%)")
    
    if failed_tests:
        print(f"\nFailed tests:")
        for test in failed_tests:
            print(f"  - {test.test_name} ({test.language})")
    
    # Save results to JSON if requested
    if args.output:
        output_data = {
            'summary': {
                'total_tests': total_tests,
                'passed_tests': passed_tests,
                'failed_tests': len(failed_tests),
                'success_rate': passed_tests/total_tests*100 if total_tests > 0 else 0,
                'language_stats': language_stats
            },
            'results': [
                {
                    'test_name': r.test_name,
                    'rule_file': r.rule_file,
                    'code_file': r.code_file,
                    'language': r.language,
                    'matches': r.matches,
                    'differences': r.differences,
                    'cr_findings_count': len(r.cr_findings),
                    'semgrep_findings_count': len(r.semgrep_findings),
                    'cr_error': r.cr_error,
                    'semgrep_error': r.semgrep_error
                }
                for r in results
            ]
        }
        
        with open(args.output, 'w') as f:
            json.dump(output_data, f, indent=2)
        print(f"\nResults saved to {args.output}")
    
    return len(failed_tests) == 0

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
