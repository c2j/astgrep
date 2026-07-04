-- @rule GAUSSDB-LOCK-009
-- @desc [AND回归] DO块内同时有SELECT INTO FOR UPDATE+赋值+UPDATE 应命中
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
