> **R2 supersession:** Historical R1/planner record; current dispatch is committed `bd4c30ff` plus `repair-r2/R2-src.patch`, docs `a0e8504e` plus R2 docs patch. Use `HANDOFF-repair-r2.md` and `MEASUREMENTS-r2.md` for the targeted fold and current verification; R2 measurements and gates are complete on the documented successful-pair set; STOP is none.

# Handoff — js-param-defs PR-A R1 repair

**Written:** 2026-10-05T23:39:34.266875+00:00 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-pd-plan · plan/js-param-defs · **Measured state:** `[MEASURED]` HEAD 6b0be4ff46ce0bfaa398ca6d008f6cb623c6a956 · Tree DIRTY · Probe git status --short; pinned blob comparison · Output repair-r1/binding.json and artifact-manifest.json
**Predecessor:** planner; sol and Opus spec R1 reviews.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner dispatched the brief; root is sole worker; no subagents dispatched — **RESOLVED** by repair-r1-brief.md.
**(b) Custody exposure** — `[MEASURED]` no git writes; controller must commit. Final patches relative to pinned prototype/docs, source/docs/binaries/receipts snapshot and external hash manifest saved under repair-r1 — **RESOLVED** by r1-final-snapshot.tgz, R1-src.patch/R1-docs.patch and artifact-manifest.json. Dirty tree is intentional, not an unbacked handoff.
**(c) In flight / irreversible** — `[MEASURED]` all known task command sessions completed; final capture and adjudication exited0. Process census unavailable after sandbox denied ps; no further census attempted. Seven literal lane build directories removed, frozen bins/cache/evidence preserved — **RESOLVED** by cleanup.json and bin/binding.json. No known task process remains.
**(d) Authorization granted but not exercised** — "Read and follow /Users/wesleyjinks/prism-evidence/js-param-defs/repair-r1-brief.md exactly." Repair work completed; controller F, independent review and commits are separate controller work, not silently adopted here.

## 1. Resume order

1. Run `cat /Users/wesleyjinks/prism-evidence/js-param-defs/repair-r1/results-summary.json` and read MEASUREMENTS-prA.md (seconds). Verify the patch/binary hashes against artifact-manifest.json.
2. Controller: bind an implementation checkout to `1b2dfdc9`; apply R1-src.patch. Apply docs patch to `6b0be4ff` or carry the repaired packet into that checkout. Both patches have pinned-archive `git apply --check` receipts; source patch contains exactly five R1 files.
3. Controller: invoke CONTROLLER-pd.sh with the separately supplied F root, a new private evidence directory and the six documented arguments. The frozen executable inputs survive cleanup. Worker/reviewer sessions must not access F.
4. Controller: dispatch the second spec review within the existing capped review sequence. Declare remaining cap before dispatch; do not restart the artifact or merge automatically. Commit only under controller authority after its gates.

