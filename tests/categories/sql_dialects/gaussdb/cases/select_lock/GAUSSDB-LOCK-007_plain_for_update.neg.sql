-- @rule GAUSSDB-LOCK-007
-- @desc 标准行锁: 不命中
-- @expect NO_MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR UPDATE;
