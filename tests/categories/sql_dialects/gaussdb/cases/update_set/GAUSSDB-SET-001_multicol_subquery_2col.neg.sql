-- @rule GAUSSDB-SET-001
-- @desc 2 columns SET subquery (within limit, should NOT trigger)
-- @expect NO_MATCH
UPDATE employees e
SET (e.salary, e.dept) = (
    SELECT s.salary, s.dept
    FROM new_data s WHERE s.id = e.id
);
