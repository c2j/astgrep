-- @rule GAUSSDB-CLAUSE-006
-- @desc HAVING 聚合条件 — 应匹配
-- @expect MATCH
SELECT COUNT(*) FROM orders GROUP BY status HAVING COUNT(*) > 10
