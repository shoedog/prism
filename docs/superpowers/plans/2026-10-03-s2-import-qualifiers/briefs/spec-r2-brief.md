# S2 spec review, round 2 of 2 (FINAL under the cap): JS/TS import qualifiers under S2-O7

## Your role
You are a senior/principal engineer reviewing for the project's long-term outcome. Be rigorous about finding real defects, and never invent them. Read-only: no git writes, and keep disk use lean. If you build, delete `target/` afterward.

## What changed since round 1
Round 1 (both reviewers returned FIX: `~/prism-evidence/s2/review/spec-r1-{opus,sol}.md`) found seven identity-escape families. The repair measured them:
- F3 (dynamic re-acquisition), F4 (namespace enumeration) and F7 (eval / reflected codegen) **each reduce X yield to 0**;
- F7 cannot be closed at all (`[].filter.constructor(...)` reaches `Function`).

The **owner then decided S2-O7, "Static-binding contract"** (SPEC §0). This is prism's documented contract (repo `CLAUDE.md`):

> `Exact` is a static-binding grade … It does not assert runtime truth. Runtime mutation of module objects (monkey-patching, reflective writes, `eval`, host globals, re-acquisition through `require`/`import()`) … is out of model for every rung and every language.

Under S2-O7, the in-model rules are:
- the closed whitelist of uses;
- static aliases and member writes keep base;
- F1: own-member calls, and refusal of classes with heritage;
- F2: no construction admission;
- F5: `new.target` and `super`;
- F6: `this` carriers;
- F8: parse-incomplete files revoke.

F3, F4 and F7 are **out of model, disclosed**, and pinned by `*_out_of_model_*` tests.

## Goal the review serves
Correct Exact call edges for JS/TS import-qualifier calls, with **no false Exact edges within the static-binding contract**. Measured yield: X +132 CORRECT (module, ownership and full span agree), installed X +132, R 0, T 0, F 0 changed / 0 lost.

Tag each finding MATERIAL or IMMATERIAL to that goal. Findings that only re-raise a runtime-mutation channel the SPEC §2a table already marks out of model are **not defects**. Report them only if the disposition is wrong, meaning the channel is actually expressible through static bindings that the contract covers, or if it is undisclosed.

## What to review
Check out branch `review` = origin `plan/s2-import-qualifiers` `ecdb6b0f` (PR #343). The prototype is `origin/proto/s2-import-qualifiers` = `c35719e1` + `75a35a5e`. Run `git diff origin/main origin/proto/s2-import-qualifiers` for the cumulative source.

Priorities:
1. **Completeness of the §2a channel table** within the static-binding contract. Is there a static-binding channel — a name, import, export or scope construct prism models — through which C's identity or members change, or the site binds to something else, that is neither on the whitelist nor refused? Check both grammars.
2. **Correctness of each in-model rule** (whitelist, F1, F2, F5, F6, F8) and its regression test. Would each test fail on `c35719e1`? Is the cross-file refusal join complete for static bindings, including renamed imports, re-exports and default exports?
3. **Round-1 findings disposition.** Confirm each round-1 WRONG is either fixed in model or correctly classified out of model.
4. **Measurement admissibility** (complete population, not a sample) and the mutant registry. S2-02 still survives.
5. **IMPLEMENTOR.md dispatch.** It currently names an evidence-path patch. Is an implementer dispatch from the committed branch `c35719e1` + `75a35a5e` unambiguous?

## Output format
Write the review to `REVIEW_OUT`. For each finding:
- **Tag:** WRONG (provably wrong: name the input or state and the incorrect result) or SMELL, plus MATERIAL or IMMATERIAL.
- **Evidence:** file:line, and a minimal snippet, labelled reproduced or static.
- **Fix:** a recommended fix plus alternatives, with tradeoffs.
- **Self-critique:** under what assumptions or scope the finding would not apply.

Also state, for the controller's convergence decision, whether any material WRONG is a **new family** that the round-1 families and the §2a table do not cover (open-class), or a **closed instance** with a bounded fix.

End with a verdict:
- **APPROVE**: no material WRONG;
- **FIX**: closed, enumerable fixes;
- **REJECT**.

List what you did not check.
