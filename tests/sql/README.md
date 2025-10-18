# SQL Security Analysis Tests

This directory contains comprehensive test cases for SQL security analysis functionality.

## Test Structure

- `sql_injection.sql` - SQL injection vulnerability test cases
- `sql_injection.yaml` - Rules for detecting SQL injection
- `select_star.sql` - SELECT * usage test cases
- `select_star.yaml` - Rules for detecting SELECT * issues
- `missing_where.sql` - Missing WHERE clause test cases
- `missing_where.yaml` - Rules for detecting missing WHERE clauses
- `privilege_escalation.sql` - Privilege escalation test cases
- `privilege_escalation.yaml` - Rules for detecting privilege escalation
- `weak_encryption.sql` - Weak encryption test cases
- `weak_encryption.yaml` - Rules for detecting weak encryption
- `information_disclosure.sql` - Information disclosure test cases
- `information_disclosure.yaml` - Rules for detecting information disclosure
- `stored_procedures.sql` - Stored procedure test cases
- `stored_procedures.yaml` - Rules for analyzing stored procedures
- `run_tests.py` - Test runner script

## Test Categories

### 1. SQL Injection (sql_injection.*)
Tests for detecting SQL injection vulnerabilities including:
- String concatenation in WHERE clauses
- UNION-based injection
- Dynamic query construction
- Parameterized query detection

### 2. SELECT * Usage (select_star.*)
Tests for detecting SELECT * queries that may expose sensitive data:
- SELECT * without column specification
- SELECT * with LIMIT (should be safe)
- SELECT * in subqueries

### 3. Missing WHERE Clause (missing_where.*)
Tests for detecting dangerous UPDATE/DELETE without WHERE:
- DELETE without WHERE
- UPDATE without WHERE
- Safe DELETE/UPDATE with WHERE

### 4. Privilege Escalation (privilege_escalation.*)
Tests for detecting excessive privilege grants:
- GRANT ALL PRIVILEGES
- GRANT ALL ON *.*
- Limited privilege grants (safe)

### 5. Weak Encryption (weak_encryption.*)
Tests for detecting weak cryptographic algorithms:
- MD5 hashing
- SHA1 hashing
- DES encryption
- Strong algorithms (SHA256, etc.)

### 6. Information Disclosure (information_disclosure.*)
Tests for detecting information gathering queries:
- SELECT USER()
- SELECT VERSION()
- SHOW DATABASES
- SHOW TABLES

### 7. Stored Procedures (stored_procedures.*)
Tests for analyzing stored procedures:
- Dynamic SQL in procedures
- Parameter handling
- Security issues in procedures

## Running Tests

```bash
# Run all SQL tests
python3 run_tests.py

# Run specific test category
python3 run_tests.py --category sql_injection

# Run with verbose output
python3 run_tests.py --verbose
```

## Test Format

Each test file contains:
- **VULNERABLE** comments: Code that should trigger security rules
- **SAFE** comments: Code that should NOT trigger security rules
- **ruleid** comments: Expected rule IDs that should match

Example:
```sql
-- VULNERABLE: String concatenation in WHERE clause
-- ruleid: sql-injection-001
SELECT * FROM users WHERE id = 'admin' + @input;

-- SAFE: Parameterized query
SELECT * FROM users WHERE id = ?;
```

## Expected Results

Each test case is designed to:
1. Trigger specific security rules for vulnerable code
2. NOT trigger rules for safe code
3. Validate rule accuracy and reduce false positives

## Adding New Tests

To add new test cases:
1. Create a new `.sql` file with test code
2. Create a corresponding `.yaml` file with rules
3. Add comments indicating expected behavior
4. Update this README with the new test category
5. Run tests to validate

