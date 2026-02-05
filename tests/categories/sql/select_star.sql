-- SELECT * Usage Test Cases
-- This file contains examples of SELECT * usage and best practices

-- ============================================================================
-- 1. Basic SELECT * Issues
-- ============================================================================

-- VULNERABLE: SELECT * without column specification
-- ruleid: select-star-001
SELECT * FROM users;

-- VULNERABLE: SELECT * from sensitive table
-- ruleid: select-star-001
SELECT * FROM user_profiles;

-- VULNERABLE: SELECT * from payment data
-- ruleid: select-star-001
SELECT * FROM credit_cards;

-- VULNERABLE: SELECT * from admin table
-- ruleid: select-star-001
SELECT * FROM admin_users;

-- SAFE: SELECT specific columns
SELECT id, username, email FROM users;

-- SAFE: SELECT specific columns from sensitive table
SELECT id, name FROM user_profiles;

-- SAFE: SELECT with column aliases
SELECT u.id, u.username, u.email FROM users u;

-- ============================================================================
-- 2. SELECT * with LIMIT
-- ============================================================================

-- SAFE: SELECT * with LIMIT 1 (single row)
SELECT * FROM users LIMIT 1;

-- SAFE: SELECT * with LIMIT 10 (small result set)
SELECT * FROM logs LIMIT 10;

-- SAFE: SELECT * with LIMIT and OFFSET
SELECT * FROM products LIMIT 20 OFFSET 0;

-- VULNERABLE: SELECT * with large LIMIT
-- ruleid: select-star-001
SELECT * FROM users LIMIT 1000;

-- VULNERABLE: SELECT * with LIMIT but no number
-- ruleid: select-star-001
SELECT * FROM users LIMIT @limit_value;

-- ============================================================================
-- 3. SELECT * in Subqueries
-- ============================================================================

-- VULNERABLE: SELECT * in subquery
-- ruleid: select-star-002
SELECT * FROM (SELECT * FROM users) AS u;

-- VULNERABLE: SELECT * in JOIN subquery
-- ruleid: select-star-002
SELECT * FROM users u JOIN (SELECT * FROM orders) o ON u.id = o.user_id;

-- VULNERABLE: SELECT * in WHERE subquery
-- ruleid: select-star-002
SELECT * FROM users WHERE id IN (SELECT * FROM admin_ids);

-- SAFE: SELECT specific columns in subquery
SELECT u.id, u.username FROM (SELECT id, username FROM users) AS u;

-- SAFE: SELECT specific columns in JOIN
SELECT u.id, u.username, o.order_id 
FROM users u 
JOIN (SELECT order_id, user_id FROM orders) o ON u.id = o.user_id;

-- ============================================================================
-- 4. SELECT * with WHERE Clause
-- ============================================================================

-- VULNERABLE: SELECT * with WHERE but no column limit
-- ruleid: select-star-001
SELECT * FROM users WHERE active = 1;

-- VULNERABLE: SELECT * with complex WHERE
-- ruleid: select-star-001
SELECT * FROM orders WHERE status = 'pending' AND created_date > '2023-01-01';

-- SAFE: SELECT specific columns with WHERE
SELECT id, username, email FROM users WHERE active = 1;

-- SAFE: SELECT specific columns with complex WHERE
SELECT order_id, customer_name, total 
FROM orders 
WHERE status = 'pending' AND created_date > '2023-01-01';

-- ============================================================================
-- 5. SELECT * with JOIN
-- ============================================================================

-- VULNERABLE: SELECT * with JOIN
-- ruleid: select-star-001
SELECT * FROM users u JOIN orders o ON u.id = o.user_id;

-- VULNERABLE: SELECT * with multiple JOINs
-- ruleid: select-star-001
SELECT * 
FROM users u 
JOIN orders o ON u.id = o.user_id 
JOIN products p ON o.product_id = p.id;

-- VULNERABLE: SELECT * with LEFT JOIN
-- ruleid: select-star-001
SELECT * FROM users u LEFT JOIN profiles p ON u.id = p.user_id;

-- SAFE: SELECT specific columns with JOIN
SELECT u.id, u.username, o.order_id, o.total
FROM users u 
JOIN orders o ON u.id = o.user_id;

-- SAFE: SELECT specific columns with multiple JOINs
SELECT u.id, u.username, o.order_id, p.product_name
FROM users u 
JOIN orders o ON u.id = o.user_id 
JOIN products p ON o.product_id = p.id;

-- ============================================================================
-- 6. SELECT * with GROUP BY
-- ============================================================================

