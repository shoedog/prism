# S2 spec review, round 1 of 2: JS/TS import qualifiers (fail-closed)

## Your role
You are a senior/principal engineer reviewing this plan for the project's long-term outcome. Be rigorous about finding real defects, and never invent them. Read-only: no git writes, and keep disk use lean. If you build, delete `target/` afterward.

## Goal the review serves
S2 adds **correct Exact call edges** for JS/TS calls made through an imported class or object qualifier (`import {C} from './m'; C.m()`, `new C()`). There must be **zero false Exact edges**. The owner's decision S2-O6, "Ship X gain, fail-closed", governs: a qualifier keeps base unless every use of its identifier is on a closed whitelist:
- a direct `C.m(...)` call;
- `new C(...)`;
- a type position;
- its own declaration, import or same-name export.

Measured yield is X +132 CORRECT, R 0, T 0, F 0 changed. Tag each finding MATERIAL or IMMATERIAL to this goal. A finding that cannot produce a false Exact, a lost correct base edge, or a wrong measurement is IMMATERIAL.

## What to review
- **Plan packet:** `docs/superpowers/plans/2026-10-03-s2-import-qualifiers/`, on this checkout, branch `review` = origin `plan/s2-import-qualifiers` `b8f5b2f3`, PR #343. Read the SPEC (§0 decisions), IMPLEMENTOR.md, OQ-s2.md, MEASUREMENTS.md, PROBES.md and VERIFICATION.md.
- **Prototype implementation:** `git show c35719e1` (branch `origin/proto/s2-import-qualifiers`). The main files are:
  - `src/ast/js_import_qualifiers.rs`, especially `js_ts_qualifier_refused_uses`, `js_ts_qualifier_use_allowed` and `js_ts_qualifier_member_writes`;
  - `src/js_import_qualifiers.rs`;
  - the resolver wiring in `src/resolution*.rs` and `src/call_graph.rs`;
  - `tests/integration/js_import_qualifiers_test.rs`.

## Questions to answer
1. **Is the whitelist actually closed?** Can any identifier occurrence, in either grammar (JSX/JS via tree-sitter-javascript, TSX/TS via tree-sitter-typescript), escape C's identity while being classified as allowed?
   - Consider node kinds the walk does not visit: `jsx_*` element names, `nested_identifier`, `type_identifier`, decorators, `satisfies`/`as` expressions, `export =`, `import x = require`, labelled statements, `arguments`, `eval`/`with`, and shadowing by a local of the same name.
   - Is the refusal name-keyed per file, and does the resolver apply it across every file the proof sees (defining module and all importers)? What happens with two different bindings named `C` in one file?
2. **Is every pre-whitelist rule still sound under the whitelist?** That means the member-write closure, the `this` carrier mapping, default-export plus active-use, and namespace imports.
3. **Is the measurement and oracle chain admissible?** Do changed rows require agreement on module, ownership and full span? Is "132 CORRECT" a complete population, not a sample?
4. **Are the tests, controls and mutant registry sufficient?** Would each new behaviour's test fail on main? Note that S2-02 survives the advisory mutgate.
5. **Does IMPLEMENTOR.md give an implementer an unambiguous dispatch?** Cherry-pick, the gates, and what must stay byte-identical.

## Output format
Write your review to `REVIEW_OUT` (given in your launch prompt). For each finding:
- **Tag:** WRONG (provably wrong: name the input or state and the incorrect result) or SMELL (a risk with no demonstrated wrong output), plus MATERIAL or IMMATERIAL.
- **Evidence:** file:line, and a minimal JS/TS snippet when you claim a false Exact. Run it through the prototype if you can; label it as reproduced or static reasoning.
- **Fix:** a recommended fix plus one or two alternatives, with their tradeoffs.
- **Self-critique:** under what assumptions or scope the finding would not apply.

End with a **verdict**:
- **APPROVE**: no material WRONG;
- **FIX**: closed, enumerable fixes;
- **REJECT**: a design defect.

Also list what you did not check.
