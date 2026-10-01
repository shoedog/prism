# Handoff — S1b-4 repair round 2, targeted fold at the cap

**Written:** 2026-10-01T17:42:20.937464+00:00 · **By:** repairer /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-s1b-4-impl · feat/s1b-4-namespace · **Measured state:** `[MEASURED]` HEAD 29686b668d3c225542da6edc1709bec5ea14c585 · Tree DIRTY · Probe git status/diff · Output target/repair-r2/FINAL-CHECK.json
**Predecessor:** controller round-2 dispatch; impl-s1b4-r2-opus.md and supplied sol61 APPROVE.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` controller assigned this clone to repairer; no delegation authorized or used — RESOLVED.
**(b) Custody exposure** — `[MEASURED]`14 Git-visible changed/new files plus ignored VERIFICATION.md, manifest, frozen binaries and final snapshot in target/repair-r2/; controller must retain ignored evidence and commit — OPEN.
**(c) In flight / irreversible** — `[MEASURED]` all foreground suites, mutation runs, rebuilds, matrix and public dumps finished; final-layout default4787/0/1 and MCP4980/0/1 — RESOLVED locally.
**(d) Authorization granted but not exercised** — "No git writes." "Never open corpus F."

## 1. Resume order

1. From this clone run `shasum -a 256 -c target/repair-r2/MANIFEST.sha256`; retain the ignored evidence/frozen binaries before cleanup.
2. Controller commits the exact14 files in COMMIT-FILES-s1b4.md with its two recommended messages; record the commit association with the verified dirty snapshot.
3. Controller privately runs CONTROLLER-S1b4.sh and audits F. A clean rebuild requires refreshed binary/source provenance.

**STOP conditions:** no implementer Git writes or F reads. No third review round or restart. Do not transfer historical F acceptance or incomplete quick receipts to this fold.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| W1 | done | `[MEASURED]` six pre-fix exact-base failures; six CLI base/head equals; final arm mutant six named failures; w1/results.json, pre-fix-red.log, arm-mutant-final/results.json |
| Producer | done | `[MEASURED]` entire map/ast producer remains live for VerifiedLocal/SpannedLocal name and span checks; no layout change |
| S1 | done | `[MEASURED]` two single-caller helpers mechanically inlined; full suites and controls preserve behavior |
| S2 | done | `[INHERITED]` reviewer synthetic timings disclosed IMMATERIAL; implementation unchanged, no local perf replication |
| Final suites | done | `[MEASURED]` full default4787/0/1,29 groups; MCP4980/0/1,31 groups; final-layout-verification.json |
| Fmt/clippy/matrix | done | `[MEASURED]` fmt/split fmt pass;26 warnings each,0 new;166 matrix ok; clippy-comparison.json, matrix-final.json |
| Controls | done | `[MEASURED]`411 r5 byte-identical;381 unchanged/30 changed vs base;15 filters/15 regrades; controls-audit.json |
| X/R/T | done | `[MEASURED]`19219/953/61712 sites each;0 changed/lost/missing keys; corpora.json |
| Mutants | done | `[MEASURED]`26 variants:25 killed,1 P6 coverage survivor,0 inadmissible; final arm recheck killed; mutants/results.json |
| Final custody | done | `[MEASURED]` public preflight, complete input/binary/packet hashes and snapshot checked; BUILD-MANIFEST.json, FINAL-CHECK.json |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Handoff/verification/file list/controller/manifest | r1 is current | `[MEASURED]` replaced with r2 bindings/results and dirty starting29686b66 |
| MEASUREMENTS/CONTROLS/IMPLEMENTOR/REVIEWER/OQ/SPEC | r1-only state | `[MEASURED]` dated r2 fold/dispositions; old measurements retain historical labels |
| Old P6 kill | still killed after W1 | `[MEASURED]` disclosed1 survivor; no global equivalence claim; current arm killed |
| Memory | None | no relevant memory used or edited |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Git/evidence custody | pending | retain ignored evidence and commit exact14 files | controller-only Git | COMMIT-FILES-s1b4.md |
| 2 | Private F | pending | execute updated script and audit privately | explicit F boundary | CONTROLLER-S1b4.sh |
| 3 | Broader excluded checks | parked | separately authorize if needed | bounded fold scope | VERIFICATION.md |

## 5. Invariants and traps — do not do these

- Never open F or write Git; explicit scope.
- Local/UnprovenLocal/Class refuse identity; same-named function expressions are not binding proof.
- Keep the callable map/producer for proven targets; match name AND span.
- Mutations run alone and restore exact bytes; failed compilation/setup/zero selection is inadmissible.
- The test umbrella is599 lines; repair_r1 includes repair_r2 without changing any body.
- uv default cache is blocked; use installed eval/.venv/bin/python, immediately after rebuilding.
- P6 is a disclosed coverage survivor. Do not quote the historical25-kill receipt as current all-green.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Starting head | `29686b668d3c225542da6edc1709bec5ea14c585` |
| Base | `5048f443` (retained binary built915fca43, equivalent source) |
| Evidence | `target/repair-r2/` |
| Head | `target/repair-r2/prism-head` |
| Head SHA256 | `3e57da928119a21cab33baea4b0d46667758917d889cf38e586332e28cb2ca12` |
| Manifest | `target/repair-r2/BUILD-MANIFEST.json` |
| Reference | `probes/S1b-controls-s1b4-r5-impl.txt` |
| New tests | `tests/integration/js_binding_namespace/repair_r2.rs` |
| Cache | `104 / 60` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "unproven export terminals keep exact base rows; positive narrowing and E7 behavior remain preserved" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: target/repair-r2/{pre-fix-red.log,w1/results.json,arm-mutant-final/results.json,r5-comparison.json,corpora.json}

**Questions the owner owes an answer to:** None. Controller-only custody and F remain operational work.
