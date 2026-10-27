-- What if orders streamed in instead of loading nightly? Try it in a zero-copy clone, never in FILAGREE itself.
-- Run after 00_setup.sql and 10_marts.sql.

USE WAREHOUSE FILAGREE_WH;

-- The clone shares FILAGREE's storage until something changes, so it is instant and free to make.
-- The SNOWFLAKE_SAMPLE_DATA share itself is read-only and cannot be cloned; the clone keeps reading it.
CREATE DATABASE IF NOT EXISTS FILAGREE_WHATIF CLONE FILAGREE;

-- Cloned procedures and tasks keep their fully qualified FILAGREE names, so calling
-- FILAGREE_WHATIF.MART.REFRESH_MARTS would rebuild the real marts. Remove the nightly
-- task from the clone (clones of tasks start suspended) and never call the cloned procedure.
DROP TASK IF EXISTS FILAGREE_WHATIF.MART.REFRESH_MARTS_DAILY;

-- The nightly rebuild of REVENUE_BY_NATION_MONTH, replaced by a dynamic table that keeps
-- itself within a minute of its sources. Same SELECT as REFRESH_MARTS in 10_marts.sql.
-- The share has no change tracking we can turn on, so REFRESH_MODE = FULL. Run on 2026-10-26,
-- this created directly over the share in 4.7s (no need to copy the five tables into the clone).
-- The first refresh runs at creation, so the comparisons below work straight away.
CREATE OR REPLACE DYNAMIC TABLE FILAGREE_WHATIF.MART.REVENUE_BY_NATION_MONTH_LIVE
  TARGET_LAG = '1 minute'
  WAREHOUSE = FILAGREE_WH
  REFRESH_MODE = FULL
AS
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

-- Compare: same row count and the same total net revenue as the nightly mart.
-- 2026-10-26: both 10,000 rows, both 218,102,223,885.0001.
SELECT 'nightly' AS mart, COUNT(*) AS row_count, SUM(net_revenue) AS net_revenue
FROM FILAGREE.MART.REVENUE_BY_NATION_MONTH
UNION ALL
SELECT 'dynamic', COUNT(*), SUM(net_revenue)
FROM FILAGREE_WHATIF.MART.REVENUE_BY_NATION_MONTH_LIVE;

-- Row by row: expect 0 (got 0 on 2026-10-26).
SELECT COUNT(*) AS mismatched_rows
FROM (
  SELECT * FROM FILAGREE.MART.REVENUE_BY_NATION_MONTH
  MINUS
  SELECT * FROM FILAGREE_WHATIF.MART.REVENUE_BY_NATION_MONTH_LIVE
);

-- Target lag, refresh mode and scheduling state (2026-10-26: 1 minute, FULL, ACTIVE).
SHOW DYNAMIC TABLES IN DATABASE FILAGREE_WHATIF;

-- The sample share never changes, so later refreshes only re-read 6M lines and spend credits.
-- Stop them once compared. The clone and its 10,000 rows stay for the demo.
ALTER DYNAMIC TABLE FILAGREE_WHATIF.MART.REVENUE_BY_NATION_MONTH_LIVE SUSPEND;

-- Clean up when the what-if is done:
-- DROP DATABASE FILAGREE_WHATIF;
