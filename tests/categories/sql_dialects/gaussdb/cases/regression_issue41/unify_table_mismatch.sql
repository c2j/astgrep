-- @rule GAUSSDB-UNIFY-001
-- @desc SELECT + UPDATE 不同表 — 不应匹配
-- @expect NO_MATCH
SELECT id, name FROM accounts WHERE status = 'active';
UPDATE orders SET balance = balance + 100
