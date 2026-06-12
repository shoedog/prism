# Merged Plan Review — Tier-A Harness + WP2 (synthesis of Executability + Coverage lenses)

Both lenses returned full reviews. Findings below are de-duplicated; lens disagreements resolved inline.

## BLOCKER

1. **Tasks 2–5 — consolidation script lives at `/tmp`, uncommitted.** Task 2 creates `/tmp/consolidate.py` ("not committed", plan lines 95–101) and Tasks 3–5 keep invoking it (lines 192, 226, 262–265). A fresh worker or resumed session from the committed tree cannot execute batches 2–4. **Fix:** commit it under `scripts/` (or embed its full re-creation in each task that uses it).

2. **Task 19 — runner CLI is prose, not code, and its load-bearing logic is untested.** (Merging both lenses; Executability's BLOCKER rating is right — the plan as written produces a nonfunctional `uv run tier-a`.) Step 4 (line 3323) gives no `main`, no arg parsing, no orchestration of oracle/SUT/metrics/floors/report-writing. Compounding it (Coverage): spec §2.12's *required* self-tests for `inventory_miss`/`sut_error`/`oracle_error` accounting and validity floors have no task — they gate G4 via `baseline_invalid`, and a bug there silently corrupts P/R. And the sole cli.py test pins only the no-`probes` identity branch of `recompute_metrics_from_stored` — the real G3 replay property goes untested until the live run, where a no-op implementation would pass the pinned test. **Fix:** extract a TDD'd `accounting.py` task (probe outcome bookkeeping, floors, §2.5 `inventory_miss`→all-FN rule) between Tasks 16 and 19; give the replay test a fixture *with* a `probes` key; provide complete coded `cli.py` (or split it into coded, tested subtasks) so it is genuinely thin glue.

3. **Task 12 → 19 — oracle lifecycle API gap.** `LspOracle`'s docstring promises "per-method wrappers that raise OracleError" (line 1253), but no `OracleError` class or wrappers for `documentSymbol`, `definition`, `prepareCallHierarchy`, incoming/outgoing calls exist — and Task 19's run flow requires exactly those operations (line 3328). The pure mapping functions are fine (first-pass overstatement corrected); the gap is the callable lifecycle methods. **Fix:** add concrete wrapper methods + `OracleError`, tested against the fake client, before wiring the runner. Fold in `version()` on both seams (Coverage): Task 19's own report fixture displays `"rust-analyzer 1.94.0"` with no producer.

4. **Task 20 — tokio is in `corpora.toml` but never prepared.** Config declares `~/code/bench-repos/tokio` (lines 3202–3204) and Step 3 watches tokio indexing, but Step 1 only preps caddy/flask/click (lines 3396–3402). (Coverage verified tokio currently exists on this host — but the plan must not depend on that accident.) **Fix:** add tokio clone/copy + SHA pinning to Step 1.

## MAJOR

5. **Tasks 14/19 — the §2.4 universe filter is never applied to prism's inventory.** Spec §2.4 requires the *same* include/exclude filter on `nav functions` output; `functions_inventory` walks the whole repo, `inventory_diff` takes raw lists, and Task 19's M1 flow names no drop step. On the prism corpus specifically, `tests/fixtures/` and Task 18's 29 new deliberately-broken `eval/fixtures/` sources flood `prism_extra` and corrupt the headline M1 numbers. **Fix:** a tested prism-records∩universe filter step in Task 14, named explicitly in Task 19's run flow.

6. **Task 20 (contingent) — the pyright references-based fallback is invoked but never built.** Step 2 says "switch to the references-based fallback (spec §2.2) before scoring," and §2.2 flags pyright call-hierarchy support as the unverified precondition — yet no task implements references→caller-fn mapping. If pyright and basedpyright both fail, the orchestrator baseline stalls into unplanned development. **Fix:** either a small contingency task (`compare.caller_fn_sets` already fits), or cheaper: run the 5-minute live pyright probe immediately after Task 12 so the fallback is planned or struck early.

7. **Tasks 11/12 — LSP initialization is likely insufficient for live servers.** `LspClient.start()` hardcodes `"rootUri": None` (line 967) and `LspOracle` passes no `rootUri`/`workspaceFolders`/initializationOptions. rust-analyzer/gopls/pyright commonly need workspace context for documentSymbol and call hierarchy. **Fix:** parameterize initialize params from `root` before Task 20.

8. **Task 1 — writes to `docs/eval/wp2-timing.md` but `docs/eval/` doesn't exist.** **Fix:** `mkdir -p docs/eval` before writing.

9. **Task 6 — the backtrace verification verifies nothing.** The command (line 344) is expected to *pass*, so no backtrace is emitted and the `src/...:NN` claim is never checked. **Fix:** use a deliberate temporary failing test (or known-failing command) to confirm `line-tables-only` preserves file:line frames.

## MINOR

10. **Task 16 — adjudication tests skip two verdict classes.** §2.12 requires "each verdict class"; `ambiguous` and `alias_site` ride the untested `else → excluded` branch. Add two test rows.

11. **Task 18 — the §2.12 matrix-vs-real-binary self-test never lands.** Tests use `FakeSut` only; Step 5's reconciliation is an uncommitted one-off. Add the spec-named `pytest.mark.skipif`(binary absent) test so future fixture edits get an automated real-binary check.

12. **Tasks 13/15 — pinned probe #4 has no execution pathway.** The `ambiguous_symbol_error` probe needs a bare-symbol invocation, but `PrismCli` only exposes location-seeded calls and `_run` collapses every nonzero exit into generic `SutError`, so the safe-fail contract is indistinguishable from a crash. Add `callers_by_symbol` + error classification to Task 13.

13. **Task 5 — sweep scope is narrower than G6's "repo-wide grep clean".** Stale `--test <old-name>` refs verified in root `STATUS-prism-cwe-phase2/3.md`, three `docs/prism-query-layer/*.md` docs, and `.claude/settings.local.json` (live config — a permission entry that silently stops matching). Fix the settings entry; explicitly exempt or fix the historical docs in the commit message.

14. **Task 10 — `eval/tier_a/interfaces.py` is created by no task.** Spec §2.1 names the file and defines the `Oracle`/`SystemUnderTest` PEP 544 protocols (the multilspy/SCIP/Rust-rewrite swap seam); the plan leaves the seam as duck-typed convention. Fold into Task 10.

15. **Task 18 Step 5 — needs an execution-model annotation.** Reconciliation needs the release binary *and* Python+uv in one environment; whether the container has the latter is unresolved until kickoff. Mark the orchestrator-fallback condition. (Task 13 Step 1 is fine — in-container, offline.)

16. **Spec hygiene — corpus-count drift.** Spec §2.11's comment says "4 corpora" while G4/§2.9 and the plan say 5. Executability rated this MAJOR for gate-interpretation risk; Coverage's call is right that it's a one-word spec fix with no build impact — MINOR, but do it before handoff.

**Dropped from both first passes:** the `use common::*` rewrite concern (the script's prefix replace does handle it — verified) and the Task 13 container-split concern (the container builds and runs the binary offline).

**Strengths preserved:** WP2 arithmetic verified exact against the live tree (48/37/32/4 over 121); Cargo.toml block order matches the transform regex; externally-tagged `Reason` serde matches `_why()`; the `slicing`-named clap command makes Task 8's regex correct; "the frozen wire sample is the contract" and "the binary is the referee for fixture statuses" are the right epistemics.

**Verdict:** Not executable as-is — fix the four BLOCKERs (commit the consolidation script, code+test the runner/accounting layer with a real G3 replay fixture, add the oracle lifecycle wrappers, prep tokio) plus the M1 universe filter before building; everything else is line-item edits to existing tasks.