-- @rule GAUSSDB-SET-001
-- @desc 3 columns SET subquery (should trigger)
-- @expect MATCH
UPDATE employees e
SET (e.salary, e.dept, e.title) = (
    SELECT s.salary, s.dept, s.title
    FROM new_data s WHERE s.id = e.id
);
