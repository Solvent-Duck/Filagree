# Snowflake challenge: TPC-H you never have to read

Filagree turns a Snowflake dataset into a 3D model you talk to and steer with
your hands. CoCo explores the data and builds the pipeline, Visualyze shows it, and an
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
  questions through `AI.ASK_LAKE`. Hand gestures use MediaPipe's open-source
  gesture recognizer (Apache-2.0).
- **Open source:** MIT, public on GitHub.

## Set up (once)

1. Get a Snowflake account (organizer's trial instructions, or a 30-day trial).
2. Install CoCo and sign in:

   ```sh
   curl -qLsS https://ai.snowflake.com/static/cc-scripts/install.sh | sh
   ```

   Run `cortex` once and choose "Sync from app.snowflake.com". Note the
   connection name it creates (ours is `QN29986`), because `ask.py` reads it from
   `SNOWFLAKE_CONNECTION`.
3. Install the skill and connect Visualyze, from the repo root:

   ```sh
   mkdir -p ~/.snowflake/cortex/skills && cp -R public/skills/cortex-code/visualyze ~/.snowflake/cortex/skills/
   ```

   ```sh
   cd demo/tpch-lake && cortex mcp add visualyze "$(git rev-parse --show-toplevel)/target/debug/visualyze" mcp
   ```

4. Build the lake. Start `cortex` in `demo/tpch-lake` and say: "Use role
   ACCOUNTADMIN. Explore SNOWFLAKE_SAMPLE_DATA.TPCH_SF1, then run sql/00_setup.sql,
   sql/10_marts.sql and sql/20_ai.sql in order and fix anything that fails."
5. Check the AI end to end:

   ```sh
   SNOWFLAKE_CONNECTION=QN29986 uv run --with snowflake-connector-python ask/ask.py "Which region earned the most in 1997?"
   ```

## Demo script (3 minutes)

The stage run uses the `launch` skill in `demo/tpch-lake/.cortex/skills/`. It
calls up the checked model and tours from `tpch-fallback/` instead of building
them live, and it starts the gesture bridge with `gestures.sh`. The Snowflake
answers (`ASK_LAKE` and the what-if comparison) still run live. Before you
start, turn off Continuity Camera on any nearby iPhone so the Mac's own camera
is camera 0, and run CoCo once from the terminal app you will present in, so
macOS has granted it camera access.

Start a new session with `cd demo/tpch-lake && cortex -c QN29986`.

1. "Show me how the data flow in Snowflake works. Use the launch skill." CoCo
   copies the model and tours into `.visualyze/`, publishes the model, starts
   the gestures, and plays the overview tour. Step through it with a
   one-finger swipe.
2. Click `LINEITEM` to select it, then dictate to CoCo: "what's in this
   table?". CoCo reads the selection with `get_scene_state` and calls
   `ASK_LAKE('what''s in this table?', 'LINEITEM')`, and Llama answers from
   `TABLE_DOCS`. Gestures rotate, zoom and step through the tour; they don't
   point at boxes.
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
6. Propose a change: "What if PartsHub orders streamed into Snowflake instead
   of loading nightly?" The zero-copy clone `FILAGREE_WHATIF` was made with
   `sql/30_whatif.sql`, which builds the revenue mart there as a dynamic table
   (`TARGET_LAG = '1 minute'`, `REFRESH_MODE = FULL`); on stage CoCo reruns
   only the comparison query against it. Run for real on Oct 26:
   it built straight over the sample share in 4.7s, and it matched the nightly
   mart exactly, with 10,000 rows on both sides, 218,102,223,885.0001 net
   revenue on both, and 0 mismatched rows. Then it publishes the proposal model
   (`tpch-lake-proposal-streaming`) and plays its tour. The tour shows the
   nightly load (REMOVED), then Snowpipe Streaming into `ORDERS_LIVE` (NEW),
   then the dynamic table, then the dashboard and `ASK_LAKE`, now
   minutes-fresh. Tradeoffs to say out loud: freshness drops from a day to a
   minute, but a FULL refresh re-reads 6M lines every minute. That is why the
   clone's dynamic table is suspended. The clone did not test Snowpipe
   Streaming, `ORDERS_LIVE` or the scorecard.
7. "Switch back." CoCo republishes `model.json`.

PartsHub's apps, orders database and nightly load are `declared`: TPC-H
describes the business but ships no app code. Everything in Snowflake stays
`supported` with SQL citations. Say so if a judge asks.

## Fallback

If the launch skill doesn't load, use the checked model (revision 2) and
tours by hand:

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
| `proposal-streaming.walkthrough.json` | What if orders streamed in instead of nightly? Publish `proposal-streaming.model.json` first. |

Start the gestures after publishing with `demo/tpch-lake/gestures.sh`.

## If Cortex refuses the model

`llama3.3-70b` may not be in your account's region. Either swap the model
name in `sql/20_ai.sql` for `mistral-large3` or `qwen3-32b` (also open-weight),
or, as ACCOUNTADMIN, run
`ALTER ACCOUNT SET CORTEX_ENABLED_CROSS_REGION = 'ANY_REGION';`.
