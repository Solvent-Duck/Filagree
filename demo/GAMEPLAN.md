# Snowflake track gameplan (Oct 26, submissions close 21:00)

Demo arc, three questions:

1. **Ask the data.** "Which region earned the most in 1997?" Llama in Cortex answers.
2. **Explain the system.** "Walk me through how a customer orders a part." Visualyze tour.
3. **Propose a change.** "What if orders streamed in instead of loading nightly?" CoCo tries it in a zero-copy clone and shows the new architecture.

## Done (session 1)

- `demo/tpch-lake/`: setup SQL, marts, Cortex `TABLE_DOCS` and `ASK_LAKE`, `ask/ask.py`.
- `demo/tpch-fallback/`: 22-box model and 8-step walkthrough. Validated with zero warnings and rendered.
- `public/skills/cortex-code/visualyze/SKILL.md`: the CoCo skill.
- `demo/SNOWFLAKE.md`: setup, demo script, and what to tell judges.

## Rules every session keeps

- Real objects stay `supported` with SQL citations. PartsHub app parts and proposals are `declared`, and their descriptions start with "Proposed:" when hypothetical.
- A proposal gets its own `model_id` (`tpch-lake-proposal-streaming`). It is never a new revision of `tpch-lake`.
- Keep existing IDs. Add, don't rename.
- Finish by validating with the MCP `validate_model` (model and walkthrough), then rendering with
  `cargo run --locked --example walkthrough_smoke -- <model> <walkthrough>` and looking at the frames.
- Don't touch `private/runtime/` because the teammate is working there.

## Sessions

| # | When | Who | Depends on | Touches |
| --- | --- | --- | --- | --- |
| A | 18:20–19:00 | You, in CoCo | — | Snowflake account only |
| B | 18:25–19:10 | Claude | — | `demo/tpch-fallback/model.json`, new walkthroughs |
| C1 | 18:25–19:00 | Claude | — | `SKILL.md`, `demo/tpch-lake/sql/30_whatif.sql` |
| C2 | 19:10–19:50 | Claude | B | `demo/tpch-fallback/proposal-streaming.*` |
| D | 19:30–20:15 | You and teammate | A | teammate's voice/gesture code, `ask.py` |
| E | 20:15–20:50 | Claude and you | all | READMEs, submission text, push |

B and C1 run in parallel because they touch different files. C2 waits for B's IDs.

### A: make Snowflake real (you, no Claude needed)

The live demo and the "how CoCo helped" claim both depend on this, so it goes first.

1. Get a trial account, install CoCo, and make a connection named `filagree` (see `SNOWFLAKE.md`).
2. In `demo/tpch-lake`, run `cortex -c filagree` and ask it to explore TPCH_SF1, then run `sql/00`, `10` and `20`.
3. Run `ask.py "Which region earned the most in 1997?"` and save the answer.
4. Write down what CoCo did. Screenshots or a note are enough, because judges ask for it.
5. If something fails, keep the error text. Session E or a fix session needs it.

### B: PartsHub application architecture

Prompt:

> Read demo/GAMEPLAN.md (session B) and demo/SNOWFLAKE.md. Extend demo/tpch-fallback/model.json (raise revision to 2, keep every ID) with PartsHub, the wholesale parts app TPC-H was designed around:
>
> - Customer app: customer portal and Orders API exposing /orders.
> - Supplier app: supplier portal and Catalog API exposing /catalog.
> - Fulfillment: a shipping worker.
> - An operational orders database, with a nightly load task that copies it into Snowflake. This gives the what-if something to replace.
> - An ops dashboard client in Analytics & AI that reads both marts.
>
> App parts are basis declared. TPC-H tables, marts and Cortex stay supported. Links:
>
> - Portal requests Orders API, which writes orders and reads CUSTOMER.
> - Catalog API reads PART, PARTSUPP and SUPPLIER.
> - Supplier portal requests Catalog API.
> - Shipping worker reads orders and writes line-item dates.
> - Dashboard reads the marts.
>
> Map each feature to real TPC-H columns in descriptions (o_orderstatus, l_returnflag, ps_availqty and so on). Write three walkthroughs: "A customer orders a part", "Why was this order late?" and "Which region earned the most in 1997?". Update the existing walkthrough's expected_revision. Validate and render all of them. Stay under 16 boxes per scope.

### C1: change questions in the skill, plus clone SQL

Prompt:

> Read demo/GAMEPLAN.md (session C1). Add a "Change questions" section to public/skills/cortex-code/visualyze/SKILL.md. It covers three kinds of question:
>
> - Impact: trace downstream reads and writes in the current model and walk them. No new model.
> - What-if: copy the model to a separate proposal model_id under .visualyze/proposals/. Keep unchanged IDs, mark new parts declared with "Proposed:", try the SQL in a zero-copy clone first, and end the walkthrough on the affected downstream boxes.
> - Before/after: two walkthroughs back to back.
>
> Note that the viewer has no visual diff, so use NEW and REMOVED labels instead. Write demo/tpch-lake/sql/30_whatif.sql:
>
> - CREATE DATABASE FILAGREE_WHATIF CLONE FILAGREE.
> - In the clone, a dynamic table replacing REVENUE_BY_NATION_MONTH's nightly rebuild.
> - Row-count comparison queries against the original.
> - A cleanup DROP, commented out.
>
> The TPC-H share itself cannot be cloned. Don't touch demo/tpch-fallback/.

### C2: rehearsed what-if proposal

Prompt (after B is merged):

> Read demo/GAMEPLAN.md (session C2) and the "Change questions" section of the CoCo skill. Write demo/tpch-fallback/proposal-streaming.model.json, model_id tpch-lake-proposal-streaming, revision 1. Copy B's model, keeping IDs. Replace the nightly load task with Snowpipe Streaming into an ORDERS_LIVE table plus a dynamic table revenue mart, all declared, citing sql/30_whatif.sql where it applies.
>
> Also write proposal-streaming.walkthrough.json:
>
> - The old nightly path, labelled REMOVED.
> - The new path traced, labelled NEW.
> - Downstream impact: the marts, ASK_LAKE and the dashboard, now minutes-fresh.
> - A recap.
>
> Validate and render. Add the "propose a change" step to the demo script in SNOWFLAKE.md.

### D: voice and gesture to ASK_LAKE (with teammate)

- Agree on the call: `ask.py "<question>" <TABLE>`, which prints `{"question","focus_table","answer"}`.
- The selected box is the table name. TPC-H boxes are labelled with the table name (`LINEITEM`), and their IDs are `store:<lowercase>`.
- Route questions: data questions go to `ask.py`; "walk me through…" and "what if…" go to CoCo or Claude.
- Test one spoken question end to end.

### E: ship

- Top-level README: lead with the Filagree pitch, the dataset, how CoCo helped, open-weight models, and run steps.
- Make sure the GitHub repo is public with the MIT license and everything is pushed.
- MLH submission text: name both challenges and use the judge bullets from SNOWFLAKE.md.
- Rehearse the three-question arc twice, once live and once on the fallbacks.
- Before the live run, delete `demo/tpch-lake/.visualyze/`.

## Cut list, if behind

Cut in this order: C2 → B's "late order" walkthrough → the D gesture-to-table mapping (voice only). Never cut A or E.
