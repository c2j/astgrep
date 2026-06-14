-- @rule GAUSSDB-SET-001
-- @desc 5 columns SET subquery (should trigger)
-- @expect MATCH
UPDATE target t
SET (t.a, t.b, t.c, t.d, t.e) = (
    SELECT s.a, s.b, s.c, s.d, s.e
    FROM source s WHERE s.id = t.id
);
