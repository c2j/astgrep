-- @rule GAUSSDB-CLAUSE-002
-- @desc SELECT 多列 — 应匹配
-- @expect MATCH
SELECT id, name, email, created_at FROM users