-- VULNERABLE: SELECT * with GROUP BY
-- ruleid: select-star-001
SELECT * FROM orders GROUP BY user_id;

-- VULNERABLE: SELECT * with GROUP BY and aggregation
-- ruleid: select-star-001
SELECT *, COUNT(*) FROM orders GROUP BY user_id;

-- SAFE: SELECT specific columns with GROUP BY
SELECT user_id, COUNT(*) as order_count FROM orders GROUP BY user_id;

-- SAFE: SELECT aggregated columns
SELECT user_id, SUM(total) as total_spent FROM orders GROUP BY user_id;

-- ============================================================================
-- 7. SELECT * with ORDER BY
-- ============================================================================

-- VULNERABLE: SELECT * with ORDER BY
-- ruleid: select-star-001
SELECT * FROM users ORDER BY created_date DESC;

-- VULNERABLE: SELECT * with multiple ORDER BY
-- ruleid: select-star-001
SELECT * FROM users ORDER BY last_login DESC, created_date ASC;

-- SAFE: SELECT specific columns with ORDER BY
SELECT id, username, email FROM users ORDER BY created_date DESC;

-- SAFE: SELECT with ORDER BY and LIMIT
SELECT id, username FROM users ORDER BY created_date DESC LIMIT 10;

-- ============================================================================
-- 8. SELECT * with DISTINCT
-- ============================================================================

-- VULNERABLE: SELECT DISTINCT *
-- ruleid: select-star-001
SELECT DISTINCT * FROM users;

-- VULNERABLE: SELECT DISTINCT * with WHERE
-- ruleid: select-star-001
SELECT DISTINCT * FROM orders WHERE status = 'completed';

-- SAFE: SELECT DISTINCT specific columns
SELECT DISTINCT user_id, status FROM orders;

-- SAFE: SELECT DISTINCT with WHERE
SELECT DISTINCT username FROM users WHERE active = 1;

-- ============================================================================
-- 9. SELECT * in UNION
-- ============================================================================

-- VULNERABLE: SELECT * in UNION
-- ruleid: select-star-003
SELECT * FROM users UNION SELECT * FROM admin_users;

-- VULNERABLE: SELECT * in UNION ALL
-- ruleid: select-star-003
SELECT * FROM current_users UNION ALL SELECT * FROM archived_users;

-- SAFE: SELECT specific columns in UNION
SELECT id, username FROM users UNION SELECT id, username FROM admin_users;

-- SAFE: SELECT specific columns in UNION ALL
SELECT id, name FROM current_users UNION ALL SELECT id, name FROM archived_users;

-- ============================================================================
-- 10. SELECT * with Window Functions
-- ============================================================================

-- VULNERABLE: SELECT * with window function
-- ruleid: select-star-001
SELECT *, ROW_NUMBER() OVER (ORDER BY created_date) as row_num FROM users;

-- VULNERABLE: SELECT * with PARTITION BY
-- ruleid: select-star-001
SELECT *, SUM(amount) OVER (PARTITION BY user_id) as total FROM transactions;

-- SAFE: SELECT specific columns with window function
SELECT id, username, ROW_NUMBER() OVER (ORDER BY created_date) as row_num FROM users;

-- SAFE: SELECT specific columns with PARTITION BY
SELECT user_id, amount, SUM(amount) OVER (PARTITION BY user_id) as total FROM transactions;

-- ============================================================================
-- 11. SELECT * in CTE (Common Table Expression)
-- ============================================================================

-- VULNERABLE: SELECT * in CTE
-- ruleid: select-star-001
WITH user_data AS (SELECT * FROM users)
SELECT * FROM user_data;

-- VULNERABLE: SELECT * in CTE definition
-- ruleid: select-star-001
WITH active_users AS (SELECT * FROM users WHERE active = 1)
SELECT * FROM active_users;

-- SAFE: SELECT specific columns in CTE
WITH user_data AS (SELECT id, username FROM users)
SELECT id, username FROM user_data;

-- SAFE: SELECT specific columns in CTE definition
WITH active_users AS (SELECT id, username FROM users WHERE active = 1)
SELECT id, username FROM active_users;

-- ============================================================================
-- 12. SELECT * Performance Impact
-- ============================================================================

-- VULNERABLE: SELECT * from large table
-- ruleid: select-star-001
SELECT * FROM transaction_history;

-- VULNERABLE: SELECT * with unnecessary columns
-- ruleid: select-star-001
SELECT * FROM users WHERE id = 1;

-- SAFE: SELECT only needed columns
SELECT id, username, email FROM users WHERE id = 1;

-- SAFE: SELECT with column projection
SELECT id, name, email FROM users WHERE id = 1;

