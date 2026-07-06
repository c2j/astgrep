-- @rule plsql-read-modify-write
-- @desc [已知限制] 2列 WHERE 条件不一致（id=1 vs id=2）- 跨语句metavar不统一
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 2;
END $$;
