# tpch-lake

A small Snowflake data project over the free TPC-H sample share, built with
CoCo (Snowflake Cortex Code) for the Filagree demo. Ask it a question out loud
and an open-weight model answers from the marts.

| Part | Where | What |
| --- | --- | --- |
| TPC-H source | `SNOWFLAKE_SAMPLE_DATA.TPCH_SF1` | Eight sample tables: region, nation, customer, orders, lineitem, part, supplier, partsupp |
| Marts | `sql/10_marts.sql` | `REFRESH_MARTS` builds `MART.REVENUE_BY_NATION_MONTH` and `MART.SUPPLIER_SCORECARD` |
| Cortex AI | `sql/20_ai.sql` | Llama 3.3 70B writes `AI.TABLE_DOCS`; `AI.ASK_LAKE` answers questions |
| Voice bridge | `ask/ask.py` | Calls `ASK_LAKE` with the spoken question and the selected box |

## Run it

Start `cortex` in this directory and ask: "Run sql/00_setup.sql, sql/10_marts.sql
and sql/20_ai.sql in order, and fix anything that fails." Then:

```sh
uv run --with snowflake-connector-python ask/ask.py "Which region earned the most in 1997?"
```

TPC-H SF1 is about 6 million line items; the marts build in under a minute on
an XSMALL warehouse.
