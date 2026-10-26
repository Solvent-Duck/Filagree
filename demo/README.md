# Demo

**Snowflake challenge:** see [SNOWFLAKE.md](SNOWFLAKE.md). `tpch-lake/` runs on the free TPC-H sample share, and `tpch-fallback/` holds its checked model and walkthrough.

`acme-data/` is a small made-up data repo for the agent to read live. `fallback/` holds a checked model and walkthrough of it, in case the live run goes wrong.

## Run it

Build the viewer from the repo root (first build takes several minutes):

```sh
cargo build --locked
```

Claude Code, from `demo/acme-data`:

```sh
claude mcp add visualyze -- ~/Filagree/target/debug/visualyze mcp
claude
```

CoCo (Cortex Code), once installed and signed in, from `demo/acme-data`:

```sh
cortex mcp add visualyze ~/Filagree/target/debug/visualyze mcp
cortex
```

Prompt: "Show me the flow from our client tables to our Snowflake data lake."

## Fallback

Load the checked model and walkthrough into the demo repo, then ask the agent to publish `.visualyze/model.json` and present `.visualyze/walkthrough.json`:

```sh
mkdir -p acme-data/.visualyze && cp fallback/*.json acme-data/.visualyze/
```

Delete `acme-data/.visualyze/` before a live run so the agent builds the model itself.
