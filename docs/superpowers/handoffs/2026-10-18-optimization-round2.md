# Handoff — Prism optimization round 2, option B

**Written:** 2026-10-18 · **By:** Codex /root · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-opt2` · `plan/optimization-round2-b` · **Measured state:** `[MEASURED]` HEAD `ed88c7650feb2f103b5ee98d426383604e469e9e`; starting tree CLEAN; delivery contains only plan/handoff edits. Probes: `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`, `git log -2 --oneline`; outputs: workspace above, empty initial status, named branch, ed88c765 option-B docs commit over 433b252c (#480). Controller commits; no Git writes attempted.
**Predecessor:** committed option-B revision `ed88c765` (PR #481), single review `plan-review/review-b-sonnet.md`, and controller dispositions W1/S1–S7; S8 unchanged.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0.
**Provenance:** written live from the committed documents, supplied review/dispositions and limited current source inspection. `[MEASURED]` claims were inspected this turn; `[INHERITED]` claims name prior receipts, not reruns. No performance measurement or test execution occurred. Uses the installed steering `bootstrap/handoff-template.md` structure.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorizes only the two named docs; controller owns commits and future dispatch. No subagents dispatched. **RESOLVED for this final single-round fold**; external worker activity was not inventoried, so confirm it before implementation/measurement.

**(b) Custody exposure** — `[MEASURED]` original option-B revision is committed at `ed88c765`; starting tree was clean. Only this final review fold awaits controller commit/snapshot. **OPEN for controller custody of this fold**, not the earlier revision; no extra artifact or commit created by this worker.

**(c) In flight / irreversible** — `[INHERITED]` committed plan records landed R0 #479 (`0d3a5feb`) and R0b #480 (`433b252c`). Owner schedules S6 soak end **Tue 2026-10-20 ~12:08 MDT**; completion is not verified here. `[UNKNOWN]` Phase 2 plus Phase 3 refresh counts/log completeness and #9v peak phase remain unverified. **OPEN:** R1b requires ≥20% no-change over ≥10 refreshes; #9v requires the derived P+S screen before V9a; D13 needs Phase 2; estimate/acceptance runs require completed soak and admission. No implementation or measurement process launched.

**(d) Authorization granted but not exercised** — owner: “Read/write only; no cargo, builds, tests, measurements; never read or print ~/.zshrc or other dotfiles.” Also: “Controller commits.” Future-work wording grants no execution authority for estimate/prototypes.

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-opt2 status --short --branch` and `git -C /Users/wesleyjinks/code/prism-opt2 rev-parse HEAD` (seconds, read-only). Expect `plan/optimization-round2-b`, HEAD `ed88c765` plus only these two edits or the controller’s subsequent docs commit. Read the current plan and its “Option-B review fold” table.
2. Controller commits/snapshots this final fold. Resolve the six owner questions at their gates. This fold has one final round; future implementation slices have two review rounds each. No measurement reservation/job exists.
3. **Next daytime task is preparation only:** dispatch the bounded #6/#7 test-only estimate script/ignored test and prepare pinned mirrors. **Run only after confirmed S6 soak completion**, expected Tue 2026-10-20 ~12:08 MDT, with ≥50 GiB free on input/cache/output volumes, no other index load and swap non-increasing at admission/between sequential corpus loads. Record separate net M estimates; no activation claim. A qualifying result needs a bounded spec; default placement is after #9v and before #3, moved before V9a only if its same-corpus net estimate exceeds #9v’s estimated benefit (plan §2).
4. Ship **S0 — changed-refresh seam** regardless of R1b (350–550 weighted lines including Rust/Python tests, feature-gated, behavior-neutral, recorded edit/hash and restore, identical support patch on base/candidate). It precedes R1b and V9b. Extract from real `refresh_index` tool results in Claude Code/Codex session transcripts (soak logs carry no refresh summary; controller-verified 2026-10-18): count `refresh_index` responses with summary `stale_before_refresh == false` over all refreshes; deduplicate log copies and confirm actual log fields/identities with the controller (plan §7 Q5). **R1b only at ≥20%, denominator ≥10, complete classification**; absent/insufficient/below-threshold data parks it for later usage. R1a stays deferred.
5. Before **V9a → V9b**, derive from existing C1 samples whether the refresh peak occurs in verification or construction and apply the inherited tree-set ceilings. **P+S are decisive**: park #9v if they cannot clear M even at 100% of the ceiling, or if phase evidence is unavailable. Required changed rows are E/P/S plus **ruff changed-refresh spare**; no no-change acceptance proxy. Use accepted R1b parent if it lands; parked R1b does not block S0/#9v. Place any qualifying #6/#7 prototype as in step 3, then **owner tree-lease/API design → T3a → T3b → bounded T3c families → T3d**. T3b owns the fixed-array per-request recorder, report validation and mutant; T3d reuses it for the unchanged AST p95 gate.
6. Keep **D13a → D13b** separate and gated on Phase 2 frequency. Queue/CAS/freshness/failure/retirement contracts remain; cold-old-edge allowance is the existing report’s `after_call_edges` minus `after_index` stage delta, not a census (plan §6).

