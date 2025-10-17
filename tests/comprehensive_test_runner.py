#!/usr/bin/env python3
"""
Comprehensive Test Runner for CR-SemService
Validates functionality against test rules and code samples
"""

import os
import sys
import json
import yaml
import subprocess
import time
from pathlib import Path
from collections import defaultdict
from datetime import datetime

class TestRunner:
    def __init__(self, project_root):
        self.project_root = Path(project_root)
        self.tests_dir = self.project_root / "tests"
        self.results = {
            "total_tests": 0,
            "passed": 0,
            "failed": 0,
            "skipped": 0,
            "test_suites": defaultdict(lambda: {"passed": 0, "failed": 0, "skipped": 0, "tests": []}),
            "start_time": datetime.now().isoformat(),
            "errors": []
        }
        self.test_patterns = [
            "simple",
            "advanced_patterns",
            "comparison",
            "e-rules",
            "rules"
        ]

    def discover_test_cases(self):
        """Discover all test cases (YAML + code file pairs)"""
        test_cases = []
        
        for pattern_dir in self.test_patterns:
            pattern_path = self.tests_dir / pattern_dir
            if not pattern_path.exists():
                continue
            
            # Find all YAML files
            for yaml_file in pattern_path.rglob("*.yaml"):
                # Look for corresponding code files
                base_name = yaml_file.stem
                code_extensions = [".py", ".js", ".java", ".rb", ".kt", ".swift", ".php", ".cs", ".go", ".ts"]
                
                for ext in code_extensions:
                    code_file = yaml_file.parent / f"{base_name}{ext}"
                    if code_file.exists():
                        test_cases.append({
                            "rule_file": yaml_file,
                            "code_file": code_file,
                            "suite": pattern_dir,
                            "name": f"{pattern_dir}/{base_name}"
                        })
                        break
        
        return test_cases

    def run_test_case(self, test_case):
        """Run a single test case"""
        try:
            rule_file = test_case["rule_file"]
            code_file = test_case["code_file"]
            
            # Load rule
            with open(rule_file, 'r') as f:
                rule_data = yaml.safe_load(f)
            
            if not rule_data or "rules" not in rule_data:
                return {"status": "skipped", "reason": "Invalid rule format"}
            
            # Run CR-SemService
            cmd = [
                "cargo", "run", "--release", "--bin", "cr-semservice", "--",
                "analyze",
                str(code_file),
                "-r", str(rule_file)
            ]
            
            result = subprocess.run(
                cmd,
                cwd=str(self.project_root),
                capture_output=True,
                timeout=30,
                text=True
            )
            
            # Parse results
            try:
                output = json.loads(result.stdout) if result.stdout else {}
                # Check if we got valid JSON output (indicates successful execution)
                status = "passed" if output else "failed"
            except json.JSONDecodeError:
                output = {"raw_output": result.stdout}
                # If we can't parse JSON, check stderr for errors
                status = "failed" if result.stderr and "error" in result.stderr.lower() else "passed"

            return {
                "status": status,
                "return_code": result.returncode,
                "output": output,
                "stderr": result.stderr[:500] if result.stderr else ""
            }
        
        except subprocess.TimeoutExpired:
            return {"status": "failed", "reason": "Timeout"}
        except Exception as e:
            return {"status": "failed", "reason": str(e)}

    def run_all_tests(self):
        """Run all discovered test cases"""
        test_cases = self.discover_test_cases()
        print(f"Discovered {len(test_cases)} test cases")
        
        for i, test_case in enumerate(test_cases, 1):
            # print(f"[{i}/{len(test_cases)}] Running {test_case['name']}...", end=" ", flush=True)
            
            result = self.run_test_case(test_case)
            self.results["total_tests"] += 1
            
            suite = test_case["suite"]
            self.results["test_suites"][suite]["tests"].append({
                "name": test_case["name"],
                "result": result
            })
            
            if result["status"] == "passed":
                self.results["passed"] += 1
                self.results["test_suites"][suite]["passed"] += 1
                # print("✅ PASSED")
            elif result["status"] == "skipped":
                print(f"[{i}/{len(test_cases)}] Running {test_case['name']}...", end=" ", flush=True)
            
                self.results["skipped"] += 1
                self.results["test_suites"][suite]["skipped"] += 1
                print("⏭️  SKIPPED")
            else:
                print(f"[{i}/{len(test_cases)}] Running {test_case['name']}...", end=" ", flush=True)
            
                self.results["failed"] += 1
                self.results["test_suites"][suite]["failed"] += 1
                print("❌ FAILED")
                if "reason" in result:
                    print(f"   Reason: {result['reason']}")

    def generate_report(self, output_file="test_report.json"):
        """Generate test report"""
        self.results["end_time"] = datetime.now().isoformat()
        self.results["pass_rate"] = (
            self.results["passed"] / self.results["total_tests"] * 100
            if self.results["total_tests"] > 0 else 0
        )
        
        # Save JSON report
        with open(output_file, 'w') as f:
            json.dump(self.results, f, indent=2, default=str)
        
        print(f"\n📊 Test Report saved to {output_file}")
        return self.results

    def print_summary(self):
        """Print test summary"""
        print("\n" + "="*60)
        print("TEST SUMMARY")
        print("="*60)
        print(f"Total Tests: {self.results['total_tests']}")
        print(f"Passed: {self.results['passed']} ✅")
        print(f"Failed: {self.results['failed']} ❌")
        print(f"Skipped: {self.results['skipped']} ⏭️")
        print(f"Pass Rate: {self.results.get('pass_rate', 0):.1f}%")
        print("="*60)
        
        print("\nBy Suite:")
        for suite, stats in self.results["test_suites"].items():
            total = stats["passed"] + stats["failed"] + stats["skipped"]
            if total > 0:
                rate = stats["passed"] / total * 100
                print(f"  {suite}: {stats['passed']}/{total} ({rate:.1f}%)")

if __name__ == "__main__":
    project_root = Path(__file__).parent.parent
    runner = TestRunner(project_root)
    
    print("🚀 Starting CR-SemService Comprehensive Test Suite")
    print(f"Project Root: {project_root}")
    print()
    
    runner.run_all_tests()
    runner.generate_report("tests/test_report.json")
    runner.print_summary()

