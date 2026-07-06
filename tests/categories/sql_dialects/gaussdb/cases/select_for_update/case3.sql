-- @rule plsql-read-modify-write
-- @desc [已知限制] 表名不一致（accounts3 vs accounts4）- 跨语句metavar不统一，仍命中
-- @expect MATCH
SELECT cnt INTO v_cnt3 FROM accounts3 WHERE id = 1 FOR UPDATE;
v_cnt3 := v_cnt3 + 1;
UPDATE accounts4 SET cnt = v_cnt3 WHERE id = 1;
