# Snowflake challenge: TPC-H you never have to read

Filagree turns a Snowflake dataset into a 3D model you can talk to and point
at. CoCo explores the data and builds the pipeline, Visualyze shows it, and an
open-weight model running in Snowflake answers questions about it out loud.

## What the judges need to hear

- **Dataset:** `SNOWFLAKE_SAMPLE_DATA.TPCH_SF1`, Snowflake's free TPC-H sample
  share. It has eight tables, including 6 million line items and 1.5 million orders.
- **How CoCo helped:** CoCo profiled the eight tables (row counts and join
  keys), wrote and ran the marts and the Cortex SQL in `tpch-lake/sql/`, then
  used the Visualyze MCP to build the 3D model of the dataset and narrate a
  walkthrough of it. The `visualyze` skill in
  `public/skills/cortex-code/` teaches CoCo how Snowflake objects map to boxes.
- **Open-weight AI:** Llama 3.3 70B through Cortex `AI_COMPLETE`. It writes a
  plain-English doc for every table (`AI.TABLE_DOCS`) and answers spoken
  questions through `AI.ASK_LAKE`. Hand tracking and speech-to-text are
  open-source models too.
- **Open source:** MIT, public on GitHub.

## Set up (once)

1. Get a Snowflake account (organizer's trial instructions, or a 30-day trial).
2. Install CoCo and sign in:

   ```sh
   curl -qLsS https://ai.snowflake.com/static/cc-scripts/install.sh | sh
   ```

   Run `cortex` once and create a connection named `filagree`, then check it
   with `cortex sql "select current_role()"`.
3. Install the skill and connect Visualyze, from the repo root:

   ```sh
   mkdir -p ~/.snowflake/cortex/skills && cp -R public/skills/cortex-code/visualyze ~/.snowflake/cortex/skills/
   ```

   ```sh
   cd demo/tpch-lake && cortex mcp add visualyze "$(git rev-parse --show-toplevel)/target/debug/visualyze" mcp
   ```

4. Build the lake. Start `cortex -c filagree` in `demo/tpch-lake` and say:
   "Explore SNOWFLAKE_SAMPLE_DATA.TPCH_SF1, then run sql/00_setup.sql,
   sql/10_marts.sql and sql/20_ai.sql in order and fix anything that fails."
5. Check the AI end to end:

   ```sh
   uv run --with snowflake-connector-python ask/ask.py "Which region earned the most in 1997?"
   ```

## Demo script (3 minutes)

1. In CoCo, from `demo/tpch-lake`: "Show me how a spoken question about
   revenue gets answered from the TPC-H tables." CoCo builds
   `.visualyze/model.json`, publishes it, and plays the walkthrough.
2. Point at `LINEITEM` and ask "what's in this table?". The voice layer calls
   `ask.py "what's in this table?" LINEITEM`, and Llama answers from `TABLE_DOCS`.
3. Ask "which region earned the most in 1997?". The answer comes from the
   revenue mart, and CoCo can `set_view` on it, or present the
   `region-1997` tour: voice, `ASK_LAKE`, the revenue mart, Llama, then the
   five TPC-H tables the mart joins.
4. "Walk me through how a customer orders a part." CoCo presents the
   `customer-order` tour over PartsHub, the wholesale parts app TPC-H models:
   the customer portal finds stock (`ps_availqty`) through the Catalog API, the
   Orders API writes an open order (`o_orderstatus 'O'`), the shipping worker
   sets ship dates, and the nightly load copies it into `ORDERS`.
5. Follow-up: "Why was this order late?" The `late-order` tour goes from the
   shipping worker's `l_receiptdate` versus `l_commitdate`, through the
   nightly load into `LINEITEM`, to `late_share` in the supplier scorecard
   and the ops dashboard. It ends on the nightly hop, which sets up the
   what-if.

PartsHub's apps, orders database and nightly load are `declared`: TPC-H
describes the business but ships no app code. Everything in Snowflake stays
`supported` with SQL citations. Say so if a judge asks.

## Fallback

If the live model build goes wrong, use the checked model (revision 2) and
tours:

```sh
mkdir -p demo/tpch-lake/.visualyze && cp demo/tpch-fallback/*.json demo/tpch-lake/.visualyze/
```

Then ask CoCo to publish `.visualyze/model.json` and present one of:

| File | Tour |
| --- | --- |
| `walkthrough.json` | From TPC-H tables to a spoken answer |
| `region-1997.walkthrough.json` | Which region earned the most in 1997? |
| `customer-order.walkthrough.json` | A customer orders a part |
| `late-order.walkthrough.json` | Why was this order late? |

Delete `.visualyze/` before a live run.

## If Cortex refuses the model

`llama3.3-70b` may not be in your account's region. Either swap the model
name in `sql/20_ai.sql` for `mistral-large3` or `qwen3-32b` (also open-weight),
or, as ACCOUNTADMIN, run
`ALTER ACCOUNT SET CORTEX_ENABLED_CROSS_REGION = 'ANY_REGION';`.
