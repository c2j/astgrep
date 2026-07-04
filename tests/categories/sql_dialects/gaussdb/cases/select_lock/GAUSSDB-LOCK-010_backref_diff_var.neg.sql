-- @rule GAUSSDB-LOCK-010
-- @desc [反向引用] 变量名不一致 不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_other := v_other + 1;
    UPDATE accounts SET cnt = v_other WHERE id = 1;
END $$;
