-- @rule GAUSSDB-SET-001
-- @desc PL/pgSQL procedure: UPDATE SET 2 columns (within limit, should NOT trigger)
-- @expect NO_MATCH
CREATE OR REPLACE PROCEDURE simple_config_update()
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE config_table c
    SET (c.value, c.updated_at) = (
        SELECT s.value, NOW()
        FROM staging_config s
        WHERE s.config_key = c.config_key
    )
    WHERE c.is_frozen = FALSE;
END;
$$;
