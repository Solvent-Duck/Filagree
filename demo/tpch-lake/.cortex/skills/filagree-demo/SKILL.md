---
name: filagree-demo
description: Run the scripted Filagree stage demo over the TPC-H lake. Use when the person says "launch", "launch the demo", or asks one of the scripted demo questions below (region 1997, what's in this table, customer orders a part, why was this order late, what if orders streamed, switch back).
---

# Filagree stage demo

This is a rehearsed demo. The model and every tour are already drafted and
validated in `../tpch-fallback/`. Your job is to call up the right one fast and
narrate it. Do not explore Snowflake, do not build or edit any model or
walkthrough JSON, and do not run `sql/00`–`30`. The lake is already built.

Tools: the Visualyze MCP (`publish_model`, `present_walkthrough`,
`get_scene_state`, `set_view`, `control_playback`) and your Snowflake SQL tool.
Paths below are relative to `demo/tpch-lake`, where this session runs.

For every tour: present it with `autoplay: false`, then narrate in chat with one
`###` heading per step title, in order, one or two spoken sentences each. The
presenter steps through with hand gestures or arrow keys while you talk. Keep
the whole reply short enough to read aloud in under a minute.

## Script

### "launch"

1. Run in the shell: `mkdir -p .visualyze && cp ../tpch-fallback/*.json .visualyze/`
2. `publish_model` with path `.visualyze/model.json`.
3. `present_walkthrough` with path `.visualyze/walkthrough.json`.
4. Narrate. Open with one line: "This is Snowflake's free TPC-H sample, 6 million
   line items across eight tables, with marts and an open-weight Llama 3.3 70B
   running inside Snowflake through Cortex."

### Box selected + "what's in this table?"

1. `get_scene_state`. Use the selected entity's label as the table name (for
   example `LINEITEM`). If nothing is selected, say "Click a table first" and stop.
2. Run `CALL FILAGREE.AI.ASK_LAKE('what''s in this table?', '<TABLE>');`
3. Say the answer back verbatim, then one line: "That came from Llama 3.3 70B in
   Cortex, reading the table doc it wrote in `AI.TABLE_DOCS`."

### "Which region earned the most in 1997?"

1. Run `CALL FILAGREE.AI.ASK_LAKE('Which region earned the most in 1997?');`
   and say the answer back. Expected: AMERICA, about 6.66 billion.
2. `present_walkthrough` with path `.visualyze/region-1997.walkthrough.json`
   and narrate it.

### "Walk me through how a customer orders a part."

`present_walkthrough` with `.visualyze/customer-order.walkthrough.json` and
narrate. PartsHub is the wholesale parts business TPC-H models. Its apps are
`declared` (TPC-H ships no app code); say so in one clause.

### "Why was this order late?"

`present_walkthrough` with `.visualyze/late-order.walkthrough.json` and narrate.
End on the point that the nightly load puts a day between shipping and the
scorecard.

### "What if PartsHub orders streamed into Snowflake instead of loading nightly?"

1. Say: "I tried it in a zero-copy clone, `FILAGREE_WHATIF`, so the real lake
   is untouched." Then run this to show the clone's dynamic table matches:

   ```sql
   SELECT 'nightly' AS mart, COUNT(*) AS row_count, SUM(net_revenue) AS net_revenue
   FROM FILAGREE.MART.REVENUE_BY_NATION_MONTH
   UNION ALL
   SELECT 'dynamic', COUNT(*), SUM(net_revenue)
   FROM FILAGREE_WHATIF.MART.REVENUE_BY_NATION_MONTH_LIVE;
   ```

   Expected: 10,000 rows and 218,102,223,885.0001 on both. Do not recreate the
   clone or resume the dynamic table.
2. `publish_model` with `.visualyze/proposal-streaming.model.json`.
3. `present_walkthrough` with `.visualyze/proposal-streaming.walkthrough.json`
   and narrate, saying "Before:" on the REMOVED step and "After:" on the NEW ones.
4. Close with the tradeoff: freshness goes from a day to a minute, but a FULL
   refresh re-reads 6 million line items every minute, which is why the clone's
   dynamic table is suspended. The clone did not test Snowpipe Streaming or
   `ORDERS_LIVE`.

### "Switch back"

`publish_model` with `.visualyze/model.json`, then say "Back to today's lake."

## Anything off-script

Answer briefly. For a question about the data, call `ASK_LAKE` with it. For a
question about a box, `get_scene_state` and `set_view` on what they mean, using
IDs from `.visualyze/model.json`. Never write new model files on stage.

## If something fails

- Visualyze says the revision is stale: publish `.visualyze/model.json` again,
  then retry.
- A Snowflake call fails: say the expected answer from this script and keep going.
