-- Missing WHERE Clause Test Cases
-- This file contains examples of dangerous UPDATE/DELETE without WHERE clauses

-- ============================================================================
-- 1. DELETE without WHERE
-- ============================================================================

-- VULNERABLE: DELETE without WHERE clause
-- ruleid: missing-where-001
DELETE FROM temp_table;

-- VULNERABLE: DELETE with semicolon
-- ruleid: missing-where-001
DELETE FROM users;

-- VULNERABLE: DELETE from sensitive table
-- ruleid: missing-where-001
DELETE FROM admin_users;

-- VULNERABLE: DELETE with whitespace
-- ruleid: missing-where-001
DELETE FROM orders  ;

-- SAFE: DELETE with WHERE clause
DELETE FROM temp_table WHERE created_date < '2023-01-01';

-- SAFE: DELETE with WHERE and AND
DELETE FROM users WHERE status = 'inactive' AND last_login < '2022-01-01';

-- SAFE: DELETE with WHERE and OR
DELETE FROM logs WHERE level = 'DEBUG' OR level = 'TRACE';

-- ============================================================================
-- 2. UPDATE without WHERE
-- ============================================================================

-- VULNERABLE: UPDATE without WHERE clause
-- ruleid: missing-where-001
UPDATE user_settings SET active = 0;

-- VULNERABLE: UPDATE with multiple columns
-- ruleid: missing-where-001
UPDATE users SET status = 'inactive', updated_at = NOW();

-- VULNERABLE: UPDATE with calculation
-- ruleid: missing-where-001
UPDATE accounts SET balance = balance - 100;

-- VULNERABLE: UPDATE with string concatenation
-- ruleid: missing-where-001
UPDATE users SET name = CONCAT(name, '_archived');

-- SAFE: UPDATE with WHERE clause
UPDATE user_settings SET active = 0 WHERE user_id = 1;

-- SAFE: UPDATE with WHERE and multiple conditions
UPDATE users SET status = 'inactive', updated_at = NOW() WHERE last_login < '2022-01-01';

-- SAFE: UPDATE with WHERE and calculation
UPDATE accounts SET balance = balance - 100 WHERE id = 1;

-- ============================================================================
-- 3. DELETE with Complex Conditions
-- ============================================================================

-- VULNERABLE: DELETE with JOIN but no WHERE
-- ruleid: missing-where-002
DELETE u FROM users u JOIN orders o ON u.id = o.user_id;

-- VULNERABLE: DELETE with subquery but no WHERE
-- ruleid: missing-where-002
DELETE FROM users WHERE id IN (SELECT user_id FROM orders);

-- SAFE: DELETE with JOIN and WHERE
DELETE u FROM users u 
JOIN orders o ON u.id = o.user_id 
WHERE o.status = 'cancelled';

-- SAFE: DELETE with subquery and WHERE
DELETE FROM users WHERE id IN (SELECT user_id FROM orders WHERE status = 'cancelled');

-- ============================================================================
-- 4. UPDATE with Complex Conditions
-- ============================================================================

-- VULNERABLE: UPDATE with JOIN but no WHERE
-- ruleid: missing-where-002
UPDATE users u JOIN orders o ON u.id = o.user_id SET u.status = 'active';

-- VULNERABLE: UPDATE with CASE but no WHERE
-- ruleid: missing-where-002
UPDATE users SET status = CASE WHEN active = 1 THEN 'active' ELSE 'inactive' END;

-- SAFE: UPDATE with JOIN and WHERE
UPDATE users u 
JOIN orders o ON u.id = o.user_id 
SET u.status = 'active' 
WHERE o.status = 'completed';

-- SAFE: UPDATE with CASE and WHERE
UPDATE users 
SET status = CASE WHEN active = 1 THEN 'active' ELSE 'inactive' END 
WHERE updated_at < NOW() - INTERVAL 30 DAY;

-- ============================================================================
-- 5. DELETE with Transactions
-- ============================================================================

-- VULNERABLE: DELETE in transaction without WHERE
-- ruleid: missing-where-001
START TRANSACTION;
DELETE FROM temp_data;
COMMIT;

-- VULNERABLE: DELETE with rollback but no WHERE
-- ruleid: missing-where-001
BEGIN;
DELETE FROM users;
ROLLBACK;

-- SAFE: DELETE in transaction with WHERE
START TRANSACTION;
DELETE FROM temp_data WHERE created_date < NOW() - INTERVAL 7 DAY;
COMMIT;

