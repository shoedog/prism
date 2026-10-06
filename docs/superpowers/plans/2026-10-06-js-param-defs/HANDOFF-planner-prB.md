> **Current authority (R2b, 2026-10-06):** The retained owner-authorized copy fold and full583-root measurement are complete with explicit exclusions. Row/identity contract SELF-PASS (NOT INDEPENDENT); Rust quick remains INVALID, so full gates are unmet. Original row1 is KEPT/Exact; rows2–4 LOST WRONG by disclosed E12. Source cache111/binaries and cumulative cb996630/2cb7102a patches are bound under repair-r2b. Initial synthetic ADDED attribution was refuted by unchanged frozen-R2 WRONG parity. Use MEASUREMENTS-prB-R2b.md/HANDOFF-repair-prB-r2b.md. STOP:none on admitted population. Historical body below has no current dispatch authority.

> **Historical custody/dispatch (R2, 2026-10-06):** R1 source/docs patches are committed as `cb996630`/`2cb7102a`. Resume from those commits plus `prB/repair-r2/R2-src.patch` and `R2-docs.patch`; follow IMPLEMENTOR-prB and HANDOFF-repair-prB-r2. R2 is STOP on four checker-CORRECT LOST SecBench rows; no acceptance or continuation authority. See current R2 measurements/handoff and STOP-secbench-evidence. The record below retains historical R1 observations, not current R2 gates or closure.

> **Historical planner record — superseded for current authority (2026-10-06):** source is committed `0660b3c5`, docs `41de010c`. Owner STOP-1(a), O1 and O2 are resolved; the old zero-WRONG/zero-loss claims are refuted by both R1 reviews. Use SPEC/IMPLEMENTOR R1 clauses, `MEASUREMENTS-prB-R1.md` and `HANDOFF-repair-prB-r1.md` (also `~/prism-evidence/js-param-defs/prB/repair-r1/HANDOFF.md`). Current STOP is none on the admitted population; clippy warning parity remains unmet by one SMELL. Old gates are inherited, not R1 verification. TS 5.9.3 parameter-default scope is an oracle blind spot (Opus F4).

# Handoff — lane js-param-defs, PR-B "callback identity" planner (spec + prototype + measurement)

**Written:** 2026-10-06 · **By:** planner subagent (Opus-5.5), controller session `0ec85e7b` · **Provider:** claude
**Workspace:** `~/code/prism-pd-plan` · `plan/js-param-defs-prB` · **Measured state:** `[MEASURED]` HEAD `da0604b3` · Tree DIRTY (prototype + packet, uncommitted by design) · Probe `git status --short` · Output: final report
**Predecessor:** PR-A planner + R1/R2 repairs (`HANDOFF-planner-prA.md`, `HANDOFF-repair-r1/r2.md`)
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below
**(a) Lane ownership** — `[INHERITED]` the controller owns lane 1, `CACHE_VERSION` and F. This planner has finished. — **RESOLVED** at handback.
**(b) Custody exposure** — `[MEASURED]` everything is uncommitted: 12 modified tracked files + 2 new source files (`src/ast_callback_identity.rs`, `src/cpg/callback_identity_tests.rs`) + packet docs/probes. Snapshot `~/prism-evidence/js-param-defs/prB/custody/` (`prB-tracked.patch`, `prB-untracked.tgz`, `prB-untracked.list`). Source hashes: `prB/final-source-manifest.txt`. — **OPEN** until the controller commits.
**(c) In flight / irreversible** — `[MEASURED]` none at handback (all background chains finished; see §2).
**(d) Authorization granted but not exercised** — "No git commits/pushes (controller commits)." "Never open any directory named frontend-portal." "STOP and report (do not choose) on … a nav or call-site change."

## 1. Resume order
1. Owner: decide **STOP-1** (SPEC-prB §8: accept the DataFlow-derived nav delta of the E3 fence, or fence synthetic passes only). Also O1 (harness credit for anonymous sources) and O2 (ship PR-B before or with PR-C).
2. Controller: commit prototype (src/tests/mutants) and packet (docs/probes) as two commits; messages in the final report.
3. Controller: F diff with a NEW `PRIVATE_EVIDENCE_ROOT`: `CORPUS_F_ROOT=~/code/frontend-portal PRIVATE_EVIDENCE_ROOT=~/prism-evidence/js-param-defs/f-prB-<sha> bash docs/superpowers/plans/2026-10-06-js-param-defs/CONTROLLER-pd.sh diff ~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js ~/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3 <HEAD_BIN> ~/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3-bytes <HEAD_BYTES>` with HEAD built from the committed tree together with `examples/r1_byte_dump.rs` = `probes/byte_dump.rs` (or the measured `prism-head-b10` / `-bytes` if the committed tree hashes equal `final-source-manifest.txt`).
4. Spec review: SPEC-prB.md, Opus ∥ sol, cap 2.

**STOP conditions:** LOST checker-correct row; RE-OWNED row not proved correct; nav or call-site change beyond STOP-1's measured mechanism; non-JS non-identity; build time > +25 %; F `status: STOP`.

