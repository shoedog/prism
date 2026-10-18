# Handoff — Prism index optimization round 2 planning

**Written:** 2026-10-18, round-1 review fold · **By:** Codex /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-opt2 · plan/optimization-round2
**Measured starting state:** `[MEASURED]` committed clean HEAD `ccc5ae8ae70d707b3210de46aebce5a97c958680`, merge of main `9000343e`; branch was ahead of its local remote-tracking ref by 8. Read-only probes: `git status --short --branch`, `git rev-parse HEAD`, `git log -4 --oneline`. The original docs were already committed at review baseline `9640b821`; their old commit-pending state is superseded. **Delivery state:** this fold modifies the existing plan and handoff only; its controller commit remains pending. No `.git` writes attempted.
**Predecessor:** original planning handoff, then both round-1 reviews under `plan-review/`; folded on the existing artifact, no restart. Round 1 of the two-round cap; round 2 remains outstanding.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts remain OPEN in §0.
**Provenance:** `[MEASURED]` denotes a live observation by this writer, not necessarily a performance measurement. `[INHERITED]` denotes supplied controller/reviewer/previous-planning receipts. No builds, tests or performance measurements were run in this fold.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorized only the review fold in these two documentation files. No subagents dispatched. Controller owns R0, commits and future implementation dispatch. **RESOLVED for document delivery.**

**(b) Custody exposure** — initial plan/handoff are committed and the starting tree was clean. Only the new fold awaits commit. Snapshot copies and SHA256 receipt are in `~/prism-evidence/mcp-memory/optimization-round2/plan-review/snapshot-r1-fold/`; original `snapshot-planning/` is historical. **RESOLVED for local snapshot custody; OPEN for controller commit/publication.**

**(c) In flight / irreversible** — `[INHERITED]` controller is fixing nav PartialHit retention in **R0**, branch `fix/nav-partialhit-drop` (numeric PR ID not supplied; status **in flight**). R0 must land before A1 and before batch binary preparation. S6 soak occupies intervening nights; D13 waits for Phase 2 refresh frequency, R1b also waits for its no-change/changed split. **OPEN for those dependencies; no worker implementation/process launched.**

**(d) Authorization** — current request: fold both reviews and all controller dispositions, read/write only, no cargo/builds/tests/measurements, no dotfile reads; controller commits. Those restrictions were honored. This handoff grants no implementation or measurement authority. Preserve the existing handoff shape; no dotfile template was reread during this fold.

## 1. Resume order

1. Read the plan and its §9 finding-to-fold table. Recheck branch/HEAD/status; expect the merge baseline above plus this fold until the controller commits it. Do not mistake the fold's dirty state for the historical uncommitted original plan.
2. Controller reviews/commits the fold and dispatches round 2 of 2. Confirm plan §7's nine owner questions before their respective gates; no scheduling job was created.
3. Obtain R0 landing/verification receipt. Then dispatch **A1 → A2 → A3** as separate accepted-parent PRs: scalar core/hooks and neutrality; census; production report modes plus harness/readout. Each has a guessed size envelope and two-round argument in plan §2. Fixture predictions are part of A1/A2 verification, not a separate duplicate phase.
4. Bind clean post-R0+A1–A3 binary receipts and exact corpus mirrors. Require byte-neutrality, production report-case preflight, finished soak, ≥50 GiB free and quiet admission before the sequential October 21 batch.
5. Record all seven candidates and #9 sub-arms; #8 starts PARK BELOW RULE (derived), reopen only on its timer falsifier. Re-plan only supported implementation. R1b needs Phase 2/share threshold; D13 remains separate from R1a/R1b and uses matched daemon acceptance.

