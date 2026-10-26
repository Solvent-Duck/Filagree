tier: scratch
stack: rust
check: none (scratch)

# Filagree

Hackathon demo: Visualyze as a live, conversation-driven MCP for Claude Code and CoCo (Snowflake Cortex Code). Demo quality only, not a product. Started 2026-10-26.

- The Rust workspace is a fresh copy of Visualyze at 523a4dc; build with `cargo build --locked`, binary at `target/debug/visualyze`.
- `demo/acme-data/` is the made-up repo the agent reads live; `demo/fallback/` holds a validated model and walkthrough. Run steps in `demo/README.md`.

## Visualyze reference

- Read-only clone of github.com/Solvent-Duck/Visualyze at `~/ref/visualyze` (chmod a-w, Edit denied in `.claude/settings.json`). Never edit it, commit to it or run write commands in it. `~/Visualyze` is the working checkout; leave it alone too.
- Reuse code from it freely, but everything in this repo is created from 2026-10-26 on: copy into new files here, never import history.
- Build it with `CARGO_TARGET_DIR=~/ref/visualyze-target` so `target/` lands outside the clone.
- Start with: `README.md`, `docs/decisions/0001`, `public/schema/architecture-format.md`, `docs/decisions/0008`, `public/schema/walkthrough-format.md` + `public/examples/notes-crud.walkthrough.json`, `docs/decisions/0007`, `docs/decisions/0010`.
- Carry over: agent supplies semantics, renderer owns geometry; the agent explains, the viewer shows; group past ~16 boxes per layer or ~27 per view; never read or write the visualized repo.