-- SAFE: DELETE with rollback and WHERE
BEGIN;
DELETE FROM users WHERE status = 'deleted';
COMMIT;

-- ============================================================================
-- 6. UPDATE with Transactions
-- ============================================================================

-- VULNERABLE: UPDATE in transaction without WHERE
-- ruleid: missing-where-001
START TRANSACTION;
UPDATE users SET last_login = NOW();
COMMIT;

-- VULNERABLE: UPDATE with savepoint but no WHERE
-- ruleid: missing-where-001
BEGIN;
UPDATE accounts SET balance = 0;
SAVEPOINT sp1;

-- SAFE: UPDATE in transaction with WHERE
START TRANSACTION;
UPDATE users SET last_login = NOW() WHERE id = 1;
COMMIT;

-- SAFE: UPDATE with savepoint and WHERE
BEGIN;
UPDATE accounts SET balance = 0 WHERE status = 'closed';
SAVEPOINT sp1;

-- ============================================================================
-- 7. DELETE with Triggers
-- ============================================================================

-- VULNERABLE: DELETE in trigger without WHERE
-- ruleid: missing-where-001
CREATE TRIGGER delete_old_logs
AFTER INSERT ON logs
FOR EACH ROW
BEGIN
    DELETE FROM archive_logs;
END;

-- SAFE: DELETE in trigger with WHERE
CREATE TRIGGER delete_old_logs
AFTER INSERT ON logs
FOR EACH ROW
BEGIN
    DELETE FROM archive_logs WHERE created_date < DATE_SUB(NOW(), INTERVAL 1 YEAR);
END;

-- ============================================================================
-- 8. UPDATE with Triggers
-- ============================================================================

-- VULNERABLE: UPDATE in trigger without WHERE
-- ruleid: missing-where-001
CREATE TRIGGER update_user_status
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE user_cache SET status = NEW.status;
END;

-- SAFE: UPDATE in trigger with WHERE
CREATE TRIGGER update_user_status
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE user_cache SET status = NEW.status WHERE user_id = NEW.id;
END;

-- ============================================================================
-- 9. DELETE with Stored Procedures
-- ============================================================================

-- VULNERABLE: DELETE in procedure without WHERE
-- ruleid: missing-where-001
CREATE PROCEDURE CleanupOldData()
BEGIN
    DELETE FROM temp_tables;
END;

-- SAFE: DELETE in procedure with WHERE
CREATE PROCEDURE CleanupOldData()
BEGIN
    DELETE FROM temp_tables WHERE created_date < DATE_SUB(NOW(), INTERVAL 30 DAY);
END;

-- ============================================================================
-- 10. UPDATE with Stored Procedures
-- ============================================================================

-- VULNERABLE: UPDATE in procedure without WHERE
-- ruleid: missing-where-001
CREATE PROCEDURE ResetAllUsers()
BEGIN
    UPDATE users SET status = 'inactive';
END;

-- SAFE: UPDATE in procedure with WHERE
CREATE PROCEDURE ResetAllUsers(IN days INT)
BEGIN
    UPDATE users SET status = 'inactive' WHERE last_login < DATE_SUB(NOW(), INTERVAL days DAY);
END;

-- ============================================================================
-- 11. Bulk Operations
-- ============================================================================

-- VULNERABLE: Bulk DELETE without WHERE
-- ruleid: missing-where-001
DELETE FROM users;

-- VULNERABLE: Bulk UPDATE without WHERE
-- ruleid: missing-where-001
UPDATE products SET price = price * 1.1;

-- SAFE: Bulk DELETE with WHERE
DELETE FROM users WHERE status = 'deleted';

-- SAFE: Bulk UPDATE with WHERE
UPDATE products SET price = price * 1.1 WHERE category = 'electronics';

-- ============================================================================
-- 12. Conditional Operations
-- ============================================================================

-- VULNERABLE: DELETE with IF but no WHERE
-- ruleid: missing-where-001
DELETE FROM users IF EXISTS;

-- VULNERABLE: UPDATE with IF but no WHERE
-- ruleid: missing-where-001
UPDATE users IF EXISTS SET status = 'active';

-- SAFE: DELETE with IF and WHERE
DELETE FROM users WHERE status = 'deleted' LIMIT 100;

-- SAFE: UPDATE with IF and WHERE
UPDATE users SET status = 'active' WHERE last_login > NOW() - INTERVAL 30 DAY;

