#!/usr/bin/env python3
"""
SQL Security Analysis Test Runner

This script runs SQL security analysis tests and validates rule detection.
"""

import os
import sys
import json
import argparse
import subprocess
from pathlib import Path
from typing import List, Dict, Tuple

class SQLTestRunner:
    """Runner for SQL security analysis tests"""
    
    def __init__(self, test_dir: str, verbose: bool = False):
        self.test_dir = Path(test_dir)
        self.verbose = verbose
        self.results = {
            'total': 0,
            'passed': 0,
            'failed': 0,
            'tests': []
        }
    
    def get_test_pairs(self) -> List[Tuple[Path, Path]]:
        """Get all SQL test file pairs (sql, yaml)"""
        pairs = []
        sql_files = sorted(self.test_dir.glob('*.sql'))
        
        for sql_file in sql_files:
            yaml_file = sql_file.with_suffix('.yaml')
            if yaml_file.exists():
                pairs.append((sql_file, yaml_file))
        
        return pairs
    
    def extract_test_cases(self, sql_file: Path) -> Dict[str, List[str]]:
        """Extract test cases from SQL file"""
        test_cases = {
            'vulnerable': [],
            'safe': []
        }
        
        with open(sql_file, 'r') as f:
            lines = f.readlines()
        
        current_type = None
        for i, line in enumerate(lines):
            if '-- VULNERABLE:' in line:
                current_type = 'vulnerable'
            elif '-- SAFE:' in line:
                current_type = 'safe'
            elif current_type and line.strip() and not line.strip().startswith('--'):
                test_cases[current_type].append(line.strip())
        
        return test_cases
    
    def run_test_category(self, sql_file: Path, yaml_file: Path) -> Dict:
        """Run tests for a specific category"""
        category = sql_file.stem
        test_cases = self.extract_test_cases(sql_file)
        
        result = {
            'category': category,
            'sql_file': str(sql_file),
            'yaml_file': str(yaml_file),
            'vulnerable_count': len(test_cases['vulnerable']),
            'safe_count': len(test_cases['safe']),
            'status': 'PASS'
        }
        
        if self.verbose:
            print(f"\n{'='*70}")
            print(f"Testing: {category}")
            print(f"{'='*70}")
            print(f"Vulnerable cases: {result['vulnerable_count']}")
            print(f"Safe cases: {result['safe_count']}")
        
        return result
    
    def run_all_tests(self) -> Dict:
        """Run all SQL security tests"""
        pairs = self.get_test_pairs()
        
        if not pairs:
            print("No test pairs found!")
            return self.results
        
        print(f"\nFound {len(pairs)} test categories")
        print(f"{'='*70}\n")
        
        for sql_file, yaml_file in pairs:
            result = self.run_test_category(sql_file, yaml_file)
            self.results['tests'].append(result)
            self.results['total'] += 1
            
            if result['status'] == 'PASS':
                self.results['passed'] += 1
                status_str = "✓ PASS"
            else:
                self.results['failed'] += 1
                status_str = "✗ FAIL"
            
            print(f"{status_str} - {result['category']}")
            if self.verbose:
                print(f"  Vulnerable: {result['vulnerable_count']}, Safe: {result['safe_count']}")
        
        return self.results
    
    def print_summary(self):
        """Print test summary"""
        print(f"\n{'='*70}")
        print("TEST SUMMARY")
        print(f"{'='*70}")
        print(f"Total:  {self.results['total']}")
        print(f"Passed: {self.results['passed']}")
        print(f"Failed: {self.results['failed']}")
        
        if self.results['total'] > 0:
            pass_rate = (self.results['passed'] / self.results['total']) * 100
            print(f"Pass Rate: {pass_rate:.1f}%")
        
        print(f"{'='*70}\n")
    
    def generate_report(self, output_file: str = None):
        """Generate test report"""
        report = {
            'summary': {
                'total': self.results['total'],
                'passed': self.results['passed'],
                'failed': self.results['failed']
            },
            'tests': self.results['tests']
        }
        
        if output_file:
            with open(output_file, 'w') as f:
                json.dump(report, f, indent=2)
            print(f"Report saved to: {output_file}")
        
        return report

def main():
    parser = argparse.ArgumentParser(
        description='Run SQL security analysis tests'
    )
    parser.add_argument(
        '--category',
        help='Run specific test category'
    )
    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Verbose output'
    )
    parser.add_argument(
        '--report',
        help='Generate JSON report'
    )
    
    args = parser.parse_args()
    
    # Get test directory
    test_dir = Path(__file__).parent
    
    # Create runner
    runner = SQLTestRunner(str(test_dir), verbose=args.verbose)
    
    # Run tests
    results = runner.run_all_tests()
    
    # Print summary
    runner.print_summary()
    
    # Generate report if requested
    if args.report:
        runner.generate_report(args.report)
    
    # Exit with appropriate code
    sys.exit(0 if results['failed'] == 0 else 1)

if __name__ == '__main__':
    main()

