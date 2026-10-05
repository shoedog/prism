# Handoff — MEAS-A SecBench ground truth

**Written:** 2026-10-05T04:19:04Z · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-secbench · plan/secbench-ground-truth · **Measured state:** `[MEASURED]` HEAD 4e592daa7858a195eb3a9eb77c83dfbc763b49fa · Tree DIRTY (only harness/registration/packet/root verification) · Probe git status/branch/rev-parse and product diff · Output /Users/wesleyjinks/prism-evidence/meas/secbench/custody.json
**Predecessor:** none — first in lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` user dispatch owns MEAS-A; `[MEASURED]` no workers were delegated — **RESOLVED by dispatch**.
**(b) Custody exposure** — `[MEASURED]` no commits (controller-only); new harness/packet are untracked, root VERIFICATION.md added, eval/pyproject.toml modified. snapshots/pre-run.tar.gz, reconciled.tar.gz and final.tar.gz retain stable points. final.tar.gz includes original binary/observations and final packet; SHA256 in artifact-index.json — **OPEN until controller commit; snapshot custody established**.
**(c) In flight / irreversible** — `[MEASURED]` full run, replay, live controls and tests completed; no measurement subprocess in flight — **RESOLVED**.
**(d) Authorization granted but not exercised** — "Do (measure first; no product changes in this lane)"; "No git writes (the controller commits). No network; inputs are pre-fetched. Keep disk use lean."

## 1. Resume order

1. Run `git -C /Users/wesleyjinks/code/prism-secbench status --short`, then read `/Users/wesleyjinks/prism-evidence/meas/secbench/artifact-index.json` and this packet's MEASUREMENTS.md (seconds). Expected dirty scope is eval/pyproject.toml plus eval/secbench/, this packet and root VERIFICATION.md; no src changes.
2. Controller commits the two owned slices described in VERIFICATION.md, including root VERIFICATION.md in the docs slice. Do not commit raw external inputs, release artifacts, caches or unrelated files. This worker may not write Git.
3. For an audit, run `PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench.replay --run /Users/wesleyjinks/prism-evidence/meas/secbench/run --inspection /Users/wesleyjinks/prism-evidence/meas/secbench/inspection-final.jsonl --adjudications eval/secbench/adjudications.json --out /Users/wesleyjinks/prism-evidence/meas/secbench/replay-check` from the repo root (seconds; authenticates all input and decisive raw stdout bytes).
4. A fresh full measurement is one command, `PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench`, approximately 21 minutes plus build/inspection. After the harness commit, supply `--sut-repo` pointing at a controller-provided exact 4e592daa checkout; the docs/harness commit must not silently become a new measured SUT.

**STOP conditions:** input/binary revision drift, new seed/sink replay, guessed GT,
errors represented as misses, or claimed conversions from first-break counts.
Three-pass validation cap reached; broad GT recovery/causal adjudication is parked,
not a new round or an artifact restart.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Checkout/input pins | done | `[MEASURED]` custody.json; eval/secbench/input-pins.json; 583 tarball hashes match |
| Release SUT | done | `[MEASURED]` run/binding.json; binary SHA d603e7cd7f372137c254f4d7cf98ea8c33f92ac22dc510bea0e1b00e919aa455 |
| Full corpus observations | done | `[MEASURED]` run/raw/; 2,123 original invocations; all 583 attempted |
| Final GT/outcome ledger | done | `[MEASURED]` final/entries.jsonl: 600 rows, 192 eligible, 84 traced, 391 GT unavailable, 17 acquisition excluded |
| First-break screen/sample | done | `[MEASURED]` adjudications.json: frozen 30 entries; 7/13 agreement, 3/9 resolved agreement; same worker |
| Full causal attribution | parked | `[MEASURED]` 40 unresolved and 26 error categories remain; partial/frontier is union-seed evidence |
| Path syntax evidence | done | `[MEASURED]` final/summary.json: destructure 0/165/27; rest 9/159/24 (present/absent/unknown among 192) |
| Full population path point estimate | parked | `[MEASURED]` 391 GT exclusions plus unresolved path syntax; MEASUREMENTS.md carries bounds |
| Tests/control/replay | done | `[MEASURED]` VERIFICATION.md: Rust 4946/0/1, eval 965 passed/3 skipped, Node 22, Python 27; live controls and replay identical |
| Actual conversion ranking | parked | `[MEASURED]` no capability fixes/counterfactuals; all demonstrated conversion lower bounds zero |
| Packet/harness | done | `[MEASURED]` five required packet Markdown files plus summary/binding; controller commit pending |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Original run/summary and entries | 108 traced / initial 220 candidates | `[MEASURED]` historical, superseded by final/; run/SUPERSEDED.md and MEASUREMENTS.md explicitly reconcile |
| Interim handoff | measurement next, old test counts | `[MEASURED]` replaced here with final ledger/checks |
| Memory | none relevant | `[MEASURED]` quick registry search found no relevant hits; no memory update authorized/performed |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Commit authorized artifacts | pending | controller reviews/stages only harness + eval registration + packet, two messages in VERIFICATION.md | no Git-write authorization for worker | MEAS-A |
| 2 | Broader source GT recovery | parked | design bounded HTTP/returned API/indirect payload fixture slice, preserve current baseline | three-pass cap and open-class API resolution limits | 391 GT unavailable |
| 3 | Root-specific causal/path validation | parked | design per-root frontier/independent sample method before stronger claims | retained frontier is union of seeds; low agreement | WS7 measurement quality |
| 4 | Workstream comparison | parked | owner may use provisional 3 > 6; validate actual conversions in a separately authorized bounded product slice | no product changes allowed here | opportunities 3=13,6=6; weights31,15 |

## 5. Invariants and traps — do not do these

- No worker Git writes, network, exploit execution, product changes or private corpus access.
- Source is test-fed data, never callback/factory/HTTP setup configuration substituted for payload.
- CLI seeds lines: parameter byte identity and sink value/member identity are required for traced credit.
- Use class+entry composite keys; entry names collide across classes. The stale-adjudication guard caught the manual join mistake.
- Missing/ambiguous GT is neither a taint miss nor a product capability failure.
- Syntax anywhere in a callable is not payload-path syntax; hangersteak config spread is the concrete trap.
- First-break opportunity counts are not actual conversions; weights are declared scenarios, not CVSS.
- Unreached frontier is union seeded; preserve unresolved mechanism alternatives.
- Keep original run raw/binding immutable. final/ is current; do not resurrect the old 108-traced claim.
- Canonical template is known; no broad home-directory search needed.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| SUT | `4e592daa7858a195eb3a9eb77c83dfbc763b49fa` |
| SecBench | `5d362353550a8baa42bba34edd26e5fb86d41b60` |
| Inputs | `/Users/wesleyjinks/prism-evidence/inputs/secbench-pkgs/manifest.json` |
| Evidence | `/Users/wesleyjinks/prism-evidence/meas/secbench` |
| Final ledger SHA256 | `84d58d1d1c3fc1bf987a6be9ad6882d58acfe29516e2d0f6fb57537c353c023a` |
| Final summary SHA256 | `e520c42d73b41701dfd9f67a8e38db189cfa255e3adb3c3b980ea30405ba2951` |
| Input pins SHA256 | `922ba44ba017253a8a7d3667495682e4659ece525d84447a39f7bf102c6a243e` |
| Compiler SHA256 | `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` |
| Template | `/Users/wesleyjinks/code/prompts-skills-steering/bootstrap/handoff-template.md` |

## 7. Refutation verdict and owner questions

**§2c verdict:** REFUTED — corrected in place · claim: "The original 108-traced classification satisfies strict payload-parameter and terminal identity" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: PROBES.md, VERIFICATION.md, control-tests.log, live-controls/results.json and determinism.json

**Questions the owner owes an answer to:** None. Controller commit is the already assigned remaining integration action; stronger conversion claims require future authorized work.
