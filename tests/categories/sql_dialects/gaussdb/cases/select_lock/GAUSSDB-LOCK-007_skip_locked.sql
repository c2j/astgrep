-- @rule GAUSSDB-LOCK-007
-- @desc 跳行锁: 命中
-- @expect MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR UPDATE SKIP LOCKED;