**STOP conditions:** current read/write-only authorization; unmet frequency/design/feasibility gate; active soak or competing work; missing source/corpus/workload/cache identity; failed output parity; missing required changed rows; admission failure or insufficient disk/window; open-class findings at the two-round implementation cap. No automatic extra window or extension to rescue an admissible FAIL.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Original option-B revision | done | `[MEASURED]` committed at ed88c765, PR #481; not awaiting its first commit. |
| Final single-review fold | done | Current two docs fold W1/S1–S7, leave S8 density as is; plan §8 mapping. Controller custody remains pending. |
| R0 / R0b | done | `[INHERITED]` committed plan: #479 0d3a5feb / #480 433b252c. `[MEASURED]` macro branch re-read at `src/cpg/build.rs:676-682`: drop :678, return :681; review’s :684 return does not match this checkout. No tests rerun. |
| A1/A2/A3a/A3b program | parked | Dropped by owner; historical inventory/reviews are not dispatch authority. |
| #6/#7 estimate | blocked | Preparation can be next; corpus run waits for confirmed soak end and ≥50 GiB/no-other-index-load/non-increasing-swap admission. Plan §2. |
| S0 changed-refresh seam | next | Independent behavior-neutral Rust/Python/tests predecessor, ships regardless of R1b; plan §§3/6. Requires new implementation dispatch. |
| R1b | blocked | `[UNKNOWN]` parsed refresh_index results in session transcripts: ≥20%, denominator ≥10 and complete classification; S0 predecessor. Plan §7 Q5. |
| #9v | blocked | Derived P+S phase/ceiling screen before V9a; V9b uses S0, changed E/P/S plus ruff spare. Plan §§4–6. |
| Qualifying #6/#7 prototype | pending | Estimate M on two corpora then bounded spec; after #9v/before #3 unless estimate exceeds #9v’s, then before V9a. |
| #3 | blocked | Q3 design before T3a/T3b; recorder included in T3b’s 450–600-line total, reused in T3d. |
| #2 / #8 | parked | Plan §4 derivations; sole #8 ≥100 ms/two-corpus array-loop falsifier only if cheap in another PR. |
| #9a / R1a | parked | Allocator recoverability/practical retry frequency UNKNOWN; no new work authorized. |
| D13a/D13b | blocked | Phase 2 frequency; separate daemon acceptance, existing report-stage allowance. |
| Commit, tests and acceptance | pending | Controller commits fold; full implementation verification and measurements are future work. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Headers / custody / resume | HEAD 433b252c; original revision uncommitted. | ed88c765 is committed; only final fold awaits controller custody. |
| Plan §§3/5/6; this handoff | p95 without per-request data; changed seam without owner. | T3b recorder (reused by T3d), S0 independent predecessor, sized tests/mutants. |
| Plan §§4/6/7; this handoff | Undefined Phase 2 frequency and untested #9v feasibility. | Explicit logs/extraction/minimum ≥10; derived P+S peak/ceiling screen, ruff spare. |
| Plan §§2/5/6; resume order | Estimate during soak; full n for every kind; qualifying #6/#7 unordered. | Post-soak admission; per-slice unaffected-kind sanity pairs; explicit #6/#7 placement. |
| Plan §6 / D13 handoff | Cold allowance needs a census. | Existing report `after_call_edges` minus `after_index` stage delta. |
| Plan §1 / R0b anchor | Review alleges return at :684. | Current macro branch :676-682 contains drop :678 and return :681. |
| Historical attribution inventory/reviews | Proposed instrumentation predecessor program. | Superseded by committed option B; retained as history. No edits outside two authorized docs. |
| Memory | No relevant registry match. | No memory-derived facts used; no memory update requested/performed. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Fold custody | pending | Controller commit/snapshot of two docs | Controller | PR #481, ed88c765 parent |
| 2 | #6/#7 estimate | blocked | Prepare script; run once post-soak with admission, record net M arithmetic | New dispatch; mirrors; completed soak; disk/load/swap | Plan §2/Q1 |
| 3 | Changed-refresh seam | next | S0 Rust/Python/tests, edit/hash/restore controls, neutral support patch | New dispatch | Plan §§3/6 |
| 4 | Frequency / R1b | blocked | Parse refresh_index tool results from session transcripts; implement only if gate passes | ≥20% over ≥10, complete data; S0 | Plan §7 Q5 |
| 5 | #9v | blocked | Existing C1 P+S phase/ceiling screen, then V9a → V9b | Feasibility evidence/Q2; S0 for V9b | Plan §4; E/P/S + ruff spare |
| 6 | Qualifying #6/#7 | pending | Bounded spec; place after #9v/before #3 unless larger estimate, then before V9a | Estimate M on two corpora | Plan §2 |
| 7 | #3 | blocked | Owner design, T3a/T3b recorder, T3c families, T3d activation | Q3 | Plan §§3/5/6 |
| 8 | Matched batches | pending | AB/BA ≥3 each for affected/scored kinds; one sanity pair only for per-slice unaffected kinds; ≤240-minute budget + ≥60-minute margin | Implementation; completed soak; quiet host; ≥50 GiB | Plan §§4–5; Q1/Q4/Q6 |
| 9 | Row-13 stall | blocked | D13a → D13b with report-stage cold allowance | Phase 2 frequency | Plan §6 D13 |

