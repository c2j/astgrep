-- @rule plsql-read-modify-write
-- @desc PACKAGE BODY 内 FUNCTION 的 RMW 模式（应命中）
-- @expect MATCH
--CREATE OR REPLACE PACKAGE BODY test_pkg_func AS
    procedure get_and_update(in_var in varchar2) IS
        v_cnt number;
    BEGIN
        begin
            SELECT d.cnt INTO v_cnt FROM accounts d WHERE d.id = in_var FOR UPDATE;
            v_cnt := v_cnt + 1;
            UPDATE accounts  d SET d.cnt = v_cnt WHERE d.id = in_var;

        exception when others then v_cnt := 0;
        end;
        return;
    END;
--END test_pkg_func;
