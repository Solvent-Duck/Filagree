-- Open-weight AI inside Snowflake: Llama 3.3 70B via Cortex AI_COMPLETE.
-- If llama3.3-70b is not in your region, swap in mistral-large3 or qwen3-32b,
-- or enable cross-region inference (ACCOUNTADMIN):
--   ALTER ACCOUNT SET CORTEX_ENABLED_CROSS_REGION = 'ANY_REGION';

-- Plain-English docs for every source table and mart, written from the column catalog.
CREATE OR REPLACE TABLE FILAGREE.AI.TABLE_DOCS AS
WITH catalog AS (
  SELECT table_catalog, table_schema, table_name,
         LISTAGG(column_name || ' ' || data_type, ', ')
           WITHIN GROUP (ORDER BY ordinal_position) AS columns
  FROM SNOWFLAKE_SAMPLE_DATA.INFORMATION_SCHEMA.COLUMNS
  WHERE table_schema = 'TPCH_SF1'
  GROUP BY ALL
  UNION ALL
  SELECT table_catalog, table_schema, table_name,
         LISTAGG(column_name || ' ' || data_type, ', ')
           WITHIN GROUP (ORDER BY ordinal_position)
  FROM FILAGREE.INFORMATION_SCHEMA.COLUMNS
  WHERE table_schema = 'MART'
  GROUP BY ALL
)
SELECT
  table_catalog || '.' || table_schema || '.' || table_name AS table_fqn,
  table_name,
  columns,
  AI_COMPLETE(
    'llama3.3-70b',
    'You document data for people who will never read SQL. In two short sentences, '
    || 'say what one row of the table ' || table_name || ' represents and one question '
    || 'it can answer. Columns: ' || columns
  )                    AS summary,
  'llama3.3-70b'       AS model,
  CURRENT_TIMESTAMP()  AS generated_at
FROM catalog;

-- Answer a spoken question. focus_table is the box the viewer has selected (by gesture), or NULL.
CREATE OR REPLACE PROCEDURE FILAGREE.AI.ASK_LAKE(question STRING, focus_table STRING DEFAULT NULL)
RETURNS STRING
LANGUAGE SQL
AS
$$
DECLARE
  docs STRING;
  revenue STRING;
  suppliers STRING;
  answer STRING;
BEGIN
  SELECT LISTAGG(table_name || ': ' || summary, '\n') INTO :docs
  FROM FILAGREE.AI.TABLE_DOCS
  WHERE :focus_table IS NULL OR table_name = UPPER(:focus_table)
     OR table_name IN ('REVENUE_BY_NATION_MONTH', 'SUPPLIER_SCORECARD');

  SELECT LISTAGG(region || ' ' || yr || ': ' || TO_VARCHAR(rev, '999,999,999,999'), '\n') INTO :revenue
  FROM (SELECT region, YEAR(month) AS yr, ROUND(SUM(net_revenue)) AS rev
        FROM FILAGREE.MART.REVENUE_BY_NATION_MONTH GROUP BY ALL ORDER BY region, yr);

  SELECT LISTAGG(supplier || ' (' || nation || ') margin ' || ROUND(margin)
                 || ', late ' || ROUND(late_share * 100) || '%', '\n') INTO :suppliers
  FROM (SELECT * FROM FILAGREE.MART.SUPPLIER_SCORECARD ORDER BY margin DESC LIMIT 10);

  SELECT AI_COMPLETE(
    'llama3.3-70b',
    'You answer questions about the TPC-H sample dataset for someone looking at a 3D map '
    || 'of it, not at SQL. Answer in at most three spoken sentences, using only the facts below. '
    || 'Name the table the answer comes from.\n\nTables:\n' || :docs
    || '\n\nNet revenue by region and year:\n' || :revenue
    || '\n\nTop 10 suppliers by margin:\n' || :suppliers
    || '\n\nSelected box: ' || COALESCE(:focus_table, 'none')
    || '\n\nQuestion: ' || :question
  ) INTO :answer;

  RETURN answer;
END;
$$;

-- Try it:
-- CALL FILAGREE.AI.ASK_LAKE('Which region earned the most in 1997?');
-- CALL FILAGREE.AI.ASK_LAKE('What is in this table?', 'LINEITEM');