**STOP conditions:** current read/write-only freeze; active soak or competing work; missing R0/A1–A3 dependency; absent/mismatched corpus/binary/workload identity; failed parity; missing counters/overflow; failed admission; broad recurring findings at the two-round cap. The original 01:15 planning deadline is historical, not this fold's deadline. No automatic second measurement window or inherited result substituted for a current control.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Committed starting identity | done | `[MEASURED]` clean `ccc5ae8a`, includes main `9000343e`; original docs already committed. |
| Both round-1 reviews | folded | Read in full; sol W1 and Opus W1 corrected as WRONG, remaining findings remain SMELL/recommendations. Plan §9 maps every ID. Round-2 review not yet performed. |
| Changed source mechanisms | checked | `[MEASURED]` PartialHit arm/owned payload, lazy call edges, daemon write/publication, tree-hook restriction and cfg(test) decode counter read directly. Original other source anchors and historical sizes remain inherited planning context. |
| R0 retention fix | in flight | `[INHERITED]` controller branch `fix/nav-partialhit-drop`; required landed-before-A1/batch. Pre-fix size unmeasured, derived/guessed; no independent #2 memory arm after it lands. |
| A1/A2/A3 | proposed | Scalar 800–1,000, census 500–700, harness 700–900 guessed weighted lines across small modules; split before dispatch if inventory exceeds envelope. No implementation performed. |
| Static document checks | done | Only requested two docs changed; whitespace/fences, finding coverage, schedule arithmetic and final diff inspected. Plan is fewer than 400 physical lines, hence below ~1,000 weighted lines; no code/test/benchmark execution. |
| Snapshot custody | done | Copies and SHA256SUMS under `plan-review/snapshot-r1-fold/`; includes a short read/write-only receipt. |
| Fold commit/publication | pending | Controller action; no worker Git mutation or publication. |
| Batch / optimization acceptance | pending | Future gates only. Prior DFG/Go/C1/S6 receipts are inherited, not rerun or reauthenticated in this fold. |
| D13 / R1b | deferred | S6 Phase 2 frequency / no-change split respectively; D13 has its own transition/barrier and matched daemon contract in plan §6. |

## 3. Corrections to standing documents and memory

| Location | Superseded assertion | Correction |
|---|---|---|
| Plan §1 S5 | Nav discards PartialHit before rebuild. | It retains decoded CG+DFG through rebuild/save/nav construction at this HEAD; R0 fixes that before the round-2 baseline. |
| Plan fixture rules | Tiny #6/#7 counts close/park corpus opportunities. | All fixtures validate mechanism/counter correctness only; candidate-wide absence requires population-independent structural proof. |
| Plan PR A / §3 | One large PR plus duplicate daytime phase. | A1/A2/A3, all seven candidates represented; §3 predictions belong to A verification. |
| Plan gates | Decision-only L2; full #8 census/decoder slices; ambiguous M footprint level. | L2 ≥5% and ≥1 s of stale-to-ready on two corpora; #8 derived PARK with sole timer; steady 15% M arm fixed to median live heap, print footprint too. |
| Plan row 13 | Stall fix introduces old+new overlap; standalone is control. | Current refresh already overlaps old+new. D13 adds concurrency; matched current/candidate daemon control, first-use old edges and N refreshes included. |
| Plan schedule | Changed refresh optional/dropped first, vague boundary quiet waits. | Required P/S changed rows immediately after core; retry drops first, block projection applies drop order, instantaneous child rechecks. |
| Original handoff | Original plan uncommitted; original deadline still live. | Original docs committed, starting tree clean; only this fold pending. Old deadline historical. |
| Opus I-5 | Sidecar version comment says #443/CPG v122. | Recorded only at `src/navigation/call_edge_cache.rs:112`; current CPG is v123. No code edit. |
| Memory | No relevant registry match. | No memory-derived facts used and no memory update requested or performed. |

## 4. Open work

| # | Work | Exact next action / dependency | Identifier |
|---:|---|---|---|
| 1 | Fold commit and round-2 review | Controller commits these docs, dispatches remaining review round; no restart. | PR #478 |
| 2 | R0 landing | Controller finishes drop-before-rebuild fix and negative/byte verification; record actual commit/PR ID before A1. | fix/nav-partialhit-drop |
| 3 | Attribution implementation | Separate A1/A2/A3, full gates and predictions per plan; no fixture-based size closure. | Plan §§2–3 |
| 4 | Window preparation | Confirm date, mirrors, L2, idle-daemon permission; build only in authorized daytime; df ≥50 GiB, finished soak. | 2026-10-21 02:00–07:00 MDT |
| 5 | Sequential batch | 170 core/admission + 20 stale + 40 census + 10 optional retry + 15 custody = **255 min**, plus **45 min margin**; 94 children maximum. | Plan §5 |
| 6 | Supported implementation | Review all dispositions; separate accepted-parent slices and matched acceptance. #8 has no D8 dispatch. | Plan §6 |
| 7 | Frequency-dependent work | R1b waits for split/minimum share; D13 waits for frequency and matched daemon scenario design. | S6 Phase 2; Q8 |

