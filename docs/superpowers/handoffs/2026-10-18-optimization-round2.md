# Handoff — Prism optimization round 2, option B

**Written:** 2026-10-18 · **By:** Codex /root · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-opt2` · `plan/optimization-round2-b` · **Measured state:** `[MEASURED]` HEAD `f6c179276e9e3bdbe470df9a06f0af591380ced4`; starting tree CLEAN. Probes: `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`; outputs: workspace above, empty initial status, named branch and full HEAD above. Delivery edits only plan/handoff; controller commits, no Git writes attempted.
**Predecessor:** committed option-B revision `ed88c765` and review fold `f6c17927` (PR #481); prior review/dispositions inherited. **Status 2026-10-18:** owner answers folded; <1 GiB/cache-hit index and 12–16 indexed checkouts, serial heavy refreshes; C0 Python screen and #3→#10 priority recorded. Awaiting controller custody, no implementation or measurements performed.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0.
**Provenance:** written live from the two documents, owner answers/controller-supplied evidence and installed steering `bootstrap/handoff-template.md` structure. `[MEASURED]` claims are this turn’s identity/static checks; `[INHERITED]` source anchors, prior reviews and performance receipts were not re-read or rerun. No performance measurement or test execution occurred.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorizes only the two named docs; controller owns commits and future dispatch. No subagents dispatched. **RESOLVED for this owner-answer fold**; external worker activity was not inventoried, so confirm it before implementation/measurement.

**(b) Custody exposure** — `[MEASURED]` starting HEAD `f6c17927`, clean tree. Prior option-B/review folds are committed; **OPEN for controller commit/snapshot of this owner-answer fold** in the two tracked docs. No extra artifact or commit created by this worker.

**(c) In flight / irreversible** — `[INHERITED]` committed plan records landed R0 #479 (`0d3a5feb`) and R0b #480 (`433b252c`). Owner schedules S6 soak end **Tue 2026-10-20 ~12:08 MDT**; completion is not verified here. `[UNKNOWN]` Phase 2 plus Phase 3 refresh counts/log completeness and #9v peak phase remain unverified. **OPEN:** R1b requires ≥20% no-change over ≥10 refreshes; #9v requires the derived P+Py screen before V9a; D13 needs Phase 2; C0/estimate/acceptance runs require completed soak and admission. No implementation or measurement process launched.

**(d) Authorization granted but not exercised** — owner: “Edit only” the named plan and handoff; “No cargo, builds, index loads or measurements. Never read or print ~/.zshrc or other dotfiles.” “The controller commits; you cannot write `.git`.” Future-work wording grants no current execution authority. Owner permission for stopping clientless prism-mcp daemons is settled for post-soak measurement preparation, with immediate client recheck.

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-opt2 status --short --branch` and `git -C /Users/wesleyjinks/code/prism-opt2 rev-parse HEAD` (seconds, read-only). Expect `plan/optimization-round2-b`, HEAD `f6c17927` plus only these two edits or the controller’s subsequent docs commit. Read plan §8 “Owner-answer fold”.
2. Controller commits/snapshots this fold. One document-fold round, no further review dispatch; future implementation slices retain two review rounds. No measurement reservation/job exists. Owner answers settled; lifetime designs and evidence gates still apply.
3. Prepare pinned clean **pydantic-ai / NVIDIA-NeMo/Automodel / django** mirrors and the #6/#7 test-only estimate. **After confirmed S6 soak completion** (expected Tue 2026-10-20 ~12:08 MDT), run C0 daytime, one cold + two hits each, one process at a time; ≥50 GiB free on all input/cache/output volumes, no other index load, swap non-increasing at admission/boundaries. Select largest valid hit heap with cold ≤~25 min, modern/agentic tie preference; freeze winner’s pin and 200-request workload/hash. Nightly E/P/ruff/Py replaces slicing; existing profile stages name top cold/refresh costs on all four. Then run the one-off estimate on the revised four; no activation claim (plan §2).
4. Ship **S0 → T3a design immediately → T3b → bounded T3c families → T3d first acceptance → #10**. Public API preserved, zero idle trees, bounded concurrent leases; measure reparse memory/time and cap latency, evaluate per-language pools/partitions. T3b owns recorder, T3d reuses it. #10 seeds unchanged main cache plus incremental worktree edits; follow-on immutable shared daemon base + overlays is design-only, the route to 12–16 checkouts. R1b may interleave only after complete real-refresh ≥20%/≥10 gate (plan §4); R1a stays deferred.
5. Rebind **#9v** phase/tree ceilings after #3/#10: P+Py decisive, ruff spare; Py needs its own evidence, never slicing’s 419 MiB. UNKNOWN or below M parks before V9a. Then V9a→V9b, qualifying #6/#7; larger same-corpus #6/#7 estimate can move before V9a, never before #3/#10. Required changed rows E/P/Py plus ruff spare; no no-change proxy. #10/#11/#12 require bounded specs and M acceptance; #14 storage study is design-only, #13 skipped (plan §§4–6).
6. Keep **D13a → D13b** separate and Phase-2-gated. Default rollback/queued FullOnly/per-request results remain; only #11 expected strategy labels and opt-in #12 availability/recovery differ as scoped in §4. D13 cold-edge allowance remains existing `after_call_edges` minus `after_index` stage delta, no census.

