-- @rule GAUSSDB-LOCK-004
-- @desc 无锁流程: 不命中
-- @expect NO_MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
