-- @rule GAUSSDB-LOCK-009
-- @desc [AND修复] DO块内缺FOR UPDATE
-- @expect NO_MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
