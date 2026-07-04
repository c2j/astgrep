-- @rule GAUSSDB-LOCK-009
-- @desc [AND回归] DO块内仅有赋值和UPDATE 无SELECT FOR UPDATE 不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
