-- @rule plsql-read-modify-write
-- @desc PACKAGE BODY 内 PROCEDURE 的 RMW 模式（应命中）
-- @expect MATCH
CREATE OR REPLACE PACKAGE BODY test_pkg AS
    PROCEDURE do_update IS
        v_cnt INTEGER;
    BEGIN
        SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
    END do_update;
END test_pkg;
/
