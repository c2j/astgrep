-- @rule plsql-read-modify-write
-- @desc CREATE FUNCTION 内缺少 FOR UPDATE（不应命中）
-- @expect NO_MATCH
CREATE OR REPLACE FUNCTION simple_func_no_lock()
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END;
$$;
