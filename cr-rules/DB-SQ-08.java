public class SelectStarExample {
    
    public void badSelectStar() {
        // ruleid: DB-SQ-08
        String sql1 = "SELECT * FROM user";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ruleid: DB-SQ-08
        String sql2 = "select * from order";
        jdbcTemplate.query(sql2, rowMapper);
        
        // ruleid: DB-SQ-08 - 嵌套查询中的 SELECT *
        String sql3 = "SELECT id FROM user WHERE id IN (SELECT * FROM temp_user)";
        jdbcTemplate.query(sql3, rowMapper);
    }
    
    public void goodSelectSpecific() {
        // ok: DB-SQ-08
        String sql1 = "SELECT id, name, email FROM user";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ok: DB-SQ-08
        String sql2 = "SELECT u.id, u.name FROM user u WHERE u.status = 1";
        jdbcTemplate.query(sql2, rowMapper);
        
        // ok: DB-SQ-08 - 嵌套查询指定字段
        String sql3 = "SELECT id FROM user WHERE id IN (SELECT user_id FROM temp_user)";
        jdbcTemplate.query(sql3, rowMapper);
    }
}
