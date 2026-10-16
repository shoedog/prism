# Handoff — DFG compaction PR B ordered views

**Written:** 2026-10-16 · **By:** Codex · **Provider:** codex
**Workspace:** impl-dfgB · perf/dfg-views · **Measured state:** `[MEASURED]` HEAD 5a63c7e830332a49e7c0b5ae6e27f8ab7699bebf · Tree DIRTY · Probe git status/diff and task gates · Output ../dfgB-scratch/logs/
**Predecessor:** owner dispatch; PR A merged as #407.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. Conflicts remain open below.
**Provenance:** written live by worker. Measured claims were rerun here; inherited claims name their source.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner assigns Task 2 only; controller owns commits and two review rounds — RESOLVED.
**(b) Custody exposure** — `[MEASURED]` uncommitted implementation and new files; source/evidence snapshots and hashes in ../dfgB-scratch — RESOLVED for handoff, controller must preserve/commit.
**(c) In flight / irreversible** — `[MEASURED]` all worker processes finished; no commit/push attempted; final-state and runner logs retained — RESOLVED.
**(d) Authorization granted but not exercised** — “leave changes in the working tree, keep task-local evidence under the scratch directory below, and write a summary.” No Task 3–8 implementation authorized.

Raw targets cross-build comparison remains OPEN because it conflicts with existing build provenance. Original pre-A bytes are now reconciled without any new rebaseline.

## 1. Resume order

1. Read `../dfgB-scratch/SUMMARY.md`, `progress.md`, `replay-differences.json`, and `file-size-ledger.csv` from this checkout. No build needed to inspect them.
2. Adjudicate targets provenance: every one of 21 pairs differs only `/producer/build_identity`, both values attested to binary receipts. Plan only permits SARIF provenance normalization, so raw targets gate stays FAILED. Do not remove production metadata to satisfy the replay.
3. Read original-payload-audit.json and prea-rebind-reconciliation.json plus the final raw comparison: retained 131 original byte payloads are verified; 112 remain unchanged and 19 are the exact previously reviewed #403/#406 rebind population. One fixture was added on main. No new rebaseline is authorized.
4. On a socket-capable host run full default/MCP suites and the exact 67 excluded selectors in `mcp-exclusions.txt`. Run `python3 ../dfgB-scratch/run_gates.py mutgate-full` at this checkout root: authoritative, jobs 1, no since/only/lane restriction. All 472 anchors are valid; full execution is not claimed here.
5. Controller conducts the plan's two capped review rounds, then stages only `touched-paths.txt` entries and commits `refactor: isolate ordered DFG accessors` with the plan's two trailers.

