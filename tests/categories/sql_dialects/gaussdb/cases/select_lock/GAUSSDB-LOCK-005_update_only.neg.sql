-- @rule GAUSSDB-LOCK-005
-- @desc 纯更新语句: 不命中
-- @expect NO_MATCH
UPDATE accounts SET cnt = cnt + 1 WHERE id = 1;
