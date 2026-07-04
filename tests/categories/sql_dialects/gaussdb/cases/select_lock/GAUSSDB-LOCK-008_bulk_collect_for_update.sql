-- @rule GAUSSDB-LOCK-008
-- @desc 批量锁定: 命中
-- @expect MATCH
SELECT cnt BULK COLLECT INTO v_counts FROM accounts WHERE status = 'active' FOR UPDATE;
