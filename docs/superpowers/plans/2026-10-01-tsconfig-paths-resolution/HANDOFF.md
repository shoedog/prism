> **Current adopted fix `b28f6e72`: full requested verification PASS.** Production behavior unchanged; no Git writes or F access. Complete totals, gates, driver rebindings and limits: [VERIFY-FABLE-RESULTS.md](VERIFY-FABLE-RESULTS.md). Earlier r4 and lint-stop receipts are historical.



> **Historical r4:** owner-authorized accepted-risk repair and its then-requested local verification. See [REPAIR-R4-RESULTS.md](REPAIR-R4-RESULTS.md), [REPAIR-R4-HANDOFF.md](REPAIR-R4-HANDOFF.md) and [REPAIR-R4-FILES.md](REPAIR-R4-FILES.md). Earlier receipts do not certify the adopted performance fix.

> Historical r3 was **PARKED / NOT SHIPPABLE**. [REPAIR-R3-HANDOFF.md](REPAIR-R3-HANDOFF.md) supersedes the operational state below. The r2d facts and controller commands are historical; their binaries and wrapper defaults do not certify r3. Do not execute the historical acceptance flow for the parked patch.

# Historical handoff — Lane P P1 r2d local repair complete; controller acceptance pending

**Written:** 2026-10-02 · **By:** /root repairer · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-paths-impl · **Measured state:** [MEASURED] HEAD 6965da75eb85f32d3ae0dda9e96704ad432375e6 · Tree DIRTY · Probe git rev-parse/status, frozen input/binary hashes and completed verification · Output target/repair-r2d/boundary-final
**Predecessor:** owner-supplied squashed r2c a66b877f; controller-created custody WIP6965da75 during this turn.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by repairer; [MEASURED] claims were run this turn; [INHERITED] claims came from the owner.

## 0. Gating facts — settle these before starting anything below
**(a) Lane ownership** — [INHERITED] assigned repairer; no delegates. RESOLVED for local work.
**(b) Custody exposure** — [MEASURED] controller WIP exists, final delta and receipts locally archived/byte-checked. OPEN for controller commit/external backup.
**(c) In flight / irreversible** — [MEASURED] all own verification jobs and18 serial measurement children ended. RESOLVED.
**(d) Authorization granted but not exercised** — [INHERITED] “No git writes.” “Never open F.” “Cache stays at105/61.” Controller runs F before acceptance.

## 1. Resume order
1. Compare head/prism-r2d-boundary SHA256 ab4fdc0091c53696dfe38239001c3d689f592fcd3546162030b5ad958fe8222c against boundary-final/custody-final.json (seconds).
2. Controller runs probes/CONTROLLER-paths.sh with CORPUS_F_ROOT, PRIVATE_EVIDENCE_ROOT and TS_JS; default head is corrected. Only F aggregates may return to this lane.
3. Obtain independent review/required quiet-host attestation, then controller handles Git custody using REPAIR-R2D-FILES.md.
**STOP conditions:** repairer F access, Git writes, installs/network, cache/baseline/threshold changes or open-class findings. Original three-round cap reached; disclosed closed ordering, normalization, lexical, coverage and boundary extensions folded the existing artifact. No restart.

## 2. State ledger
| Item | State | Evidence |
|---|---|---|
| First-pass repair; retained rules | done | [MEASURED] source citations; preserved-rules.json |
| Suites | done | [MEASURED]4897/5090/5113,0fail/1existingignore each |
| Mutants | done | [MEASURED]68kernel/57integration behavior kills |
| Public and controls | done | [MEASURED]3121/0/0;38certified+1parked recovered;5 retained cost |
| Cache and S1b | done | [MEASURED]105/61;411 controls/1234 identical artifacts |
| Matrix and quick | done with limit | [MEASURED]170matrix; shared4/6 oracle floor;28 identical SUT outputs |
| Serial performance | done with limit | [MEASURED]18 children pass1.20; host quiescence UNKNOWN |
| F and independent acceptance | not-started | [INHERITED] controller owns them |

## 3. Corrections to standing documents and memory
| Location | Stale assertion | Correction |
|---|---|---|
| settled/lexical earlier completion | eligible final candidate | [MEASURED] REFUTED by native whitespace/root/filesystem witnesses; corrected and rebound in boundary-final |
| SPEC/IMPLEMENTOR/REVIEWER | prior operational authority | [MEASURED] current r2d section and current reports supersede historical sections |
| Earlier HEAD claims | current=a66b877f | [MEASURED] original comparison base=a66b877f; controller custody HEAD=6965da75eb85f32d3ae0dda9e96704ad432375e6 |
| Memory | None used for lane facts | No memory write authorized |

## 4. Open work
| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
|1|F|pending|controller private wrapper run|private controller access|corrected binary SHA above|
|2|Independent acceptance|pending|review bound final delta; attest host if required|controller workflow|MEASUREMENTS.md|
|3|Git/external custody|pending|controller commits/backs up approved final delta|owner no-repairer-Git instruction|REPAIR-R2D-FILES.md|

## 5. Invariants and traps — do not do these
- Never open F or write Git as repairer; private measurement and Git custody belong to controller.
- Keep cuts2–6, r2c classifier, cache105/61, threshold1.20 and parked S6/OQ2.
- Compiler/setup/zero-test failures are inadmissible; kernel externs must be exact and integration targets isolated.
- A single paths omission is redundantly protected; paired invariant mutation is required.
- Aliased parents remain opaque after child deletion; remove the parent before asserting Exact.
- Default uv-cache class was sandbox-denied; installed Python ran the same harness without installation/network.
- Earlier receipts certify earlier binaries only. Local archives do not claim an external backup.

## 6. Identifiers
| Item | Verbatim |
|---|---|
| Current HEAD | 6965da75eb85f32d3ae0dda9e96704ad432375e6 |
| Original base | a66b877f49ba858c27b749b0a36bfccf4bc7da7d |
| Binary | target/repair-r2d/head/prism-r2d-boundary |
| Binary SHA256 | ab4fdc0091c53696dfe38239001c3d689f592fcd3546162030b5ad958fe8222c |
| Final evidence | target/repair-r2d/boundary-final |
| Cache |105/61|

## 7. Refutation verdict and owner questions
**§2c verdict:** REFUTED — corrected in place · claim: “JS Exact admission requires readable native first-pass absence” · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: hypothesis-probe-result.log; boundary-final/native/cache/mutation witnesses and final source binding.
**Questions the owner owes an answer to:** None beyond the already assigned controller acceptance/custody work; S6/OQ2 remains explicitly parked.
