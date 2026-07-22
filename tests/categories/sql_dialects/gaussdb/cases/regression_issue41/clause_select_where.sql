-- @rule GAUSSDB-CLAUSE-001
-- @desc SELECT + WHERE 单列 — 应匹配
-- @expect MATCH
SELECT id FROM users WHERE id = 1
