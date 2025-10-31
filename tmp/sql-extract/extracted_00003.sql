-- SOURCE: test/sql-1/mybatis_sample/selects.xml | line~1 | ctx=<select>
SELECT * FROM T0 WHERE a IN ( SELECT b FROM t2 WHERE t2.c1=t1.c1 AND t2.c2=1 ) ORDER BY t1.id;
