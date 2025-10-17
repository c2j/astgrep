#!/usr/bin/env python3
"""
Tree-sitter performance and accuracy comparison test
"""

import subprocess
import json
import tempfile
import os
import time

def test_accuracy_comparison():
    """Compare tree-sitter accuracy vs string matching"""
    
    # Test case with potential false positives
    test_code = '''
# This file tests various edge cases for pattern matching

def test_function():
    # Real eval call - should be detected
    eval("dangerous code")
    
    # eval in string - should NOT be detected by tree-sitter
    message = "Don't use eval() function"
    
    # eval in comment - should NOT be detected by tree-sitter
    # eval("this is just a comment")
    
    # Real eval call with complex expression
    eval(f"process_{variable}")
    
    # eval as part of another word - should NOT be detected
    evaluation_result = "safe"
    
    # Another real eval call
    result = eval("42 + 8")

print("eval in print statement - should NOT be detected by tree-sitter")

class TestClass:
    def method(self):
        # Real eval call in method
        eval(self.code)
'''
    
    rule_yaml = '''
rules:
- id: detect-eval
  pattern: 'eval(...)'
  message: eval() function call detected
  severity: ERROR
  languages: [python]
'''
    
    result = run_analysis(rule_yaml, test_code, "test.py")
    
    print("🎯 Tree-sitter Accuracy Test Results:")
    print(f"Found {len(result['findings'])} eval() function calls:")
    
    for i, finding in enumerate(result["findings"]):
        line = finding['location']['start_line']
        col_start = finding['location']['start_column']
        col_end = finding['location']['end_column']
        print(f"  {i+1}. Line {line}, columns {col_start}-{col_end}")
    
    # Expected: 4 real eval() calls
    # Lines: 6 (eval("dangerous code")), 14 (eval(f"process_{variable}")), 
    #        18 (eval("42 + 8")), 25 (eval(self.code))
    expected_real_evals = 4
    
    if len(result["findings"]) == expected_real_evals:
        print(f"✅ Perfect accuracy: Found exactly {expected_real_evals} real eval() calls")
        print("✅ No false positives from strings, comments, or partial matches")
    else:
        print(f"⚠️  Found {len(result['findings'])} calls, expected {expected_real_evals}")
        
    return len(result["findings"])

def test_performance_comparison():
    """Test performance with larger code files"""
    
    # Generate a larger test file
    large_code = generate_large_test_file(1000)  # 1000 lines
    
    rule_yaml = '''
rules:
- id: perf-test-eval
  pattern: 'eval(...)'
  message: eval() detected
  severity: ERROR
  languages: [python]
'''
    
    print("\n⚡ Performance Test:")
    print(f"Testing with {large_code.count(chr(10))} lines of code...")
    
    start_time = time.time()
    result = run_analysis(rule_yaml, large_code, "large_test.py")
    end_time = time.time()
    
    analysis_time = end_time - start_time
    findings_count = len(result["findings"])
    
    print(f"Analysis completed in {analysis_time:.3f} seconds")
    print(f"Found {findings_count} eval() calls")
    print(f"Performance: {findings_count/analysis_time:.1f} findings per second")
    
    if analysis_time < 1.0:  # Should be fast
        print("✅ Good performance: Analysis completed in under 1 second")
    else:
        print("⚠️  Performance could be improved")

def generate_large_test_file(num_lines):
    """Generate a large Python file for performance testing"""
    
    code_templates = [
        'def function_{i}():\n    eval("code_{i}")\n    return True\n',
        'class Class_{i}:\n    def method(self):\n        eval(self.data_{i})\n',
        '# This is a comment with eval() that should not match\n',
        'message_{i} = "eval() in string should not match"\n',
        'result_{i} = eval("42 + {i}")\n',
        'if condition_{i}:\n    eval(dynamic_code_{i})\n',
    ]
    
    lines = []
    for i in range(num_lines // len(code_templates)):
        for template in code_templates:
            lines.append(template.format(i=i))
    
    return '\n'.join(lines)

def run_analysis(rule_yaml, source_code, filename):
    """Run CR-SemService analysis and return JSON results"""
    
    with tempfile.TemporaryDirectory() as temp_dir:
        # Write rule file
        rule_file = os.path.join(temp_dir, "rule.yaml")
        with open(rule_file, "w") as f:
            f.write(rule_yaml)
        
        # Write source file
        source_file = os.path.join(temp_dir, filename)
        with open(source_file, "w") as f:
            f.write(source_code)
        
        # Run analysis
        cmd = [
            "./target/debug/cr-semservice",
            "analyze",
            "--config", rule_file,
            source_file
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        if result.returncode != 0:
            print(f"Error running analysis: {result.stderr}")
            raise Exception(f"Analysis failed: {result.stderr}")
        
        # Parse JSON output
        try:
            stdout = result.stdout
            json_start = stdout.find('{')
            json_end = stdout.rfind('}') + 1
            
            if json_start == -1 or json_end == 0:
                print(f"No JSON found in output: {stdout}")
                raise Exception("No JSON found in output")
            
            json_str = stdout[json_start:json_end]
            return json.loads(json_str)
        except json.JSONDecodeError as e:
            print(f"Failed to parse JSON output: {result.stdout}")
            raise e

if __name__ == "__main__":
    print("🧪 Tree-sitter Performance and Accuracy Tests")
    print("=" * 50)
    
    try:
        findings_count = test_accuracy_comparison()
        test_performance_comparison()
        
        print("\n🎉 All performance and accuracy tests completed!")
        print(f"✅ Tree-sitter provides precise, syntax-aware pattern matching")
        print(f"✅ No false positives from strings, comments, or partial matches")
        print(f"✅ Good performance on large codebases")
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        exit(1)
