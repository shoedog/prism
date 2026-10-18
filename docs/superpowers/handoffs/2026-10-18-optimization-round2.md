# Handoff — Prism index optimization round 2 planning

**Written:** 2026-10-18, final targeted round-2 review fold · **By:** Codex /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-opt2 · plan/optimization-round2
**Measured starting state:** `[MEASURED]` committed clean HEAD `dcd0eb5a56da2635347468b9d48dc972133da2b9`, branch `plan/optimization-round2`; read-only probes: `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`. `[INHERITED]` prior merge `ccc5ae8a` includes main `9000343e`; original review baseline `9640b821`. **Delivery state:** only the plan and handoff are modified; controller commit remains pending. No `.git` writes attempted.
**Predecessor:** both round-2 reviews read in full: sol CHANGES (W1), Opus APPROVE (N1–N4, I-1–I-6). Closed findings folded on the existing artifact after the two-round cap as a converging targeted fold, **without a third review**.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts remain OPEN in §0.
**Provenance:** `[MEASURED]` denotes a live observation by this writer, not necessarily a performance measurement. `[INHERITED]` denotes supplied controller/reviewer/previous-planning receipts. No builds, tests or performance measurements were run in this fold.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorized only the review fold in these two documentation files. No subagents dispatched. Controller owns R0, commits and future implementation dispatch. **RESOLVED for document delivery.**

**(b) Custody exposure** — initial plan/handoff are committed and the starting tree was clean. Only the new fold awaits commit. Prior `snapshot-r1-fold/` and `snapshot-planning/` receipts are historical and do not contain this final fold. The current authorization permits edits to these two files only; no new snapshot files were written. **OPEN for controller commit/snapshot/publication custody.**

**(c) In flight / irreversible** — `[INHERITED]` controller is fixing nav PartialHit retention in **R0**, branch `fix/nav-partialhit-drop` (numeric PR ID not supplied; status **in flight**). R0 must land before A1 and before batch binary preparation. S6 soak occupies intervening nights; D13 waits for Phase 2 refresh frequency, R1b also waits for its no-change/changed split; R1a retry is UNKNOWN/deferred until Phase 2 or later evidence shows verifier divergence in practice. **OPEN for those dependencies; no worker implementation/process launched.**

**(d) Authorization** — current request: fold both reviews and all controller dispositions, read/write only, no cargo/builds/tests/measurements, no dotfile reads; controller commits. Those restrictions were honored. This handoff grants no implementation or measurement authority. Preserve the existing handoff shape; no dotfile template was reread during this fold.

## 1. Resume order

1. Read the plan and its §9 finding-to-fold table. Recheck branch/HEAD/status; expect `dcd0eb5a` plus this fold until the controller commits it. Do not mistake the fold's dirty state for the historical uncommitted original plan.
2. Controller commits the final fold; no third review is scheduled or required by this handoff. Confirm plan §7's nine owner questions before their respective gates; no scheduling job was created.
3. Obtain R0 landing/verification receipt. Then dispatch **A1 → A2 → A3a → A3b** as separate accepted-parent PRs: scalar core/hooks and neutrality; census; Rust production report modes; Python harness/readout. A3 means both A3a and A3b. No retry report mode/session barrier. Each has a guessed size envelope and two-round argument in plan §2. Fixture predictions are part of A1/A2 verification, not a separate duplicate phase.
4. Bind clean post-R0+A1–A3 binary receipts and exact corpus mirrors. Require byte-neutrality, production report-case preflight, finished soak, ≥50 GiB free and quiet admission before the sequential batch. Daytime-prime E/P/S changed mirrors with the bound binary, verify both true hits and record cache SHA256s. R0, A1, A2 and both A3 children must be merged and preflight green by Tue 2026-10-20 18:00 MDT for the Wed 10-21 window; otherwise move to the next 02:00–07:00 quiet window after acceptance. Wed is unlikely: the original four sequential review-capped PRs are now five. Never shorten reviews/gates.
5. Record all seven candidates and #9 sub-arms; #8 starts PARK BELOW RULE (derived), reopen only on its timer falsifier. Re-plan only supported implementation. R1b needs Phase 2/share threshold; D13a → D13b remains separate from R1a/R1b and uses matched daemon acceptance. Record R1a UNKNOWN/deferred; reopen only on practical verifier-divergence evidence.