## 5. Invariants and traps — do not do these

- No cargo/build/test/measurement/dotfile reads in this fold; controller commits. Next steps are not execution authority.
- Never revive A1–A3. S0 owns only the necessary behavior-neutral changed-refresh seam and ships even if R1b stays parked; other small counters stay prototype-local.
- R0/R0b are already in the base; their historical retention is not a new saving.
- R1b needs ≥20% no-change over ≥10 real refreshes from confirmed complete logs. Missing/below-minimum data parks it for later usage; synthetic M cannot satisfy the gate.
- #9v needs the derived P+S screen before V9a, then changed-refresh acceptance; ruff is the spare. An end-of-refresh lifetime peak alone cannot locate the phase, and no-change data cannot accept changed-refresh relief.
- #3 keeps p95 ≤max(10%,10 ms) increase. T3b records each request in a fixed 256-entry u64 array (2 KiB plus state), emits report data and validates it; T3d reuses the identical base/candidate probe. Failed/incomplete/overflow timing invalidates a run.
- M is ≥200 MiB or ≥15% on two corpora; steady denominator is live heap with same-level footprint corroboration. Named peak candidates keep lifetime-footprint peaks. No subtraction of lifetime maxima to invent phase peaks.
- #6/#7 estimate is one-off/test-only, after confirmed soak end (expected Tue 2026-10-20 ~12:08 MDT); ≥50 GiB, no other index load, swap non-increasing. Estimate qualification needs a later bounded spec and matched activation; do not double-count overlapping families.
- One sanity pair only for explicitly unaffected kinds in plan §5; it proves no M/median gate, anomalies expand to full n. All affected/scored rows retain ≥3 runs/side and full non-target gates.
- #8 sole array-loop falsifier remains conditional; #9a residual does not prove reclaimable pages.
- Acceptance: one measurement at a time, ≥50 GiB, completed soak, five-minute quiet admission and per-child rechecks. Idle-daemon stopping requires Q6 and immediate client-free recheck; never drop required rows to fit a window.
- D13: at most one active build/provider; explicit requests queue with individual FullOnly summaries; automatic waiters join/recheck. Cold allowance is preregistered existing-report `after_call_edges` minus `after_index` current-footprint delta, with heap delta alongside, matched identity/report hash; absent/negative delta ⇒ UNKNOWN. Warm gate unchanged; no new census.
- Preserve rollback/generations/output/cache/summary bytes and immutable old readers. No drop-active shortcut, temporary-tree borrowed Nodes or unbounded retention. Oversized AST/CPG files do not grow; extraction is separate and move-only.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting HEAD (committed option B) | `ed88c7650feb2f103b5ee98d426383604e469e9e` |
| Base main | `433b252c08dec870e009e70bd5af6d7603f94629` |
| Workspace / branch / PR | `/Users/wesleyjinks/code/prism-opt2` / `plan/optimization-round2-b` / #481 |
| Plan | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| Evidence / review | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/` / `plan-review/review-b-sonnet.md` |
| Landed fixes | R0 #479 `0d3a5feb`; R0b #480 `433b252c` |
| C1 harness / pins | `scripts/memreport/` / `scripts/memreport/workloads/MANIFEST.json` |
| Expected soak end | `Tue 2026-10-20 ~12:08 MDT`; verify completion before any corpus estimate/acceptance run |
| Future acceptance windows | `02:00–07:00 America/Denver`; no reservation made |
| Handoff template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: “The option-B plan supplies the evidence and ownership needed to dispatch and accept its ordered prototypes.” · pass: SELF-PASS (NOT INDEPENDENT) for this final fold · evidence tier: STATIC-ONLY · record: supplied single review `plan-review/review-b-sonnet.md` found W1 and S1–S7; controller dispositions folded in plan §8 and this handoff. No second independent review, implementation, test or performance acceptance claimed.

**Questions the owner owes an answer to:**

1. **Corpus mirrors:** exact clean pinned C1 Git mirrors in evidence root, unchanged workload bytes and fresh path-bound caches.
2. **Refresh priority / changed evidence:** S0 regardless of R1b; gated R1b then #9v only after derived P+S phase/ceiling screen. E/P/S changed rows plus ruff spare; ≥2 admissible corpora. Qualifying #6/#7 goes after #9v/before #3 unless its estimate exceeds #9v’s, then before V9a. D13 separate; retry deferred.
3. **Tree-lease/API boundary:** session-local immutable-source leases, eager public compatibility, zero idle retention, bounded concurrency; owner design before T3 code. T3b owns latency recorder, T3d reuses it.
4. **Host target:** define distinct retained checkouts and simultaneous refreshes; recommend intended S6 fan-out and initially serial heavy refreshes with non-Prism headroom.
5. **R1b frequency:** source = parsed `refresh_index` tool results in Claude Code/Codex session transcripts (the soak logs carry stderr only; controller-verified 2026-10-18). Numerator = `refresh_index` responses with `stale_before_refresh == false`; denominator = all refreshes, deduplicated across logs, failures retained in denominator. Require ≥20% and ≥10 with complete classification; absent/ambiguous data, fewer than 10 or lower share parks R1b until later usage. Plan §7 Q5 defines extraction; synthetic M is insufficient.
6. **Idle daemons:** permission to stop only client-free prism-mcp daemons after confirmed soak end, rechecked immediately; active clients remain untouched.

Calendar, L2 and allocator policy questions fell away under B. No pending approval is needed for this document fold.
