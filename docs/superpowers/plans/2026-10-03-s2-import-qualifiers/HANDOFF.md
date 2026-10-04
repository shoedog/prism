# Handoff — S2 R2 static-binding candidate, measured and ready for controller review

**Written:** 2026-10-04T11:04:08.696791+00:00 · **By:** Codex repair engineer · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-s2-plan · planning · **Measured state:** `[MEASURED]` HEAD b8f5b2f3fdc39de32877952fdb4ccf01f1483548 · Tree DIRTY · Probe git status --short; git rev-parse HEAD · Output repair-r2/initial-custody.json and final-custody.json
**Predecessor:** R1b historical runtime-contract experiments
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — [INHERITED] repair-r2-brief dispatches worker; controller owns Git/private F/review/adoption. [MEASURED] actual transcript gpt-6.1-sol/model-attestation.json — RESOLVED within scope.
**(b) Custody exposure** — [MEASURED] initial archive, candidate-source/bin/bindings, exact patches and final source/evidence archives under repair-r2 — RESOLVED locally, uncommitted; controller preserves/commits before cleanup.
**(c) In flight / irreversible** — [MEASURED] required gates and primary public replay complete; supplemental replay disposition in supplemental-measurement.json. No irreversible action; both replays completed, no known active job — RESOLVED.
**(d) Authorization granted but not exercised** — [INHERITED] “Skip Tier-A quick” /“never open private F” /“no git writes.” [MEASURED] followed; quick unverified. Worker authorized repair complete; controller review/adoption pending, not inferred from gates.

## 1. Resume order

1. `cat /Users/wesleyjinks/prism-evidence/s2/repair-r2/PATCHES.md` (seconds), then patch-application-check.json and BUILD-MANIFEST. Cherry-pick c357 and apply both R2 patches on the declared composite parent; verify bytes.
2. Rebuild from source using IMPLEMENTOR, rebind changed embedded build identity/tool hashes and replay public/native gates. Skip quick per owner brief; never treat it green.
3. Controller runs private F wrapper with bound tools and returns aggregates, then serial model-bound cumulative R2 review (cap2; R1 used). Commits/adoption belong to controller; no auto-merge.

**STOP conditions:** changed unproven row/owner mismatch/lost base row/input drift; X below122; at review cap classify before extending; no restart. Current combined X132 passes cutoff.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| S2-O7 scope/source | done | [INHERITED] brief; [MEASURED] source patch/binding; F1/F2/F5/F6/corrected F8, epochs118/74, no F3/F4/F7 cuts |
| Combined yield | done | [MEASURED] public/summary.json132/132/0/0, all correct/module/owner/span, no losses; lane-p.json49,220 preserved bound comparisons |
| Tests/REDs | done | [MEASURED] full5168/0/1; docs2; targeted31; c357 five new-cut REDs/two preservation GREEN controls |
| fmt/clippy/matrix | done | [MEASURED] fmtPASS, clippyPASS371 warnings(no attribution), matrix182OK |
| S1b/advisory mutations | done | [MEASURED]411/639/822byte comparisons;22registered16selected16admissible15killed, S2-02survives/six omitted; eight repair mutants killed |
| Patches/custody | done | [MEASURED] source/docs exact application checks, final source/evidence archives/custody |
| Private F/R2 review/adoption | pending | [INHERITED] controller only; no worker execution |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| SPEC/OQ and dispatch packet | Old runtime scope/STOP/current binary/gate claims | [MEASURED] S2-O7, combined132/132/0/0/current gates and exclusions; all channel dispositions explicit |
| R1/R1b/PROBES | Historical bytes/results read as current | [MEASURED] supersession banners; old runtime witnesses preserved, F3/F4/F7 now out of model by owner decision |
| BUILD-MANIFEST/OWNED-FILES/root verification | Old111/67/tools/input hashes/strict-base gates | [MEASURED] rebound770inputs/118/74/fresh head tools/current gates; root verification local/excluded |
| Memory | None | No relevant hit/update requested; unchanged |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Authoritative registry coverage | pending | Controller all22 run after anchors committed; retain S2-02 SMELL | Advisory scope; controller | advisory-mutgate/summary.json |
| 2 | Private F/independent R2/adoption | pending | Rebuild/rebind and controller wrapper/model-bound review | Controller custody/authorization | IMPLEMENTOR/CONTROLLER-s2.sh |
| 3 | Tier-A quick/full/platform/runtime | parked | Quick skipped by brief; other checks separately scoped | Out of this repair | VERIFICATION exclusions |

## 5. Invariants and traps — do not do these

- Never open private F or write Git; controller only.
- Never fold F3/F4/F7 whole-project cuts under S2-O7; runtime mutations excluded.
- Preserve visible static C alias/write refusals and every populated base row.
- Retain incomplete-file facts and legacy missing-star opacity together; omission found/fixed in assembly.
- Do not overlap Cargo builds with a full nextest run; observed executable-spawn ENOENT is inadmissible for behavioral attribution.
- No runtime safety proof from static oracle/pinned Exact tests; prior reflected-codegen witness remains under old contract.
- Quick stays unverified; no baseline edits. Mutant survivor remains a SMELL, no equivalence claim.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Evidence | `/Users/wesleyjinks/prism-evidence/s2/repair-r2` |
| Planning HEAD | `b8f5b2f3fdc39de32877952fdb4ccf01f1483548` |
| Prototype | `c35719e1132809dc835f64f476cf31e2c18bd35f` |
| Main/mutgate ancestor | `4e592daa7858a195eb3a9eb77c83dfbc763b49fa` |
| Brief | `/Users/wesleyjinks/prism-evidence/s2/planning/repair-r2-brief.md` |
| Patches | `repair-r2/R2-src.patch`, `repair-r2/R2-docs.patch` |
| Epochs | `CPG118 /navigation74` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "selected S2-O7 candidate preserves >=122 correct X gains and all populated base rows" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: repair-r2/public/summary.json; lane-p.json; c357-red.log; nextest-serial.log

**Questions the owner owes an answer to:** None for authorized repair; controller adoption and pending S2-O1/O2 confirmation remain outside worker decision.
