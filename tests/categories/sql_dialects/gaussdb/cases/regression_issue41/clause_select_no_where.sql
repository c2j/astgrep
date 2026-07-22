-- @rule GAUSSDB-CLAUSE-001
-- @desc SELECT 无 WHERE — 不应匹配（模式要求 WHERE $...COND）
-- @expect NO_MATCH
SELECT id FROM users
