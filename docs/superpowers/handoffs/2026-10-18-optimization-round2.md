# Handoff — Prism optimization round 2, option B

**Written:** 2026-10-18 · **By:** Codex /root · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-opt2` · `plan/optimization-round2-b` · **Measured state:** `[MEASURED]` HEAD `433b252c08dec870e009e70bd5af6d7603f94629`; starting tree CLEAN; delivery has only plan/handoff edits. Probes: `pwd`, `git status --short --branch`, `git rev-parse HEAD`, `git log -6 --oneline`; output: branch `plan/optimization-round2-b...origin/main`, no initial changes, HEAD `433b252c`, history includes #479/#480. Controller commits; no Git writes attempted.
**Predecessor:** committed attribution-first plan (#478), stopped A1 inventory, and explicit owner option B decision of 2026-10-18.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0.
**Provenance:** written live by the worker from the owner decision, current source/history, prior plan and A1 inventory. `[MEASURED]` claims were inspected this turn; `[INHERITED]` claims name prior receipts, not reruns. No performance measurement or test execution occurred. Uses the installed steering `bootstrap/handoff-template.md` structure.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner authorizes revision of the two named docs; controller owns commits and future dispatch. No subagents dispatched. **RESOLVED for this document revision**; live external worker activity was not inventoried, so confirm it before future implementation/measurement.

**(b) Custody exposure** — `[MEASURED]` starting tree was clean at `433b252c`; revised plan and handoff now await controller commit/snapshot. Earlier snapshots and `a1/INVENTORY.md` are historical and do not contain this revision. **OPEN for controller custody**; no extra artifact or commit created by this worker.

**(c) In flight / irreversible** — `[MEASURED]` R0 #479 (`0d3a5feb`) and R0b #480 (`433b252c`) are landed in current history, with early drops inspected in source. `[INHERITED]` S6 soak through October 20 is owner context. `[UNKNOWN]` Phase 2 real-round no-change frequency and completed-soak status have not been supplied/verified here. **OPEN:** R1b dispatch needs ≥20%; D13 needs Phase 2; measurements need completed soak and quiet admission. No implementation or measurement process launched by this worker.

**(d) Authorization granted but not exercised** — owner: “Read and write only: no cargo, builds, tests or measurements. Never read or print ~/.zshrc or other dotfiles.” Also: “The controller commits.” This revision grants no execution authority for the future estimate or prototypes; those are the next controller dispatches.

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-opt2 status --short --branch` and `git -C /Users/wesleyjinks/code/prism-opt2 rev-parse HEAD` (seconds, read-only). Expect branch `plan/optimization-round2-b`, starting HEAD `433b252c` plus these two edits or the controller's subsequent docs commit. Read the current plan, not the superseded attribution inventory as a dispatch spec.
2. Controller commits/snapshots the plan and handoff. Resolve the six owner questions below at their respective gates; no fixed measurement date or job exists.
3. **Next daytime task:** dispatch the bounded one-off #6/#7 estimate script or `#[ignore]`d census test on the pinned C1 corpora, per plan §2. Test-only, run once, record each candidate's net estimate; advance only if M on two corpora. It adds no shipped instrumentation and cannot accept an optimization.
4. **Wait for S6 Phase 2:** record real-round no-change numerator/denominator/period. Dispatch first prototype **R1b only at ≥20%**; otherwise park it. R1a retry stays deferred pending practical verifier divergence evidence.
5. Second prototype **#9v: V9a → V9b**, judged on changed-refresh peak (E/P/S rows, ≥2 admissible corpora). Use accepted R1b parent if it lands; a parked R1b does not block #9v. Third **#3: owner tree-lease/API design → T3a → T3b → bounded T3c consumer families → T3d activation**. Each slice has two review rounds; each optimization needs its own matched acceptance batch.
6. Keep **D13a → D13b** separate and gated on Phase 2 frequency. Its queue/CAS/freshness/failure/retirement and warm/cold old-edge contracts are unchanged in plan §6.

