# S2 repair after spec review round 1 (you are the repair engineer, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-s2-plan`. The docs are committed at `b8f5b2f3`. The uncommitted `src`/`tests` working tree is byte-identical to the committed prototype `c35719e1` (branch `proto/s2-import-qualifiers`).
- **Rules:** no git writes. Never open the private corpus F. Keep disk use lean.

## Goal
Zero false Exact edges for JS/TS import-qualifier calls, under owner decision S2-O6 ("Ship X gain, fail-closed"), while keeping as much of the measured X +132 as is sound. The predicate stays a **closed whitelist of uses**. Do not add blacklist special cases.

## The reviews (both returned FIX; read both in full)
- Opus: `/Users/wesleyjinks/prism-evidence/s2/review/spec-r1-opus.md`
- gpt-6.1-sol: `/Users/wesleyjinks/prism-evidence/s2/review/spec-r1-sol.md` (evidence is alongside it)

Combined, they describe these distinct escape families. Every one was reproduced as a false Exact:
- **F1. Non-owned members on the direct-call path:** `Q.valueOf()`, `__defineGetter__`, inherited statics, classes with `extends`. Opus W1.
- **F2. Construction exposes the class:** `new C().constructor`. Opus W2, sol W2.
- **F3. Modules reached without a named binding:** dynamic `import()` with string, computed or `.default` keys; TS `import = require`; and refusals keyed only by the local name rather than the exported name. Opus W3, sol W1.
- **F4. A revoked re-exported namespace (`export * as M`, then `Object.values(M)`) leaves its classes admitted.** Opus W4.
- **F5. `new.target` and `super` bypass the class write guards.** sol W3.
- **F6. A nested object initializer steals the surrounding `this` carrier.** sol W4.
- **F7. Direct `eval` leaves closure certified.** Opus W5, sol W5.
- **F8 (SMELL, fold fail-closed).** A file with a parse error and no import/export facts is dropped together with its refusals (`call_graph.rs:~2192`). Such files must revoke, not vanish.

## Do
1. **Write the escape-channel table first (Opus S1),** in SPEC as a new section. Enumerate every JS/TS channel by which a class or namespace object can become reachable without its own identifier:
   - module objects: static and dynamic namespaces, `require`, `default`, re-export;
   - receivers: `this`, `super`, `new.target`, inherited and static `this`;
   - instances: `.constructor` and prototype walks;
   - reflection: `Object.*`, `Reflect.*`, `globalThis`/`window` lookups;
   - code evaluation: `eval`, `new Function`, `with`;
   - parse-incomplete files.

   For each channel, state the closed-whitelist rule that keeps base, and name the test that pins it. The goal is a closed table, so that round 2 checks completeness against the table instead of discovering new families.
2. **Fold F1–F8 against the table,** preferring the reviewers' recommended fixes. Where both reviewers offered fixes, choose the simpler sound one and record why.
3. **Measure before adopting any yield-costing cut** (standing rule: never fold a refusal cut without measuring its yield).
   - For each fix, report its X, installed X, R and T delta against the current +132, with every changed row CORRECT and ownership agreeing.
   - The **F2 fix (dropping or guarding `new`) is the most likely to cost yield.** Measure the drop-`new` option and at least one guarded option, such as refusing when any `.constructor` access, prototype walk or reflection is visible in the proof's file universe, and report both.
   - **If any single cut costs more than 10 rows of X, or the total falls below 110,** implement it on its own and STOP. Report the options with their yields; do not pick one. The controller escalates that choice to the owner.
4. **Add a regression in both grammars** for every reviewer repro (each must fail on `c35719e1` head and keep base after the fix), plus a mutant per new whitelist or table rule in `mutants/lane-s2-import-qualifiers.json`.
5. **Fix the stale IMPLEMENTOR.md** (both reviews flag it, Opus S5 and sol S2): dispatch from committed `c35719e1` plus this repair; a mutgate `--since` that is an ancestor; and binaries rebuilt from source, not taken from the planner clone.
6. **Gates (tiered):**
   - one `cargo nextest run --features mcp`;
   - the advisory scoped mutgate;
   - fmt and clippy;
   - the Tier-A matrix;
   - S1b-4 controls byte-identical;
   - lane-P public rows unchanged.

   Rebuild `target/s2-plan/bin/head-*`, update the BUILD-MANIFEST binary hashes so `CONTROLLER-s2.sh` binds the new head, and refresh VERIFICATION.md.

## Final message
- the channel table summary;
- a per-family fix with the chosen option and the reason;
- a per-cut yield table;
- the final X / installed X / R / T;
- files, split into src/tests and docs, with commit messages;
- the gates with totals;
- what you did not verify;
- any STOP triggered under step 3.
