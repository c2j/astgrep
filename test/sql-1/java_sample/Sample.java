package demo;

public class Sample {
    public void run(java.sql.Connection conn) throws Exception {
        String sqlPart = "select * from t1 where exists (select 1 from t2 where t2.c1=t1.c1 and t2.c2=1)";
        String sql = sqlPart + " and t1.id>1000000 order by t1.id";
        conn.prepareStatement(sql);
        System.out.println("  Hello World");
    }

    @org.apache.ibatis.annotations.Select("SELECT * FROM t1 WHERE a IN (SELECT b FROM t2 WHERE t2.c1=t1.c1 AND t2.c2=1) ORDER BY t1.id")
    public java.util.List<String> mapperLike() { return null; }
}

