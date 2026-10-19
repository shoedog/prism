# Handoff — Prism optimization round 2, I0 first prototype

**Written:** 2026-10-18 · **By:** Codex /root · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-idle-plan` · `plan/idle-unload` · **Measured state:** `[MEASURED]` starting HEAD `8d24912e6c7d2a095fd68fd8b720273a979956b7`, starting tree CLEAN. Probes: `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`; outputs: workspace/branch/HEAD above and empty initial status. Only two authorized tracked document edits; controller commits, no Git writes attempted.
**Predecessor:** `[INHERITED]` PR #486 I0 plan at `8d24912e`; sol and Opus round-1 reviews both APPROVE WITH FIXES, read in full. **Status:** all 11 controller dispositions folded; implementation and acceptance pending. I0 first; S0/T3a independent.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0.
**Provenance:** written live using `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md`. `[MEASURED]` identity/static document checks and cited I0 source reads under `/Users/wesleyjinks/code/prism-t3a`; equivalence to `f0ce8b6a` is owner-supplied. `[INHERITED]` performance/frequency/adoption and 2,303.5-line transport-tests receipts from plan/reviews, not rerun. Plan §1 “Round-1 review fold” maps every disposition; no implementation evidence claimed.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorizes only plan/handoff edits this turn; no subagents. **RESOLVED for docs.** `[UNKNOWN]` other implementation workers; controller must allocate shared session/memreport hooks against S0 and coordinator/connection hooks against D13 before parallel implementation.

**(b) Custody exposure** — `[MEASURED]` clean start at `8d24912e`; two tracked documents edited. **OPEN for controller commit/snapshot of round-1 fold.** No extra single-copy artifact created; controller owns `.git` writes.

**(c) In flight / irreversible** — `[INHERITED]` S6 soak expected through **Tue 2026-10-20 ~12:08 MDT**, completion not verified. **OPEN:** C0/estimates/measurements await completed soak and quiet-host admission. I0 implementation may start before soak ends under a new controller dispatch; builds and index-loading tests wait for controller scheduling. R1b is PARKED: receipt has 0 soak-window explicit refreshes (<10); auto-refresh visibility remains UNKNOWN. #9v needs rebound P+Py evidence; D13 remains Phase-2-gated. No implementation, build, test or measurement process launched here.

**(d) Authorization granted but not exercised** — owner: “Edit only” the named plan/handoff; “No cargo, builds, index loads or measurements”; “Never read or print ~/.zshrc or other dotfiles”; “The controller commits; you cannot write `.git`.” Owner also says implementation may start before soak ends, measurement waits; this is future dispatch scope, not source-edit authority for this docs-only turn. Existing post-soak permission to stop only immediately rechecked clientless daemons remains settled.

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-idle-plan status --short --branch` and `git -C /Users/wesleyjinks/code/prism-idle-plan rev-parse HEAD` (seconds, read-only). Expect `plan/idle-unload`, `8d24912e` with only these two edits, or controller’s subsequent docs commit. Read plan §1 I0 design and “Round-1 review fold”, then §4 I0 acceptance.
2. Controller snapshots/commits both docs. Inventory **I0a stdio lifecycle**, **I0b daemon integration**, **I0c shim/operator surface**, plus separate small read-then-stamp dependency PR; allocate exact files/tests/harness/A+C support against parallel lanes. No per-slice LOC budget; file-size rules remain, oversized `transport_tests.rs` cannot grow. Two review rounds per implementation PR; design round 1 folded, round 2 remains available. No new review dispatch/window reservation.
3. Implement A with preregistered A+C allocator-release arm: provisional 10-minute default, 0 disables; autonomous stdio completion starts clock at finished_at; daemon timeout is first-spawner policy outside key. RAII serving lease acquired with Ready and released at owned response, before write; try_write unload, off-lock/off-loop disposal. Reload uses configured wait (20 s default), ≤3 s ready target; clients must poll/pre-warm on misses. Bridge pre-warm design in progress. B design only if both arms miss physical bar.
4. Continue **S0 and T3a design independently in parallel with I0**. Tree lane retains S0→T3a→T3b→bounded T3c→T3d first acceptance→#10; public API, zero idle trees, bounded leases and p95 gates remain. R1b stays parked; #9v/qualifying #6/#7 follow #3/#10 with existing feasibility/estimate rules. #11/#12 specs follow #10 and phase-location screen; #14 and daemon shared base remain design-only.
5. After **confirmed soak completion**, run C0 daytime on pinned pydantic-ai / NVIDIA-NeMo/Automodel / django, one cold + two hits each, one process at a time under ≥50 GiB/no-competing-load/non-increasing-swap admission. Select largest valid hit heap with cold ≤~25 min, freeze Py pin and 200-request workload/hash. Then I0 matched E/P/ruff/Py batches and the #6/#7 estimate per plan §§2/4/5; I0 measurement is first-prototype priority, C0 is its corpus prerequisite.
6. Run earliest admitted post-soak **vertical-slice A/A+C check before full matrix**. Genuine pre-index/loaded/+5/+30 s footprint/RSS/heap/compression; +30 s residual inventory (pool stacks/query cache/identity/allocator). ≥3/side, footprint M on two corpora and idle ≤128 MiB per checkout (15% reported) on all four/both modes; +16 MiB pre-index target report-only. Same-batch arms, no-op control, live heap ±1 MiB. Preregister per-corpus exact-hit/miss first-call served/warming/failure distributions; measure ready/served latency, cycles and concurrency. Collect both log sinks; no relief inferred from drop witnesses.
7. Keep D13a→D13b separate and Phase-2-gated. Current refresh locks stay in I0; later off-lock flights and old-reader leases pin against unload. Default resident refresh rollback/queued FullOnly remains; unloaded readiness follows I0’s scoped warming contract. D13 cold-old-edge allowance remains existing `after_call_edges` minus `after_index` stage delta.

