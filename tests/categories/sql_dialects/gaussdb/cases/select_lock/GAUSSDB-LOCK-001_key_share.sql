-- @rule GAUSSDB-LOCK-001
-- @desc 键共享: 命中
-- @expect MATCH
SELECT cnt FROM accounts WHERE id = 1 FOR KEY SHARE;
