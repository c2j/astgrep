-- @rule plsql-read-modify-write
-- @desc 2列缺少 FOR UPDATE 不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