**STOP conditions:** docs-only scope; unresolved file ownership or concern split; active soak for builds/index loads/measurements; missing freshness dependency, corpus/cache/workload identity, required rows, physical relief or byte parity; admission/window failure; open-class findings at two-round PR cap. I0 is independent of S0/R1b/D13; source-bound freshness fix is required. No extra window to rescue an admissible FAIL.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Prior option-B/owner-answer folds | done | `[INHERITED]` plan §8; source base f0ce8b6a, reviewed I0 docs parent 8d24912e (PR #486). |
| I0 documentation | done | All 11 controller dispositions in plan §1 fold table, I0 rows and §4 exception/acceptance; static-only, controller custody pending. |
| I0 implementation | next | FIRST prototype: I0a/b/c by concern and separate freshness dependency; A/A+C support allocated before dispatch. |
| I0 acceptance | blocked | Soak/C0/full verification/controller admission; M + goal-derived idle gate, both arms and reload/byte/concurrency tests; no measurements this turn. |
| R0 / R0b | done | `[INHERITED]` #479 0d3a5feb / #480 433b252c, already in base; no new saving claimed. |
| A1/A2/A3a/A3b | parked | Owner dropped instrumentation predecessor program; never revive it under I0. |
| S0 / T3a | next | Independent parallel lane; S0 changed-refresh seam and immediate T3a tree lifetime design remain scheduled. |
| R1b | parked | `[INHERITED]` r1b-frequency/RESULT.md: 17 historical, 14 no-change, 0 soak; ~4/month, 0–2 s no-change. Share gate ≥20% over ≥10 complete current-period results remains; low value even if later eligible. |
| #3 / #10 | pending | Tree implementation follows safe lifetime design/S0; #10 after first tree acceptance. Active <1 GiB target remains distinct from I0 idle relief. |
| C0 / #6/#7 estimate | blocked | Post-soak sequential C0, then revised-four estimate under plan §2 admission. Preparation may proceed independently. |
| #9v | blocked | After #3/#10, Py-specific P+Py phase/ceiling screen before V9a; S0 for V9b; ruff spare. |
| Qualifying #6/#7; #11/#12; #14 | pending | Existing M/spec/ordering gates; #14 and #10 daemon overlays design-only. |
| #2 / #8; #9a / R1a | parked | Existing derivations; allocator benefit UNKNOWN. Only bounded I0 A+C arm admitted, no broader #9a project. |
| D13a / D13b | blocked | Separate Phase 2 frequency gate; later integrate flights/reader leases with I0 exclusion. |
| Full tests / mutants / performance | pending | Not executed; implementation must satisfy plan §6 and report exclusions, including controller-only socket/host facilities. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Headers / resume / identifiers | Earlier fold started at f0ce8b6a. | Current fold starts at 8d24912e; original plan header/§8 retain historical provenance; current instructions are §1 round-1 fold and this handoff. |
| Plan §§1/4; ledger | Refresh frequency entirely unknown. | Controller receipt: 0 soak-window explicit calls; R1b PARK, historical value low. Auto-refresh frequency still UNKNOWN. |
| Plan §§1/4/5/6 | Universal resident-query latency/no-warming rules would reject intended unload/reload. | Explicit I0 readiness/changed-snapshot scope; same-snapshot served query and executed refresh-summary parity retained. |
| Plan §§1/4 | +16 MiB was a hard gate; C deferred. | Corrected: M plus idle ≤128 MiB per checkout (15% reported; round-2 Opus N2), +16 MiB report-only; A/A+C same batch, B only if both miss. §1 explicitly supersedes historical §8 I0 decisions and generic #9a deferral for this arm. |
| Plan §1 / B option | Existing shim transparently respawns after daemon exit. | Source shows closure ends MCP session; B needs separate supervisor/reconnect design, no replay of possibly executed requests. |
| Existing owner-answer gates | Older §6 blanket 600-line slice budget could override new I0 split. | §1 explicitly overrides it for I0; §6 I0 row corrected to I0a/b/c, four mutants and physical gate. Other slices/§8 historical text untouched; no unrelated reopening. |
| Memory | No task-relevant memory evidence used. | Registry quick pass found unrelated idle-adapter history; no memory-derived facts or updates. Only two authorized docs edited. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Fold custody | pending | Controller snapshot/commit both docs | Controller | 8d24912e parent; no worker commit |
| 2 | I0 first prototype | next | Inventory/allocate I0a/b/c and separate freshness dependency; include A+C arm | New source-edit dispatch, file ownership | Plan §1 round-1 fold / §6 I0 row |
| 3 | S0 / T3a | next | Parallel changed-refresh seam / tree design, allocate shared hooks | Separate dispatch; safe tree lifetimes before code | Plan §§3/6 |
| 4 | C0 / estimates | blocked | Pin mirrors/workloads, run C0 then estimate after soak | Completed soak; disk/load/swap | Plan §2 |
| 5 | I0 measured acceptance | blocked | Early vertical slice, then full both-mode A/A+C batch and outcome distributions | Implementation/full suites; Py pin; soak/admission/window | Plan §§1/4/5 |
| 6 | R1b / D13 frequency | parked | Controller later reruns transcript extraction; retain low-priority conclusion | ≥10 complete current-period explicit refreshes for R1b; Phase 2 for D13 | r1b-frequency/RESULT.md; plan §4 |
| 7 | Downstream prototypes | pending | Retain #3→#10→#11/#12 specs→#9v→qualifying #6/#7 order and screens | Existing independent evidence/design gates | Plan §§2/4/6 |

## 5. Invariants and traps — do not do these

- No cargo/build/test/index load/measurement/dotfile read in this docs-only fold; controller commits. Future steps are not this worker’s execution authority.
- I0 first does not block S0/T3a design. Do not run measurements during soak, or hide shared-file collisions behind parallel dispatch.
- A drops whole session; activity/RAII pins readiness through response construction, not socket write. Daemon Ready+lease atomic; try_write and off-loop disposal. Stdio consumes BuildDone without input; finished results cannot remain busy forever. EOF/shutdown disposes late results.
- `--index-idle-secs` differs from clientless `--idle-ttl`; first-spawner timeout stays outside key, effective STATUS/READY and one mismatch line. Eager/owner ignore env with note, reject explicit nonzero flag; no-cache defaults off. Default gap-data tuning remains unmeasured. I0a ships **off by default**; measurement runs pass `--index-idle-secs 600`.
- Retain G on exact fingerprint, increment changed fingerprint; discard sticky divergence. Unloaded retains last_published so old unheard failure cannot resurface. Preserve per-attempt wait budgets/client lifecycle; configured 20 s default wait, ≤3 s exact-hit target. Unloaded refresh builds twice: recorded, not optimized.
- Bind freshness to captured source in separate prerequisite PR; read A→edit B→stamp B must not be falsely clean. Barriers before/after stamps assert metadata and graph/source consistency. Changed stamps/content or prior divergence use fresh-reference bytes with matched generation/history; resident WarnOnly/rollback contracts remain.
- I0 M is ≥200 MiB or ≥15% idle footprint on two corpora, ≥3/side and candidate max < base min; goal-derived idle bar applies per corpus/mode. Report +16 MiB target separately. Pool/query-cache residual inventory stays distinct from measured attribution; no host relief from heap/drop alone.
- Measure A and A+C in same batch with no-op control/unchanged live heap; only both missing physical bar triggers B design. Current shim cannot reconnect; B needs session-preserving supervision. Broader #9a remains UNKNOWN.
- One bounded stderr event per lifecycle/refresh result, no MCP tool output changes; STATUS/READY expose config. Daemon key-log caps mean shim stderr alone is incomplete. Unobserved auto-refresh is UNKNOWN, not zero.
- Full suite, same-environment base-red controls, negative/edge cases and feature-aware mutants are future gates; exclusions must be explicit. No implementation acceptance from static review.
- Preserve other plan gates: R1b ≥20%/≥10, #9v P+Py screen, #3 p95 ≤max(10%,10 ms), D13 flight/summary/cold-edge controls, ≥50 GiB and quiet sequential matched batches, two-round PR cap. One sanity pair only for expressly unaffected kinds; I0 has no such exemption yet.
- Final host claim distinguishes active and unloaded checkouts, records simultaneous reload peaks/headroom/swap, and retains <1 GiB active cache-hit index target. I0 does not prove 12–16 simultaneously resident indexes.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting HEAD / source base | `8d24912e6c7d2a095fd68fd8b720273a979956b7` / `f0ce8b6aa0cce571068de9d51067fafdb6250dcf` |
| Workspace / branch | `/Users/wesleyjinks/code/prism-idle-plan` / `plan/idle-unload` |
| Plan | `/Users/wesleyjinks/code/prism-idle-plan/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-idle-plan/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| I0 notes/readout + reviews | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/idle-unload/`; `review-r1-sol.md`, `review-r1-opus.md` |
| Frequency receipt | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/r1b-frequency/RESULT.md` |
| C1 harness / pins | `scripts/memreport/` / `scripts/memreport/workloads/MANIFEST.json`; Python pin/workload pending C0 |
| Expected soak end | `Tue 2026-10-20 ~12:08 MDT`; verify completion, never infer it from date |
| Future acceptance windows | `02:00–07:00 America/Denver`; no reservation made |
| Handoff template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: “I0 can be the first prototype without silently weakening acceptance or breaking transport.” · pass: SELF-PASS (NOT INDEPENDENT) for this fold; inherited independent round-1 reviews APPROVE WITH FIXES · evidence tier: STATIC-ONLY · record: plan §1 “Round-1 review fold” maps W1–W6/S1–S10 and sol S1–S3 to all 11 controller dispositions. No new independent review, implementation or measured acceptance claimed.

**Questions the owner owes an answer to:** None; all 11 controller dispositions folded. Remaining work: custody/dispatch, freshness prerequisite, concern/file inventory, future full suites/mutants and post-soak physical/reload evidence. B remains conditional on both physical arms missing; no source implementation authorized this turn.
