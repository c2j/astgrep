-- @rule GAUSSDB-LOCK-001
-- @desc 共享锁: 命中
-- @expect MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR SHARE;
