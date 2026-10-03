# Handoff — PR 340 mutation gate repair round 2

**Written:** 2026-10-03T18:28:58.453478+00:00 · **By:** /root repairer · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-mutation-gate · tooling/mutation-gate · **Measured state:** `[MEASURED]` HEAD 7cd7db1bbd8b797ff00ef5999385ad8a7a75bbb2 · Tree DIRTY · Probe git status/rev-parse/diff · Output provenance.json and production-src.diff
**Predecessor:** round-1 repair, committed by controller, not pushed per owner
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by worker; [MEASURED] claims have current probes; [INHERITED] claims are identified. Unverified facts are [UNKNOWN], never implied by an [ASSUMPTION].

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — [INHERITED] user assigns this agent as repairer — RESOLVED within this session; no subagents dispatched.
**(b) Custody exposure** — [MEASURED] modified mutgate.py, test_mutgate.py, README.md and new repair-r2 evidence; final snapshot /private/tmp/mutgate-repair-r2-final.tar.gz — RESOLVED for worker snapshot custody; controller commit pending, no git writes authorized to repairer.
**(c) In flight / irreversible** — [MEASURED] all gate/test processes completed; worker-created target cleanup recorded in cleanup.json — RESOLVED after finalization command.
**(d) Authorization granted but not exercised** — None for worker. User: "No git writes." Committing remains the controller's responsibility, not an approval request from this worker.

## 1. Resume order

1. Run `python3 scripts/mutgate/evidence/repair-r2/compare.py` to recheck bound input hashes and the 93-ID comparison; seconds, no build/network.
2. Review mutgate.py, test_mutgate.py and README.md with the receipts in this directory. Runtime changes are already certified; build artifacts were reclaimed, so any new gate execution needs a rebuild.
3. Controller may commit the reviewed artifact with commit-message.txt; do not auto-push or merge. New changes require new source-bound verification.

**STOP conditions:** no git writes by this repairer, never open F, preserve empty `git diff origin/main -- src`. Repair cap was two cycles, both completed; classify before any new extension.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Narrow demotion and targeted confirmation | done | [MEASURED] mutgate.py, test_mutgate.py; plain unwrap/local-safe-macro kills remain schema-only; README documents external/lexical limits |
| Python regressions | done | [MEASURED] unit-green-cycle2.log: 49 tests pass, including W1/W2/W3/W4/W5; fixture-receipts.json gives matched HEAD/candidate behavioral controls |
| Full paired certification | done | [MEASURED] comparison.json: 93 admissible kills in both modes, zero ID-bound differences; 89 schema/four structural demotions/zero confirmations |
| Timing | done | [MEASURED] schema.log 90.00 s; text.log 513.64 s; scoped.log 33.50 s |
| Nonempty fn-scope control | done | [MEASURED] scoped-control.json: synthetic changed lines, two schema kills and one structural text kill; actual src diff empty |
| Full Cargo suite | done | [MEASURED] full-suite-totals.json and cargo-suite.log: offline all features, 5145 passed, zero failed, one existing ignored test, 31 result blocks, 283.14 s |
| External safe macro audit | done | [MEASURED] external-macro-audit.json: locked serde_json 1.0.149 macros.rs, no direct location APIs in expansion bodies; user serialization/helper behavior outside this claim |
| Provenance | done | [MEASURED] provenance.json binds driver/tests/registry/docs and 747 production/test/build inputs; base driver equals git show HEAD |
| Snapshot and lean disk | done | [MEASURED] final snapshot and cleanup.json; only the worker-created target tree was removed |
| Controller commit | pending | [INHERITED] no git writes permitted to repairer; commit-message.txt supplied |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| scripts/mutgate/README.md | Every kill confirmed; all macros/unwraps demoted | [MEASURED] corrected to explicit direct/expanded observers, audited macro lists and targeted confirmation; includes current timings and limitations |
| repair-r1 receipts | Historical 12-minute wall times | [INHERITED] retain as history; round-2 receipts are current for this candidate |
| Round-2 initial plan/test output | Negated conditions misread as macros; thread ID panic header unsupported | [MEASURED] plan refreshed; unit-green.log retains first-cycle failures, unit-green-cycle2.log is green; hypotheses.md explains bounded corrections |
| Memory | None | No memory updates requested or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Controller review/commit | pending | Review diff and evidence, then use commit-message.txt if accepted | Controller authority; repairer prohibited from git writes | HEAD 7cd7db1b |

## 5. Invariants and traps — do not do these

- Never open F; no git writes by repairer; production src diff must stay empty.
- Automatic libtest panic headers are diagnostic metadata. Handle optional numeric thread IDs; scan the complete payload before log-tail truncation.
- Test observer scan must include referenced AFTER constants and helpers to preserve the hidden track-caller regression.
- `if !(...)` and similar negated control expressions are not macro invocations.
- Only killing tests justify test-based confirmation; passing location-sensitive selectors must not force it.
- Use the pinned local TypeScript asset and offline Cargo. Arbitrary future lanes/aliased/dynamic/external observations are not certified by this lexical scan.
- Scoped receipt uses synthetic changed lines; do not present it as a real source diff or whole-feature acceptance.
- Existing ignored test is `resolution_test::slice_elem_variant_reserved` (reserved until future slice); Tier-A was not triggered by tooling-only changes.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base HEAD | `7cd7db1bbd8b797ff00ef5999385ad8a7a75bbb2` |
| Lane | `mutants/lane-p-tsconfig-paths.json` |
| Evidence | `scripts/mutgate/evidence/repair-r2` |
| Snapshot | `/private/tmp/mutgate-repair-r2-final.tar.gz` |
| Compiler | `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` |
| Suggested commit | `fix(mutgate): restore schema speed with targeted location checks` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "The current 93-ID candidate preserves text verdicts, retains W1-W5 refusals and runs its full schema gate in 90.00 seconds" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: comparison.json, fixture-receipts.json, unit-green-cycle2.log, full-suite-totals.json

**Questions the owner owes an answer to:** None. Independent review, Tier-A, ignored reserved SliceElem test and arbitrary future/external observations were not verified; these are explicitly scoped limits, not implied passes.
