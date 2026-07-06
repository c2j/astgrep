-- @rule plsql-read-modify-write
-- @desc PACKAGE BODY 内 PROCEDURE 缺少 FOR UPDATE（不应命中）
-- @expect NO_MATCH
CREATE OR REPLACE PACKAGE BODY test_pkg_no_lock AS
    PROCEDURE do_update_no_lock IS
        v_cnt INTEGER;
    BEGIN
        SELECT cnt INTO v_cnt FROM accounts WHERE id = 1;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
    END do_update_no_lock;
END test_pkg_no_lock;
/
