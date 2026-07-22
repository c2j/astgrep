-- @rule GAUSSDB-CLAUSE-003
-- @desc SELECT + GROUP BY — 应匹配
-- @expect MATCH
SELECT dept, COUNT(*) FROM employees GROUP BY dept
