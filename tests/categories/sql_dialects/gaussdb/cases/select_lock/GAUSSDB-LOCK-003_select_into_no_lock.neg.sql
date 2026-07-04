-- @rule GAUSSDB-LOCK-003
-- @desc 同语句赋值无锁: 不命中
-- @expect NO_MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1;
