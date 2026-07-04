-- @rule plsql-read-modify-write
-- @desc 完整三语句 RMW 模式检测
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
