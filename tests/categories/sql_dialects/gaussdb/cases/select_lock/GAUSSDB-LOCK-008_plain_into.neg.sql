-- @rule GAUSSDB-LOCK-008
-- @desc 单行锁定: 不命中
-- @expect NO_MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1 FOR UPDATE;