**STOP conditions:** current read/write-only authorization; unmet frequency/design gate; active soak or competing work; missing/mismatched source/corpus/workload/cache identity; failed output parity; missing required changed-refresh rows; quiet admission failure or insufficient disk/window budget; open-class findings at the two-round cap. No automatic extra window or extension to rescue an admissible FAIL.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Option B plan/handoff revision | done | `[MEASURED]` current two documents replace attribution-first dependencies with ordered prototypes and per-prototype matched batches. |
| R0 nav PartialHit drop | done | `[MEASURED]` #479, `0d3a5feb`; `src/navigation/cache.rs:77-91`, explicit drop at :82 before rebuild. Tests not rerun here. |
| R0b CPG fallback drops | done | `[MEASURED]` #480, `433b252c`; `src/cpg/build.rs:628-632,676-681,714-719`, `src/cpg/context.rs:149-153`, `src/navigation/mod.rs:157-166`. Tests not rerun here. |
| A1/A2/A3a/A3b program | parked | Dropped by owner, not awaiting dispatch. `[INHERITED]` stopped inventory estimates ~1,855 A1 lines; owner estimates six PRs/~3,500 lines before optimization. Plan opening decision and §8 dropped record. |
| #6/#7 estimate | next | Plan §2: one-off test-only estimate on four C1 corpora; separate net M decisions, no shipped behavior. |
| R1b first prototype | blocked | `[UNKNOWN]` Phase 2 ≥20% real-round no-change frequency; plan §§4/6/7 Q5. |
| #9v second prototype | pending | V9a contract then V9b activation; changed-refresh peak M, no no-change proxy. Plan §§3–6. |
| #3 third prototype | blocked | Owner tree-lease/API design before code, then T3a/T3b and bounded completion slices. Plan §6/Q3. |
| #2 / #8 | parked | Derivations retained in plan §4. #8 sole ≥100 ms/two-corpus array-loop falsifier only if cheap in another PR; otherwise unmeasured record. |
| #9a / R1a | parked | `[UNKNOWN]` allocator recoverability / practical retry frequency. No allocator work absent a large prototype-batch residual; retry stays deferred. |
| D13a/D13b row-13 stall | blocked | `[UNKNOWN]` Phase 2 frequency; existing split/contracts and matched daemon acceptance retained unchanged. |
| Commit, tests and acceptance | pending | Controller commit; implementation/full verification and matched measurements are future work, not executed by this docs revision. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Plan summary/sequence and this handoff | Attribution PRs and one selection batch precede all optimization. | Corrected to owner B; prototype-local counters only, each prototype judged by matched C1 batch. |
| Plan §1 / prior handoff R0 state | R0 in flight; source anchors describe pre-fix retention. | Corrected to landed #479 and #480, current early-drop anchors and baseline. Pre-fix magnitude remains unmeasured. |
| Candidate dispositions | #2/#6/#7/#9a have ready implementation slices after attribution. | #2 parked by derivation; #6/#7 one-off test-only estimate with M gate; #9a unknown, no work absent large residual. #8 sole falsifier retained conditionally. |
| Owner questions/schedule | Nine questions, fixed attribution date, L2 and allocator decisions. | Exactly six questions; fixed calendar, active L2 and allocator questions fell away. Each ready prototype gets bounded 02:00–07:00 windows. |
| Historical A1 inventory/review/snapshot evidence | Proposed six-PR instrumentation path. | Superseded by explicit owner B; retained as historical evidence only. No edits outside the two authorized docs. |
| Memory | No relevant registry match. | No memory-derived facts used; no memory update requested or performed. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Revision custody | pending | Controller commit/snapshot of the two docs | Controller | `plan/optimization-round2-b` |
| 2 | #6/#7 estimate | next | Bounded test-only script/ignored test, once on C1 corpora, record net arithmetic and M result | New execution dispatch; corpus mirrors | Plan §2/Q1 |
| 3 | Frequency gate | blocked | Obtain Phase 2 real-round no-change share with counts/period | S6 Phase 2 | R1b ≥20%; Q5 |
| 4 | First prototype | blocked | R1b with complete reuse proof, negative tests/mutants and matched peak acceptance | #3 above | Plan §6 R1b |
| 5 | Second prototype | pending | V9a loader-equivalent contract → V9b, changed-refresh batch | Refresh evidence/priority decision | Q2; E/P/S |
| 6 | Third prototype | blocked | Owner lease/API design, T3a/T3b then family migrations/activation | Tree boundary decision | Q3 |
| 7 | Matched batches | pending | Bind base/candidate, AB/BA ≥3 each, complete required kinds, reserve ≤240-minute batch + ≥60-minute margin | Implementation gates, soak, quiet host, disk ≥50 GiB | Plan §§4–5; Q1/Q4/Q6 |
| 8 | Row-13 stall | blocked | D13a seam under current write lock → D13b off-lock construction | Phase 2 frequency | Plan §6 D13 |

