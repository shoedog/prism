# Lane MEAS-B: make prism's accuracy measurement trustworthy again: Tier-A quick oracle repair (you are the planner and implementer, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-tiera`, branch `plan/tier-a-quick-repair` off main `4e592daa`.
- **Rules:** no git writes (the controller commits). No network unless already cached. Keep disk use lean.
- **Packet:** `docs/superpowers/plans/2026-10-04-tier-a-quick-repair/`.

## The decision this serves
Owner, 2026-10-04: "fix/do number 7 so we can measure 3 and 6". Workstream 7 includes "trustworthy accuracy measurement: Tier-A's matrix passes, but its quick oracle run remains INVALID". The owner wants to measure JS/TS workstreams 3 (member/occurrence data-flow) and 6 (Node module/runtime/async) before choosing between them.
- A parallel lane builds a SecBench.js ground-truth harness (`eval/secbench/`, in another clone; do not touch it).
- This lane makes the **LSP-oracle Tier-A quick run valid, bounded in time, and able to report JS/TS call-resolution accuracy.**

Portable compiler-worker packaging, freshness-safe reuse and acquisition performance are **out of scope** unless they block this. Note them in OQ.

## Known symptoms (verify; do not trust these)
- `cd eval && uv run tier-a --quick --allow-stale-sut` emitted no output for more than an hour and was interrupted. This happened in the S2 R2 repair (2026-10-04) in another clone.
- Earlier readouts (for example `docs/eval/receiver-closure/2026-09-05-inline-props-readout.md`) record quick exiting 2 as "baseline-invalid solely corpus SHA drift d26ae0a02c13 vs pinned 20c8490591a3", plus stale adjudications.
- `uv` discovery once failed on protected cache access.
- Memory notes: tokio is oracle-INVALID; adjudication outputs can glue the first JSON line to narration.

See `eval/README.md`, the `eval/` tier-a sources, and the committed baseline in `docs/eval/tier-a/`.

## Do
1. **Reproduce and diagnose.** Run quick with a wall-clock bound, e.g. `timeout 1800`. Find where it hangs: LSP startup (rust-analyzer, gopls, pyright or basedpyright, tsserver), corpus acquisition, or the oracle query loop. Find why the baseline is invalid (corpus SHA drift). Keep a hypothesis/probe/result log.
2. **Repair:**
   - **Hangs:** per-LSP and per-query timeouts with explicit `oracle_timeout` classification (never silent), plus progress output.
   - **Validity:** the corpus pin and baseline drift. Either re-pin the corpus to a durable, reproducible snapshot under `~/prism-evidence` or `~/.local/share/prism`, or make the drift check report which component drifted.

     **Do not re-baseline to hide regressions.** If a new baseline is needed, produce it and list every flip candidate and regression for the controller and owner.
3. **JS/TS coverage:** make sure quick, or a documented `--lang ts,js` mode, reports JS/TS call-resolution precision and recall against the tsserver oracle on at least one public JS/TS corpus (X = Excalidraw, pinned at `~/prism-evidence/inputs/excalidraw-0642e72c/source`). That's needed so workstream 3/6 changes can be measured.
4. **Tests** for the timeout and classification logic. Keep `uv run tier-a --matrix-only` green.

## Final message
- the root causes, with evidence;
- the fixes;
- a quick run's wall time and validity status;
- the JS/TS numbers;
- any baseline changes, with their flip and regression lists;
- files with commit messages;
- what you did not verify.