**STOP conditions:** current read/write-only authorization; unmet frequency/design/feasibility gate; active soak or competing work; missing source/corpus/workload/cache identity; failed output parity; missing required changed rows; admission failure or insufficient disk/window; open-class findings at the two-round implementation cap. No automatic extra window or extension to rescue an admissible FAIL.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Original option-B revision / review fold | done | `[INHERITED]` ed88c765 option B; `[MEASURED]` current HEAD f6c17927, clean start, PR #481. |
| Owner-answer fold | done | Current docs fold six answers, controller evidence and gate exceptions; plan §8 mapping. Controller custody pending only for this fold. |
| R0 / R0b | done | `[INHERITED]` committed plan: #479 0d3a5feb / #480 433b252c. `[INHERITED]` prior review rebound macro branch at `src/cpg/build.rs:676-682`: drop :678, return :681; review’s :684 return does not match this checkout. No tests rerun. |
| A1/A2/A3a/A3b program | parked | Dropped by owner; historical inventory/reviews are not dispatch authority. |
| #6/#7 estimate | blocked | Preparation can be next; revised-four corpus run follows C0 and waits for confirmed soak end and ≥50 GiB/no-other-index-load/non-increasing-swap admission. Plan §2. |
| S0 changed-refresh seam | next | Independent behavior-neutral Rust/Python/tests predecessor, ships regardless of R1b; plan §§3/6. Requires new implementation dispatch. |
| R1b | blocked | `[UNKNOWN]` parsed refresh_index results in session transcripts: ≥20%, denominator ≥10 and complete classification; S0 predecessor. Plan §4 frequency gate. |
| #9v | blocked | After #3/#10, derived P+Py phase/ceiling screen with Py-specific evidence before V9a; V9b uses S0, changed E/P/Py plus ruff spare. Plan §§4–6. |
| Qualifying #6/#7 prototype | pending | Estimate M on two corpora then bounded spec; after #3/#10 and normally #9v; larger estimate can move before V9a. |
| #3 | next | T3a design immediately after S0; owner API/no-idle/cap answer settled. Safe lifetime design still before code; T3b recorder within 450–600 lines, reused in T3d. |
| C0 / #10 / #11 / #12 / #14 | pending | C0 post-soak; #10 after first #3 acceptance, #11/#12 bounded evaluations under M, #10 daemon base and #14 design-only. Plan §§1/2/4/6. |
| #2 / #8 | parked | Plan §4 derivations; sole #8 ≥100 ms/two-corpus array-loop falsifier only if cheap in another PR. |
| #9a / R1a | parked | Allocator recoverability/practical retry frequency UNKNOWN; no new work authorized. |
| D13a/D13b | blocked | Phase 2 frequency; separate daemon acceptance, existing report-stage allowance. |
| Commit, tests and acceptance | pending | Controller commits fold; full implementation verification and measurements are future work. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Headers / custody / resume | Starting HEAD ed88c765; six unanswered owner questions. | f6c17927 checked clean; six answers folded; only owner-answer fold awaits custody. |
| Plan §§3/5/6; this handoff | p95 without per-request data; changed seam without owner. | T3b recorder (reused by T3d), S0 independent predecessor, sized tests/mutants. |
| Plan §§4/6/7; this handoff | Undefined Phase 2 frequency and untested #9v feasibility. | Explicit logs/extraction/minimum ≥10; derived P+Py peak/ceiling screen, ruff spare. |
| Plan §§2/5/6; resume order | Estimate during soak; full n for every kind; qualifying #6/#7 unordered. | Post-soak admission; per-slice unaffected-kind sanity pairs; explicit #3→#10 priority, #6/#7 placement after them. |
| Plan §6 / D13 handoff | Cold allowance needs a census. | Existing report `after_call_edges` minus `after_index` stage delta. |
| Plan §1 / R0b anchor | Review alleges return at :684. | Current macro branch :676-682 contains drop :678 and return :681. |
| Historical attribution inventory/reviews | Proposed instrumentation predecessor program. | Superseded by committed option B; retained as history. No edits outside two authorized docs. |
| Owner answer changes | Slicing nightly; trees late; unconditional rollback/FullOnly; unspecified fan-out; idle permission pending. | Plan §§1–8 updated: Python C0, T3a after S0, #10 next, scoped #11/#12 exceptions, #14 design, 12–16 target, post-soak clientless stopping authorized. |
| Memory | No relevant registry match. | No memory-derived facts used; no memory update requested/performed. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Fold custody | pending | Controller commit/snapshot of two docs | Controller | PR #481, f6c17927 parent |
| 2 | #6/#7 estimate | blocked | Prepare script/mirrors; C0 post-soak, then revised-four estimate with admission | New dispatch; mirrors; completed soak; disk/load/swap | Plan §2/C0 |
| 3 | Changed-refresh seam | next | S0 Rust/Python/tests, edit/hash/restore controls, neutral support patch | New dispatch | Plan §§3/6 |
| 4 | Frequency / R1b | blocked | Parse refresh_index tool results from session transcripts; implement only if gate passes | ≥20% over ≥10, complete data; S0 | Plan §4 frequency gate |
| 5 | #9v | blocked | Rebound P+Py screen with Py-specific evidence after #3/#10, then V9a → V9b | Feasibility evidence; S0 for V9b | Plan §4; E/P/Py + ruff spare |
| 6 | Qualifying #6/#7 | pending | Bounded spec after #3/#10, normally #9v; larger estimate before V9a | Estimate M on two corpora | Plan §2 |
| 7 | #3 | blocked | T3a design immediately after S0, T3b recorder, T3c families, T3d activation | Safe lifetime design; S0 | Plan §§3/5/6 |
| 8 | Matched batches | pending | AB/BA ≥3 each for affected/scored kinds; one sanity pair only for per-slice unaffected kinds; ≤240-minute budget + ≥60-minute margin | Implementation; completed soak; quiet host; ≥50 GiB | Plan §§4–5; C0 pins/host admission |
| 9 | Row-13 stall | blocked | D13a → D13b with report-stage cold allowance | Phase 2 frequency | Plan §6 D13 |
| 10 | New candidates | pending | #10 seed spec after first #3 acceptance; #11/#12 mechanism/metric/gate specs; #14 and daemon base design notes | New dispatch; plan §4 M and scoped contracts | Plan §§1/4/6 |

