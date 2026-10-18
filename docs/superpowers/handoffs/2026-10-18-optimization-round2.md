# Handoff — Prism optimization round 2, I0 first prototype

**Written:** 2026-10-18 · **By:** Codex /root · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-idle-plan` · `plan/idle-unload` · **Measured state:** `[MEASURED]` starting HEAD `f0ce8b6aa0cce571068de9d51067fafdb6250dcf`, starting tree CLEAN. Probes: `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`; outputs: workspace/branch/HEAD above and empty initial status. Delivery contains only the two authorized tracked document edits; controller commits, no Git writes attempted.
**Predecessor:** `[INHERITED]` option-B/owner-answer folds and review history in plan §8; current owner decision makes I0 first, ahead of S0-dependent prototypes. **Status:** I0 design folded; implementation and acceptance pending. S0/T3a continue independently in parallel, not blocked by I0.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0.
**Provenance:** written live using the installed `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md`. `[MEASURED]` identity/static source inspection at this HEAD; I0 anchors read directly, old non-I0 anchors remain `[INHERITED]`. Frequency RESULT.md read; its extraction and expanded owner-supplied call counts were not rerun. Performance receipts are inherited, not measurements by this worker.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — owner authorizes only plan/handoff edits this turn; no subagents. **RESOLVED for docs.** `[UNKNOWN]` other implementation workers; controller must allocate shared session/memreport hooks against S0 and coordinator/connection hooks against D13 before parallel implementation.

**(b) Custody exposure** — `[MEASURED]` clean start at `f0ce8b6a`; two tracked documents now edited. **OPEN for controller commit/snapshot of I0 fold.** No extra single-copy notes artifact created; evidence destination reserved below. Controller owns `.git` writes.

**(c) In flight / irreversible** — `[INHERITED]` S6 soak expected through **Tue 2026-10-20 ~12:08 MDT**, completion not verified. **OPEN:** C0/estimates/measurements await completed soak and quiet-host admission. I0 implementation may start before soak ends under a new controller dispatch; builds and index-loading tests wait for controller scheduling. R1b is PARKED: receipt has 0 soak-window explicit refreshes (<10); auto-refresh visibility remains UNKNOWN. #9v needs rebound P+Py evidence; D13 remains Phase-2-gated. No implementation, build, test or measurement process launched here.

**(d) Authorization granted but not exercised** — owner: “Edit only” the named plan/handoff; “No cargo, builds, index loads or measurements”; “Never read or print ~/.zshrc or other dotfiles”; “The controller commits; you cannot write `.git`.” Owner also says implementation may start before soak ends, measurement waits; this is future dispatch scope, not source-edit authority for this docs-only turn. Existing post-soak permission to stop only immediately rechecked clientless daemons remains settled.

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-idle-plan status --short --branch` and `git -C /Users/wesleyjinks/code/prism-idle-plan rev-parse HEAD` (seconds, read-only). Expect `plan/idle-unload`, `f0ce8b6a` with only these two edits, or controller’s subsequent docs commit. Read plan §1 “I0 — idle index unload” and §8 “I0 fold”.
2. Controller snapshots/commits both docs. Inventory **I0 first** at ≤~600 weighted changed lines including tests/harness, allocate exact files against parallel lanes, then dispatch. One drafting/static-verification pass for this fold; future PRs have two review rounds. No job/window reservation exists.
3. Implement **A: in-process whole-session unload**, both ordinary stdio and shared daemon; 10-minute tool-idle default, 0 disables, activity pinned across readiness/response completion, ≤3 s post-unload wait. If too large, I0a common lifecycle+stdio then I0b daemon integration; no both-mode claim until both land. B process exit/reconnect and C allocator release remain separate conditional designs, not silently included.
4. Continue **S0 and T3a design independently in parallel with I0**. Tree lane retains S0→T3a→T3b→bounded T3c→T3d first acceptance→#10; public API, zero idle trees, bounded leases and p95 gates remain. R1b stays parked; #9v/qualifying #6/#7 follow #3/#10 with existing feasibility/estimate rules. #11/#12 specs follow #10 and phase-location screen; #14 and daemon shared base remain design-only.
5. After **confirmed soak completion**, run C0 daytime on pinned pydantic-ai / NVIDIA-NeMo/Automodel / django, one cold + two hits each, one process at a time under ≥50 GiB/no-competing-load/non-increasing-swap admission. Select largest valid hit heap with cold ≤~25 min, freeze Py pin and 200-request workload/hash. Then I0 matched E/P/ruff/Py batches and the #6/#7 estimate per plan §§2/4/5; I0 measurement is first-prototype priority, C0 is its corpus prerequisite.
6. For I0, sample pre-index/loaded/post-unload +5 s/+30 s footprint/RSS/heap/compression; ≥3 runs/side, footprint M on two corpora and near-baseline gate. Measure exact-hit full reload/first served query on all four, changed-while-unloaded rebuild, concurrent clients and repeated cycles in both modes. Compare same-snapshot bytes/generations; no allocator relief inferred from drop witnesses. Collect daemon key logs as well as stdio stderr.
7. Keep D13a→D13b separate and Phase-2-gated. Current refresh locks stay in I0; later off-lock flights and old-reader leases pin against unload. Default resident refresh rollback/queued FullOnly remains; unloaded readiness follows I0’s scoped warming contract. D13 cold-old-edge allowance remains existing `after_call_edges` minus `after_index` stage delta.

