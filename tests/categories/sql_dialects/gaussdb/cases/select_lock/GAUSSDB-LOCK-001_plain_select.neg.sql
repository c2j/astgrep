-- @rule GAUSSDB-LOCK-001
-- @desc 无锁语句: 不命中
-- @expect NO_MATCH
SELECT cnt FROM accounts WHERE id = 1;
