# Submission text

Paste-ready text for the MLH submission. The numbers come from the real run on
October 26.

**Name:** Filagree

**Tagline:** Stop reading code. Ask about your system out loud and watch it in 3D.

**Challenges:** Best Open-Source AI Project, and Best Open-Source AI Project with Snowflake.

**Repo:** https://github.com/Solvent-Duck/Filagree (MIT)

**Demo video:** `demo/video/renders/filagree-hackathon.mp4`

## What it does

Filagree lets you understand a software and data system without reading code.
You ask CoCo, Snowflake's coding agent, a question. It builds a 3D model of
the project and walks you through the answer. Your voice asks and your hands
steer: rotate, zoom, and step to the next part of the tour. It answers three
kinds of question:

- **Ask the data:** "Which region earned the most in 1997?" Llama 3.3 70B,
  running in Snowflake Cortex, answers "AMERICA, 6.656 billion" and names the
  table it came from, the revenue-by-nation-and-month mart.
- **Ask how it works:** "Walk me through how a customer orders a part." A
  narrated 3D tour lights each step, from the portal to the tables.
- **Ask what if:** "What if orders streamed in instead of loading nightly?"
  CoCo tries the change on a zero-copy clone, checks the result, and shows the
  proposed architecture.

## Snowflake: dataset, CoCo, open-weight AI

- **Dataset:** `SNOWFLAKE_SAMPLE_DATA.TPCH_SF1`, Snowflake's free TPC-H sample
  data: 6,001,215 line items, 1.5M orders, 150K customers, and 10K suppliers.
- **How CoCo helped:**
  - It explored the eight tables, with row counts and join keys.
  - It ran our setup, mart and Cortex SQL without needing changes, and checked
    Llama's answer against the mart itself.
  - It ran the what-if in clone `FILAGREE_WHATIF`: a dynamic table matched the
    nightly mart exactly (10,000 rows each, 218,102,223,885 net revenue, 0 rows
    that differed).
  - It builds and narrates the 3D view through the Visualyze MCP, guided by a
    skill we wrote for it.
- **Open-weight AI:** Llama 3.3 70B through Cortex `AI_COMPLETE` writes a
  plain-English description of every table (`TABLE_DOCS`) and answers questions
  (`ASK_LAKE`). Gestures use MediaPipe's open-source gesture recognizer.

## How we built it

- A Rust and Bevy 3D viewer (Visualyze). The agent supplies only what the
  system is made of; the viewer owns all layout and animation. Text reads in
  Inter, with JetBrains Mono for small labels, and each caption fades and rises
  in as the tour reaches it.
- An MCP bridge that lets CoCo publish models, play tours, and read what's
  selected in the scene. It starts the viewer on its own, including on a
  machine where it has never run.
- Skills that teach CoCo how Snowflake objects map to boxes, and how to answer
  "what if" questions safely in a clone.
- A Python gesture bridge on MediaPipe, which rotates, zooms and steps through
  the tour over a local socket.
- A demo project over TPC-H: two marts, a daily refresh task, Cortex docs, a
  question procedure, and PartsHub, the wholesale parts app TPC-H describes.
- A demo video made in HyperFrames from frames the viewer rendered itself.

## Challenges

- The viewer has no visual diff, so proposals use their own model and
  NEW/REMOVED labels instead of being a new revision of the real model.
- A cloned procedure still writes to the original database, so the what-if
  skill forbids calling procedures inside a clone.
- Getting Llama to cite its source table meant labelling each block of
  context with the table it came from.
- Gestures can't point at a box, so you select one by clicking and CoCo reads
  the selection from the scene.

## Honest scope

The dataset, the marts, Cortex and the clone are real and cited to SQL.
PartsHub's apps are a declared design: TPC-H describes the business but ships
no app code, and the model marks those parts as declared.

## Built with

Rust, Bevy, MCP, Snowflake, Snowflake Cortex Code (CoCo), Cortex AI_COMPLETE,
Llama 3.3 70B, dynamic tables, zero-copy cloning, Python, MediaPipe,
HyperFrames, Inter, JetBrains Mono.
