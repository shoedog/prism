# S2: implement the owner's fail-closed qualifier-identity rule, then finish the plan (you are the planner, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-s2-plan`, on the plan branch. Your prototype is the uncommitted `src`/`tests` working tree; the controller committed the docs.
- **Rules:** no git writes. Never open F. Keep disk use lean.

## Owner decision (2026-10-03, recorded in `OQ-s2.md`)
**S2-O6: "Ship X gain, fail-closed."** A qualifier class or object keeps **base** whenever its value identity escapes, or its member-write closure is unproved. Escapes and unproved writes include:
- aliasing: `const A = C`, `let A; A = C`, destructuring that yields C;
- being passed as an argument, returned, stored in a property, array or map, or spread;
- any member assignment `X.m = …` or `X[k] = …` on C or on any alias of it;
- `Object.assign`, `Object.defineProperty` or `Reflect.*` with C as an argument;
- computed member access on C;
- re-export under another name, or `export default C` together with other uses.

This applies across every file the proof can see, both the defining module and the importers. **A conservative lexical over-approximation is acceptable** for this rule: if C's identifier appears anywhere other than a direct `C.member(...)` call, `new C(...)`, a type position, or its own declaration and export, keep base. Design it so the rule is a closed predicate (a whitelist of allowed uses), not a growing blacklist.

## Do
1. **Implement the whitelist-of-uses predicate.** S2-W1 must now keep base in both grammars. Re-run your `e5-alias-boundary` controls; every alias, escape or member-write case must keep base.
2. **Measure the yield cost.** Report X / installed X / R / T against main, with every changed row CORRECT and ownership agreeing. State how many of the 179 survive, and why the rest were refused, by cause.
3. **Update `CONTROLLER-s2.sh`** so it takes a head binary and reports changed-row correctness on F, as the lane-P controller does. The controller will run it.
4. **Finish the packet:**
   - the SPEC S2 design with §0 decisions;
   - the `IMPLEMENTOR.md` S2 dispatch;
   - controls in both grammars;
   - the mutant registry `mutants/lane-s2-import-qualifiers.json`, including a mutant for the whitelist predicate;
   - a Tier-A fixture.
5. **Gates (tiered):**
   - one `cargo nextest run --features mcp`;
   - the advisory scoped mutgate;
   - fmt and clippy;
   - the Tier-A matrix;
   - S1b-4 controls byte-identical;
   - lane-P public rows unchanged.

## Final message
- the yield table and the refusal breakdown;
- files, split into src/tests and docs, with commit messages;
- the F command;
- the gates;
- what you did not verify.
