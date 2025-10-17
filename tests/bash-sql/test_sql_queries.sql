-- Test SQL file for security analysis

-- 1. SQL Injection vulnerabilities
-- VULNERABLE: String concatenation in WHERE clause
SELECT * FROM users WHERE username = 'admin' + @input;  -- Should trigger sql.injection-risk
SELECT * FROM products WHERE name = 'item' || user_input;  -- Should trigger sql.injection-risk

-- VULNERABLE: Direct user input in query
SELECT * FROM accounts WHERE id = 1 OR 1=1;  -- Should trigger sql.injection-risk

-- SAFE: Parameterized queries
SELECT * FROM users WHERE username = ?;  -- Should NOT trigger
SELECT * FROM users WHERE id = $1;  -- Should NOT trigger

-- 2. UNION-based injection
-- VULNERABLE: UNION injection attempts
SELECT name FROM products UNION SELECT password FROM users;  -- Should trigger sql.union-injection
SELECT id, name FROM items UNION SELECT 1, version();  -- Should trigger sql.union-injection

-- SAFE: Legitimate UNION
SELECT name, price FROM products UNION SELECT name, cost FROM services;  -- Should NOT trigger

-- 3. Dynamic query construction
-- VULNERABLE: String concatenation
SELECT * FROM table1 WHERE column1 = 'value' + @param;  -- Should trigger sql.dynamic-query-construction

-- SAFE: Prepared statements
PREPARE stmt FROM 'SELECT * FROM users WHERE id = ?';  -- Should NOT trigger

-- 4. Hardcoded passwords
-- VULNERABLE: Hardcoded credentials
CREATE USER 'testuser'@'localhost' IDENTIFIED BY 'password123';  -- Should trigger sql.hardcoded-password
ALTER USER 'admin'@'%' IDENTIFIED BY 'supersecret';  -- Should trigger sql.hardcoded-password

-- SAFE: Environment variable or parameter
CREATE USER 'testuser'@'localhost' IDENTIFIED BY '${DB_PASSWORD}';  -- Should NOT trigger

-- 5. SELECT * usage
-- VULNERABLE: SELECT * without limits
SELECT * FROM sensitive_data;  -- Should trigger sql.select-star
SELECT * FROM user_profiles;  -- Should trigger sql.select-star

-- SAFE: SELECT * with LIMIT
SELECT * FROM logs LIMIT 1;  -- Should NOT trigger

-- 6. Missing WHERE clauses
-- VULNERABLE: DELETE/UPDATE without WHERE
DELETE FROM temp_table;  -- Should trigger sql.missing-where-clause
UPDATE user_settings SET active = 0;  -- Should trigger sql.missing-where-clause

-- SAFE: DELETE/UPDATE with WHERE
DELETE FROM temp_table WHERE created_date < '2023-01-01';  -- Should NOT trigger
UPDATE user_settings SET active = 0 WHERE last_login < '2022-01-01';  -- Should NOT trigger

-- 7. Privilege escalation
-- VULNERABLE: Excessive privileges
GRANT ALL ON *.* TO 'webapp'@'%';  -- Should trigger sql.privilege-escalation
GRANT ALL PRIVILEGES ON *.* TO 'service'@'localhost';  -- Should trigger sql.privilege-escalation

-- SAFE: Limited privileges
GRANT SELECT, INSERT ON myapp.* TO 'webapp'@'%';  -- Should NOT trigger

-- 8. Weak encryption
-- VULNERABLE: Weak hashing algorithms
SELECT MD5(password) FROM users;  -- Should trigger sql.weak-encryption
SELECT SHA1(sensitive_data) FROM records;  -- Should trigger sql.weak-encryption
SELECT DES_ENCRYPT(credit_card, 'key') FROM payments;  -- Should trigger sql.weak-encryption

-- SAFE: Strong encryption
SELECT SHA256(password) FROM users;  -- Should NOT trigger

