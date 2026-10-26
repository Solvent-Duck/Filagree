-- Two marts built from SNOWFLAKE_SAMPLE_DATA.TPCH_SF1, rebuilt daily by one task.

CREATE OR REPLACE PROCEDURE FILAGREE.MART.REFRESH_MARTS()
RETURNS STRING
LANGUAGE SQL
AS
$$
BEGIN
  -- Net revenue per region, nation, month and market segment.
  CREATE OR REPLACE TABLE FILAGREE.MART.REVENUE_BY_NATION_MONTH AS
  SELECT
    r.r_name                                    AS region,
    n.n_name                                    AS nation,
    DATE_TRUNC('month', o.o_orderdate)          AS month,
    c.c_mktsegment                              AS segment,
    COUNT(DISTINCT o.o_orderkey)                AS orders,
    SUM(l.l_extendedprice * (1 - l.l_discount)) AS net_revenue
  FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.LINEITEM l
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.ORDERS   o ON o.o_orderkey  = l.l_orderkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.CUSTOMER c ON c.c_custkey   = o.o_custkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.NATION   n ON n.n_nationkey = c.c_nationkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.REGION   r ON r.r_regionkey = n.n_regionkey
  GROUP BY ALL;

  -- One row per supplier: what they ship, how late, and how much it earns.
  CREATE OR REPLACE TABLE FILAGREE.MART.SUPPLIER_SCORECARD AS
  SELECT
    s.s_suppkey                                                  AS supplier_key,
    s.s_name                                                     AS supplier,
    n.n_name                                                     AS nation,
    COUNT(DISTINCT p.p_partkey)                                  AS parts_shipped,
    SUM(l.l_extendedprice * (1 - l.l_discount))                  AS net_revenue,
    SUM(l.l_extendedprice * (1 - l.l_discount))
      - SUM(ps.ps_supplycost * l.l_quantity)                     AS margin,
    AVG(IFF(l.l_receiptdate > l.l_commitdate, 1, 0))             AS late_share
  FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.LINEITEM l
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.SUPPLIER s  ON s.s_suppkey   = l.l_suppkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.PARTSUPP ps ON ps.ps_partkey = l.l_partkey
                                                 AND ps.ps_suppkey = l.l_suppkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.PART     p  ON p.p_partkey   = l.l_partkey
  JOIN SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.NATION   n  ON n.n_nationkey = s.s_nationkey
  GROUP BY ALL;

  RETURN 'marts rebuilt';
END;
$$;

CREATE OR REPLACE TASK FILAGREE.MART.REFRESH_MARTS_DAILY
  WAREHOUSE = FILAGREE_WH
  SCHEDULE = 'USING CRON 0 6 * * * UTC'
AS
  CALL FILAGREE.MART.REFRESH_MARTS();

-- Build now; resume the task only if you want the daily schedule.
CALL FILAGREE.MART.REFRESH_MARTS();
