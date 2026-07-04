-- @rule GAUSSDB-LOCK-004
-- @desc [已修复] DO块完整流程 反向引用
-- @expect MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
