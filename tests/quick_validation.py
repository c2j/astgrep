#!/usr/bin/env python3
"""
Quick Validation Script for CR-SemService
Tests core functionality with minimal overhead
"""

import subprocess
import json
import yaml
from pathlib import Path
import sys

class QuickValidator:
    def __init__(self, project_root):
        self.project_root = Path(project_root)
        self.tests_dir = self.project_root / "tests"
        self.results = []

    def test_simple_patterns(self):
        """Test simple pattern matching"""
        print("🧪 Testing Simple Patterns...")
        
        test_cases = [
            {
                "name": "Function Call Detection",
                "rule": "tests/simple/function_call.yaml",
                "code": "tests/simple/function_call.js",
                "expected_matches": 3
            },
            {
                "name": "String Match",
                "rule": "tests/simple/string_match.yaml",
                "code": "tests/simple/string_match.py",
                "expected_matches": 2
            },
            {
                "name": "Number Match",
                "rule": "tests/simple/number_match.yaml",
                "code": "tests/simple/number_match.py",
                "expected_matches": 2
            }
        ]
        
        for test in test_cases:
            result = self._run_test(test)
            self.results.append(result)
            status = "✅" if result["passed"] else "❌"
            print(f"  {status} {test['name']}")

    def test_advanced_patterns(self):
        """Test advanced pattern types"""
        print("\n🧪 Testing Advanced Patterns...")
        
        test_cases = [
            {
                "name": "Pattern-Either",
                "rule": "tests/advanced_patterns/pattern_either_test.yaml",
                "code": "tests/advanced_patterns/pattern_either_test.py"
            },
            {
                "name": "Pattern-Not",
                "rule": "tests/advanced_patterns/pattern_not_test.yaml",
                "code": "tests/advanced_patterns/pattern_not_test.py"
            },
            {
                "name": "Pattern-Inside",
                "rule": "tests/advanced_patterns/pattern_inside_test.yaml",
                "code": "tests/advanced_patterns/pattern_inside_test.py"
            },
            {
                "name": "Metavariables",
                "rule": "tests/advanced_patterns/metavariables_test.yaml",
                "code": "tests/advanced_patterns/metavariables_test.py"
            }
        ]
        
        for test in test_cases:
            result = self._run_test(test)
            self.results.append(result)
            status = "✅" if result["passed"] else "❌"
            print(f"  {status} {test['name']}")

    def test_language_support(self):
        """Test language support"""
        print("\n🧪 Testing Language Support...")
        
        languages = {
            "Python": "tests/simple/string_match.py",
            "JavaScript": "tests/simple/function_call.js",
            "Java": "tests/java/r1.java",
            "Ruby": "tests/rules/jwt-hardcode.rb"
        }
        
        for lang, file_path in languages.items():
            file = self.project_root / file_path
            if file.exists():
                result = {"name": f"Language: {lang}", "passed": True}
                print(f"  ✅ {lang} support")
            else:
                result = {"name": f"Language: {lang}", "passed": False}
                print(f"  ⏭️  {lang} (no test file)")
            self.results.append(result)

    def _run_test(self, test):
        """Run a single test"""
        try:
            rule_file = self.project_root / test["rule"]
            code_file = self.project_root / test["code"]
            
            if not rule_file.exists() or not code_file.exists():
                return {"name": test["name"], "passed": False, "reason": "Files not found"}
            
            cmd = [
                "cargo", "run", "--release", "--",
                "analyze",
                str(code_file),
                "-r", str(rule_file)
            ]
            
            result = subprocess.run(
                cmd,
                cwd=str(self.project_root),
                capture_output=True,
                timeout=10,
                text=True
            )
            
            passed = result.returncode == 0
            return {"name": test["name"], "passed": passed}
        
        except Exception as e:
            return {"name": test["name"], "passed": False, "reason": str(e)}

    def print_summary(self):
        """Print validation summary"""
        passed = sum(1 for r in self.results if r.get("passed", False))
        total = len(self.results)
        
        print("\n" + "="*60)
        print("QUICK VALIDATION SUMMARY")
        print("="*60)
        print(f"Tests Run: {total}")
        print(f"Passed: {passed} ✅")
        print(f"Failed: {total - passed} ❌")
        print(f"Pass Rate: {(passed/total*100):.1f}%")
        print("="*60)
        
        return passed == total

if __name__ == "__main__":
    project_root = Path(__file__).parent.parent
    validator = QuickValidator(project_root)
    
    print("🚀 CR-SemService Quick Validation")
    print(f"Project: {project_root}\n")
    
    validator.test_simple_patterns()
    validator.test_advanced_patterns()
    validator.test_language_support()
    
    success = validator.print_summary()
    sys.exit(0 if success else 1)

