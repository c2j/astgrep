-- @rule privilege-escalation-001
-- @desc Safe SQL that should not trigger
-- @expect NO_MATCH

SELECT 1;
