-- @rule GAUSSDB-LOCK-005
-- @desc [实验] DO块简化跨语句
-- @expect MATCH
DO $$
DECLARE
    v_cnt INTEGER;
BEGIN
    SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
    UPDATE accounts SET cnt = v_cnt WHERE id = 1;
END $$;
