-- @rule plsql-read-modify-write
-- @desc [已知限制] 变量名不一致（v_cnt5 vs v_cnt6）- 跨语句metavar不统一，仍命中
-- @expect MATCH
SELECT cnt INTO v_cnt5 FROM accounts5 WHERE id = 1 FOR UPDATE;
v_cnt5 := v_cnt5 + 1;
UPDATE accounts5 SET cnt = v_cnt6 WHERE id = 1;