**STOP conditions:** unexplained output drift; widening metadata normalization without owner authority; oversized-file growth; new open-class review defects at the cap.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Clean-main recapture | done | `[MEASURED]` 132 payload + 132 lookup hashes unchanged; logs/repin.txt; no re-pin commit |
| Ordered views / query moves | done | `[MEASURED]` views.rs, queries.rs, legacy_storage.rs; mechanical-controls.json |
| Mechanical consumers / W4 | done | `[MEASURED]` privacy-enforced migration, normal default/MCP execution of both missing-label tests |
| S3 shifted edit | done | `[MEASURED]` stale-label mutant fails a.js delete=false; pre-B append control fails only b.js delete=true |
| S5 storage span observation | done | `[MEASURED]` test calls current_storage; edge-observation omission mutant killed |
| Full default | done | `[MEASURED]` 5472 passed, 0 failed, 7 ignored; 29 summaries |
| Runnable full MCP | done | `[MEASURED]` 6023 passed, 0 failed, 7 ignored, 67 socket exclusions; 31 summaries |
| Oracle umbrella | done | `[MEASURED]` 164 passed, 0 failed, 1 ignored; includes pins |
| Root verification report | done | `[MEASURED]` stop hook requested VERIFICATION.md; exact commands, suite totals, RED/GREEN evidence, Verified/Not verified, all 67 exclusions |
| Final focused / compile / fmt | done | `[MEASURED]` 14 DFG + 2 fallback + 7 example tests pass; all-targets mcp,dfg-oracle and fmt pass |
| Mutation evidence | done | `[MEASURED]` 7/7 isolated text plus two manual mutants killed; scoped advisory 13/13; 472-anchor preflight valid |
| Release / Tier-A | done | `[MEASURED]` parent/candidate binaries retained; matrix 178/178; quick three VALID corpora; pinned observations retained |
| Executable replay | pending | `[MEASURED]` 63 json/review/SARIF pairs match; fixture dfg-stats matches; 21 raw targets pairs differ only build provenance |
| Original pre-A raw bytes | done | `[MEASURED]` 112 byte-identical; 19 exact reviewed #403/#406 drift cases; new fixture pinned; 132 candidate raw hashes equal current main; no unexplained difference |
| Host / pre-merge checks | pending | `[MEASURED]` exact socket exclusions and full authoritative gate commands retained |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Historical plan cache baseline | 117 | `[MEASURED]` current parent/candidate cache version 119; sidecar 65; neither changed |
| Historical absolute size forecast | optional_parameter_tests B <=1320 | `[MEASURED]` live parent 1292.5 ->1322.5 (+30), below plan's +40 migration allowance; hard ceilings enforced |
| Replay provenance scope | only SARIF may differ | `[MEASURED]` targets also emits the compiled fingerprint on main; keep FAIL until owner ruling |
| Memory | None changed | Owner did not authorize memory writes |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Targets gate contract | pending | Review 21 raw pairs and build receipts; owner decides narrow metadata exception | Explicit no-other-normalization rule | replay-differences.json |
| 2 | Pre-A raw byte gate | done | Read raw-byte-control.json and historical rebind reconciliation | None | original-payload-audit.json |
| 3 | Socket-dependent tests | pending | Host full MCP / exact 67 selectors | Sandbox prohibits listeners | mcp-exclusions.txt |
| 4 | Full authoritative mutations | pending | run_gates.py mutgate-full before merge | Controller pre-merge slot | 472 registered mutants |
| 5 | Publication | pending | Two review rounds, controller commit | Gates above | no git writes by worker |

## 5. Invariants and traps — do not do these

- Keep legacy/exact identity, stable sorting, representatives, multiplicity, queries and serialized field order unchanged. No compact engine/codec activation in B.
- Frozen oracle and shared producer are byte-identical to parent. No production whole-graph shadow copy was added.
- `edit_legacy_parts` is cfg(test) only; default/MCP missing-label tests retain all three removal sites.
- `nice` priority setting is denied; owner authorized `env CARGO_BUILD_JOBS=4`. One cargo at a time, file logs, no cargo pipelines.
- Preserve failed raw targets evidence. Diagnostic identification of provenance is not a passing raw byte gate.
- Preserve seven inherited ignores and 67 exact environmental exclusions; no test was silently feature-gated away.
- No network/install/browser, .git writes, or real Prometheus/full-slicing memory acceptance runs.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| HEAD | `5a63c7e830332a49e7c0b5ae6e27f8ab7699bebf` |
| Branch | `perf/dfg-views` |
| Scratch | `../dfgB-scratch` |
| Parent binary SHA-256 | `c4512fe1e43f808bf5154835f19613c182e14bd52a6001dae27f90616fb405c4` |
| Candidate binary SHA-256 | `825fd4dc3bf5b98d57789884086696b776932d06577f8370499c5ae76cf3d90e` |
| Implementation commit | `refactor: isolate ordered DFG accessors` |
| Trailer | `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` |
| Trailer | `Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED within measured parity scope; raw targets gate unresolved · claim: "Ordered views preserve the observed DFG/CPG contract" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: ../dfgB-scratch/SUMMARY.md, mutation/replay receipts. External PR reviews remain controller-owned.

**Questions the owner owes an answer to:** targets provenance exception. Original pre-A artifacts were located; no location answer is needed. No approval to change production metadata or goldens has been assumed.
