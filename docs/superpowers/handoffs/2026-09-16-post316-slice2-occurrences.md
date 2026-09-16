# Handoff — post-316 Slice 2 exact same-line occurrences, second-review freeze

**Written:** 2026-09-16T04:20:14-06:00 · **By:** /root/implement_slice1 · **Provider:** codex
**Workspace:** /private/tmp/prism-post316-slice1 · feat/post316-slice2-occurrences · **Measured state:** `[MEASURED]` HEAD 498fb6f68acaa7b87a5551b840967a9c28b50011 · Tree DIRTY with test/custody delta only · Probe `cargo test --offline same_line_occurrence_ -- --nocapture` · Output `/private/tmp/prism-post316-orchestration/slice2/focused-final.log`
**Predecessor:** /root/implement_slice1 — same agent continued from accepted Slice 1
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker and folded from the root-ratified design decision. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[MEASURED]` /root/implement_slice1 owns implementation; /root owns commits and review routing — **RESOLVED by root dispatch**
**(b) Custody exposure** — `[MEASURED]` production is committed at `498fb6f6`; review hardening tests/docs are uncommitted and hashed — **OPEN until root second-review checkpoint**
**(c) In flight / irreversible** — `[MEASURED]` no command is running; no irreversible action — **RESOLVED 2026-09-16**
**(d) Authorization granted but not exercised** — “Implement replaced oracle + O13–15, all remaining negatives, capture meaningful RED, then freeze complete population for first reviewer.”

## 1. Resume order

1. Controller commits the exact round-1 test/custody delta without staging unrelated files and binds the already-started second review to it.
2. Keep source frozen while running final Rust gates. Frozen-production non-Rust/Tier-A gates are complete and remain valid because production hashes did not change.
3. Reconcile the second and final review; do not extend review beyond the declared cap.

**STOP conditions:** no RD/VarLocation/public-API widening; no synthetic caller edge to later Use77–82; no review beyond cap two without convergence classification; no full multicorpus or rebaseline.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Ratified O01 replacement | done | `[MEASURED]` JS/TS/TSX two-hop path and explicit absent incoming caller edge in focused log |
| O01–O12 | done | `[MEASURED]` includes asserted-member full vector and actual same-name/same-line collision |
| O13/O14 | done | `[MEASURED]` serial/parallel/full/incremental/warm, Step5c/legacy queries and source epochs |
| O15 | done | `[MEASURED]` genuine v94 mismatch then v95 rebuild Hit parity |
| Review round 1 | done | `[INHERITED]` 0 WRONG / 1 nonblocking performance SMELL; finite G1–G4 closed |
| Focused final source | done | `[MEASURED]` 16 passed, 0 failed, 0 ignored |
| Frozen-production gates | done | `[MEASURED/INHERITED]` Rust default4532/MCP4725/widest4748; non-Rust receipt binds binaries |
| Final Rust gates | running | `[UNKNOWN]` required after test-only hardening |
| Independent review round 2 | running | `[UNKNOWN]` final cap round |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Original Slice2 O01 incoming-edge requirement | Later same-key caller Use can retain an independently keyed incoming DFG edge | `[MEASURED]` ratified design decision starts bounded end-to-end proof at exact argument Use; caller RD remains first-wins successor scope |
| Historical namespace/same-line audit assertions | Exact same-line argument occurrence is refused | `[MEASURED]` only the supported exact occurrence rows now expect selection; owner/out-of-span refusals remain |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Second checkpoint | pending | Root commits exact test/custody manifest | controller `.git` authority | source-manifest.md |
| 2 | Review round 2 | running | Replay G1–G4 against immutable test hash | item 1 for final binding | cap 2 |
| 3 | Final Rust gates | running | Default/MCP/widest/examples/clippy/fmt on frozen test source | no source edits | final totals required |

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
| Focused log | `/private/tmp/prism-post316-orchestration/slice2/focused-final.log` SHA-256 `f3c6db8c587dc033d3431ee5081cd19ee7f4453d31016dc4572cff4f329aa205` |
| CPG log | `/private/tmp/prism-post316-orchestration/slice2/cpg-focused-green.log` SHA-256 `e3234675fb51854780f5004f44c5b2d8a5404ec75929123a37b7b66c05c8703c` |
| O15 log | `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94.log` SHA-256 `c07ff72ad55a1ae7415091ae348f8a552c1fc7b0090ba04450ecd2062617809a` |
| G1 base RED | `/private/tmp/prism-post316-orchestration/slice2/base-g1-red.log` SHA-256 `d848103e1cd071851914226f27c3739ba527455f030596e8302aa4c01b99073e` |
| G4 receipt | `/private/tmp/prism-post316-orchestration/slice2/g4/receipt.md` SHA-256 `b8df17d0ebbdfb46c6ee0471f575ddb7dba70ffc3f7b5a3d2ba8e1ab6cd141bb` |
| Non-Rust receipt | `/private/tmp/prism-post316-orchestration/slice2/final-gates/nonrust/nonrust-gates-receipt.md` SHA-256 `1872b67c5fcf76a18a0047209235c681fbdc0c1444509128748f6cac76c06c0e` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: “the bounded exact occurrence repair supplies the genuine later argument-to-callee-body path without inventing caller RD and preserves full/incremental/warm semantics” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: `/private/tmp/prism-post316-orchestration/slice2/focused-green.log`

**Questions the owner owes an answer to:** None before checkpoint/review; final acceptance still requires reviewer and full gates.
