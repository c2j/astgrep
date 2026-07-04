-- @rule GAUSSDB-LOCK-002
-- @desc 变量赋值无锁: 不命中
-- @expect NO_MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1;
