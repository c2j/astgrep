# SQL Security Analysis Test Suite - File Index

## Directory Structure

```
tests/sql/
├── README.md                          # Main documentation
├── TEST_SUMMARY.md                    # Comprehensive test summary
├── INDEX.md                           # This file
├── run_tests.py                       # Test runner script
│
├── sql_injection.sql                  # SQL injection test cases (100+)
├── sql_injection.yaml                 # SQL injection detection rules (10)
│
├── select_star.sql                    # SELECT * usage test cases (80+)
├── select_star.yaml                   # SELECT * detection rules (6)
│
├── missing_where.sql                  # Missing WHERE clause test cases (80+)
├── missing_where.yaml                 # Missing WHERE clause rules (6)
│
├── privilege_escalation.sql           # Privilege escalation test cases (90+)
├── privilege_escalation.yaml          # Privilege escalation rules (12)
│
├── weak_encryption.sql                # Weak encryption test cases (90+)
├── weak_encryption.yaml               # Weak encryption rules (12)
│
└── information_disclosure.sql         # Information disclosure test cases (100+)
    information_disclosure.yaml        # Information disclosure rules (14)
```

## File Descriptions

### Documentation Files

| File | Purpose | Size |
|------|---------|------|
| README.md | Overview and usage guide | ~200 lines |
| TEST_SUMMARY.md | Detailed test statistics and coverage | ~300 lines |
| INDEX.md | This file - directory structure | ~150 lines |

### Test Execution

| File | Purpose | Language |
|------|---------|----------|
| run_tests.py | Main test runner script | Python 3 |

### Test Categories (6 total)

#### 1. SQL Injection Tests
| File | Type | Content |
|------|------|---------|
| sql_injection.sql | Test Cases | 100+ vulnerable and safe SQL examples |
| sql_injection.yaml | Rules | 10 detection rules for SQL injection |

**Coverage:**
- String concatenation injection
- UNION-based injection
- Dynamic query construction
- Comment-based injection
- Blind SQL injection
- Time-based injection
- Stacked queries
- Second-order injection
- ORDER BY/LIMIT injection
- Quote escape bypass

#### 2. SELECT * Usage Tests
| File | Type | Content |
|------|------|---------|
| select_star.sql | Test Cases | 80+ test cases for SELECT * usage |
| select_star.yaml | Rules | 6 detection rules for SELECT * issues |

**Coverage:**
- Basic SELECT * issues
- SELECT * with LIMIT
- SELECT * in subqueries
- SELECT * with WHERE
- SELECT * with JOIN
- SELECT * with GROUP BY
- SELECT * with ORDER BY
- SELECT * with DISTINCT
- SELECT * in UNION
- SELECT * with window functions
- SELECT * in CTE
- Performance impact

#### 3. Missing WHERE Clause Tests
| File | Type | Content |
|------|------|---------|
| missing_where.sql | Test Cases | 80+ test cases for missing WHERE |
| missing_where.yaml | Rules | 6 detection rules for missing WHERE |

**Coverage:**
- DELETE without WHERE
- UPDATE without WHERE
- DELETE with JOIN
- UPDATE with JOIN
- DELETE in transactions
- UPDATE in transactions
- DELETE in triggers
- UPDATE in triggers
- DELETE in procedures
- UPDATE in procedures
- Bulk operations
- Conditional operations

#### 4. Privilege Escalation Tests
| File | Type | Content |
|------|------|---------|
| privilege_escalation.sql | Test Cases | 90+ test cases for privilege issues |
| privilege_escalation.yaml | Rules | 12 detection rules for privilege escalation |

**Coverage:**
- GRANT ALL PRIVILEGES
- GRANT ALL ON *.*
- ALTER USER changes
- CREATE USER with excessive privileges
- GRANT with GRANT OPTION
- Administrative privileges
- Wildcard host grants
- Incomplete revocation
- Default user privileges
- Role-based escalation
- Procedure DEFINER escalation
- View DEFINER escalation

#### 5. Weak Encryption Tests
| File | Type | Content |
|------|------|---------|
| weak_encryption.sql | Test Cases | 90+ test cases for weak encryption |
| weak_encryption.yaml | Rules | 12 detection rules for weak encryption |

**Coverage:**
- MD5 hashing
- SHA1 hashing
- DES encryption
- Weak hashing in triggers
- Weak encryption in procedures
- Weak encryption in views
- Multiple weak algorithms
- Weak encryption in indexes
- Weak encryption in constraints
- Weak encryption in transactions
- Legacy weak functions
- Hardcoded keys

