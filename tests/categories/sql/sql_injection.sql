-- SQL Injection Vulnerability Test Cases
-- This file contains examples of SQL injection vulnerabilities and safe alternatives

-- ============================================================================
-- 1. Basic String Concatenation Injection
-- ============================================================================

-- VULNERABLE: String concatenation in WHERE clause
-- ruleid: sql-injection-001
SELECT * FROM users WHERE username = 'admin' + @input;

-- VULNERABLE: String concatenation with pipe operator
-- ruleid: sql-injection-001
SELECT * FROM users WHERE email = 'user@' || user_input;

-- VULNERABLE: Direct user input in WHERE clause
-- ruleid: sql-injection-001
SELECT * FROM accounts WHERE id = 1 OR 1=1;

-- SAFE: Parameterized query with placeholder
SELECT * FROM users WHERE username = ?;

-- SAFE: Parameterized query with numbered placeholder
SELECT * FROM users WHERE id = $1;

-- SAFE: Parameterized query with named placeholder
SELECT * FROM users WHERE email = :email;

-- ============================================================================
-- 2. UNION-based Injection
-- ============================================================================

-- VULNERABLE: UNION injection to extract data
-- ruleid: sql-injection-002
SELECT name FROM products UNION SELECT password FROM users;

-- VULNERABLE: UNION with system functions
-- ruleid: sql-injection-002
SELECT id, name FROM items UNION SELECT 1, version();

-- VULNERABLE: UNION with multiple columns
-- ruleid: sql-injection-002
SELECT user_id, username FROM users UNION SELECT id, password FROM admin_users;

-- SAFE: Legitimate UNION with same table structure
SELECT name, price FROM products UNION SELECT name, cost FROM services;

-- SAFE: UNION with same columns
SELECT id, name FROM table1 UNION SELECT id, name FROM table2;

-- ============================================================================
-- 3. Dynamic Query Construction
-- ============================================================================

-- VULNERABLE: String concatenation in dynamic SQL
-- ruleid: sql-injection-003
SELECT * FROM table1 WHERE column1 = 'value' + @param;

-- VULNERABLE: Format string in dynamic SQL
-- ruleid: sql-injection-003
SELECT * FROM users WHERE name = CONCAT('user_', @input);

-- VULNERABLE: Multiple concatenations
-- ruleid: sql-injection-003
SELECT * FROM orders WHERE customer_id = @id AND status = @status + 'pending';

-- SAFE: Prepared statement
PREPARE stmt FROM 'SELECT * FROM users WHERE id = ?';

-- SAFE: Stored procedure with parameters
CALL GetUserData(@user_id);

-- ============================================================================
-- 4. Comment-based Injection
-- ============================================================================

-- VULNERABLE: Comment injection to bypass WHERE clause
-- ruleid: sql-injection-004
SELECT * FROM users WHERE id = 1 OR 1=1 -- ;

-- VULNERABLE: Comment injection with block comment
-- ruleid: sql-injection-004
SELECT * FROM users WHERE id = 1 /* OR 1=1 */;

-- SAFE: Properly escaped comment
SELECT * FROM users WHERE id = 1 AND status = 'active';

-- ============================================================================
-- 5. Blind SQL Injection
-- ============================================================================

-- VULNERABLE: Blind injection with boolean logic
-- ruleid: sql-injection-005
SELECT * FROM users WHERE id = 1 AND (SELECT COUNT(*) FROM users) > 0;

-- VULNERABLE: Blind injection with CASE statement
-- ruleid: sql-injection-005
SELECT * FROM users WHERE id = CASE WHEN (1=1) THEN 1 ELSE 2 END;

-- SAFE: Normal conditional logic
SELECT * FROM users WHERE id = 1 AND active = 1;

-- ============================================================================
-- 6. Time-based Blind Injection
-- ============================================================================

-- VULNERABLE: Time-based injection with SLEEP
-- ruleid: sql-injection-006
SELECT * FROM users WHERE id = 1 AND SLEEP(5);

-- VULNERABLE: Time-based injection with BENCHMARK
-- ruleid: sql-injection-006
SELECT * FROM users WHERE id = 1 AND BENCHMARK(1000000, MD5('test'));

-- VULNERABLE: SQL Server time-based injection
-- ruleid: sql-injection-006
SELECT * FROM users WHERE id = 1 AND WAITFOR DELAY '00:00:05';

-- SAFE: Normal query without delays
SELECT * FROM users WHERE id = 1;

-- ============================================================================
-- 7. Stacked Queries
-- ============================================================================

-- VULNERABLE: Multiple statements in one query
-- ruleid: sql-injection-007
SELECT * FROM users WHERE id = 1; DROP TABLE users;

-- VULNERABLE: Stacked query with INSERT
-- ruleid: sql-injection-007
SELECT * FROM users WHERE id = 1; INSERT INTO users VALUES ('hacker', 'password');

-- SAFE: Single statement
SELECT * FROM users WHERE id = 1;

-- ============================================================================
-- 8. Second-order Injection
-- ============================================================================

-- VULNERABLE: Data stored and later used in query
-- ruleid: sql-injection-008
INSERT INTO logs (message) VALUES (@user_input);
SELECT * FROM logs WHERE message = @stored_value;

-- SAFE: Parameterized storage and retrieval
INSERT INTO logs (message) VALUES (?);
SELECT * FROM logs WHERE message = ?;

-- ============================================================================
-- 9. Injection in Different Contexts
-- ============================================================================

-- VULNERABLE: Injection in ORDER BY
-- ruleid: sql-injection-009
SELECT * FROM users ORDER BY @sort_column;

-- VULNERABLE: Injection in LIMIT
-- ruleid: sql-injection-009
SELECT * FROM users LIMIT @limit_value;

-- VULNERABLE: Injection in table name
-- ruleid: sql-injection-009
SELECT * FROM @table_name WHERE id = 1;

-- SAFE: Whitelist validation for ORDER BY
SELECT * FROM users ORDER BY id;

-- ============================================================================
-- 10. Injection with Special Characters
-- ============================================================================

-- VULNERABLE: Single quote escape bypass
-- ruleid: sql-injection-010
SELECT * FROM users WHERE username = 'admin' OR '1'='1';

-- VULNERABLE: Double quote escape bypass
-- ruleid: sql-injection-010
SELECT * FROM users WHERE username = "admin" OR "1"="1";

-- SAFE: Properly parameterized
SELECT * FROM users WHERE username = ? OR status = ?;

