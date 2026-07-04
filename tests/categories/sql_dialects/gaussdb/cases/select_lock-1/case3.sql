-- ok: plsql-read-modify-write
-- 不命中：表名不一致（accounts3 vs accounts4）
SELECT cnt INTO v_cnt3 FROM accounts3 WHERE id = 1 FOR UPDATE;
v_cnt3 := v_cnt3 + 1;
UPDATE accounts4 SET cnt = v_cnt3 WHERE id = 1;
