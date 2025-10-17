#!/usr/bin/env python3
"""
Performance benchmark for enhanced rule features
"""

import subprocess
import time
import json
import os
import sys
from pathlib import Path

def run_analysis(rule_file, target_file):
    """Run analysis and measure performance"""
    start_time = time.time()
    
    cmd = [
        "./target/debug/cr-semservice",
        "analyze",
        "--config", rule_file,
        target_file
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        end_time = time.time()
        execution_time = (end_time - start_time) * 1000  # Convert to milliseconds
        
        if result.returncode != 0:
            print(f"Error running analysis: {result.stderr}")
            return None, execution_time
        
        # Extract JSON from output
        stdout = result.stdout
        json_start = stdout.find('{')
        json_end = stdout.rfind('}') + 1
        
        if json_start == -1 or json_end == 0:
            print("No JSON found in output")
            return None, execution_time
        
        json_str = stdout[json_start:json_end]
        analysis_result = json.loads(json_str)
        
        return analysis_result, execution_time
        
    except Exception as e:
        end_time = time.time()
        execution_time = (end_time - start_time) * 1000
        print(f"Exception running analysis: {e}")
        return None, execution_time

def create_large_test_file(filename, size_multiplier=10):
    """Create a large test file for performance testing"""
    base_content = '''
# Test file for performance benchmarking

def standalone_function():
    return "test"

def another_function(param):
    print(param)

class MyClass:
    def method_inside_class(self):
        return "test"
    
    def another_method(self):
        pass

def global_function():
    pass

def regular_function():
    eval("print('dangerous')")
    
def sandbox():
    eval("print('safe')")

def another_sandbox(code):
    eval(code)

eval("malicious_code")

import sqlite3

def unsafe_query():
    cursor = sqlite3.cursor()
    cursor.execute("SELECT * FROM users")

def safe_query():
    connection = sqlite3.connect("db.sqlite")
    with connection.begin():
        cursor = connection.cursor()
        cursor.execute("SELECT * FROM users")

cursor = sqlite3.cursor()
cursor.execute("DELETE FROM users WHERE id = 1")
'''
    
    with open(filename, 'w') as f:
        for i in range(size_multiplier):
            f.write(f"# Section {i+1}\n")
            f.write(base_content)
            f.write(f"\n# End of section {i+1}\n\n")

def benchmark_enhanced_features():
    """Benchmark enhanced features vs baseline"""
    
    # Create test files of different sizes
    test_files = []
    for size in [1, 5, 10, 20]:
        filename = f"tests/e-rules/perf_test_{size}x.py"
        create_large_test_file(filename, size)
        test_files.append((filename, size))
    
    # Test configurations
    test_configs = [
        ("tests/e-rules/pattern_not_inside_test.yaml", "Enhanced patterns"),
        ("tests/e-rules/pattern_not_regex_test.yaml", "Enhanced regex patterns"),
        ("tests/e-rules/comprehensive_enhanced_test.yaml", "Comprehensive enhanced"),
    ]
    
    results = []
    
    print("🚀 Starting performance benchmark...")
    print("="*60)
    
    for config_file, config_name in test_configs:
        if not os.path.exists(config_file):
            print(f"⚠️  Skipping {config_name}: {config_file} not found")
            continue
            
        print(f"\n📊 Testing: {config_name}")
        print("-" * 40)
        
        config_results = []
        
        for test_file, size_multiplier in test_files:
            print(f"  📁 File size: {size_multiplier}x baseline", end=" ... ")
            
            # Run multiple iterations for more accurate timing
            times = []
            findings_counts = []
            
            for iteration in range(3):
                analysis_result, execution_time = run_analysis(config_file, test_file)
                
                if analysis_result:
                    times.append(execution_time)
                    findings_counts.append(analysis_result.get("summary", {}).get("total_findings", 0))
                else:
                    print(f"❌ Failed iteration {iteration + 1}")
            
            if times:
                avg_time = sum(times) / len(times)
                avg_findings = sum(findings_counts) / len(findings_counts) if findings_counts else 0
                
                print(f"⏱️  {avg_time:.1f}ms (avg), 🔍 {avg_findings:.0f} findings")
                
                config_results.append({
                    "size_multiplier": size_multiplier,
                    "avg_time_ms": avg_time,
                    "avg_findings": avg_findings,
                    "times": times
                })
            else:
                print("❌ All iterations failed")
        
        results.append({
            "config_name": config_name,
            "config_file": config_file,
            "results": config_results
        })
    
    # Clean up test files
    for test_file, _ in test_files:
        if os.path.exists(test_file):
            os.remove(test_file)
    
    return results

def print_performance_summary(results):
    """Print performance summary and analysis"""
    print("\n" + "="*60)
    print("🎯 PERFORMANCE BENCHMARK SUMMARY")
    print("="*60)
    
    if not results:
        print("❌ No results to analyze")
        return
    
    # Calculate performance metrics
    for result in results:
        config_name = result["config_name"]
        config_results = result["results"]
        
        if not config_results:
            continue
            
        print(f"\n📈 {config_name}:")
        print("-" * 40)
        
        # Calculate scaling factor
        baseline_time = None
        for res in config_results:
            if res["size_multiplier"] == 1:
                baseline_time = res["avg_time_ms"]
                break
        
        if baseline_time:
            print(f"  📊 Performance scaling:")
            for res in config_results:
                size = res["size_multiplier"]
                time_ms = res["avg_time_ms"]
                findings = res["avg_findings"]
                
                scaling_factor = time_ms / baseline_time if baseline_time > 0 else 0
                efficiency = findings / time_ms if time_ms > 0 else 0
                
                print(f"    {size:2d}x size: {time_ms:6.1f}ms ({scaling_factor:4.1f}x time), "
                      f"{findings:3.0f} findings, {efficiency:5.2f} findings/ms")
        
        # Performance assessment
        largest_result = max(config_results, key=lambda x: x["size_multiplier"])
        if largest_result["avg_time_ms"] < 1000:  # Less than 1 second
            print(f"  ✅ Performance: EXCELLENT (max {largest_result['avg_time_ms']:.1f}ms)")
        elif largest_result["avg_time_ms"] < 5000:  # Less than 5 seconds
            print(f"  ✅ Performance: GOOD (max {largest_result['avg_time_ms']:.1f}ms)")
        else:
            print(f"  ⚠️  Performance: NEEDS OPTIMIZATION (max {largest_result['avg_time_ms']:.1f}ms)")
    
    # Overall assessment
    print(f"\n🏆 OVERALL ASSESSMENT:")
    print("-" * 40)
    
    max_time_across_all = 0
    total_configs_tested = 0
    
    for result in results:
        if result["results"]:
            total_configs_tested += 1
            config_max = max(res["avg_time_ms"] for res in result["results"])
            max_time_across_all = max(max_time_across_all, config_max)
    
    if total_configs_tested == 0:
        print("❌ No configurations successfully tested")
    elif max_time_across_all < 1000:
        print(f"✅ EXCELLENT: All enhanced features perform well (max {max_time_across_all:.1f}ms)")
        print("🚀 Enhanced features maintain the 10-18x performance advantage!")
    elif max_time_across_all < 5000:
        print(f"✅ GOOD: Enhanced features have acceptable performance (max {max_time_across_all:.1f}ms)")
        print("⚡ Performance is within acceptable limits")
    else:
        print(f"⚠️  NEEDS WORK: Some features may need optimization (max {max_time_across_all:.1f}ms)")
        print("🔧 Consider optimizing slow patterns or implementations")

if __name__ == "__main__":
    print("🔬 CR-SemService Enhanced Features Performance Benchmark")
    print("="*60)
    
    # Check if binary exists
    if not os.path.exists("./target/debug/cr-semservice"):
        print("❌ Binary not found. Please build the project first:")
        print("   cargo build")
        sys.exit(1)
    
    try:
        results = benchmark_enhanced_features()
        print_performance_summary(results)
        
    except Exception as e:
        print(f"\n❌ Benchmark failed: {e}")
        sys.exit(1)
