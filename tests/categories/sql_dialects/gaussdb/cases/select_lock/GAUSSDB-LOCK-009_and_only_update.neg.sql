-- @rule GAUSSDB-LOCK-009
-- @desc [AND回归] DO块内仅有UPDATE SET 无SELECT FOR UPDATE 不应命中
-- @expect NO_MATCH
DO $$ BEGIN
    UPDATE accounts SET cnt = 100 WHERE id = 1;
END $$;