**STOP conditions:** docs-only authorization for this worker; unresolved file ownership/slice size; active soak for builds/index loads/measurements; missing corpus/cache/workload identity; failed same-snapshot bytes; missing physical relief or required corpus/mode rows; admission/window failure; open-class findings at the two-round PR cap. I0 is not blocked on S0, R1b or D13; none is blocked solely on I0. No extra window to rescue an admissible FAIL.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Prior option-B/owner-answer folds | done | `[INHERITED]` plan §8 history; current checked parent is main f0ce8b6a. |
| I0 documentation | done | Plan §§1/4/5/6/8 and this handoff; static-only, controller custody pending. |
| I0 implementation | next | FIRST prototype, no S0 prerequisite; A recommended, both-mode lifecycle/transport/config/test inventory before dispatch. |
| I0 acceptance | blocked | Completed soak, C0 Py pin, implementation/full verification, controller window/admission; physical M and near-baseline footprint/RSS plus reload/byte/concurrency gates. |
| R0 / R0b | done | `[INHERITED]` #479 0d3a5feb / #480 433b252c, already in base; no new saving claimed. |
| A1/A2/A3a/A3b | parked | Owner dropped instrumentation predecessor program; never revive it under I0. |
| S0 / T3a | next | Independent parallel lane; S0 changed-refresh seam and immediate T3a tree lifetime design remain scheduled. |
| R1b | parked | `[INHERITED]` r1b-frequency/RESULT.md: 17 historical, 14 no-change, 0 soak; ~4/month, 0–2 s no-change. Share gate ≥20% over ≥10 complete current-period results remains; low value even if later eligible. |
| #3 / #10 | pending | Tree implementation follows safe lifetime design/S0; #10 after first tree acceptance. Active <1 GiB target remains distinct from I0 idle relief. |
| C0 / #6/#7 estimate | blocked | Post-soak sequential C0, then revised-four estimate under plan §2 admission. Preparation may proceed independently. |
| #9v | blocked | After #3/#10, Py-specific P+Py phase/ceiling screen before V9a; S0 for V9b; ruff spare. |
| Qualifying #6/#7; #11/#12; #14 | pending | Existing M/spec/ordering gates; #14 and #10 daemon overlays design-only. |
| #2 / #8; #9a / R1a | parked | Existing derivations; #9a allocator recoverability UNKNOWN. I0 A does not authorize C allocator work. |
| D13a / D13b | blocked | Separate Phase 2 frequency gate; later integrate flights/reader leases with I0 exclusion. |
| Full tests / mutants / performance | pending | Not executed; implementation must satisfy plan §6 and report exclusions, including controller-only socket/host facilities. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Headers / resume / identifiers | prism-opt2, f6c17927, S0 first. | Current prism-idle-plan / plan/idle-unload / f0ce8b6a; I0 first, independent S0/T3a. |
| Plan §§1/4; ledger | Refresh frequency entirely unknown. | Controller receipt: 0 soak-window explicit calls; R1b PARK, historical value low. Auto-refresh frequency still UNKNOWN. |
| Plan §§1/4/5/6 | Universal resident-query latency/no-warming rules would reject intended unload/reload. | Explicit I0 readiness/changed-snapshot scope; same-snapshot served query and executed refresh-summary parity retained. |
| Plan §§1/4 | Heap M alone could imply I0 host relief. | I0 primary denominator is base idle footprint; M on two corpora, near-pre-index footprint/RSS also required. #9a remains UNKNOWN. |
| Plan §1 / B option | Existing shim transparently respawns after daemon exit. | Source shows closure ends MCP session; B needs separate supervisor/reconnect design, no replay of possibly executed requests. |
| Existing owner-answer gates | I0 might erase tree, capacity, D13, review or admission gates. | Retained; distinguish active versus unloaded checkouts and resident refresh versus unloaded readiness. §8 classifies conflicts WRONG/SMELL and MATERIAL/IMMATERIAL. |
| Memory | No task-relevant memory evidence used. | Registry quick pass found unrelated idle-adapter history; no memory-derived facts or updates. Only two authorized docs edited. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Fold custody | pending | Controller snapshot/commit both docs | Controller | f0ce8b6a parent; no new commit by worker |
| 2 | I0 first prototype | next | Inventory lifecycle/timer/config/logging/tests; dispatch A; split if >600 | New source-edit dispatch, file ownership | Plan §§1/6, idle-unload/ |
| 3 | S0 / T3a | next | Parallel changed-refresh seam / tree design, allocate shared hooks | Separate dispatch; safe tree lifetimes before code | Plan §§3/6 |
| 4 | C0 / estimates | blocked | Pin mirrors/workloads, run C0 then estimate after soak | Completed soak; disk/load/swap | Plan §2 |
| 5 | I0 measured acceptance | blocked | Full both-mode matched batches, physical and reload gates | Implementation/full suites; Py pin; soak/admission/window | Plan §§1/4/5 |
| 6 | R1b / D13 frequency | parked | Controller later reruns transcript extraction; retain low-priority conclusion | ≥10 complete current-period explicit refreshes for R1b; Phase 2 for D13 | r1b-frequency/RESULT.md; plan §4 |
| 7 | Downstream prototypes | pending | Retain #3→#10→#11/#12 specs→#9v→qualifying #6/#7 order and screens | Existing independent evidence/design gates | Plan §§2/4/6 |

