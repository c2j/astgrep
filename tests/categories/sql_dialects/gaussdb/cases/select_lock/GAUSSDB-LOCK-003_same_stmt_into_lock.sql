-- @rule GAUSSDB-LOCK-003
-- @desc 同语句赋值加锁: 命中
-- @expect MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1 FOR UPDATE;
