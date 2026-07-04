-- @rule GAUSSDB-LOCK-006
-- @desc [已知限制] 缺lookahead 有锁也被击中
-- @expect MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
