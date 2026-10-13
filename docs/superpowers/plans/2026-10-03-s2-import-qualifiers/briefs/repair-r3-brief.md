# S2 repair R3: fold spec review round 2 (you are the repair engineer, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-s2-plan`. Rules: no git writes, never open private F.
- **Committed state:** proto `origin/proto/s2-import-qualifiers` = `c35719e1` + `75a35a5e` (byte-equal to `c35719e1` + R2-src.patch); plan `ecdb6b0f`.
- **Working tree:** `src`/`tests` should equal `75a35a5e`, and docs should equal `ecdb6b0f`. Verify both before editing.

## Contract (owner S2-O7): `Exact` is a static-binding grade
Runtime re-acquisition, enumeration of runtime namespace objects, and eval/reflection are out of model. Static name, import, export and scope channels that prism models are **in model**: a static alias or member write reachable through them must keep base.

## Findings to fold (round 2: both FIX, all closed instances; this fold is a disclosed one-time cap extension)
- **Opus:** `~/prism-evidence/s2/review/spec-r2-opus.md`
- **sol:** `~/prism-evidence/s2/review/spec-r2-sol.md` (evidence alongside it)

**A. Static namespace cascade** (Opus W1(a)(b), sol W1). Revoking a namespace identity must revoke every qualifier identity it can expose, through `export * as M`, forwarded `import * as N; export {N}` / `export default N`, and named forwarders, iterated to a fixpoint.
- Refuse the own-export/default-export whitelist rule when the exported name is a namespace import.
- The R2 pin `repair_r2_out_of_model_namespace_enumeration` has the wrong disposition: *static* access through a forwarded namespace is in model. Flip it to a keep-base regression.
- Keep only genuinely runtime enumeration out of model, and only where it is not expressible through a static binding.
- Correct §2a so F4's static half is in model.

**B. Fail-closed refusal joins** (sol W2, Opus W1(c)). Refusal traversal must have explicit outcomes: `joined`, `proved unrelated/absent`, or `unavailable`.
- **`unavailable` refuses** every possibly-admitted identity. Causes include: past the positive hop cap, escaped or unprovable spelling (`C`), B0 capture failure, or resolution failure.
- Resolve the writer's specifiers with the **writer's own project** first (Opus `call_graph.rs:~2405`).
- Traverse static import and re-export source edges, cycle-safe, independently of the positive two-hop admission cap. Keep the positive cap.
- No spelling or depth exception lists. This is one failure policy.

**C. Housekeeping** (W2, S2 and S3 from both reviews):
- Commit-bind the 22-mutant registry (hash `7f1d4e2c`), plus new mutants for A and B (including "remove the unavailable-join fallback").
- IMPLEMENTOR.md dispatches from committed `75a35a5e` (plus the R3 patch), with plan HEAD as committed.
- Add a §2a disclosure row for the indexed-file boundary (`.mts`/`.cts` writers are invisible).
- Rebind installed-X in a FRESH evidence directory (the json5 BOM drift).
- Optional, only if cheap: make `compare-head.py` assert unique keys.

## Measure
Report X, installed X, R and T for the folded candidate, with every changed row CORRECT and ownership agreeing. Expected yield is about +132 (the reviewers found none of these shapes reaching the surviving X classes).

**STOP** if the fold costs more than 10 X rows. In that case, report attribution and the options, and choose nothing. This applies especially to B's unavailable-refusal: if it revokes broadly, report which unavailable joins in X trigger it.

## Tests and gates
- A regression for every reviewer repro, in both grammars. Each must fail on `75a35a5e` and keep base after the fix. Include depth-boundary (2/3 hops) and escaped/unescaped pairs.
- nextest `--features mcp`, fmt and clippy, the advisory mutgate, the Tier-A matrix (skip quick), S1b-4 controls byte-identical, lane-P rows unchanged.
- Rebuild `target/s2-plan/bin/head-*`, rebind the BUILD-MANIFEST, and refresh VERIFICATION.md.
- Write `~/prism-evidence/s2/repair-r3/R3-src.patch` (relative to `75a35a5e`) and `R3-docs.patch` (relative to `ecdb6b0f`).

## Final message
- the yield;
- each finding and how it was fixed;
- files with commit messages;
- the gates with totals;
- what you did not verify;
- any STOP.
