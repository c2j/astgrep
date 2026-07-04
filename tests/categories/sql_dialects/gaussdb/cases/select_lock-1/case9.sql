-- @rule plsql-read-modify-write
-- @desc PACKAGE BODY 内 FUNCTION 的 RMW 模式（应命中）
-- @expect MATCH
CREATE OR REPLACE PACKAGE BODY test_pkg_func AS
    FUNCTION get_and_update RETURN INTEGER IS
        v_cnt INTEGER;
    BEGIN
        SELECT cnt INTO v_cnt FROM accounts WHERE id = 1 FOR UPDATE;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
        RETURN v_cnt;
    END get_and_update;
END test_pkg_func;
/
