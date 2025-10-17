public class SqlCaseExample {
    
    public void badSqlCase() {
        // ruleid: DB-SQ-10
        String sql1 = "select id, name from user where status = 1";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ruleid: DB-SQ-10
        String sql2 = "insert into user (name, email) values (?, ?)";
        jdbcTemplate.update(sql2, name, email);
        
        // ruleid: DB-SQ-10
        String sql3 = "update user set status = 1 where id = ?";
        jdbcTemplate.update(sql3, id);
        
        // ruleid: DB-SQ-10
        String sql4 = "delete from user where status = 0";
        jdbcTemplate.update(sql4);
    }
    
    public void goodSqlCase() {
        // ok: DB-SQ-10
        String sql1 = "SELECT id, name FROM user WHERE status = 1";
        jdbcTemplate.query(sql1, rowMapper);
        
        // ok: DB-SQ-10
        String sql2 = "INSERT INTO user (name, email) VALUES (?, ?)";
        jdbcTemplate.update(sql2, name, email);
        
        // ok: DB-SQ-10
        String sql3 = "UPDATE user SET status = 1 WHERE id = ?";
        jdbcTemplate.update(sql3, id);
        
        // ok: DB-SQ-10
        String sql4 = "SELECT u.id, u.name FROM user u LEFT JOIN profile p ON u.id = p.user_id";
        jdbcTemplate.query(sql4, rowMapper);
    }
}
