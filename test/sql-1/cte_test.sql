WITH my_cte AS (
  SELECT one, two
  FROM my_table
)
SELECT *
FROM my_cte;

-- 也会命中（多个 CTE）
WITH
  cte1 AS (SELECT id FROM t1),
  cte2 AS (SELECT id FROM t2)
SELECT * FROM cte1 JOIN cte2 USING (id);

