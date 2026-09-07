# Handoff — bounded UMD bridge and dirty checkout recovery

**Written:** 2026-09-07 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/callable-umd-bridge · **Measured state:** `[MEASURED]` implementationfa67821/control56106b0/closeout553bad5 pushed; base9a34ef62; recovery and all local gates complete; PR271 open.
**Predecessor:** PR270 merged9a34ef62bb48b6abb569ff62ad95e266e3124f51.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only; no agents; isolated recovery from implementation — RESOLVED.
(b) All15 dirty files committed locally in a86bbeb on archive/dirty-callable-20260907;
    not pushed or merged. Original checkout switched cleanly to new main branch — RESOLVED.
(c) Implementation and local gates complete; two SELF-PASS rounds completed — RESOLVED.
    Publication RESOLVED: https://github.com/shoedog/prism/pull/271 against main.
    Remote CI/review is separate; check live state before merge. No merge performed.
(d) Owner: "270 merged, proceed to next" plus analyze/save or reset dirty checkout.
    Authorizes bounded UMD observer bridge, not runtime authority or new installs.

## 1. Resume order

1. `git status --short --branch` in /Users/wesleyjinks/code/slicing.
2. Read same-date UMD bridge readout: final141 observer tests,4017/4207 Rust passed.
3. Verify PR publication and current remote CI; do not repeat acquisition/install.

**STOP conditions:** open-class review; no React spelling heuristics, general UMD,
imports as export= targets, augmentation support, private writes or runtime changes.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| PR270 merge | done | `[MEASURED]` gh pr view270; origin/main9a34ef62 |
| Dirty recovery | done | `[MEASURED]` local commita86bbeb; recovery readout |
| UMD acceptance RED | done | `[MEASURED]` initial10 failures; final exact-main32-test control19 pass/13 fail, umd-final-base-red.log |
| Bounded implementation | done | `[MEASURED]`32 UMD tests pass in full141 suite; schema7/producer0.8.0 |
| Full gates | done | `[MEASURED]` observer-final141/0/0; Rust4017/0/1 and4207/0/1 including doctests; authority40 results clean; helpers7/7 |
| Public replay | done | `[MEASURED]` same source30 observations/53 calls,4 class anchors program_unproven; valid:true reproduction; compact evidence JSON |

## 3. Corrections to standing documents and memory

PR270 is merged, not awaiting CI. Dirty alias source/tests are exact copies of
mergedfb2ffd9b; historical local docs do not authorize unfinished new source work.
Recovery archive retains historical statements, not current operational truth.
No memory edits authorized.

## 4. Open work

PR271 is published; check current remote CI/review. No unique implementation
was recovered that needs a separate completion slice; historical ledger/eval
evidence stays local. Next proposal: source-backed classification of remaining
Program-closure lookup failures, with negative fixtures before any expansion.
Local public archive `umd-public-evidence.tgz` SHA256
`0e1a5188796a969f289b73efcecbe97d86ec990479d8c15690d1a70c94e5618d`.

## 5. Invariants and traps — do not do these

- Do not push or merge recovery branch: it contains historical local evidence.
- No hard reset or git clean was needed; recover exact files with git show a86bbeb:path.
- Compiler recovery symbols and singleton syntax alone are not binding authority.
- Keep full Program/provider and duplicate/write/cache barriers; flags stay false.
- LSP tools absent; pinned compiler/source fallback, not structural navigation proof.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | /private/tmp/prism-umd-bridge-LqRG69 |
| Recovery branch | archive/dirty-callable-20260907 |
| Recovery commit | a86bbeb |
| Base | 9a34ef62bb48b6abb569ff62ad95e266e3124f51 |
| Compiler | /private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js |
| Profiles | /private/tmp/prism-callable-authority-98TLLN/public/profiles |
| Public source | /private/tmp/prism-acquire-w2FtSq/source |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "UMD provenance requires bounded source-owned singleton identity" · pass: two SELF-PASS rounds (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: umd-final-base-red.log, observer-final.log, public-summary.json; no open in-scope findings

**Questions the owner owes an answer to:** None.