## 2. State ledger
| Item | State | Evidence / correction |
|---|---|---|
| Gap-1 census on main | done | `[MEASURED]` CENSUS.md PR-B section; `prB/census/` |
| Prototype (B-D1…B-D14) | done, uncommitted | `[MEASURED]` `final-source-manifest.txt`; binary `prism-head-b10` |
| Public rows X/Xi/T + prism JS fixtures | done | `[MEASURED]` MEASUREMENTS-prB §1; `prB/public-b10/` |
| SecBench rows | done | `[MEASURED]` 574/583; ADDED 255,886 (12 WRONG, E11 minified residue); LOST 594,226 all WRONG; `prB/sb-rows-b10/` |
| SecBench callback / joint / eligible | done | `[MEASURED]` 0/97 standalone; joint 90/97 (BFS 90/90); eligible 120→121 traced, `dot-prop` base credit withdrawn (E3, PB12) |
| Non-JS identity | done | `[MEASURED]` §3 |
| Nav identity | STOP-1 | `[MEASURED]` X 1/241 (b6 and b10), T 2/81 (b6) — all `ego`; `prB/nav-X*/`, `nav-T/` |
| Gates | done | `[MEASURED]` nextest 5,196/1 skip; doctests 2/2; mutgate 26/26 + 27/27; fmt; clippy 235/235; Tier-A matrix 178/178, quick VALID ×2; `prB/gates/` |
| Perf | done | `[MEASURED]` build ×0.75 / ×0.63 / ×0.57 (X / T / lodash); `prB/perf/perf.json` |
| F | next (controller) | CONTROLLER-pd.sh diff |

## 3. Corrections to standing documents and memory
| Location | Stale or false assertion | Correction |
|---|---|---|
| EVALUATION §3.2 Gap 1 | "nav output byte-identical" is achievable together with the containment fence | `[MEASURED]` not with the fence on legacy passes: removed E3 anchors/edges change `ego`/`nodes-at` (STOP-1) |
| EVALUATION §3.2 Gap 1 | `FunctionInfo.synthetic_name` | Not needed and riskier: the identity lives only in DFG owner strings (B-D1) |
| EVALUATION §3.6 / brief | PR-B "converts the 97 callback rows" | `[MEASURED]` 0 standalone traced; 90/97 jointly with a bare read (PR-C's half); harness still `prism_error` (O1) |
| SPEC-prA §6 E3 | "PR-B's containment is the natural fix" | Fixed for every JS/TS pass (B-D3); T loses 368,854 checker-WRONG rows |

## 4. Open work
| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | STOP-1 nav vs E3 | owner | choose (a)/(b)/(c) | owner | SPEC-prB §8 |
| 2 | O1 harness credit | owner | eval-only harness change or nav visibility | owner | MEASUREMENTS §2b |
| 3 | SecBench ADDED WRONG residue | review | see MEASUREMENTS §2a mechanisms | spec review | `sb-rows-b10/adjudication` |
| 4 | F diff | next | §1 step 3 | controller | CONTROLLER-pd.sh |

## 5. Invariants and traps — do not do these
- Never put the synthetic name in `FunctionInfo`/`function_name`/call graph — resolution isolation is by construction (B-D1).
- Never re-own nested-callback rows away from named passes — same-owner assignment propagation in `trace.rs` loses taint paths (B-D2, PB3).
- Never share one nav cache dir between two different binaries in a loop — the cache keeps one build identity per repo and rebuilds each time (≈40 s per X query). Use per-binary dirs (`nav_identity.py`).
- `mutgate --since` ignores untracked files; use `git add -N` then `git reset` (done here) or commit first.
- `cargo build --bin prism --example …` unifies dev-dependency features: the measured `prism-head-bN` binaries were built with the byte-dumper example; a bin-only build is byte-different (Tier-A used the bin-only build).
- Measurement binaries live in `prB/bin/`; never substitute `target/release/prism` into a receipt.

## 6. Identifiers
| Item | Verbatim |
|---|---|
| base binary | `~/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3` sha256 `294b18d0de2d33ff7faa207640fa5099e711d32100c2acb48b61e0674c50548f` |
| base byte dumper | `prism-base-da0604b3-bytes` sha256 `184812b37f3f1d536d8a71b4cb49b0a46b22479d3c664452a6cdff0882fb6d83` |
| head binary (final) | `prism-head-b10` sha256 `90e176766bb9a7467fb9dbc4353f192a66426dcee04e6a78ff5d54ff9eab1409` |
| head byte dumper | `prism-head-b10-bytes` sha256 `fc07124d99241fd9f75b6f9c86106a9ea6d989f1c61fe8d08737b9ea8b5d81da` |
| source manifest | `prB/final-source-manifest.txt` sha256 `00c62b42a1c2914bcee942b1e7734795faca000e650ad0907429eec4330438b7` |
| TS checker | `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` |

## 7. Refutation verdict and owner questions
**§2c verdict:** NOT RUN — the planner's handback goes to an independent two-reviewer spec round · claim: "PR-B adds checker-CORRECT synthetic-owner rows, removes only checker-WRONG legacy rows, leaves call sites and non-JS output identical, and changes nav only through STOP-1" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: MEASUREMENTS-prB.md

**Questions the owner owes an answer to:**
1. STOP-1: accept the DataFlow-derived nav delta of the E3 fence, or fence synthetic passes only?
2. O1: how should SecBench credit an anonymous source (`callees --location` is `LocationOutOfRange`)?
3. O2: ship PR-B before PR-C or together?
