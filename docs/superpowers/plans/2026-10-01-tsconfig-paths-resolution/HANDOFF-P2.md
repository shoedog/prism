# Handoff — lane P, P2 measure-first planning

READ: **Written:** 2026-10-03 · **By:** Codex planner · **Provider:** codex.
MEASURED: **Workspace:** `/Users/wesleyjinks/code/prism-paths-p2-plan` · branch `plan/tsconfig-paths-p2` · HEAD `1811d2fed149cd8b5794f8a7e5557f2c915b247b`. Probe: `git rev-parse HEAD`; receipt: `target/p2-plan/entry-binding.json`.
READ: **Predecessor:** P1 PR #338, owner measure-first brief.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
READ: **Provenance:** written live by the planner. This lane uses MEASURED for rerun observations, READ for supplied/source evidence, and ASSUMPTION for forecasts. These replace the template's INHERITED/UNKNOWN vocabulary per the owner's brief.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — READ: owner assigned this planner; Opus-5.5 plan review is controller-dispatched with a cap of **2 rounds**. No reviewer has been dispatched by this worker. RESOLVED within the brief's scope.

**(b) Custody exposure** — MEASURED: no Git writes; nine new probes/docs are uncommitted. Evidence under `target/p2-plan`; production/build inputs match the retained r5 binary in 304 files. READ: the final local snapshot/manifest accompanies this checkpoint; controller commit and external custody remain OPEN.

**(c) In flight / irreversible** — MEASURED: public and final native/smoke pipelines completed; `target/p2-plan/public-run.log` and `native-final-run.log`. No producer or build is in flight. RESOLVED for this checkpoint.

**(d) Authorization granted but not exercised** — READ: "Only if the yield is material" prototype the top mechanisms. "No git writes." "F is private. Never open ~/code/frontend-portal." F aggregates have been requested via `p2-probes/CONTROLLER-p2.sh`; fresh F yield is OPEN.

## 1. Resume order

1. READ: from this clone, with `CORPUS_F_ROOT`, a new private `PRIVATE_EVIDENCE_ROOT` and pinned `TS_JS` set, run `bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/CONTROLLER-p2.sh /Users/wesleyjinks/code/prism-paths-impl/target/repair-r5/head/prism /Users/wesleyjinks/code/prism-paths-impl/target/repair-r1/base/dump_imports`. Return aggregate stdout only; native/raw output stays private. ASSUMPTION: allow several minutes.
2. MEASURED: public data are complete in `target/p2-plan/summary.json`: X/installed-X 10 callable rows each, R/T 0, ten distinct sites total. Read `P2-MEASUREMENTS.md` for mechanisms and zero-yield populations.
3. ASSUMPTION: validate F binary/probe hashes and resolution ceiling, then decide materiality. Build only after this gate. Complete the conditional prototype, changed-row certification, controls, scoped mutants, fixture/amendment and implementor dispatch before a buildable handoff.

**STOP conditions:** READ: source/oracle hash drift, ownership disagreement, private F access, open-class design failures at the two-round review cap, or absent evidence for the material-yield gate.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Checkout/binary binding | done | MEASURED: `target/p2-plan/entry-binding.json`, 304 matching inputs |
| Public fresh dumps/oracles | done | MEASURED: `target/p2-plan/summary.json`; 304 production inputs; all four full streams equal r5 |
| Probe checks and custody | done | MEASURED: 22/22 checks, `target/p2-plan/smoke/checks.json`; complete denominators/native input hashes |
| F aggregate-only measurement | pending | READ: asynchronous request; `p2-probes/CONTROLLER-p2.sh` |
| Production prototype | parked | READ: conditional authorization; material yield not yet established |
| Plan review | pending | READ: controller Opus-5.5 dispatch, 2-round cap |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| P1 historical packet | P0 listed all ten X residuals as non-relative hops | MEASURED: eight non-relative + two relative import-forwarded arrows; corrected in `P2-MEASUREMENTS.md` without rewriting historical evidence |
| Memory | None | READ: no relevant memory used and no memory update authorized |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 2 | Private hop counts | pending | READ: controller executes aggregate-only script | Controller-only F access | `CONTROLLER-p2.sh` |
| 3 | Scope/prototype decision | pending | ASSUMPTION: compare measured yield and precision/size | Fresh F | P2-0 |
| 4 | Git/external custody and review | pending | READ: use `P2-FILES.md`; controller dispatches Opus-5.5, cap 2 rounds | Controller | nine new files |

## 5. Invariants and traps — do not do these

- READ: keep S6 refusing; membership refuses on doubt; do not add program-graph ownership.
- READ: keep accepted tolerant ambient-scan costs; do not reintroduce blanket cuts without X/installed-X/F measurements.
- READ: relative Node10 hops do not search ancestor node_modules; source `typescript.js:45325–45335` distinguishes them from non-relative resolution.
- MEASURED: `affectingLocations` can be absent; native supplement attempt 1 was inadmissible, corrected before attempt 2 (`target/p2-plan/hypothesis-probe-result.log`).
- MEASURED: a native JS winner from explicit non-relative paths can occur in the priority pass. Do not infer secondary-pass absence from suffix alone; `explicit-path-witness/result.json` pins the exception.
- READ: no builds or Git writes merely to rebind an already source-matching retained binary. If a prototype is built, preserve a patch/hashes and remove only this lane's build directories after producers finish.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Planning HEAD | MEASURED: `1811d2fed149cd8b5794f8a7e5557f2c915b247b` |
| Main | READ: `c50de85a` |
| P1 binary SHA256 | MEASURED: `907d110c70962063d5fde23acd22b6d92b62b117d837b97a81f6d65ed263aa03` |
| Oracle SHA256 | READ: `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` |
| Evidence | MEASURED: `target/p2-plan` |

## 7. Refutation verdict and owner questions

**§2c verdict:** MEASURED: SURVIVED within the public denominator; F gate remains OPEN · claim: "Public native evidence establishes ten distinct remaining callable sites" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: `target/p2-plan/summary.json`, `smoke/checks.json`, `explicit-path-witness/result.json`. ASSUMPTION: those ten sites are immaterial on this planner's recommended bar; independent Opus review has not run (0/2).

**Questions the owner owes an answer to:** READ: fresh F aggregate request is pending. ASSUMPTION: a numeric materiality threshold may be useful if fresh yield lies between ten public rows and the historical 749 F rows.
