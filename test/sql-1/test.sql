select * from t1 where exists (select 1 from t2 where t2.c1=t1.c1 and t2.c2=1) and t1.id>1000000 order by t1.id;
select * from t1 where a in  (select b from t2 where t2.c1=t1.c1 and t2.c2=1) and t1.id>1000000 order by t1.id;

select * from t1;

select * from t1 where t1.c1='a';

SELECT * 
from t1 
where EXISTS (select 1 from t2 where t2.c1=t1.c1 and t2.c2=1) and t1.id>1000000 order by t1.id;
