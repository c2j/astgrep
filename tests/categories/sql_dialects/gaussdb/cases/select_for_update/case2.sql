-- @rule plsql-read-modify-write
-- @desc 缺 FOR UPDATE — 不应命中 (字面约束 has_lock=true)
-- @expect NO_MATCH
DO $$ BEGIN
    SELECT cnt INTO v_cnt2 FROM accounts2 WHERE id = 1;
    v_cnt2 := v_cnt2 + 1;
    UPDATE accounts2 SET cnt = v_cnt2 WHERE id = 1;
END $$;
