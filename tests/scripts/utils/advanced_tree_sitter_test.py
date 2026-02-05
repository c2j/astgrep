#!/usr/bin/env python3
"""
Advanced tree-sitter functionality tests
"""

import subprocess
import json
import tempfile
import os

def test_complex_python_patterns():
    """Test more complex Python patterns"""
    
    python_code = '''
import os
import subprocess

def dangerous_function():
    # This should be detected
    eval("malicious_code")
    
    # This should not be detected
    evaluate_safely("safe_code")
    
    # Multiple eval calls
    eval(user_input)
    eval(f"dynamic_{variable}")

class SecurityTest:
    def __init__(self):
        self.data = "test"
    
    def process(self):
        # Another eval call
        eval(self.data)
'''
    
    rule_yaml = '''
rules:
- id: detect-eval-calls
  pattern: 'eval(...)'
  message: Dangerous eval() function call detected
  severity: ERROR
  languages: [python]
'''
    
    result = run_analysis(rule_yaml, python_code, "test.py")
    
    print(f"🔍 Found {len(result['findings'])} eval() calls:")
    for i, finding in enumerate(result["findings"]):
        line = finding['location']['start_line']
        print(f"  {i+1}. Line {line}: {finding['message']}")
    
    # Should find 4 eval calls, not the evaluate_safely call
    expected_count = 4
    if len(result["findings"]) == expected_count:
        print(f"✅ Correctly found {expected_count} eval() calls")
    else:
        print(f"❌ Expected {expected_count} eval() calls, found {len(result['findings'])}")

def test_javascript_complex_patterns():
    """Test complex JavaScript patterns"""
    
    js_code = '''
function processData(input) {
    // Dangerous eval calls
    eval("console.log('test')");
    eval(input);
    
    // Safe alternatives
    JSON.parse(input);
    evaluate(input);  // Different function
    
    // Nested eval
    if (condition) {
        eval(dynamicCode);
    }
    
    return eval("result");
}

class DataProcessor {
    process(data) {
        eval(data);
    }
}
'''
    
    rule_yaml = '''
rules:
- id: detect-js-eval
  pattern: 'eval(...)'
  message: JavaScript eval() detected
  severity: ERROR
  languages: [javascript]
'''
    
    result = run_analysis(rule_yaml, js_code, "test.js")
    
    print(f"🔍 Found {len(result['findings'])} JavaScript eval() calls:")
    for i, finding in enumerate(result["findings"]):
        line = finding['location']['start_line']
        print(f"  {i+1}. Line {line}: {finding['message']}")
    
    # Should find 5 eval calls
    expected_count = 5
    if len(result["findings"]) == expected_count:
        print(f"✅ Correctly found {expected_count} JavaScript eval() calls")
    else:
        print(f"❌ Expected {expected_count} eval() calls, found {len(result['findings'])}")

def test_mixed_language_analysis():
    """Test that language detection works correctly"""
    
    # Python file
    python_code = 'eval("python code")'
    python_result = run_analysis('''
rules:
- id: test-python-eval
  pattern: 'eval(...)'
  message: Python eval detected
  severity: ERROR
  languages: [python]
''', python_code, "test.py")
    
    # JavaScript file
    js_code = 'eval("javascript code");'
    js_result = run_analysis('''
rules:
- id: test-js-eval
  pattern: 'eval(...)'
  message: JavaScript eval detected
  severity: ERROR
  languages: [javascript]
''', js_code, "test.js")
    
    if len(python_result["findings"]) == 1 and len(js_result["findings"]) == 1:
        print("✅ Language detection and analysis works correctly")
    else:
        print(f"❌ Language detection failed: Python={len(python_result['findings'])}, JS={len(js_result['findings'])}")

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
            "./target/debug/astgrep",
            "analyze",
            "--config", rule_file,
            source_file
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        if result.returncode != 0:
            print(f"Error running analysis: {result.stderr}")
            raise Exception(f"Analysis failed: {result.stderr}")
        
        # Parse JSON output - extract JSON from mixed output
        try:
            # Find the JSON part (starts with { and ends with })
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
    print("🚀 Running advanced tree-sitter functionality tests...")
    
    try:
        test_complex_python_patterns()
        print()
        test_javascript_complex_patterns()
        print()
        test_mixed_language_analysis()
        
        print("\n🎉 All advanced tree-sitter tests completed!")
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        exit(1)
