-- @rule GAUSSDB-LOCK-010
-- @desc [反向引用] 表名不一致 不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE other_table SET cnt = v_cnt WHERE id = 1;
END $$;
