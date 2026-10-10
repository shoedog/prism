# S2 R4: rebase onto lane PKG and apply owner rule S2-O9 (you are the repair engineer, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-s2-plan`. Rules: no git writes, never open private F.
- **Committed bases:**
  - S2 proto `origin/proto/s2-import-qualifiers` = `61641bdb` (the R3b candidate) on `75a35a5e` / `c35719e1`;
  - S2 plan `origin/plan/s2-import-qualifiers` = `aa55a532`;
  - lane PKG proto `origin/proto/workspace-package-resolution` = `92c1d0bc`, plan `c89bc5b7`, both off main `4e592daa`.
- Run `git fetch` first; fetching is allowed. Your working tree should equal `61641bdb` (src/tests) plus `aa55a532` (docs). Verify before editing.

## Owner decisions in force
- **S2-O7:** `Exact` is a static-binding grade.
- **S2-O8:** build package resolution first. Done as the lane PKG prototype, measured at 0/132 S2 recovery because TypeScript itself cannot resolve the blocking imports in an unbuilt checkout (unbuilt `dist/` exports, `virtual:` loader modules).
- **S2-O9 (2026-10-04), verbatim choice "PKG + TS-unresolvable out of model":**
  - Land PKG.
  - A writer import is **out of model, and disclosed**, only when neither prism (including PKG) nor the TS resolver can bind it in the checkout.
  - It stays **fail-closed everywhere resolution succeeds**, including sol r2 W2's repros: the three-hop forward and the `C` forward past a resolved module still keep base.
  - Accepted cost: a false Exact if C is mutated through such an unresolvable import.

## Do
1. **Assemble.** Apply lane PKG's src/tests (`git diff 4e592daa 92c1d0bc -- src tests mutants`) onto the R3b working tree, resolve conflicts, and bump the cache epoch once for the combined state.
2. **Implement S2-O9 in B's refusal join.** A writer import whose specifier prism's resolver ladder (paths, then PKG package entries, then relative) cannot bind is `out_of_model_unresolvable`. It records no refusal and is counted in diagnostics.
   - `unavailable` (refuse, scoped) remains only for chains that are unprovable past a **resolved** module.
   - `node:` builtins are proved external (`absent`).
   - Prism has no TS at runtime, so prism's own resolver decides. **Measure the gap the owner's rule does not cover:** writer imports that TS 5.9.3 resolves but prism does not. Report them by category on X, installed X, R and T. These are PKG coverage gaps (PKG OQ-3), and each is a potential undisclosed cost. If any such writer, if refused, would revoke a gain row, list it explicitly; that goes to the controller.
3. **Measure.** Report X, installed X, R and T against main, with every changed row CORRECT and ownership agreeing. Expected near +132. STOP if it is more than 10 rows below 132; report the attribution and choose nothing.
4. **SPEC.**
   - Add S2-O8 and S2-O9 to §0 and OQ.
   - Add a §2a channel row: "unresolvable writer import: out of model (S2-O9), disclosed", with the measured TS-resolves-but-prism-doesn't gap.
   - IMPLEMENTOR: S2 implementation depends on lane PKG merging first. Dispatch is from the committed S2 proto plus the R4 patch, rebased onto PKG's merged main.
5. **Tests**, in both grammars:
   - `*_out_of_model_unresolvable_*` pins (unbuilt dist export, `virtual:` scheme);
   - a `node:` builtin proved-absent test;
   - the r2 W2 repros still keep base;
   - a mutant for the out-of-model classification boundary.
6. **Gates:**
   - nextest `--features mcp`, doctests, fmt and clippy, the advisory mutgate, the Tier-A matrix (skip quick), S1b-4 controls byte-identical, lane-P rows unchanged.
   - Rebuild `target/s2-plan/bin/head-*` and rebind the BUILD-MANIFEST for `CONTROLLER-s2.sh`.
   - Write `~/prism-evidence/s2/repair-r4/R4-src.patch` (relative to `61641bdb` + PKG `92c1d0bc`; also give a version relative to `61641bdb` alone that excludes PKG files) and `R4-docs.patch` (relative to `aa55a532`).

## Final message
- the yield;
- the TS-resolves-but-prism-doesn't gap table;
- files with commit messages;
- the gates;
- what you did not verify;
- any STOP.
