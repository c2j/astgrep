-- @rule plsql-read-modify-write
-- @desc 2列只命中 SELECT+UPDATE 无赋值语句不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