**STOP conditions:** current read/write-only freeze; active soak or competing work; missing R0/A1–A3 dependency; absent/mismatched corpus/binary/workload identity; failed parity; missing counters/overflow; failed admission; broad recurring findings at the two-round cap. The original 01:15 planning deadline is historical, not this fold's deadline. No automatic second measurement window or inherited result substituted for a current control.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Committed starting identity | done | `[MEASURED]` clean `dcd0eb5a`; prior fold already committed, only this final fold pending. |
| Reviews, rounds 1 and 2 | folded | Round-1 disposition inherited; both round-2 reviews read in full. Sol r2 W1 corrected as WRONG; Opus N1–N4/I-1–I-6 folded as SMELLs. Plan §9 maps every ID. Two-round cap reached; no third review. |
| Source mechanisms | inherited | Prior fold/review source inspection and historical sizes retained as receipts; this final fold checks documents, not an independent source or performance re-verification. |
| R0 retention fix | in flight | `[INHERITED]` controller branch `fix/nav-partialhit-drop`; required landed-before-A1/batch. Pre-fix size unmeasured, derived/guessed; no independent #2 memory arm after it lands. |
| A1/A2/A3a/A3b | proposed | Scalar 800–1,000, census 500–700, Rust reports 400–600, Python harness 400–600 guessed weighted lines across small modules; split before dispatch if inventory exceeds envelope. No implementation performed. |
| Static document checks | done | Only requested two docs changed; whitespace/fences, finding coverage, schedule arithmetic and final diff inspected. Both documents remain below the file-size ceiling; no code/test/benchmark execution. |
| Current fold custody | pending controller | Only two authorized docs written; `snapshot-r1-fold/` is historical, not a snapshot of this fold. Controller commits/snapshots. |
| Fold commit/publication | pending | Controller action; no worker Git mutation or publication. |
| Batch / optimization acceptance | pending | Future gates only. Prior DFG/Go/C1/S6 receipts are inherited, not rerun or reauthenticated in this fold. |
| D13a/b / R1b / R1a | deferred | Phase 2 frequency / no-change split for D13 and R1b; R1a only reopens on practical verifier-divergence evidence. D13a retains current write lock; D13b adds off-lock build/readers and split lazy-edge acceptance. |

## 3. Corrections to standing documents and memory

| Location | Superseded assertion | Correction |
|---|---|---|
| Plan §1 S5 | Nav discards PartialHit before rebuild. | It retains decoded CG+DFG through rebuild/save/nav construction at this HEAD; R0 fixes that before the round-2 baseline. |
| Plan fixture rules | Tiny #6/#7 counts close/park corpus opportunities. | All fixtures validate mechanism/counter correctness only; candidate-wide absence requires population-independent structural proof. |
| Plan PR A / §3 | One large PR plus duplicate daytime phase. | A1/A2/A3a/A3b, all seven candidates represented; retry arm deferred; §3 predictions belong to A verification. |
| Plan gates | Decision-only L2; full #8 census/decoder slices; ambiguous M footprint level. | L2 ≥5% and ≥1 s of stale-to-ready on two corpora; #8 derived PARK with sole timer; steady 15% M arm fixed to median live heap, print footprint too. |
| Plan row 13 | Stall fix introduces old+new overlap; standalone is control. | Current refresh already overlaps old+new. D13a establishes flight/CAS/failure/retirement under the write lock; D13b adds concurrency. Explicit requests queue with individual FullOnly results; automatic waiters may join/recheck. Warm-old-edge peak gate and separate cold first-use census allowance; matched daemon control. |
| Plan schedule | Changed refresh optional/dropped first, vague boundary quiet waits. | Required E/P/S changed rows after core; retry removed. Projection at every child boundary; drop Ruff scalar colds → topology diagnostics → AST passes. Swap compares previous boundary and records own-child growth. 242 min + 58 margin, 85 children. |
| Original handoff | Original plan uncommitted; original deadline still live. | Original docs committed, starting tree clean; only this fold pending. Old deadline historical. |
| Opus round-1 I-5 | Sidecar version comment says #443/CPG v122. | Recorded only at `src/navigation/call_edge_cache.rs:112`; current CPG is v123. No code edit. |
| Memory | No relevant registry match. | No memory-derived facts used and no memory update requested or performed. |

## 4. Open work

| # | Work | Exact next action / dependency | Identifier |
|---:|---|---|---|
| 1 | Final fold custody | Controller commits/snapshots these docs; no third review or restart. | PR #478 |
| 2 | R0 landing | Controller finishes drop-before-rebuild fix and negative/byte verification; record actual commit/PR ID before A1. | fix/nav-partialhit-drop |
| 3 | Attribution implementation | Separate A1/A2/A3a/A3b, full gates and predictions per plan; no fixture-based size closure. | Plan §§2–3 |
| 4 | Window preparation | Confirm date, mirrors, L2, idle-daemon permission; build only in authorized daytime; df ≥50 GiB, finished soak; daytime E/P/S prime/hit/SHA receipts. | Tue 10-20 18:00 MDT cutoff; conditional Wed or next accepted 02:00–07:00 window |
| 5 | Sequential batch | 173 core/admission + 14 stale + 40 census + 15 custody = **242 min**, plus **58 min margin**; **85 children** = 58 core/control + 3 changed + 16 stale + 8 census. No retry rows. | Plan §5 |
| 6 | Supported implementation | Review all dispositions; separate accepted-parent slices and matched acceptance. #8 has no D8 dispatch. | Plan §6 |
| 7 | Frequency-dependent work | R1b waits for split/minimum share; D13a/b waits for frequency and matched daemon scenarios; R1a remains UNKNOWN until practical verifier divergence is evidenced. | S6 Phase 2; Q8 |

