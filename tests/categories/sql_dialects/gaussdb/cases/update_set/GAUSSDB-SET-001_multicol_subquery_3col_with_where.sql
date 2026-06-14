-- @rule GAUSSDB-SET-001
-- @desc UPDATE SET 3+ columns WITH WHERE clause (should still trigger)
-- @expect MATCH
UPDATE employees e
SET (e.salary, e.dept, e.title) = (
    SELECT s.salary, s.dept, s.title
    FROM new_data s WHERE s.id = e.id
)
WHERE e.status = 'active';
