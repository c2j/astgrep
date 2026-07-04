-- @rule GAUSSDB-LOCK-010
-- @desc [反向引用] 变量名和表名一致的完整流程 应命中
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    --a := v_cnt +1;
    v_cnt := v_cnt + 1;
    --select c1 from d;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