## 5. Invariants and traps — do not do these

- No cargo/tests/index warming/measurements or dotfile reads in this fold; no Git writes.
- R0 is a direct resource bug fix, not a measurement-gated #2 memory experiment. Do not count pre-fix retention in post-R0 #9v or batch baselines.
- All fixture-local zeros stay local; no size-based close/park of any arm. Structural candidate-wide absence proof must be independent of population size.
- L2 includes rebuild. Isolated stale lookup only provides decision time; unmatched reason/workload cannot advance #2. No-change proxy cannot advance #9v.
- #8 has only `reconstruct.arrays`; reopen sizing at ≥100 ms median on two corpora. No staging counters or decoder slice.
- Corpus cases run from production prism-mcp behind index-attribution. Lib tests are tiny and never install tree allocator hooks.
- Neutrality: A1 off/scalar, A2 adds census, A3a/A3b retain all three; includes Rayon 1/4 above the split threshold, fresh-process repeats, corpus byte cmp and ordering/summary mutants. Recorder-keyed files prevent same-process collisions; cfg(test) decode counters cannot serve release.
- M's steady 15% denominator is live heap, median of admitted runs; print both footprint and heap, classify footprint levels. Residual is not reclaimable bytes; census has no performance vote.
- D13 allows at most one active build per provider. Explicit requests queue, each retaining FullOnly verification/publication and strategy/generation/stale-before; automatic waiters may join then recheck. Mixed-request barriers assert max concurrency 1 and per-request fields; retain clean/still-stale auto tests (`connection_refresh_tests.rs:139-234`). D13a keeps the write lock; D13b adds off-lock freshness/Arc readers.
- D13b warm-old-edge peaks use the ordinary non-target gate. Cold-old-edge first-use rows are separate, with a disclosed additional allowance exactly equal to preregistered census-measured old-edge bytes. Standalone is reference only; current daemon is control.
- Preserve rollback, generation, summaries, cache bytes and old readers. No destructive drop-active shortcut. No feature-PR growth in ast.rs/cpg/build.rs; extract separately if necessary.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting source HEAD | `dcd0eb5a56da2635347468b9d48dc972133da2b9` |
| Merged main / original review | `9000343e` / `9640b821` |
| Branch/workspace | `plan/optimization-round2` / `/Users/wesleyjinks/code/prism-opt2` |
| Plan | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| Reviews / historical snapshot | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/plan-review/` / `snapshot-r1-fold/` |
| R0 | `fix/nav-partialhit-drop` (in flight; controller supplies landing SHA/PR ID) |
| Conditional future window | `2026-10-21T02:00:00-06:00` through `2026-10-21T07:00:00-06:00`; cutoff Tue 10-20 18:00 MDT, else next 02:00–07:00 quiet window after acceptance |
| Template continuity | Existing handoff structure retained; dotfile template not read in this fold. |

## 7. Refutation verdict and owner questions

**Verdict:** targeted fold complete; STATIC-ONLY, SELF-CHECKED, NOT INDEPENDENTLY RE-REVIEWED. Both round-1 W1 findings and sol round-2 W1 are addressed in the plan; all Opus round-2 notes are folded. Converging targeted fold after the two-round cap, without a third review. This is neither implementation nor performance acceptance; R0 status is inherited and still in flight.

**Owner questions (plan §7):**

1. Calendar: recommend conditional Wed October 21, 02:00–07:00 MDT only if R0/A1/A2/A3a/A3b are merged and preflight green by Tue October 20 18:00 MDT, with soak complete before admission; otherwise next quiet window after acceptance. Wed is unlikely with five sequential PRs (originally four); never compress reviews/gates.
2. Corpus paths: recommend exact clean pinned evidence-root mirrors, C1 workload bytes, fresh path-bound caches.
3. Confirm revised L2: recommend ≥5% and ≥1 s stale-to-ready on two corpora, retain decision timing; prefix parsing likely parks by derivation.
4. Refresh priority: recommend required E/P/S changed evidence, supported #9v next, no-change-only #1 claims; R1a UNKNOWN/deferred until practical divergence. D13a/b separate; one active build, explicit FullOnly queue with individual strategy/generation/stale-before, automatic join/recheck.
5. Tree boundary: recommend public eager compatibility, immutable-source leases, zero idle retention and bounded concurrency.
6. Conditional allocator cadence: recommend bounded idle/publication diagnostic with no-op control first, no per-query release.
7. Host target: specify distinct checkouts and simultaneous refreshes; recommend intended fan-out/S6 workload and initially serial heavy refreshes.
8. R1b frequency: wait for Phase 2 split; recommend recorded no-change share ≥20% before dispatch, otherwise park.
9. Idle daemons: recommend permission to stop only prism-mcp daemons with no connected client, checked immediately before stopping, after S6 soak ends.
