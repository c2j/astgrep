public class UnsafeSqlInJava {
    public void deleteAll() {
        // ruleid: DB-SQ-02
        String sql1 = "DELETE FROM user";
        jdbcTemplate.update(sql1);
    }

    public void updateAll() {
        // ruleid: DB-SQ-02
        String sql2 = "UPDATE user SET status = 1";
        jdbcTemplate.update(sql2);
    }

    public void safeUpdate() {
        // ok: DB-SQ-02
        String sql3 = "UPDATE user SET status = 1 WHERE id = ?";
        jdbcTemplate.update(sql3, 42);
    }
}