-- @rule GAUSSDB-LOCK-005
-- @desc [实验] 简单跨语句 SELECT+FOR UPDATE ... UPDATE
-- @expect MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1 FOR UPDATE;
UPDATE accounts SET cnt = 100 WHERE id = 1;
