# Handoff — PR 340 mutation gate repair, round 1

**Written:** 2026-10-03 · **By:** Codex repairer · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/prism-mutation-gate` · `tooling/mutation-gate` · **Measured state:** `[MEASURED]` HEAD `a524dcff72be13c10375633f773ee4fc2200591c` · Tree DIRTY · Probe `git status --short; git rev-parse HEAD` · Output bound in `evidence/repair-r1/provenance.json`; final snapshot `/private/tmp/mutgate-repair-r1-final.tar.gz`.
**Predecessor:** `[INHERITED]` review-sol61, supplied by the owner.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` owner dispatched this repairer; no agents dispatched — **RESOLVED by explicit owner assignment**.
**(b) Custody exposure** — `[MEASURED]` four modified tracked files, report/handoff and 1.3 MiB evidence; checkpoint and final snapshots saved outside the build tree; git writes forbidden. Controller must commit — **OPEN until controller takes custody; bytes independently snapshotted**.
**(c) In flight / irreversible** — `[MEASURED]` verification complete; all build directories from this run removed; no running process — **RESOLVED by completed receipts and `cleanup.json`**.
**(d) Authorization granted but not exercised** — None. All requested repair and verification work completed. Standing limits: "No git writes. List the files and commit messages for the controller." Never open `~/code/frontend-portal`.

## 1. Resume order

1. Read `scripts/mutgate/REPAIR-R1.md` for dispositions, receipts, the 93-row table and controller commit groups. Both full runs and the scoped control passed.
2. Replay the small `python3 scripts/mutgate/evidence/repair-r1/compare.py` receipt checks if useful; it requires no build artifacts. Full gate reruns rebuild the deleted target.
3. Controller: take custody of the snapshot, perform independent repair review if desired, then commit the listed files. This worker performs no git writes.

**STOP conditions:** two repair/certification attempts total. Unresolved verdict differences or a timing failure remain defects; classify before extending. No git writes or production changes.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| W1–W6 repair | done | `[MEASURED]` final 33-test suite: original 39 failing assertions/subtests, repaired 33 pass. Includes original-registry duplication control. `unit-original-current.log`, `unit-repaired-current.log`; 20 original and 22 repaired compiled-case receipts in `regression-original.json`, `regression-repaired.json` (the original short-timeout assertion fails before its long-timeout control) |
| R4-04 | done | `[MEASURED]` registry redefines historical ID to UTF-16 BOM decoding; distinct existing test reads LE/BE bytes and includes documentation negative cases |
| S7/S8 | done | `[MEASURED]` README corrected, file scope implemented, stale anchors selected globally |
| Full suite | done | `[MEASURED]` `cargo test --offline --all-features --no-fail-fast` with pinned compiler: 5145 passed / 0 failed / 1 existing ignored. Initial missing-setting run preserved. `cargo-suite-configured.log`, `full-suite-totals.json` |
| Certification | done | `[MEASURED]` both modes 93/93 KILLED and admissible; exact 93-row comparison has zero differences. Text 724.59 s; schema 733.28 s including clean text confirmation. `comparison.json`, `certification-table.md`, mode logs/summaries |
| Scoped/fmt/source control | done | `[MEASURED]` nonempty synthetic fn scope: 3/3 KILLED/admissible in 37.54 s; fmt/check pass; zero-byte production diffs |
| Cleanup | done | `[MEASURED]` workspace `target` removed after every process completed; 4.5 GiB logical size beforehand, no physical reclaim claim. `cleanup.json`; evidence retained |
| Verification hook | done | `[MEASURED]` root `VERIFICATION.md` records exact commands, full-suite totals, pre-change failures, negative cases and limits; hook adds only one Python registry regression and documentation |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `README.md` in this directory | timeout kills, unsupported target override, all selectors must kill, 30-minute cap, schema-polluted text fallback | `[MEASURED]` corrected to fail timeouts, require all admissible and one kill, clean baseline/text builds, 15-minute target, full merge authority |
| Lane registry | R4-04 duplicated unreadable-file intent | `[MEASURED]` explicit `intent_revisions` record and new decoding edit/selector |
| External review | six findings describe predecessor | `[INHERITED]` immutable historical evidence; `REPAIR-R1.md` supersedes repaired candidate status |
| Memory | None | No memory updates authorized |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Full certification | done | Read `comparison.json` and replay `compare.py` | None | 93 IDs, zero differences |
| 2 | Controller custody | pending | Commit listed repair and evidence files | Owner forbids git writes here | PR 340 |
| 3 | Cleanup | done | Read `cleanup.json`; build artifacts absent | None | workspace `target` |

## 5. Invariants and traps — do not do these

- Never count a timeout or inadmissible result as a kill; baseline failures invalidate that selector's mutants.
- Never accept a schema kill without a clean text kill. Macros and direct location observers are conservatively demoted; hidden caller behavior needs text confirmation.
- Never apply fallback edits to schema source; Cargo artifacts can be reused, source cannot.
- Never edit production source, write git state, access the excluded frontend checkout, or fetch dependencies.
- The all-feature suite requires the explicit pinned local TypeScript compiler. The initial missing-setting failure is preserved, not attributed to this repair.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Candidate | `a524dcff72be13c10375633f773ee4fc2200591c` |
| Review | `/Users/wesleyjinks/prism-evidence/tooling/mutation-gate/review/review-sol61.md` |
| Lane | `mutants/lane-p-tsconfig-paths.json` |
| Compiler | `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` |
| Evidence | `scripts/mutgate/evidence/repair-r1` |
| Final snapshot | `/private/tmp/mutgate-repair-r1-final.tar.gz` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "The repaired full gate agrees with pure text for all 93 lane-P mutants within 15 minutes." · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: `evidence/repair-r1/comparison.json`, `unit-green.log`, `regression-repaired.json`

**Questions the owner owes an answer to:** None.
