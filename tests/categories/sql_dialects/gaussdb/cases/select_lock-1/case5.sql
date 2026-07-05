-- ok: plsql-read-modify-write
-- 命中


PROCEDURE simple_config_update()
IS
BEGIN
    SELECT cnt INTO v_cnt7 FROM accounts7 WHERE id = 1 FOR UPDATE;
    v_cnt7 := v_cnt7 + 1;
    UPDATE accounts7 SET cnt = v_cnt7 WHERE id = 1;

END;
