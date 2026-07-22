-- @rule GAUSSDB-CLAUSE-002
-- @desc SELECT 单列 — 应匹配
-- @expect MATCH
SELECT id FROM users