-- 9. Information disclosure
-- VULNERABLE: Information gathering
SELECT USER();  -- Should trigger sql.information-disclosure
SELECT VERSION();  -- Should trigger sql.information-disclosure
SELECT DATABASE();  -- Should trigger sql.information-disclosure
SHOW DATABASES;  -- Should trigger sql.information-disclosure
SHOW TABLES;  -- Should trigger sql.information-disclosure

-- 10. Time-based attacks
-- VULNERABLE: Time-based injection
SELECT * FROM users WHERE id = 1 AND SLEEP(5);  -- Should trigger sql.time-based-attack
SELECT * FROM data WHERE BENCHMARK(1000000, MD5('test'));  -- Should trigger sql.time-based-attack

-- SQL Server specific
WAITFOR DELAY '00:00:05';  -- Should trigger sql.time-based-attack

-- 11. File operations
-- VULNERABLE: File system access
SELECT LOAD_FILE('/etc/passwd');  -- Should trigger sql.file-operations
SELECT * FROM users INTO OUTFILE '/tmp/users.txt';  -- Should trigger sql.file-operations
SELECT password INTO DUMPFILE '/tmp/passwords.txt' FROM users;  -- Should trigger sql.file-operations

-- 12. Command execution
-- VULNERABLE: OS command execution
EXEC xp_cmdshell 'dir';  -- Should trigger sql.command-execution
EXECUTE sp_configure 'xp_cmdshell', 1;  -- Should trigger sql.command-execution

-- 13. Complex queries with multiple vulnerabilities
-- VULNERABLE: Multiple issues
SELECT * FROM users 
WHERE username = 'admin' + @input 
UNION SELECT password, email FROM admin_users;  -- Multiple triggers

-- 14. Stored procedures and functions
DELIMITER //
CREATE PROCEDURE GetUserData(IN user_id INT)
BEGIN
    -- VULNERABLE: Dynamic SQL in stored procedure
    SET @sql = CONCAT('SELECT * FROM users WHERE id = ', user_id);
    PREPARE stmt FROM @sql;
    EXECUTE stmt;
    DEALLOCATE PREPARE stmt;
END //
DELIMITER ;

-- 15. Triggers
CREATE TRIGGER user_audit 
AFTER INSERT ON users 
FOR EACH ROW
BEGIN
    -- VULNERABLE: Hardcoded password in trigger
    INSERT INTO audit_log (action, user_id, password) 
    VALUES ('INSERT', NEW.id, 'audit_password123');
END;

-- 16. Views
CREATE VIEW user_summary AS
SELECT username, email, MD5(password) as password_hash  -- Should trigger sql.weak-encryption
FROM users 
WHERE active = 1;

-- 17. Indexes
CREATE INDEX idx_user_search ON users(username, MD5(email));  -- Should trigger sql.weak-encryption

-- 18. Transactions
START TRANSACTION;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- 19. Common table expressions (CTEs)
WITH user_stats AS (
    SELECT user_id, COUNT(*) as login_count
    FROM login_history
    GROUP BY user_id
)
SELECT u.username, us.login_count
FROM users u
JOIN user_stats us ON u.id = us.user_id;

-- 20. Window functions
SELECT 
    username,
    email,
    ROW_NUMBER() OVER (ORDER BY created_date) as row_num
FROM users;

-- 21. JSON operations (MySQL 5.7+)
SELECT JSON_EXTRACT(profile_data, '$.email') as email
FROM user_profiles
WHERE JSON_EXTRACT(profile_data, '$.active') = true;

-- 22. Regular expressions
SELECT * FROM products 
WHERE product_name REGEXP '^[A-Z][a-z]+$';

-- 23. Date and time functions
SELECT * FROM orders 
WHERE order_date BETWEEN '2023-01-01' AND '2023-12-31'
AND DAYOFWEEK(order_date) IN (1, 7);  -- Weekends only

-- End of test file
