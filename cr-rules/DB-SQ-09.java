public class JoinConditionExample {
    
    public void badJoinWithoutCondition() {
        // ruleid: DB-SQ-09
        String sql1 = "SELECT * FROM user u JOIN order o WHERE u.status = 1";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ruleid: DB-SQ-09
        String sql2 = "SELECT u.name, o.amount FROM user u INNER JOIN order o GROUP BY u.id";
        jdbcTemplate.query(sql2, rowMapper);
        
        // ruleid: DB-SQ-09
        String sql3 = "SELECT * FROM user u LEFT JOIN profile p ORDER BY u.id";
        jdbcTemplate.query(sql3, rowMapper);
        
        // ruleid: DB-SQ-09
        String sql4 = "SELECT * FROM user u RIGHT JOIN department d";
        jdbcTemplate.query(sql4, rowMapper);
    }
    
    public void goodJoinWithCondition() {
        // ok: DB-SQ-09
        String sql1 = "SELECT * FROM user u JOIN order o ON u.id = o.user_id WHERE u.status = 1";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ok: DB-SQ-09
        String sql2 = "SELECT u.name, o.amount FROM user u INNER JOIN order o ON u.id = o.user_id";
        jdbcTemplate.query(sql2, rowMapper);
        
        // ok: DB-SQ-09
        String sql3 = "SELECT * FROM user u LEFT JOIN profile p ON u.id = p.user_id";
        jdbcTemplate.query(sql3, rowMapper);
        
        // ok: DB-SQ-09
        String sql4 = "SELECT * FROM user u RIGHT JOIN department d USING (dept_id)";
        jdbcTemplate.query(sql4, rowMapper);
    }
}
