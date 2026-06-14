-- @rule GAUSSDB-SET-001
-- @desc PL/pgSQL procedure: UPDATE SET 5 columns from complex JOIN subquery
-- @expect MATCH
CREATE OR REPLACE PROCEDURE sync_employee_data(p_batch_size INT DEFAULT 1000)
LANGUAGE plpgsql
AS $$
DECLARE
    v_count INT := 0;
BEGIN
    UPDATE target_employees t
    SET (
        t.salary,
        t.dept_name,
        t.title,
        t.location,
        t.updated_at
    ) = (
        SELECT
            s.salary,
            d.name AS department,
            s.position_title,
            s.city || ', ' || s.country,
            NOW()
        FROM source_employees s
        INNER JOIN departments d ON s.department_id = d.id
        LEFT JOIN cost_centers c ON d.cost_center = c.id
        WHERE s.employee_id = t.id
          AND s.updated_at > t.last_sync
          AND s.is_active = TRUE
    )
    WHERE t.needs_update = TRUE;

    GET DIAGNOSTICS v_count = ROW_COUNT;
    RAISE NOTICE 'Updated % employees', v_count;
END;
$$;
