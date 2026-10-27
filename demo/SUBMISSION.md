# Submission text

Paste-ready text for the MLH submission. The numbers come from the real run on
October 26.

**Name:** Filagree

**Tagline:** Stop reading code. Ask about your system out loud and watch it in 3D.

**Challenges:** Best Open-Source AI Project, and Best Open-Source AI Project with Snowflake.

**Repo:** https://github.com/Solvent-Duck/Filagree (MIT)

## What it does

Filagree lets you understand a software and data system without reading code.
You ask your coding agent a question. It builds a 3D model of the project and
walks you through the answer, and you steer the view with hand gestures. It
answers three kinds of question:

- **About the data:** "Which region earned the most in 1997?" Llama 3.3 70B,
  running in Snowflake, answers "AMERICA, 6.656 billion" and names the table it
  came from.
- **How it works:** "Walk me through how a customer orders a part." A narrated
  3D tour lights each hop from the portal to the tables.
- **What if:** "What if orders streamed in instead of loading nightly?" CoCo
  tries the change on a zero-copy clone, checks the result, and shows the
  proposed architecture.

## Snowflake: dataset, CoCo, open-weight AI

- **Dataset:** `SNOWFLAKE_SAMPLE_DATA.TPCH_SF1`, Snowflake's free TPC-H sample
  data: 6,001,215 line items, 1.5M orders, 150K customers, and 10K suppliers.
- **How CoCo helped:**
  - It explored the eight tables, with row counts and join keys.
  - It ran our setup, mart and Cortex SQL without needing changes, and checked
    Llama's answer against the mart itself.
  - It ran the what-if in a clone: a dynamic table matched the nightly mart
    exactly (10,000 rows each, 218,102,223,885 net revenue, 0 rows that differed).
  - It builds and narrates the 3D view through the Visualyze MCP, guided by a
    skill we wrote for it.
- **Open-weight AI:** Llama 3.3 70B through Cortex `AI_COMPLETE` writes a
  plain-English description of every table (`TABLE_DOCS`) and answers questions
  (`ASK_LAKE`). Gestures use MediaPipe's open-source gesture recognizer.

## How we built it

- A Rust and Bevy 3D viewer (Visualyze). The agent supplies only what the
  system is made of; the viewer owns all layout and animation.
- An MCP bridge that lets CoCo or Claude Code publish models and play tours.
- Skills that teach the agent how Snowflake objects map to boxes, and how to
  answer "what if" questions safely in a clone.
- A Python gesture bridge on MediaPipe, which rotates, zooms and steps through
  the tour over a local socket.
- A demo project over TPC-H: two marts, a daily refresh task, Cortex docs, a
  question procedure, and PartsHub, the wholesale parts app TPC-H describes.

## Challenges

- The viewer has no visual diff, so proposals use their own model and
  NEW/REMOVED labels instead of being a new revision of the real model.
- A cloned procedure still writes to the original database, so the what-if
  skill forbids calling procedures inside a clone.
- Getting Llama to cite its source table meant labelling each block of
  context with the table it came from.

## Honest scope

The dataset, the marts, Cortex and the clone are real and cited to SQL.
PartsHub's apps are a declared design: TPC-H describes the business but ships
no app code, and the model marks those parts as declared.

## Built with

Rust, Bevy, MCP, Snowflake, Snowflake Cortex Code (CoCo), Cortex AI_COMPLETE,
Llama 3.3 70B, dynamic tables, zero-copy cloning, Python, MediaPipe, Claude Code.
