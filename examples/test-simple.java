public class Test {
    public void test() {
        String sql = "DELETE FROM user";
        jdbcTemplate.update(sql);
    }
}
