-- ok: plsql-read-modify-write
-- 命中


CREATE OR REPLACE PROCEDURE simple_config_update()
LANGUAGE plpgsql
AS $$
BEGIN
    SELECT cnt INTO v_cnt7 FROM accounts7 WHERE id = 1 FOR UPDATE;
    v_cnt7 := v_cnt7 + 1;
    UPDATE accounts7 SET cnt = v_cnt7 WHERE id = 1;
    UPDATE config_table c
    SET (c.value, c.updated_at) = (
        SELECT s.value, NOW()
        FROM staging_config s
        WHERE s.config_key = c.config_key
    )
    WHERE c.is_frozen = FALSE;
END;
$$;
