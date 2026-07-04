-- @rule GAUSSDB-LOCK-006
-- @desc [实验] 缺锁反模式 简单跨语句 SELECT+INTO ... UPDATE（无FOR UPDATE）
-- @expect MATCH
SELECT cnt INTO v_count FROM accounts WHERE id = 1;
UPDATE accounts SET cnt = 100 WHERE id = 1;