## 5. Invariants and traps — do not do these

- No cargo/build/test/measurement/dotfile reads in this fold; controller commits. Next steps are not execution authority.
- Never revive A1–A3. S0 owns only the necessary behavior-neutral changed-refresh seam and ships even if R1b stays parked; other small counters stay prototype-local.
- R0/R0b are already in the base; their historical retention is not a new saving.
- R1b needs ≥20% no-change over ≥10 real refreshes from confirmed complete logs. Missing/below-minimum data parks it for later usage; synthetic M cannot satisfy the gate.
- #9v needs the derived P+Py screen before V9a, with Py’s own evidence, then changed-refresh acceptance; ruff is the spare. An end-of-refresh lifetime peak alone cannot locate the phase, and no-change data cannot accept changed-refresh relief.
- #3 keeps p95 ≤max(10%,10 ms) increase. T3b records each request in a fixed 256-entry u64 array (2 KiB plus state), emits report data and validates it; T3d reuses the identical base/candidate probe. Failed/incomplete/overflow timing invalidates a run.
- M is ≥200 MiB or ≥15% on two corpora; steady denominator is live heap with same-level footprint corroboration. Named peak candidates keep lifetime-footprint peaks. No subtraction of lifetime maxima to invent phase peaks.
- #6/#7 estimate is one-off/test-only, after confirmed soak end (expected Tue 2026-10-20 ~12:08 MDT); ≥50 GiB, no other index load, swap non-increasing. Estimate qualification needs a later bounded spec and matched activation; do not double-count overlapping families.
- One sanity pair only for explicitly unaffected kinds in plan §5; it proves no M/median gate, anomalies expand to full n. All affected/scored rows retain ≥3 runs/side and full non-target gates.
- #8 sole array-loop falsifier remains conditional; #9a residual does not prove reclaimable pages.
- Acceptance: one measurement at a time, ≥50 GiB, completed soak, five-minute quiet admission and per-child rechecks. Idle-daemon stopping is authorized post-soak with immediate client-free recheck; never drop required rows to fit a window.
- D13: at most one active build/provider; explicit requests queue with individual FullOnly summaries; automatic waiters join/recheck. Cold allowance is preregistered existing-report `after_call_edges` minus `after_index` current-footprint delta, with heap delta alongside, matched identity/report hash; absent/negative delta ⇒ UNKNOWN. Warm gate unchanged; no new census.
- Preserve default rollback/generations/output/cache/summary bytes and immutable old readers. Only #11 expected strategy labels and opt-in #12 warming/failure/recovery exceptions are authorized by plan §4; #12b must preserve/reload the last-good cache and matching source snapshot, test missing/corrupt cache and account for reader leases. No unscoped drop-active shortcut, temporary-tree borrowed Nodes or unbounded retention. Oversized AST/CPG files do not grow; extraction is separate and move-only.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting HEAD (committed review fold) | `f6c179276e9e3bdbe470df9a06f0af591380ced4` |
| Base main | `433b252c08dec870e009e70bd5af6d7603f94629` |
| Workspace / branch / PR | `/Users/wesleyjinks/code/prism-opt2` / `plan/optimization-round2-b` / #481 |
| Plan | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| Evidence / review | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/` / `plan-review/review-b-sonnet.md` |
| Landed fixes | R0 #479 `0d3a5feb`; R0b #480 `433b252c` |
| C1 harness / pins | `scripts/memreport/` / `scripts/memreport/workloads/MANIFEST.json` for E/P/ruff; Python pin/workload pending C0 |
| Supplied compact-DFG evidence | `~/prism-evidence/mcp-memory/dfg-compaction-2026-10-14/set-1/{prometheus,slicing}-verdict.json` (candidate v120; not re-derived) |
| Expected soak end | `Tue 2026-10-20 ~12:08 MDT`; verify completion before any corpus estimate/acceptance run |
| Future acceptance windows | `02:00–07:00 America/Denver`; no reservation made |
| Handoff template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: “The plan reflects owner answers without silently dropping existing gates.” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: plan §8 “Owner-answer fold” maps corpus/ordering and #11/#12 contract conflicts to scoped resolutions. Existing M, p95, admission, frequency, review caps and D13 gates retained. No independent review, implementation or performance acceptance claimed.

**Questions the owner owes an answer to:** None from the six-answer fold. Open operational evidence/design work: C0 winner pin/workload, safe tree lifetimes, R1b complete transcript counts (plan §4 retains exact extraction), rebound P+Py #9v phases, and bounded #10/#11/#12 specs. None authorizes execution in this docs-only turn.
