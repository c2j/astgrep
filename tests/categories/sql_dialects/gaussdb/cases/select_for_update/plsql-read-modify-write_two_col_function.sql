-- @rule plsql-read-modify-write
-- @desc 2列 CREATE FUNCTION RMW 模式检测
-- @expect MATCH
CREATE OR REPLACE FUNCTION two_col_func()
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_cnt INTEGER;
    v_bonus NUMERIC;
BEGIN
    SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END;
$$;
