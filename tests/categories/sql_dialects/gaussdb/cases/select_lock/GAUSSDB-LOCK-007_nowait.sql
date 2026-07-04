-- @rule GAUSSDB-LOCK-007
-- @desc 立即返回锁: 命中
-- @expect MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR UPDATE NOWAIT;
