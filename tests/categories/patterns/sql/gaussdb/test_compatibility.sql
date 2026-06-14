-- Test file for GaussDB compatibility rules
-- // MATCH: GAUSSDB-TYPE-001 (VARCHAR2)
CREATE TABLE test_varchar2 (
    id INT PRIMARY KEY,
    name VARCHAR2(100) NOT NULL
);

-- // MATCH: GAUSSDB-TYPE-002 (NUMBER)
CREATE TABLE test_number (
    amount NUMBER(10, 2)
);

-- // MATCH: GAUSSDB-STORE-001 (storage_type)
CREATE TABLE test_storage (id INT) WITH (storage_type=ustore);

-- // MATCH: GAUSSDB-CONFLICT-001 (ON CONFLICT)
INSERT INTO users (id, name) VALUES (1, 'Alice')
ON CONFLICT (id) DO UPDATE SET name = 'Alice';

-- // MATCH: GAUSSDB-PREDICT-001 (PREDICT BY)
PREDICT BY sales_model FEATURES (price, category);

-- // MATCH: GAUSSDB-TIMECAPSULE-001 (TIMECAPSULE)
TIMECAPSULE TABLE users TO TIMESTAMP now() - interval '1 hour';

-- // MATCH: GAUSSDB-SHRINK-001 (SHRINK)
SHRINK TABLE users;

-- // MATCH: GAUSSDB-SEC-001 (SELECT *)
SELECT * FROM users;

-- // MATCH: GAUSSDB-HINT-001 (Plan Hint)
SELECT /*+ indexscan(users idx_name) */ * FROM users WHERE name = 'Alice';
