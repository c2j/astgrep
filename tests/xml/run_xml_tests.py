#!/usr/bin/env python3
"""
XML Test Runner
Runs XML security and best practices tests
"""

import os
import sys
import json
import subprocess
from pathlib import Path
from typing import List, Dict, Any

# Color codes for terminal output
class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def print_header(text: str):
    """Print a formatted header"""
    print(f"\n{Colors.HEADER}{Colors.BOLD}{'=' * 80}{Colors.ENDC}")
    print(f"{Colors.HEADER}{Colors.BOLD}{text.center(80)}{Colors.ENDC}")
    print(f"{Colors.HEADER}{Colors.BOLD}{'=' * 80}{Colors.ENDC}\n")

def print_success(text: str):
    """Print success message"""
    print(f"{Colors.OKGREEN}✓ {text}{Colors.ENDC}")

def print_error(text: str):
    """Print error message"""
    print(f"{Colors.FAIL}✗ {text}{Colors.ENDC}")

def print_warning(text: str):
    """Print warning message"""
    print(f"{Colors.WARNING}⚠ {text}{Colors.ENDC}")

def print_info(text: str):
    """Print info message"""
    print(f"{Colors.OKCYAN}ℹ {text}{Colors.ENDC}")

def run_command(cmd: List[str], cwd: str = None) -> tuple:
    """Run a shell command and return output"""
    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=60
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Command timed out"
    except Exception as e:
        return -1, "", str(e)

def check_xml_files_exist() -> bool:
    """Check if XML test files exist"""
    xml_dir = Path(__file__).parent
    required_files = [
        'xml_security.yaml',
        'security_test.xml',
        'best_practices.yaml',
        'best_practices_test.xml',
        'sample.xml',
        'config.xml',
        'pom.xml',
        'image.svg'
    ]
    
    all_exist = True
    for file in required_files:
        file_path = xml_dir / file
        if file_path.exists():
            print_success(f"Found: {file}")
        else:
            print_error(f"Missing: {file}")
            all_exist = False
    
    return all_exist

def validate_xml_syntax(xml_file: Path) -> bool:
    """Validate XML syntax using xmllint if available"""
    # Skip validation for test files that contain multiple XML fragments
    test_files = ['security_test.xml', 'best_practices_test.xml']
    if xml_file.name in test_files:
        print_info(f"Skipping syntax validation for test file: {xml_file.name} (contains multiple XML fragments)")
        return True

    try:
        returncode, stdout, stderr = run_command(['xmllint', '--noout', str(xml_file)])
        if returncode == 0:
            print_success(f"Valid XML syntax: {xml_file.name}")
            return True
        else:
            print_error(f"Invalid XML syntax: {xml_file.name}")
            print_error(f"  Error: {stderr}")
            return False
    except FileNotFoundError:
        print_warning("xmllint not found - skipping XML syntax validation")
        return True

def validate_yaml_syntax(yaml_file: Path) -> bool:
    """Validate YAML syntax"""
    try:
        import yaml
        with open(yaml_file, 'r') as f:
            yaml.safe_load(f)
        print_success(f"Valid YAML syntax: {yaml_file.name}")
        return True
    except ImportError:
        print_warning("PyYAML not installed - skipping YAML validation")
        return True
    except Exception as e:
        print_error(f"Invalid YAML syntax: {yaml_file.name}")
        print_error(f"  Error: {str(e)}")
        return False

def count_rules(yaml_file: Path) -> int:
    """Count number of rules in YAML file"""
    try:
        import yaml
        with open(yaml_file, 'r') as f:
            data = yaml.safe_load(f)
            if 'rules' in data:
                return len(data['rules'])
    except:
        pass
    return 0

def count_test_cases(xml_file: Path) -> int:
    """Count number of ruleid comments in XML file"""
    try:
        with open(xml_file, 'r') as f:
            content = f.read()
            return content.count('ruleid:')
    except:
        return 0

def run_xml_tests():
    """Main test runner"""
    print_header("XML Language Support Test Suite")
    
    xml_dir = Path(__file__).parent
    
    # Step 1: Check file existence
    print_header("Step 1: Checking Test Files")
    if not check_xml_files_exist():
        print_error("Some required files are missing!")
        return False
    
    # Step 2: Validate XML syntax
    print_header("Step 2: Validating XML Syntax")
    xml_files = [
        'security_test.xml',
        'best_practices_test.xml',
        'sample.xml',
        'config.xml',
        'pom.xml',
        'image.svg'
    ]
    
    xml_valid = True
    for xml_file in xml_files:
        file_path = xml_dir / xml_file
        if file_path.exists():
            if not validate_xml_syntax(file_path):
                xml_valid = False
    
    # Step 3: Validate YAML syntax
    print_header("Step 3: Validating YAML Rule Files")
    yaml_files = [
        'xml_security.yaml',
        'best_practices.yaml'
    ]
    
    yaml_valid = True
    for yaml_file in yaml_files:
        file_path = xml_dir / yaml_file
        if file_path.exists():
            if not validate_yaml_syntax(file_path):
                yaml_valid = False
    
    # Step 4: Count rules and test cases
    print_header("Step 4: Test Coverage Statistics")
    
    security_rules = count_rules(xml_dir / 'xml_security.yaml')
    security_tests = count_test_cases(xml_dir / 'security_test.xml')
    print_info(f"Security rules: {security_rules}")
    print_info(f"Security test cases: {security_tests}")
    
    bp_rules = count_rules(xml_dir / 'best_practices.yaml')
    bp_tests = count_test_cases(xml_dir / 'best_practices_test.xml')
    print_info(f"Best practice rules: {bp_rules}")
    print_info(f"Best practice test cases: {bp_tests}")
    
    total_rules = security_rules + bp_rules
    total_tests = security_tests + bp_tests
    print_info(f"Total rules: {total_rules}")
    print_info(f"Total test cases: {total_tests}")
    
    # Step 5: Summary
    print_header("Test Summary")
    
    all_passed = xml_valid and yaml_valid
    
    if all_passed:
        print_success("All XML tests passed!")
        print_success(f"✓ {len(xml_files)} XML files validated")
        print_success(f"✓ {len(yaml_files)} YAML rule files validated")
        print_success(f"✓ {total_rules} security rules defined")
        print_success(f"✓ {total_tests} test cases created")
    else:
        print_error("Some tests failed!")
        if not xml_valid:
            print_error("  - XML syntax validation failed")
        if not yaml_valid:
            print_error("  - YAML syntax validation failed")
    
    # Step 6: Next steps
    print_header("Next Steps")
    print_info("To run the XML security analysis:")
    print(f"  {Colors.BOLD}cargo run -- analyze tests/xml/security_test.xml --rules tests/xml/xml_security.yaml{Colors.ENDC}")
    print()
    print_info("To run the best practices analysis:")
    print(f"  {Colors.BOLD}cargo run -- analyze tests/xml/best_practices_test.xml --rules tests/xml/best_practices.yaml{Colors.ENDC}")
    print()
    
    return all_passed

if __name__ == '__main__':
    try:
        success = run_xml_tests()
        sys.exit(0 if success else 1)
    except KeyboardInterrupt:
        print_error("\nTest run interrupted by user")
        sys.exit(1)
    except Exception as e:
        print_error(f"Unexpected error: {str(e)}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

