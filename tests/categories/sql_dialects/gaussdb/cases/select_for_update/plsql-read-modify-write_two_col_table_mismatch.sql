-- @rule plsql-read-modify-write
-- @desc [已知限制] 2列表名不一致（accountsA vs accountsB）- 跨语句metavar不统一
-- @expect MATCH
DO $$ BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accountsA WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accountsB SET cnt = v_cnt WHERE id = 1;
END $$;
