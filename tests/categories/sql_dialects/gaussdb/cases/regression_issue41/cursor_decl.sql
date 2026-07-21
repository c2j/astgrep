-- @rule GAUSSDB-CURSOR-001
-- @desc 游标声明 — 应匹配
-- @expect MATCH
CURSOR cur_users FOR SELECT id, name FROM users
