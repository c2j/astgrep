-- @rule GAUSSDB-SET-001
-- @desc Multi-line SET with 4 columns across lines (should trigger)
-- @expect MATCH
UPDATE target_table t
SET (
    t.col_a,
    t.col_b,
    t.col_c,
    t.col_d
) = (
    SELECT s.col_a, s.col_b, s.col_c, s.col_d
    FROM source_table s
    WHERE s.id = t.id
);
