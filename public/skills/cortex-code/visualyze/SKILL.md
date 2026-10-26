---
name: visualyze
description: Show a Snowflake dataset, data pipeline or data project as a 3D Visualyze walkthrough while you narrate in chat. Use when the user asks to see, be walked through, or understand how tables, marts, tasks, procedures or Cortex AI fit together, and the visualyze MCP server is connected.
---

# Visualyze walkthroughs for Snowflake data

Read the MCP resource `visualyze://guide/walkthroughs` first and follow it. It
is the same guide Claude Code uses. This file adds what is specific to CoCo and
to Snowflake.

## Explore before you model

Use your Snowflake connection to learn the data, then the project files to
learn what is built on it. Useful queries:

- `SHOW TABLES IN SCHEMA <db>.<schema>` and `INFORMATION_SCHEMA.TABLES` for
  row counts (`row_count`) to put in descriptions and labels.
- `INFORMATION_SCHEMA.COLUMNS` for the key columns that join tables.
- `SHOW PROCEDURES`, `SHOW TASKS`, `SHOW DYNAMIC TABLES` for what transforms data.

Never put credentials, connection strings, or row values from the data in the
model. Counts and column names are fine.

## Map Snowflake objects to the format

| Snowflake object | Visualyze |
| --- | --- |
| The project or database being explained | root `scope`, subtype `system` |
| A schema or shared database (for example `SNOWFLAKE_SAMPLE_DATA.TPCH_SF1`) | `scope`, subtype `subsystem` |
| Table, view, dynamic table's stored result | `datastore`, subtype `columnar`, technology `Snowflake` |
| Stored procedure, task, dynamic table refresh, Snowpipe | `service` (`scheduler` for tasks, `worker` for loads, `application` for procedures callers invoke) |
| A procedure's call signature | `interface`, subtype `api`, protocol `sql`, exposed by the procedure |
| Cortex `AI_COMPLETE` / `AI_EMBED` model | `service` labelled with the model name (for example `Llama 3.3 70B`) |
| Masking or row access policy | `concern`, subtype `data_access_control`, `applies_to` each covered table |

A procedure that selects from a table `reads` it; one that creates or inserts
into a table `writes` it; a call to `AI_COMPLETE` `requests` the model service.
Cite the SQL file and lines that show each claim. Objects you saw only in
Snowflake, with no file in the repo, use an opaque `reference` locator such as
`SNOWFLAKE_SAMPLE_DATA.TPCH_SF1.LINEITEM`.

## CoCo specifics

- Write `.visualyze/model.json` and `.visualyze/walkthrough.json` with your file
  tools. The Visualyze server only reads files under the directory it started in.
- The tools are `get_capabilities`, `validate_model`, `publish_model`,
  `present_walkthrough`, `set_view`, `get_scene_state`, and `control_playback`.
- When the person asks a question about the data itself, answer it with SQL as
  usual, then `set_view` on the tables the answer came from so they can see them.
- You narrate. Put the explanation in your reply, with one heading per step
  title, and say which dataset you used.

## Change questions

The viewer shows one model at a time and has no automatic visual diff. Show
change with `focus` and with step `labels` such as `NEW` and `REMOVED` on the
parts that change.

- **Impact** ("what breaks if we change LINEITEM?"). Answer from the current
  model; build no new one. From the object, follow `reads` and `writes`
  downstream: who reads it, what they write, who reads that, out to the marts,
  Cortex and clients. Walk it one reader per step: trace its read, the
  `return`, then its write. A `reads` hop leaves from the reader, so one trace
  cannot run downstream across several readers. End on a step with `"reveal":
  "cumulative"` that focuses the furthest affected boxes. Say which links are
  inferred.
- **What-if** ("what if orders streamed in instead of loading nightly?").
  1. Copy the model to `.visualyze/proposals/<name>.model.json` with its own
     `model_id` (for example `tpch-lake-proposal-streaming`) and revision 1.
     Never publish a proposal as a new revision of the real model: viewer
     revisions only move forward, so the real model could not come back.
  2. Keep every unchanged ID so boxes land where the person saw them. Add new
     IDs for new parts. To label a removed part `REMOVED`, keep it in the
     proposal with a description starting "Proposed: removed".
  3. Mark new or changed entities and relations `"basis": "declared"`, with
     descriptions starting "Proposed:".
  4. Try the SQL first in a zero-copy clone (`CREATE DATABASE <db>_WHATIF CLONE
     <db>`), compare row counts and totals with the original, and cite that SQL
     as evidence. A shared database such as `SNOWFLAKE_SAMPLE_DATA` cannot be
     cloned; the clone reads it in place. A cloned procedure with fully
     qualified names still writes to the original, so never call one. `sql/30_whatif.sql` in the
     `tpch-lake` demo is an example.
  5. Publish the proposal and present a walkthrough with its `model_id`: where
     the change sits, the old path (`REMOVED`), the new path traced (`NEW`),
     ending on the affected downstream boxes. Narrate the tradeoffs in chat:
     freshness, cost, what the clone showed and what it did not test.
- **Before/after**. Publish and walk the current model, then publish and walk
  the proposal: two short tours with the same opening shot, narrated as a
  comparison ("Before: …", "After: …"). Republishing the unchanged current
  model is accepted, so you can switch back.
