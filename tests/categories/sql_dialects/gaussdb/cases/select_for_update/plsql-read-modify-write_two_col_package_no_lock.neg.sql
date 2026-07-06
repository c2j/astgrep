-- @rule plsql-read-modify-write
-- @desc 2列 PACKAGE BODY 缺少 FOR UPDATE 不应命中
-- @expect NO_MATCH
CREATE OR REPLACE PACKAGE BODY test_two_col_no_lock AS
    PROCEDURE do_two_col_no_lock IS
        v_cnt INTEGER;
        v_bonus NUMERIC;
    BEGIN
        SELECT cnt, bonus INTO v_cnt, v_bonus FROM accounts WHERE id = 1;
        v_cnt := v_cnt + 1;
        UPDATE accounts SET cnt = v_cnt WHERE id = 1;
    END do_two_col_no_lock;
END test_two_col_no_lock;
/
