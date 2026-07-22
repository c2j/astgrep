-- @rule GAUSSDB-CLAUSE-005
-- @desc SELECT + GROUP BY + HAVING — 应匹配
-- @expect MATCH
SELECT id, COUNT(*) FROM users GROUP BY id HAVING COUNT(*) > 1