#### 6. Information Disclosure Tests
| File | Type | Content |
|------|------|---------|
| information_disclosure.sql | Test Cases | 100+ test cases for information disclosure |
| information_disclosure.yaml | Rules | 14 detection rules for information disclosure |

**Coverage:**
- System information functions
- SHOW commands
- Information schema queries
- System catalog queries
- File system access
- Error-based disclosure
- Timing-based disclosure
- Metadata queries
- Performance schema queries
- Procedure information
- View information
- Privilege information
- Configuration information
- Session information

## Quick Start

### View Documentation
```bash
# Main README
cat tests/sql/README.md

# Test summary
cat tests/sql/TEST_SUMMARY.md

# This index
cat tests/sql/INDEX.md
```

### Run Tests
```bash
# Run all tests
cd tests/sql
python3 run_tests.py

# Run with verbose output
python3 run_tests.py --verbose

# Generate report
python3 run_tests.py --report results.json
```

### View Test Cases
```bash
# View SQL injection tests
cat tests/sql/sql_injection.sql

# View corresponding rules
cat tests/sql/sql_injection.yaml

# View any category
cat tests/sql/{category}.sql
cat tests/sql/{category}.yaml
```

## Test Statistics

### Total Coverage
- **Total Test Cases**: 540+
- **Total Rules**: 60
- **Total Lines of SQL**: 2000+
- **Total Lines of YAML**: 1500+
- **Documentation Lines**: 650+

### By Category
| Category | Cases | Rules | SQL Lines | YAML Lines |
|----------|-------|-------|-----------|------------|
| SQL Injection | 100+ | 10 | 350+ | 250+ |
| SELECT * | 80+ | 6 | 280+ | 150+ |
| Missing WHERE | 80+ | 6 | 280+ | 150+ |
| Privilege Escalation | 90+ | 12 | 320+ | 300+ |
| Weak Encryption | 90+ | 12 | 320+ | 300+ |
| Information Disclosure | 100+ | 14 | 350+ | 350+ |

## File Sizes

| File | Lines | Size |
|------|-------|------|
| sql_injection.sql | 350+ | ~12 KB |
| sql_injection.yaml | 250+ | ~8 KB |
| select_star.sql | 280+ | ~10 KB |
| select_star.yaml | 150+ | ~5 KB |
| missing_where.sql | 280+ | ~10 KB |
| missing_where.yaml | 150+ | ~5 KB |
| privilege_escalation.sql | 320+ | ~12 KB |
| privilege_escalation.yaml | 300+ | ~10 KB |
| weak_encryption.sql | 320+ | ~12 KB |
| weak_encryption.yaml | 300+ | ~10 KB |
| information_disclosure.sql | 350+ | ~13 KB |
| information_disclosure.yaml | 350+ | ~12 KB |
| run_tests.py | 200+ | ~7 KB |
| README.md | 200+ | ~8 KB |
| TEST_SUMMARY.md | 300+ | ~12 KB |
| INDEX.md | 150+ | ~6 KB |

## Integration Points

### With CI/CD
```bash
# Run tests in CI pipeline
python3 tests/sql/run_tests.py --report test_results.json
```

### With Code Analyzer
```bash
# Analyze SQL files with rules
cr-cli analyze --rules tests/sql/*.yaml tests/sql/*.sql
```

### With IDE
```bash
# Use rules in IDE extensions
# Copy rules to IDE plugin directory
cp tests/sql/*.yaml ~/.config/ide-plugin/rules/
```

## Maintenance

### Adding New Tests
1. Edit appropriate `.sql` file
2. Add test cases with `-- VULNERABLE:` or `-- SAFE:` comments
3. Update corresponding `.yaml` file with rules
4. Run tests to validate
5. Update TEST_SUMMARY.md

### Updating Rules
1. Edit `.yaml` file
2. Update rule patterns and messages
3. Run tests to validate
4. Update TEST_SUMMARY.md

### Extending Coverage
- Add new test categories by creating new `.sql` and `.yaml` pairs
- Follow naming convention: `{category}.sql` and `{category}.yaml`
- Update this INDEX.md file

## References

- **CWE-89**: SQL Injection
- **CWE-200**: Information Exposure
- **CWE-250**: Execution with Unnecessary Privileges
- **CWE-327**: Use of a Broken or Risky Cryptographic Algorithm
- **OWASP Top 10 2021**: A03:2021 - Injection, A01:2021 - Broken Access Control
- **OWASP SQL Injection Prevention Cheat Sheet**

## Support

For issues or questions:
1. Check README.md for usage
2. Check TEST_SUMMARY.md for coverage details
3. Review test cases in `.sql` files
4. Review rules in `.yaml` files
5. Run tests with `--verbose` flag for debugging

