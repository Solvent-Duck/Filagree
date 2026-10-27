# Demo

**Snowflake challenge:** see [SNOWFLAKE.md](SNOWFLAKE.md). `tpch-lake/` runs on the free TPC-H sample share, and `tpch-fallback/` holds its checked model and walkthroughs.

`acme-data/` is a small made-up data repo for the agent to read live. `fallback/` holds a checked model and walkthrough of it, in case the live run goes wrong.

## Run it

From the repo root:

```sh
demo/run.sh
```

It builds the viewer (the first build takes several minutes), opens it maximized
on the checked TPC-H model and tour, and starts hand gestures. Press Q in the
viewer and type a question:

- "Explain the codebase and the data it represents"
- "Which region earned the most in 1997?"
- Click `LINEITEM`, then "What's in this table?"
- "Walk me through how a customer orders a part"
- "Why was this order late?"
- "What if PartsHub orders streamed into Snowflake?"

Close the window or press Ctrl-C to stop everything. Logs go to
`tpch-lake/.visualyze/`.

| Flag | Effect |
| --- | --- |
| `--live` | Wipe `tpch-lake/.visualyze/` instead of loading the checked model, so an agent builds it |
| `--no-gestures` | Skip the camera |
| `--warm` | Ask Snowflake one question in the background so the warehouse is awake |

Answers that need Snowflake use the connection named in `SNOWFLAKE_CONNECTION`
(default `QN29986`). If a viewer is already open, the script stops; close that
window first.

## From a coding agent instead

Claude Code or CoCo can drive the same viewer through MCP. Register it once,
from `demo/acme-data` or `demo/tpch-lake`:

```sh
claude mcp add visualyze -- ~/Filagree/target/debug/visualyze mcp
cortex mcp add visualyze ~/Filagree/target/debug/visualyze mcp
```

Then start `claude` or `cortex` and ask: "Show me the flow from our client
tables to our Snowflake data lake."

To use the checked `acme-data` model, copy it in and ask the agent to publish
`.visualyze/model.json` and present `.visualyze/walkthrough.json`:

```sh
mkdir -p acme-data/.visualyze && cp fallback/*.json acme-data/.visualyze/
```

Delete `acme-data/.visualyze/` before a live run so the agent builds the model itself.
