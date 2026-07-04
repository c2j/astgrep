-- @rule GAUSSDB-LOCK-006
-- @desc [实验] 缺锁反模式DO块
-- @expect MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
