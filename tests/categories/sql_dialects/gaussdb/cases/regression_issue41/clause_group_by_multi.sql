-- @rule GAUSSDB-CLAUSE-003
-- @desc SELECT + GROUP BY 多键 — 应匹配
-- @expect MATCH
SELECT dept, role, COUNT(*) FROM employees GROUP BY dept, role
