-- @rule GAUSSDB-CLAUSE-004
-- @desc SELECT + ORDER BY — 应匹配
-- @expect MATCH
SELECT id, name FROM users ORDER BY name
