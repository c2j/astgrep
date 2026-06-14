-- @rule GAUSSDB-SET-001
-- @desc PL/pgSQL procedure: UPDATE SET 4 columns with CTE subquery and IF branch
-- @expect MATCH
CREATE OR REPLACE PROCEDURE reconcile_inventory(p_warehouse_id INT)
LANGUAGE plpgsql
AS $$
BEGIN
    IF p_warehouse_id IS NOT NULL THEN
        UPDATE inventory_target t
        SET (
            t.quantity,
            t.unit_cost,
            t.supplier_id,
            t.last_reconciled
        ) = (
            WITH latest_stock AS (
                SELECT
                    product_id,
                    SUM(quantity) AS total_qty,
                    AVG(unit_cost) AS avg_cost,
                    MAX(supplier_id) AS primary_supplier,
                    MAX(received_date) AS last_date
                FROM stock_movements
                WHERE warehouse_id = p_warehouse_id
                  AND movement_type = 'IN'
                  AND received_date >= CURRENT_DATE - INTERVAL '30 days'
                GROUP BY product_id
            )
            SELECT ls.total_qty, ls.avg_cost, ls.primary_supplier, ls.last_date
            FROM latest_stock ls
            WHERE ls.product_id = t.product_id
        )
        WHERE t.warehouse_id = p_warehouse_id
          AND t.auto_reconcile = TRUE;
    END IF;
END;
$$;
