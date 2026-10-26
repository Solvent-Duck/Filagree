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
