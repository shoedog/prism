# S1 Follow-ups (Perf Hardening + Level-4 Index Inversion)

> **Status:** Legacy query-layer note. See `docs/prism-query-layer/README.md` for current routing.

Deferred items from the S1 execution
(spec: `docs/superpowers/specs/2026-06-11-prism-s1-perf-level4-index-design.md` rev 3;
plan: `docs/superpowers/plans/2026-06-11-prism-s1-perf-level4-index.md` rev 2).
Every slice merged with containerized verify PASS + dual diff-review APPROVE.

## Priced-in risks carried verbatim (spec r2-13)

1. **B2 trigger:** the AST-based Level-4 extractor with provenance is "scheduled only once
   the Tier-A harness is live." B1 relocated the quirky text scanner into the index builder —
   somewhere *less visible* than the old hot loop — so this trigger must not silently slip.
   The retirement list (five named quirks, FN/FP-classified) is spec §4; the
   `level4_legacy_*` pins flip from behavior-pins to corrected-behavior docs at B2 time.
2. **§2a insertion-order cap:** lifting the serial-assembly constraint on C2 is S2-adjacent
   `NodeIndex`-identity work, **not** a C2 option. Cache bytes serialize insertion order
   with no `CACHE_VERSION` bump in S1.

## New findings from execution

3. **`collect_call_args` recursion is the next dominant hotspot.** With `all_functions`
   eliminated, ~98% of remaining `assemble_graph` time at tokio scale is the deeply
   recursive `ParsedFile::collect_call_args` walk under Step 5b (profile evidence in the S1
   PR). Step 5b is deliberately serial assembly territory, so C2 does not mask it.
   Candidate S1.5: per-file call-args index (same shape as the FunctionTable move).
   **→ RESOLVED 2026-06-19 — S1.5 SHIPPED (PR #111).** Per-file call-args index
   (`BTreeMap<(start_byte, callee_name), Vec<CallArg>>`, arg byte-spans, lazy `OnceLock` on
   `ParsedFile`) built once per file; `call_argument_texts_at` is now a map lookup. The legacy
   walk is frozen as a `#[cfg(test)]` reference oracle and an exhaustive parity test asserts
   `index == walk` across all 12 `Language::all()` variants + edge cases. Behavior-preserving
   (Tier-A `--matrix-only` 40 ok / 0 regressions). Cold-build measurements (`nav --no-cache
   call-stats`, branch vs `e72a0c8`): **hugo 86.4→18.4 s (4.68×)**, tokio 8.2→6.5 s (1.25×),
   prism 27.4→22.7 s (1.21×). Design/plan:
   `docs/superpowers/specs/2026-06-19-prism-s1.5-call-args-index-design.md` (rev 2) +
   `docs/superpowers/plans/2026-06-19-prism-s1.5-call-args-index.md`. The byte-keyed plural
   API only; the line-keyed `call_argument_texts` (absence_slice) and singular
   `call_argument_text_at` (callback resolution) stay as walks (cold/narrow — deferred).
4. **Local debug test-suite wall time is compile-dominated.** ~~Full `cargo test` ≈ 21 min~~
   **CORRECTED 2026-06-11 (WP2 Task 1 baseline, `docs/eval/wp2-timing.md`):** the
   21-minute observation does not reproduce on a healthy machine — measured clean
   `cargo test` = **119.5 s** (build 16.35 s incl. all 121 links; rest is execution of
   3,802 tests). The original number was almost certainly taken while the wedged
   `spindump` (item 5 below) was parking every process launch — 121 test binaries plus
   hundreds of compiler/linker launches, each delayed, inflates 2 min to ~21. (The
   "123" originally noted here counted two non-test targets; reconciled per the Tier-A
   spec review.)
   **→ Consolidation still ships as WP2 of
   `docs/superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md`** on the
   structural + container-verify case; the dev-loop emergency is withdrawn.
5. **Wedged macOS `spindump` can park process launches at `_dyld_start`**
   (dyld `RemoteNotificationResponder::blockOnSynchronousEvent`), masquerading as test
   hangs. Diagnosed during Task 10; cleared by killing `spindump`/`spindump_agent`.
   Operational note for future profiling-heavy sessions (heavy `sample` usage appears to
   provoke it).

## Carried from the plan

6. **Call-site migration off `Node`:** consumers still use reconstructed `Node`s via
   `all_functions()`; migrating the 28 call sites to `FunctionInfo` directly removes the
   reconstruction layer (hygiene, after S2's identity work lands on the table).
7. **C2 parity-test corpus:** restricted to `src/navigation` + `src/cpg` subsets after the
   whole-repo debug corpus made the gate ~20 min (the plan's own escape hatch). If parallel
   extraction grows beyond Phases 1–2/DFG, widen the corpus accordingly.
8. **Warm-path verdict (spec §7 warm-parity): PASS, remedy not needed.** Post-A warm rose
   0.46→0.64 s on prism (the anticipated r2-11 eager-table cost); post-C1/C2 final numbers
   beat baseline everywhere (prism 0.33 s, tokio 0.47 s, hugo 1.09 s) — C1's parallel parse
   more than absorbed the table cost. Lazy `OnceLock` fallback stays unused.
9. **C wall-clock gate (spec §7 row C): NOT MET at full-command granularity — recorded, not
   waived.** Cold-hugo user/wall = 135.8/129.8 ≈ 1.05 vs the ≥1.5 target. Cause: post-B1 the
   cold path is dominated by the *serial* Step-5b `collect_call_args` walk (item 3 above),
   diluting the ratio beyond the gate's stated caveat. C1/C2 parallelism is independently
   proven (exact-order + cache-byte parity tests; warm path −28..−48% where parse dominates).
   The gate should be re-measured after the item-3 follow-up removes the serial dominator;
   until then this row is open evidence, owner-accepted at merge per the report-out policy.
   **→ RE-MEASURED 2026-06-19 after S1.5 (item 3, PR #111) removed the serial dominator:**
   cold-hugo user/wall **1.09 → 1.42** (and the absolute cold time dropped 4.68×, 86.4→18.4 s
   — the dominator is gone). The ratio is still shy of the ≥1.5 target because the *remainder*
   of Step 5b stays deliberately serial — S1.5 made `collect_call_args` cheap, not parallel.
   Closing the residual ratio gap is a distinct follow-up (parallelize the rest of Step 5b),
   not part of S1.5. Row remains open evidence at that lower-severity framing.
