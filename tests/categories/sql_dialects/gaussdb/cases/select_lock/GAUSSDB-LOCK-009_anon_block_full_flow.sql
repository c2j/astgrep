-- @rule GAUSSDB-LOCK-009
-- @desc [实验] 匿名块内三语句共存
-- @expect MATCH
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    v_cnt := v_cnt + 1;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END;