## 5. Invariants and traps — do not do these

- No cargo/tests/index warming/measurements or dotfile reads in this fold; no Git writes.
- R0 is a direct resource bug fix, not a measurement-gated #2 memory experiment. Do not count pre-fix retention in post-R0 #9v or batch baselines.
- All fixture-local zeros stay local; no size-based close/park of any arm. Structural candidate-wide absence proof must be independent of population size.
- L2 includes rebuild. Isolated stale lookup only provides decision time; unmatched reason/workload cannot advance #2. No-change proxy cannot advance #9v.
- #8 has only `reconstruct.arrays`; reopen sizing at ≥100 ms median on two corpora. No staging counters or decoder slice.
- Corpus cases run from production prism-mcp behind index-attribution. Lib tests are tiny and never install tree allocator hooks.
- Neutrality includes Rayon 1/4 above the split threshold, fresh-process repeats, corpus byte cmp and ordering/summary mutants. Recorder-keyed files prevent same-process collisions; cfg(test) decode counters cannot serve release.
- M's steady 15% denominator is live heap, median of admitted runs; print both footprint and heap, classify footprint levels. Residual is not reclaimable bytes; census has no performance vote.
- D13 single-flight/CAS/freshness/old-reader/failure/panic/retirement/lazy-edge barriers are mandatory. Standalone is reference only; current daemon is control.
- Preserve rollback, generation, summaries, cache bytes and old readers. No destructive drop-active shortcut. No feature-PR growth in ast.rs/cpg/build.rs; extract separately if necessary.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting source HEAD | `ccc5ae8ae70d707b3210de46aebce5a97c958680` |
| Merged main / original review | `9000343e` / `9640b821` |
| Branch/workspace | `plan/optimization-round2` / `/Users/wesleyjinks/code/prism-opt2` |
| Plan | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| Reviews / snapshot | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/plan-review/` / `snapshot-r1-fold/` |
| R0 | `fix/nav-partialhit-drop` (in flight; controller supplies landing SHA/PR ID) |
| Future window | `2026-10-21T02:00:00-06:00` through `2026-10-21T07:00:00-06:00` |
| Template continuity | Existing handoff structure retained; dotfile template not read in this fold. |

## 7. Refutation verdict and owner questions

**Verdict:** targeted fold complete; STATIC-ONLY, SELF-CHECKED, NOT INDEPENDENTLY RE-REVIEWED. Both round-1 W1 findings are addressed in the plan. Round 2 remains outstanding. This is neither implementation nor performance acceptance; R0 status is inherited and still in flight.

**Owner questions (plan §7):**

1. Calendar: recommend Wednesday October 21, 02:00–07:00 MDT, after soak.
2. Corpus paths: recommend exact clean pinned evidence-root mirrors, C1 workload bytes, fresh path-bound caches.
3. Confirm revised L2: recommend ≥5% and ≥1 s stale-to-ready on two corpora, retain decision timing; prefix parsing likely parks by derivation.
4. Refresh priority: recommend required changed-refresh evidence, supported #9v next, workload-specific #1 claims; D13 separate.
5. Tree boundary: recommend public eager compatibility, immutable-source leases, zero idle retention and bounded concurrency.
6. Conditional allocator cadence: recommend bounded idle/publication diagnostic with no-op control first, no per-query release.
7. Host target: specify distinct checkouts and simultaneous refreshes; recommend intended fan-out/S6 workload and initially serial heavy refreshes.
8. R1b frequency: wait for Phase 2 split; recommend recorded no-change share ≥20% before dispatch, otherwise park.
9. Idle daemons: recommend permission to stop only prism-mcp daemons with no connected client, checked immediately before stopping, after S6 soak ends.
