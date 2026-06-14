-- @rule GAUSSDB-SET-001
-- @desc PL/pgSQL procedure: UPDATE SET 3 columns from subquery with WHERE filter
-- @expect MATCH
CREATE OR REPLACE PROCEDURE batch_update_employees()
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE employees e
    SET (e.salary, e.department, e.title) = (
        SELECT s.salary, s.department, s.title
        FROM employee_staging s
        WHERE s.employee_id = e.id
          AND s.sync_status = 'pending'
    )
    WHERE e.status = 'active';
END;
$$;
