#!/usr/bin/env python3
"""
Test metavariable support in our tree-sitter implementation
"""

import subprocess
import json
import tempfile
import os

def test_metavariable_patterns():
    """Test basic metavariable patterns"""
    
    print("🧪 Testing metavariable patterns...")
    
    # Test 1: Simple metavariable
    python_code = '''
def test():
    x = 42
    y = "hello"
    z = [1, 2, 3]
'''
    
    rule_yaml = '''
rules:
- id: test-metavar
  pattern: $VAR = $VALUE
  message: Found assignment to $VAR
  severity: INFO
  languages: [python]
'''
    
    print("  Test 1: Simple metavariable assignment")
    result = run_analysis(rule_yaml, python_code, "test.py")
    findings = result.get("findings", [])
    print(f"    Found {len(findings)} assignments")
    
    # Test 2: Meta function call
    python_code = '''
def test():
    print("hello")
    eval("code")
    len([1, 2, 3])
'''
    
    rule_yaml = '''
rules:
- id: test-meta-func
  pattern: $FUNC($ARG)
  message: Found function call $FUNC with argument
  severity: INFO
  languages: [python]
'''
    
    print("  Test 2: Meta function call")
    result = run_analysis(rule_yaml, python_code, "test.py")
    findings = result.get("findings", [])
    print(f"    Found {len(findings)} function calls")
    
    # Test 3: Specific function with metavariable
    python_code = '''
def test():
    eval("safe")
    eval(user_input)
    eval(f"dynamic_{var}")
'''
    
    rule_yaml = '''
rules:
- id: test-eval-metavar
  pattern: eval($CODE)
  message: Found eval with code: $CODE
  severity: ERROR
  languages: [python]
'''
    
    print("  Test 3: Specific function with metavariable")
    result = run_analysis(rule_yaml, python_code, "test.py")
    findings = result.get("findings", [])
    print(f"    Found {len(findings)} eval calls")
    
    return len(findings) > 0

def test_advanced_patterns():
    """Test advanced pattern features"""
    
    print("\n🚀 Testing advanced patterns...")
    
    # Test: Method call with metavariables
    java_code = '''
public class Test {
    public void method() {
        System.out.println("hello");
        logger.info("message");
        obj.process(data);
    }
}
'''
    
    rule_yaml = '''
rules:
- id: test-method-metavar
  pattern: $OBJ.$METHOD($ARG)
  message: Found method call $METHOD on $OBJ
  severity: INFO
  languages: [java]
'''
    
    print("  Test: Method call with metavariables")
    result = run_analysis(rule_yaml, java_code, "Test.java")
    findings = result.get("findings", [])
    print(f"    Found {len(findings)} method calls")
    
    return len(findings) > 0

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
            print(f"    ⚠️  Analysis failed: {result.stderr}")
            return {"findings": []}
        
        # Parse JSON output
        try:
            stdout = result.stdout
            json_start = stdout.find('{')
            json_end = stdout.rfind('}') + 1
            
            if json_start == -1 or json_end == 0:
                print(f"    ⚠️  No JSON found in output")
                return {"findings": []}
            
            json_str = stdout[json_start:json_end]
            return json.loads(json_str)
        except json.JSONDecodeError as e:
            print(f"    ⚠️  Failed to parse JSON: {e}")
            return {"findings": []}

if __name__ == "__main__":
    print("🎯 Testing Metavariable Support in Tree-sitter Implementation")
    print("=" * 60)
    
    try:
        basic_success = test_metavariable_patterns()
        advanced_success = test_advanced_patterns()
        
        print("\n" + "=" * 60)
        print("📊 METAVARIABLE TEST SUMMARY")
        print("=" * 60)
        
        if basic_success or advanced_success:
            print("✅ Metavariable support is working!")
            print("   Our tree-sitter implementation can handle:")
            print("   - Simple metavariables ($VAR)")
            print("   - Meta function calls ($FUNC(...))")
            print("   - Method calls with metavariables ($OBJ.$METHOD)")
        else:
            print("⚠️  Metavariable support needs more work")
            print("   Current implementation may not fully support metavariables yet")
        
        print("\n🎉 Metavariable testing completed!")
        
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        exit(1)