## 5. Invariants and traps — do not do these

- No cargo/build/test/measurement/dotfile reads in this revision; controller commits. Next-step wording is not execution authority.
- Never revive A1–A3 as a prerequisite. A small necessary counter or deterministic changed-refresh seam belongs in its prototype PR, test-only or feature-gated.
- R0/R0b are already in the base; do not claim their pre-fix retention as a new prototype saving.
- R1b dispatch requires real-round no-change ≥20%; synthetic benchmarks alone cannot satisfy that gate. R1a stays deferred.
- #9v acceptance uses changed-refresh peak, never a no-change proxy. Existing memreport refresh is no-change; its small deterministic changed case must be built/preflighted inside the relevant prototype, with the same neutral seam on both sides.
- M is ≥200 MiB or ≥15% on two corpora, medians of admitted runs; steady 15% uses live heap. Print both footprint levels; same-level footprint corroboration is required for steady host-relief acceptance. Peak candidates keep their named lifetime-footprint metric. The historical M2 comparator is 100 MiB/5%, not this gate.
- #6/#7 corpus census is one-off/test-only and supplies estimates, not activation; no double-counted helper slack or shared keys. A fixture zero proves no corpus-wide absence.
- #8 has only the array-loop ≥100 ms/two-corpus reopen falsifier, implemented only if cheap in another PR. #9a residual is not proof of reclaimable pages.
- One measurement at a time; ≥50 GiB disk; completed soak; five-minute quiet admission and per-child rechecks. Idle-daemon stopping needs Q6, no connected clients and immediate recheck. Never silently drop required rows to fit 02:00–07:00.
- D13 allows at most one active build per provider. Explicit requests queue with individual FullOnly publication/summary; automatic waiters join/recheck. D13a keeps current write lock; D13b adds readers/off-lock build. Warm peak gate unchanged; cold first-use allowance is exactly preregistered old-edge census bytes, or UNKNOWN if absent.
- Preserve rollback, generations, output/cache/summary bytes and immutable old-reader snapshots. No drop-active shortcut, temporary-tree borrowed Nodes or unbounded tree repopulation. Oversized AST/CPG files do not grow in feature PRs; extraction is separate and move-only.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting HEAD / main | `433b252c08dec870e009e70bd5af6d7603f94629` |
| Workspace / branch | `/Users/wesleyjinks/code/prism-opt2` / `plan/optimization-round2-b` |
| Plan | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-opt2/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| Evidence root / stopped inventory | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/` / `a1/INVENTORY.md` |
| Landed fixes | R0 #479 `0d3a5feb`; R0b #480 `433b252c` |
| Existing C1 harness / pins | `scripts/memreport/` / `scripts/memreport/workloads/MANIFEST.json` |
| Future measurement windows | `02:00–07:00 America/Denver`; no calendar reservation made |
| Handoff template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: “The revised plan follows owner option B, with no attribution-program prerequisite and with gated prototypes judged by matched C1 batches.” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: current plan/handoff and this session's document/diff inspection. No implementation, test or performance acceptance claimed.

**Questions the owner owes an answer to:**

1. **Corpus mirrors:** recommend exact clean pinned C1 Git mirrors in the evidence root, unchanged workload bytes, fresh path-bound caches.
2. **Refresh priority / changed evidence:** recommend gated R1b first, then #9v with matched E/P/S changed rows and ≥2 admissible corpora; no no-change proxy. D13 separate; retry deferred.
3. **Tree-lease/API boundary:** recommend session-local immutable-source leases, eager public compatibility, zero idle retention and bounded concurrency; design before T3 code.
4. **Host target:** define distinct retained checkouts and simultaneous refreshes; recommend intended S6 fan-out and initially serial heavy refreshes with non-Prism headroom.
5. **R1b frequency:** require Phase 2 ≥20% no-change real-round refreshes, with counts/period; park below threshold even if synthetic M passes.
6. **Idle daemons:** recommend permission to stop only client-free prism-mcp daemons after soak, rechecked immediately; active clients remain untouched.

The calendar, L2 and allocator policy questions fell away under B. They are not pending approvals for this revision.
