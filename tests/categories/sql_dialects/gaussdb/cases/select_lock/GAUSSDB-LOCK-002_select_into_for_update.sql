-- @rule GAUSSDB-LOCK-002
-- @desc 变量赋值加锁: 命中
-- @expect MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1 FOR UPDATE;
