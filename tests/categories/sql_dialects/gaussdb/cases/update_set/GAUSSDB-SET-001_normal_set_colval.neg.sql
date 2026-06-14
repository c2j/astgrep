-- @rule GAUSSDB-SET-001
-- @desc Normal SET col=val (no tuple, should NOT trigger)
-- @expect NO_MATCH
UPDATE employees SET salary = 100, dept = 'IT', title = 'Engineer' WHERE id = 1;
