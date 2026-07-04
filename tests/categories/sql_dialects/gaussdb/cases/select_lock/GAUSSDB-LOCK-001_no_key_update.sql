-- @rule GAUSSDB-LOCK-001
-- @desc 弱锁: 命中
-- @expect MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR NO KEY UPDATE;
