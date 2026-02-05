#!/usr/bin/env python3

import json
import subprocess
import sys

def run_semgrep():
    """Run semgrep and return the results"""
    cmd = ["semgrep", "--config", "tests/bash-sql/bash_security_rules.yaml", 
           "tests/bash-sql/test_bash_script.sh", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Semgrep failed: {result.stderr}")
        return None
    return json.loads(result.stdout)

def run_our_tool():
    """Run our tool and return the results"""
    cmd = ["./target/debug/astgrep", "analyze", 
           "--rules", "tests/bash-sql/bash_security_rules.yaml",
           "tests/bash-sql/test_bash_script.sh", 
           "--compatible", "semgrep", "--format", "json", "--quiet"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Our tool failed: {result.stderr}")
        return None
    
    # Our tool outputs logs mixed with JSON, so we need to extract the JSON part
    lines = result.stdout.strip().split('\n')
    json_lines = []
    in_json = False
    for line in lines:
        if line.strip().startswith('{') or line.strip().startswith('['):
            in_json = True
        if in_json:
            json_lines.append(line)
    
    if json_lines:
        try:
            return json.loads('\n'.join(json_lines))
        except json.JSONDecodeError:
            print("Failed to parse JSON from our tool")
            print("Output:", result.stdout)
            return None
    else:
        print("No JSON found in our tool output")
        print("Output:", result.stdout)
        return None

def compare_results():
    """Compare the results from both tools"""
    print("Running semgrep...")
    semgrep_results = run_semgrep()
    if not semgrep_results:
        return

    print("Running our tool...")
    our_results = run_our_tool()
    if not our_results:
        return

    # Extract findings
    semgrep_findings = semgrep_results.get('results', [])
    our_findings = our_results.get('results', [])

    print(f"\nSemgrep found {len(semgrep_findings)} findings")
    print(f"Our tool found {len(our_findings)} findings")

    # Group by rule ID
    semgrep_by_rule = {}
    for finding in semgrep_findings:
        rule_id = finding.get('check_id', 'unknown')
        if rule_id not in semgrep_by_rule:
            semgrep_by_rule[rule_id] = []
        semgrep_by_rule[rule_id].append(finding)

    our_by_rule = {}
    for finding in our_findings:
        rule_id = finding.get('check_id', 'unknown')
        if rule_id not in our_by_rule:
            our_by_rule[rule_id] = []
        our_by_rule[rule_id].append(finding)

    print("\nFindings by rule:")
    all_rules = set(semgrep_by_rule.keys()) | set(our_by_rule.keys())
    for rule_id in sorted(all_rules):
        semgrep_count = len(semgrep_by_rule.get(rule_id, []))
        our_count = len(our_by_rule.get(rule_id, []))
        diff = our_count - semgrep_count
        status = ""
        if diff > 0:
            status = f" (+{diff})"
        elif diff < 0:
            status = f" ({diff})"
        print(f"  {rule_id}: semgrep={semgrep_count}, ours={our_count}{status}")

    # Show detailed line-by-line comparison for rules with differences
    print("\nDetailed differences:")
    for rule_id in sorted(all_rules):
        semgrep_findings_rule = semgrep_by_rule.get(rule_id, [])
        our_findings_rule = our_by_rule.get(rule_id, [])

        semgrep_lines = set(f.get('start', {}).get('line', 0) for f in semgrep_findings_rule)
        our_lines = set(f.get('start', {}).get('line', 0) for f in our_findings_rule)

        if semgrep_lines != our_lines:
            print(f"\n  {rule_id}:")
            print(f"    Semgrep lines: {sorted(semgrep_lines)}")
            print(f"    Our lines: {sorted(our_lines)}")

            only_semgrep = semgrep_lines - our_lines
            only_ours = our_lines - semgrep_lines

            if only_semgrep:
                print(f"    Only in semgrep: {sorted(only_semgrep)}")
            if only_ours:
                print(f"    Only in ours: {sorted(only_ours)}")

    print(f"\nTotal difference: {len(our_findings) - len(semgrep_findings)} findings")

if __name__ == "__main__":
    compare_results()