## 5. Invariants and traps — do not do these

- No cargo/build/test/index load/measurement/dotfile read in this docs-only fold; controller commits. Future steps are not this worker’s execution authority.
- I0 first does not block S0/T3a design. Do not run measurements during soak, or hide shared-file collisions behind parallel dispatch.
- A drops all session ownership; request/build/refresh/old-reader leases pin it. A write lock alone misses readiness→slot and read→auto-refresh gaps. A next-call-only idle check never frees a silent provider.
- `--index-idle-secs` is provider tool-idle, distinct from clientless `--idle-ttl`. Silent connected clients must unload; eager/detached-owner compatibility is explicitly excluded/rejected as scoped in plan §1.
- Preserve generations across reload: bootstrap resets to zero today. Existing lazy wait budgets and client lifecycle must not be accidentally restarted; changed unloaded repo uses current source and exact cache validation, never old graphs/new text.
- Default WarnOnly resident stale service differs intentionally from unloaded rebuild/warming. Exact-hit controls keep both content and stamps unchanged; changed source/stamps (including touch-only) use fresh-reference controls with matched generation/call history, no broad waiver or golden blessing.
- I0 M is ≥200 MiB or ≥15% **idle footprint** on two corpora, ≥3/side and candidate max < base min; target footprint/RSS ≤ own pre-index baseline +16 MiB. Heap drop and Weak witnesses alone do not prove host relief. Other candidates retain their original live-heap/peak denominators.
- A failed physical gate requires B/C design; B cannot just exit a daemon with connected clients because current shim never reconnects. C needs separate #9a evidence; residual bytes do not identify releasable pages.
- One bounded stderr event per lifecycle/refresh result, no stdout changes; daemon key-log redirection/caps mean shim stderr alone is incomplete. Unobserved automatic refresh is UNKNOWN, not zero.
- Full suite, same-environment base-red controls, negative/edge cases and feature-aware mutants are future gates; exclusions must be explicit. No implementation acceptance from static review.
- Preserve other plan gates: R1b ≥20%/≥10, #9v P+Py screen, #3 p95 ≤max(10%,10 ms), D13 flight/summary/cold-edge controls, ≥50 GiB and quiet sequential matched batches, two-round PR cap. One sanity pair only for expressly unaffected kinds; I0 has no such exemption yet.
- Final host claim distinguishes active and unloaded checkouts, records simultaneous reload peaks/headroom/swap, and retains <1 GiB active cache-hit index target. I0 does not prove 12–16 simultaneously resident indexes.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting HEAD / base main | `f0ce8b6aa0cce571068de9d51067fafdb6250dcf` |
| Workspace / branch | `/Users/wesleyjinks/code/prism-idle-plan` / `plan/idle-unload` |
| Plan | `/Users/wesleyjinks/code/prism-idle-plan/docs/superpowers/plans/2026-10-18-optimization-round2.md` |
| Handoff | `/Users/wesleyjinks/code/prism-idle-plan/docs/superpowers/handoffs/2026-10-18-optimization-round2.md` |
| I0 notes/readout destination | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/idle-unload/` |
| Frequency receipt | `/Users/wesleyjinks/prism-evidence/mcp-memory/optimization-round2/r1b-frequency/RESULT.md` |
| C1 harness / pins | `scripts/memreport/` / `scripts/memreport/workloads/MANIFEST.json`; Python pin/workload pending C0 |
| Expected soak end | `Tue 2026-10-20 ~12:08 MDT`; verify completion, never infer it from date |
| Future acceptance windows | `02:00–07:00 America/Denver`; no reservation made |
| Handoff template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: “I0 can be the first prototype without silently weakening existing acceptance or breaking the current transport contract.” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: plan §8 “I0 fold” records ordering, readiness/latency and shim-exit WRONG/MATERIAL conflicts plus allocator/size/visibility SMELLs and bounded resolutions. No independent review, implementation or measured acceptance claimed.

**Questions the owner owes an answer to:** None for this fold; recommendations are concrete in plan §1. Remaining gates are controller custody/dispatch, pre-code line/file inventory, full future tests, C0/soak/admission, physical reclaim/reload measurements and any separately justified B/C follow-up. No source implementation authorized by this docs-only delivery.
