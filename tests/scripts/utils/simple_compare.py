#!/usr/bin/env python3

import subprocess
import re

def run_semgrep():
    """Run semgrep and extract findings by line"""
    cmd = ["semgrep", "--config", "tests/bash-sql/bash_security_rules.yaml", 
           "tests/bash-sql/test_bash_script.sh", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Semgrep failed: {result.stderr}")
        return {}
    
    import json
    data = json.loads(result.stdout)
    findings = data.get('results', [])
    
    by_rule = {}
    for finding in findings:
        rule_id = finding.get('check_id', 'unknown')
        line = finding.get('start', {}).get('line', 0)
        if rule_id not in by_rule:
            by_rule[rule_id] = []
        by_rule[rule_id].append(line)
    
    return by_rule

def run_our_tool():
    """Run our tool and extract findings by line"""
    cmd = ["./target/debug/astgrep", "analyze", 
           "--rules", "tests/bash-sql/bash_security_rules.yaml",
           "tests/bash-sql/test_bash_script.sh", 
           "--compatible", "semgrep"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Our tool failed: {result.stderr}")
        return {}
    
    # Parse the human-readable output
    by_rule = {}
    current_rule = None
    
    for line in result.stdout.split('\n'):
        # Look for rule headers like "❯❯❱ tests.bash-sql.bash.command-injection"
        rule_match = re.search(r'❯❯❱\s+(tests\.bash-sql\.bash\.[a-z-]+)', line)
        if rule_match:
            current_rule = rule_match.group(1)
            if current_rule not in by_rule:
                by_rule[current_rule] = []
        
        # Look for line numbers like "10┆ eval "echo $user_input""
        elif current_rule:
            line_match = re.search(r'^\s*(\d+)┆', line)
            if line_match:
                line_num = int(line_match.group(1))
                by_rule[current_rule].append(line_num)
    
    return by_rule

def main():
    print("Running semgrep...")
    semgrep_by_rule = run_semgrep()
    
    print("Running our tool...")
    our_by_rule = run_our_tool()
    
    # Calculate totals
    semgrep_total = sum(len(lines) for lines in semgrep_by_rule.values())
    our_total = sum(len(lines) for lines in our_by_rule.values())
    
    print(f"\nSemgrep found {semgrep_total} findings")
    print(f"Our tool found {our_total} findings")
    print(f"Difference: {our_total - semgrep_total}")
    
    # Compare by rule
    all_rules = set(semgrep_by_rule.keys()) | set(our_by_rule.keys())
    
    print("\nFindings by rule:")
    for rule_id in sorted(all_rules):
        semgrep_lines = sorted(semgrep_by_rule.get(rule_id, []))
        our_lines = sorted(our_by_rule.get(rule_id, []))
        
        semgrep_count = len(semgrep_lines)
        our_count = len(our_lines)
        diff = our_count - semgrep_count
        
        status = ""
        if diff > 0:
            status = f" (+{diff})"
        elif diff < 0:
            status = f" ({diff})"
        
        print(f"  {rule_id}: semgrep={semgrep_count}, ours={our_count}{status}")
        
        # Show line differences if they exist
        if set(semgrep_lines) != set(our_lines):
            only_semgrep = set(semgrep_lines) - set(our_lines)
            only_ours = set(our_lines) - set(semgrep_lines)
            
            if only_semgrep:
                print(f"    Only in semgrep: {sorted(only_semgrep)}")
            if only_ours:
                print(f"    Only in ours: {sorted(only_ours)}")

if __name__ == "__main__":
    main()
