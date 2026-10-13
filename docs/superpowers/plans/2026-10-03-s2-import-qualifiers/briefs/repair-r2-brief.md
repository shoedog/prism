# S2 repair R2: build the static-binding candidate under owner decision S2-O7 (you are the repair engineer, gpt-6.1-sol)

Same clone and rules as before: `/Users/wesleyjinks/code/prism-s2-plan`, no git writes, never open private F. Prior work: `REPAIR-R1.md`, `REPAIR-R1b.md`, and `~/prism-evidence/s2/repair-r1{,b}/`.

## Owner decision S2-O7 (2026-10-04, verbatim choice: "Static-binding contract")
S2 returns to prism's documented contract (CLAUDE.md): **`Exact` is a static-binding grade.** Runtime mutation of module objects is out of model for every rung, including:
- monkey-patching and reflective writes;
- `eval`, `Function` and host globals;
- re-acquisition through `require`/`import()`.

S2-O7 supersedes S2-O6's runtime-mutation scope. R1b measured F3, F4 and F7 as each zeroing X, and found F7 unclosable (`[].filter.constructor`).

## The candidate
Base it on `c35719e1` plus the following, all of which you measured at 0 X cost each:
- the original closed whitelist of uses;
- **F1:** captured own-member calls, and refusal of classes with heritage;
- **F2:** drop every construction use;
- **F5:** `new.target` and `super`;
- **F6:** `this` carrier mapping;
- **corrected F8:** parse-incomplete files revoke.

**Excluded as out of model, and disclosed:** F3 (dynamic import, `require`, TS `import = require`, `.default` re-acquisition), F4 (namespace enumeration via `export * as M` / `Object.values`), and F7 (`eval`, `Function`, reflected codegen). Do not add whole-project revocations for these.

One exception: when a lexically visible member write or alias of C, through a *static* binding, is in a file the proof sees, that is in model and keeps base, as the whitelist already does. Do not regress it.

## Do
1. Assemble the candidate and measure its **combined** X, installed X, R and T, with every changed row CORRECT and ownership agreeing. Combined yield is unmeasured so far. If it falls more than 10 rows below 132, STOP and report the attribution.
2. **SPEC:**
   - add S2-O7 to §0 and to `OQ-s2.md`, marking S2-O6's runtime scope as superseded;
   - in the §2a channel table, give every channel a disposition: **in model, keeps base** (name the rule and test) or **out of model, disclosed** (cite the CLAUDE.md static-binding clause);
   - record the review r1 findings F3, F4 and F7, and the R1b reflected-codegen WRONG, as out of model by owner decision.
3. **Tests:** both grammars.
   - Every in-model rule gets a regression that fails on `c35719e1`.
   - Every out-of-model channel gets one pinned test documenting the current, disclosed behaviour, named `*_out_of_model_*`, so that a future contract change is visible.
   - Mutants go in the registry for each in-model rule.
4. **IMPLEMENTOR.md:**
   - the dispatch is cherry-pick `c35719e1`, then apply the R2 patch;
   - the mutgate `--since` is an ancestor;
   - binaries are rebuilt from source.

   Rebuild `target/s2-plan/bin/head-*`, rebind the BUILD-MANIFEST hashes for `CONTROLLER-s2.sh`, and refresh VERIFICATION.md.
5. **Gates:**
   - nextest `--features mcp`;
   - fmt and clippy;
   - the advisory mutgate;
   - the Tier-A matrix;
   - S1b-4 controls byte-identical;
   - lane-P rows unchanged.

   Skip Tier-A quick; it hung for over an hour last round. Note it as unverified.
6. **Patches:** write `~/prism-evidence/s2/repair-r2/R2-src.patch` (src/tests relative to `c35719e1`) and a docs patch.

## Final message
- the combined yield;
- the channel dispositions;
- files with commit messages;
- the gates with totals;
- what you did not verify.
