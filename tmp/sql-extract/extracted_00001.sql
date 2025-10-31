-- SOURCE: test/sql-1/java_sample/Sample.java | line~10 | ctx=@Select
SELECT * FROM t1 WHERE a IN (SELECT b FROM t2 WHERE t2.c1=t1.c1 AND t2.c2=1) ORDER BY t1.id;
