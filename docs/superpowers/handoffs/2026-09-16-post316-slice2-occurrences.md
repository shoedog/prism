# Handoff — post-316 Slice 2 exact same-line occurrences, first-review freeze

**Written:** 2026-09-16T04:20:14-06:00 · **By:** /root/implement_slice1 · **Provider:** codex
**Workspace:** /private/tmp/prism-post316-slice1 · feat/post316-slice2-occurrences · **Measured state:** `[MEASURED]` HEAD d2bbe7074d12fc98280248313d9767d237be33b8 · Tree DIRTY · Probe `cargo test cpg:: -- --nocapture` · Output `/private/tmp/prism-post316-orchestration/slice2/cpg-focused-green.log`
**Predecessor:** /root/implement_slice1 — same agent continued from accepted Slice 1
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker and folded from the root-ratified design decision. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` /root/implement_slice1 owns implementation; /root owns commits and review routing — **RESOLVED by root dispatch**
**(b) Custody exposure** — `[MEASURED]` source/tests/docs are uncommitted; source manifest and external logs are hashed — **OPEN until root checkpoint commit**
**(c) In flight / irreversible** — `[MEASURED]` no command is running; no irreversible action — **RESOLVED 2026-09-16**
**(d) Authorization granted but not exercised** — “Implement replaced oracle + O13–15, all remaining negatives, capture meaningful RED, then freeze complete population for first reviewer.”

## 1. Resume order

1. Controller commits the exact manifest population without staging unrelated files, then sends the immutable commit to the fresh Slice2 reviewer (cap two).
2. Keep source frozen while running final gates; use the established Slice1 runbook and delegate non-Rust/Tier-A gates to verification_setup.
3. Fold only finite reviewer findings; re-freeze/version the manifest after any source or test change.

**STOP conditions:** no RD/VarLocation/public-API widening; no synthetic caller edge to later Use77–82; no review beyond cap two without convergence classification; no full multicorpus or rebaseline.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Ratified O01 replacement | done | `[MEASURED]` JS/TS/TSX two-hop path and explicit absent incoming caller edge in focused log |
| O01–O12 | done | `[MEASURED]` 14-test focused population; exact tuples and negatives |
| O13/O14 | done | `[MEASURED]` serial/parallel/full/incremental/warm and source epochs |
| O15 | done | `[MEASURED]` genuine v94 mismatch then v95 rebuild Hit parity |
| CPG population | done | `[MEASURED]` 253 passed, 0 failed, 0 ignored |
| Full gates | pending | `[UNKNOWN]` not run against frozen commit yet |
| Independent review | pending | `[UNKNOWN]` cap two unused |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Original Slice2 O01 incoming-edge requirement | Later same-key caller Use can retain an independently keyed incoming DFG edge | `[MEASURED]` ratified design decision starts bounded end-to-end proof at exact argument Use; caller RD remains first-wins successor scope |
| Historical namespace/same-line audit assertions | Exact same-line argument occurrence is refused | `[MEASURED]` only the supported exact occurrence rows now expect selection; owner/out-of-span refusals remain |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Checkpoint | pending | Root commits exact manifest | controller `.git` authority | source-manifest.md |
| 2 | Review round 1 | pending | Review frozen commit against O01–O15 | item 1 | cap 2 |
| 3 | Full gates | pending | Run exact README/runbook populations on frozen source | item 1 | final totals required |

## 5. Invariants and traps — do not do these

- Never infer later-edge evidence from `labels.get` with a later VarLocation; equality excludes bytes.
- Never synthesize Def46→Use77 or fan Step4 edges across occurrences.
- Preserve legacy first-wins lookup in fresh and cache reconstruction.
- Multiple eligible Uses refuse before Def fallback; forged key and graph payload must both validate.
- Keep navigation cache at 53; only persisted CPG semantics changed.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | `d2bbe7074d12fc98280248313d9767d237be33b8` / tree `74eef96d0726882ea0a820b6a7b9552a17fcc3f2` |
| Manifest | `docs/eval/post316-slice2/source-manifest.md` |
| Receipt | `docs/eval/post316-slice2/candidate-receipt.md` |
| Focused log | `/private/tmp/prism-post316-orchestration/slice2/focused-green.log` SHA-256 `02ecc0ce81ce7a50d7a74730e11b2f4a17c9ce5e993b9f9bc813074d40d01bbd` |
| CPG log | `/private/tmp/prism-post316-orchestration/slice2/cpg-focused-green.log` SHA-256 `e3234675fb51854780f5004f44c5b2d8a5404ec75929123a37b7b66c05c8703c` |
| O15 log | `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94.log` SHA-256 `c07ff72ad55a1ae7415091ae348f8a552c1fc7b0090ba04450ecd2062617809a` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: “the bounded exact occurrence repair supplies the genuine later argument-to-callee-body path without inventing caller RD and preserves full/incremental/warm semantics” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: `/private/tmp/prism-post316-orchestration/slice2/focused-green.log`

**Questions the owner owes an answer to:** None before checkpoint/review; final acceptance still requires reviewer and full gates.
