-- @rule plsql-read-modify-write
-- @desc 2列 DO 块 RMW 模式检测
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
