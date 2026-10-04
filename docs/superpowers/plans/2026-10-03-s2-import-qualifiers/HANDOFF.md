# Handoff — S2 import-qualifier planning

**Written:** 2026-10-04 00:56 UTC (2026-10-03 local) · **By:** gpt-6.1-sol planner · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-s2-plan` · `plan/s2-import-qualifiers`
**Measured state:** [MEASURED] HEAD `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`; tree has only the new packet untracked (`git status --short`).
**Predecessor:** [READ] Owner brief after lane P PRs #338/#342.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries.
**Provenance:** Written live by the planner. [MEASURED] means observed this turn;
[READ] means read source or owner authority; [ASSUMPTION] means unproven.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — [READ] Owner assigned this checkout to the planner; no
subagent dispatched. RESOLVED within this session.

**(b) Custody exposure** — [MEASURED] New packet is untracked. No git writes
authorized; controller must commit it. OPEN until controller adoption. Packet
and evidence snapshots are `target/s2-plan/packet-snapshot.tar.gz` and
`evidence.tar.gz`; their index is `evidence-index.json`.

**(c) In flight / irreversible** — [MEASURED] All local builds, controls, public
runs and tests have completed. No process or irreversible operation in flight.
RESOLVED locally; F remains controller work.

**(d) Authorization granted but not exercised** — [READ] "Only if the yield is
material — Prototype the top mechanisms." No material permitted gain established
yet. "No git writes." F is controller-only.

## 1. Resume order

1. [READ] From this checkout, run the F command in README with private environment
   values and `target/s2-plan/bin/{main-prism,main-dump_imports}`; return only its
   JSON aggregate. Public replay takes several minutes; the private duration is
   unmeasured. The old supplied reference binaries are rejected by the manifest.
2. [READ] Inspect MEASUREMENTS and `current-main/summary.json` in the evidence
   archive, then adopt the packet/binaries with the commit set in FILES.
3. [ASSUMPTION] Decide S2 proportionality under the no-new-target/E5 rules before
   dispatching a product prototype.

**STOP conditions:** [READ] F source must never be opened here; no target added by
refinement; no absence proof; no git writes; no network/install attempt; no review
round beyond 2 without the at-cap classification.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Checkout binding | done | [MEASURED] Clean initial main `4e592daa`; owner branch |
| Current-main streams / facts | done | [MEASURED] 19,219 X; 19,219 installed X; 953 R; 61,712 T sites; source-bound binary manifest |
| Native instrument calibration | done | [MEASURED] Both grammars, Unicode/underscore, assignment aliases, compressed input, negative wrapper controls pass; `source-bound-controls.log` |
| Final native census | done | [MEASURED] Low rows 552 / 552 / 30 / 3,167; native-certified existing-target filter candidates 0 / 0 / 0 / 0; MEASUREMENTS |
| Supplied reference/source binding | done | [MEASURED] Refuted on 3,129 bare import-member rows per X snapshot; final census rebuilt on exact main; PROBES M5 |
| MCP full suite | done | [MEASURED] Nextest 5,137 passed, 1 existing ignored; 2 doctests passed; logs |
| F | pending | [READ] Aggregate request sent; `CONTROLLER-s2.sh` |
| Prototype / dispatch | parked | [ASSUMPTION] No material permitted public gain; F owner placeholders remain open |
| Cleanup | done | [MEASURED] Owned cargo build directory removed; runnable binaries retained; `cleanup.json` |

## 3. Corrections to standing documents and memory

[MEASURED] The supplied reference's current-main assumption was refuted and
corrected in the packet manifest, public runner, controls and F wrapper. Earlier
reference counts and the first F command are superseded. Final T low count is
3,167 after admitting underscore require names, superseding 3,149. [READ] S1b's
historical E8 non-goal remains valid. No memory write authorized.

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Public measurement | done | [READ] Adopt the archive and MEASUREMENTS | None | S2-0 |
| 2 | F measurement | pending | [READ] Controller runs wrapper | Private access boundary | F |
| 3 | Final owner decision | pending | [READ] Fill OQ-S2-1/OQ-S2-2 | F aggregate | S2 |
| 4 | Git custody | pending | [READ] Controller commits FILES set and preserves snapshots/binaries | No planner Git authority | `plan/s2-import-qualifiers` |

## 5. Invariants and traps — do not do these

- [READ] Keep every unproven row at base: positive identity must intersect the
  actual base candidates; dropped static methods are outside this lane's gain.
- [READ] E5 preserves written bindings and may-call results; member writes do not
  write the binding. The six X multi-target actions are `register` call results.
- [MEASURED] TypeScript Module flags also occur on functions with members; use
  declaration provenance before calling a value a re-exported namespace.
- [READ] Run no private source probe in this checkout. The wrapper's private
  files/logs remain in the controller's private evidence directory.
- [MEASURED] The text-regex identifier gate omitted 18 T require sites. Use the
  TypeScript AST identifier predicate; the current controls pin this correction.
- [MEASURED] Python 3.9 has no stdlib tomllib; the facts builder verifies copied
  checkout-lock versions through Cargo's own offline locked metadata JSON.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base | [MEASURED] `4e592daa7858a195eb3a9eb77c83dfbc763b49fa` |
| Packet | [READ] `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/` |
| Evidence | [READ] `target/s2-plan/` |
| Native oracle | [MEASURED] TypeScript `5.9.3`, SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` |
| F wrapper | [READ] `CONTROLLER-s2.sh BASE_BIN FACTS_BIN TS_JS` |
| Current binaries | [MEASURED] `target/s2-plan/bin/main-prism`, `target/s2-plan/bin/main-dump_imports` |

## 7. Refutation verdict and owner questions

**§2c verdict:** [MEASURED] SURVIVED — public-only claim: "No native-certified
existing-target filter candidate; all seven distinct multi-target source sites
(13 rows across snapshots) keep base under E5" · pass: SELF-PASS (NOT INDEPENDENT)
· evidence tier: TEST-BACKED · record: `MEASUREMENTS.md`, `PROBES.md`.

**Questions the owner owes an answer to:** [ASSUMPTION] F aggregates; whether to
park S2 if F also has no material permitted gain. [READ] Exact placeholders are
OQ-S2-1/OQ-S2-2 in `OQ-s2.md`; Opus review remains 0/2 dispatched here.
