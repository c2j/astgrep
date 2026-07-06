-- @rule plsql-read-modify-write
-- @desc [已知限制] 2列变量名不一致（v_cnt vs v_other）- 跨语句metavar不统一
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_other WHERE id = 1;
END $$;