**STOP conditions:** LOST checker-correct base row; non-JS non-identity; changed call-site output; new checker-WRONG outside explicitly disclosed E7; undecided added binding. E7/E8 parity remains WRONG, not downgraded. The two state-changing retry cap was classified as closed and converging; finite TS declaration fixes and redundant-mutation re-pin extensions were disclosed in the probe log.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Custody binding | done | `[MEASURED]` 722 initial src/tests/mutants files matched committed prototype; binding.json |
| Review authority | done | `[INHERITED]` repair brief; sol W1–W6/S1 and Opus F1–F8 read in full |
| Source repair | done | `[MEASURED]` widened competing-binder refusal, outgoing E5 suppression, rest comma refusal; R1-src.patch |
| RED/GREEN | done | `[MEASURED]` original8 prototype failures red.log; TS runtime additional RED ts-runtime-proto-red.log; final27/27 green-final2.log |
| Byte oracle and census | done | `[MEASURED]` reviewer-controls-final7.log and reviewer-controls/summary.json; exact UTF-8 bytes/owner/checker declaration spans, BOM, stale text rejection, hidden-file admission |
| Full suite and mutations | done | `[MEASURED]` nextest-final.log5164 passed1 skipped; doctests-hook.log2 passed; scoped advisory29/29 KILLED; authoritative lane28/28 KILLED; no inadmissible mutants |
| Format, clippy, Tier-A | done | `[MEASURED]` fmt-final2.log clean; clippy-parity.json235/235 no added/removed; matrix178; TS/JS comparison quickVALID and default Rust/TS/Node quickVALID |
| Public rows | done | `[MEASURED]` public-final/adjudication/summary.json: X36 CORRECT, Xi36 CORRECT, T1052 CORRECT+1 E7 WRONG; zero LOST/UNDECIDED; four E1 WRONG relabels. All12 wire projections agree |
| SecBench rows | done | `[MEASURED]` sb-rows-final/adjudication/summary.json:574/583 admitted,826 CORRECT+1 E7 WRONG, zero LOST/UNDECIDED; three E1 WRONG relabels;9 explicit failed pairs excluded both sides |
| Guard cost | done | `[MEASURED]` prototype→R1 loses four correct T gains to D12, zero on X/Xi/SB. This is not LOST correct base rows; D11 adds zero measured row cost |
| Flow parity controls | done | `[MEASURED]` E7 plain base controls checker-WRONG identical; E8 JS/TS/TSX after-return plain controls identical; js-data-E6-control.json and postcss-E6-control.json preserve two prior-write Exact hits |
| SecBench conversions/BFS | done | `[MEASURED]` secbench-summary.json:113→120 traced,113 preserved,7 new payload-specific traces;9 target outcomes7 traced/1 reached-only/1 partial; clean-css/natural error both sides; wind supplemental partial both sides |
| Non-JS identity | done | `[MEASURED]` controls-summary.json: Python/Go/Rust byte records, sorted wire output and actual call sites identical |
| Controller and cleanup | done | `[MEASURED]` current bare-arrow fixture positive COMPLETE; mismatched producer rejected at byte_projection; syntax-final.json; cleanup.json seven absent paths |
| Hook verification | done | `[MEASURED]` root VERIFICATION.md;722 source/test/mutant hashes match final suite; actual old aggregation3 RED/new shell3 GREEN/active byte3 GREEN; doctests2/2; hook target removed |
| Independent second spec review / F / commits | pending | `[INHERITED]` controller-only boundaries from brief and IMPLEMENTOR; no adoption or merge claim |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| SPEC/IMPLEMENTOR | Narrow D11, incoming-hole-only E5 isolation, uncommitted prototype, clippy-D warning gate, incomplete re-pins | `[MEASURED]` folded R1 scope, committed dispatch origin, four intended re-pins/nested tests, clippy same-base parity, F5/F6/F8 disclosures |
| MEASUREMENTS / packet PROBE-LOG / planner handoff | Old line oracle all-CORRECT and zero-WRONG | `[MEASURED]` historical records explicitly superseded; current byte tables show E7 and inherited E1/E8, D12 cost and failed-pair exclusions |
| Interim repair handoff | Captures pending, custody unresolved | `[MEASURED]` this final handoff and snapshot/manifest replace interim state |
| Memory | None relevant found | No memory update authorized or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Private F measurement | pending | Controller executes the revised six-argument diff command with its private root/new evidence directory | Worker has no F authority | CONTROLLER-pd.sh; frozen bin hashes |
| 2 | Independent second review / adoption | pending | Controller dispatches next capped spec review of this existing artifact; commit only after its acceptance | Independent review not performed here | W1–W6/F1–F8; R1 patches |
| 3 | E5 / E7 / E8 / PR-B / PR-C follow-ups | parked | Use disclosed bounded mechanisms when those lanes are authorized | Outside this repair scope | SPEC inherited-defect disclosures |

## 5. Invariants and traps — do not do these

- No git writes, network, corpus execution, or frontend-portal reads in this repair lane.
- Use /Users/wesleyjinks/prism-evidence/js-param-defs/cache for Prism; frozen final bins are authoritative. All build targets were deleted.
- Host Python3.9 tarfile lacks filter; py_compile ambient cache is blocked; syntax verification used compile(text) without writes.
- Old line-level CORRECT is refuted. Binding CORRECT is not independent CFG/flow proof; Exact retains D13 semantics.
- Failed producers yield no behavioral evidence. Nine SecBench failures are excluded from both base/head; React Native's successful head sites do not rescue failed byte capture.
- Do not compare incomplete prototype aggregates against complete head. Cost requires successful triples; final574 all had prototype captures.
- Go wire ordering alone cannot establish identity: the initially interrupted producer was inadmissible, then actually recaptured and sorted rows compared.
- PD12 lastness-only became equivalent under comma refusal: combined guard re-pin plus PD25 separate comma mutation, both killed. Keep the P2-M11/PD11 population coupling disclosed.
- One reserved future classifier test is ignored. All-lanes mutation149 and human-triggered all-corpus Tier-A were not run; complete independent corpus CFG reachability and languages beyond the three measured controls are unverified.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Prototype | `1b2dfdc93a37dbd87c6359c34e7723912399b682` |
| Base | `c8de720b36c24ae8a7ab274ec10c994f258eb336` |
| Docs base | `6b0be4ff46ce0bfaa398ca6d008f6cb623c6a956` |
| Evidence | `/Users/wesleyjinks/prism-evidence/js-param-defs/repair-r1` |
| Final production binary SHA256 | `4a6173413cfd226e07771d9fcd80b9e3378d559decb4f310a4e41a439be5e0a3` |
| Final byte dumper SHA256 | `961df63d41db783b6883a339c59cb48b8e6233940d221779847d1e10708b6b18` |
| Source commit suggestion | `fix(js-param-defs): refuse competing bindings and E5 source paths` |
| Docs commit suggestion | `docs(js-param-defs): fold R1 byte-binding gates and parity disclosures` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "R1 satisfies the bounded repair brief with disclosed inherited E7/E8 parity, no LOST correct base rows and measured non-JS identity" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: repair-r1/PROBE-LOG.md, results-summary.json and final source-bound gate receipts.

**Questions the owner owes an answer to:** None. Independent review/F/adoption remain controller work.
