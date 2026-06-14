-- @rule GAUSSDB-SET-001
-- @desc PL/pgSQL procedure: simple SET col=val (no tuple assignment, should NOT trigger)
-- @expect NO_MATCH
CREATE OR REPLACE PROCEDURE update_single_field(p_emp_id INT, p_salary DECIMAL)
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE employees
    SET salary = p_salary
    WHERE id = p_emp_id;
END;
$$;
