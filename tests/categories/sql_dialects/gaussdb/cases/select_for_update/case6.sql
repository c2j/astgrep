-- @rule plsql-read-modify-write
-- @desc CREATE FUNCTION 内 RMW 模式（应命中）
-- @expect MATCH
CREATE OR REPLACE FUNCTION simple_func()
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END;
$$;
