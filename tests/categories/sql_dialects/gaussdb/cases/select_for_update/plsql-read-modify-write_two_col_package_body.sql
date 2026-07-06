-- @rule plsql-read-modify-write
-- @desc 2列 PACKAGE BODY PROCEDURE RMW 模式检测
-- @expect MATCH
CREATE OR REPLACE PACKAGE BODY test_two_col_pkg AS
    PROCEDURE do_two_col_update IS
        v_cnt INTEGER;
        v_bonus NUMERIC;
    BEGIN
        SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1 FOR UPDATE;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
    END do_two_col_update;
END test_two_col_pkg;
/
