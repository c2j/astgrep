-- @rule GAUSSDB-UNIFY-001
-- @desc SELECT + UPDATE 同表 — 应匹配
-- @expect MATCH
SELECT id, name FROM accounts WHERE status = 'active';
UPDATE accounts SET balance = balance + 100
