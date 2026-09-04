# Handoff — JS/TS lexical-scope-aware receiver binding

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-lexical-receiver-binding` · `js-ts-lexical-receiver-binding-owner`
**Exact base:** `5051918f61c99fda83eb18936992fb62025b7669`

## 0. Current verdict

**IMPLEMENTED; FOCUSED GATES GREEN; TWO-ROUND REVIEW CLOSED; FULL GATES NEXT.** This is item 3, the prerequisite for item 4's JS/TS typed-parameter and `new`-constructor receiver recovery. The slices remain separate.

## 1. Authority boundary

For simple JS/TS identifier receivers, persist whether a parameter, function-scoped `var`, or reaching lexical declaration binds the receiver at the call. A locally bound receiver cannot authorize `ImportQualified`. No receiver type or new Exact receiver edge is added in this slice.

## 2. Hypothesis-probe-result log

- LSP semantic navigation was requested through the applicable skill but its tools are unavailable; targeted references and compiler-backed tests are the fallback and this remains a verification exclusion.
- Parameter/local names are already extracted into `js_ts_function_locals`; the alternative extraction-loss cause is ruled out.
- The `ImportQualified` resolver arm consults import/module facts but no scoped shadow fact, proving the cause of the parameter-shadow wrong edge.
- Full and subset call-site extraction both retain the receiver AST node and byte span. A per-call derived fact is therefore bounded and does not require reparsing or a global scope graph.
- The existing whole-function binding set is unsuitable: it cannot preserve import visibility after an ended nested block.
- Current cache pins are CPG 57 and navigation 26; both must advance because stored call-site authority and resolved edge topology change.
- Review round 1 WRONG: nested calls capturing an enclosing parameter/block binding and named function/class self bindings were initially missed, retaining the imported-module Exact edge. Exact REDs reproduced both JS and TS capture failures; the classifier now walks enclosing functions plus module/block scope.
- Review round 1 WRONG follow-up: the first class-expression fix leaked its self name to later module calls. The `loopEnded` control exposed it; class-expression self names are now descendant-only.
- TypeScript AST evidence showed `for (const api of items)` as `for_in_statement left: (identifier)` with an unnamed `const` token. An explicit header-binding branch now distinguishes lexical declarations from bare assignment headers.
- Review round 2 WRONG: TypeScript runtime value declarations outside the original JavaScript grammar subset (`enum`, abstract class, ambient function signature, namespace/module, and import alias) were not classified, so a same-named imported module could retain an Exact edge. The finite TypeScript declaration population is now explicit; interface/type-alias declarations are type-only fences, and namespace contents cannot leak to outside calls.
- Review round 2 closed at the declared cap. The population was closed and enumerable, the added value/type-only/namespace boundary matrix is green, and no WRONG or SMELL remains from the two review rounds.

## 3. Verification state

- Exact-base JavaScript RED selected 3 tests: 2 controls passed and the new parameter-shadow test failed because `param` retained the imported-module Exact edge.
- Exact-base TypeScript RED selected 6 tests: 3 controls passed and 3 shadow tests failed on the imported-module Exact edge (both typed parameter variants and the lexical-scope matrix).
- Cache pins intentionally expect CPG 58/navigation 27 while production constants remain 57/26 at the RED checkpoint.
- Focused receiver GREEN: JavaScript 5/5, TSX 1/1, TypeScript 7/7 before the round-1 expansion; both capture REDs are now 1/1 green. Review-round-2 runtime-declaration REDs failed first at the ambient function case, then the complete value/type-only/namespace matrix passed.
- CPG v58 behavior round trip, navigation v27 non-vacuous shadow-absence round trip, serde-default/comparison exclusion, and both version pins are green.
- Incremental visible-to-shadowed and shadowed-to-visible transitions match fresh builds. Same-function parse recovery fails closed while recovery inside a nested callable does not leak. A Python control proves the JS/TS-only flag cannot suppress non-JS import qualification.
- Complete language targets after review round 2: JavaScript 65/65, TSX 48/48, TypeScript 43/43. `cargo fmt --all -- --check` and `git diff --check` pass.
- `cargo check --all-targets` passed before the round-1 expansion; it will be rerun with the full gates.
- Review cap declared at two rounds.
- Full suite and Tier-A remain pending.

## 4. Custody

- Root `main` and `origin/main` were rebound to PR #230 merge `5051918f` before branch creation.
- Branch/worktree: `js-ts-lexical-receiver-binding-owner` at `/private/tmp/slicing-js-ts-lexical-receiver-binding`.
- Exact base: `5051918f61c99fda83eb18936992fb62025b7669`.
- Design/custody checkpoint: `81ad576`; intentional RED checkpoint: `0c1c296`; implementation/review-1 checkpoint: `c568c76`; review-2 checkpoint: this document's containing commit.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation was explicitly authorized on 2026-09-04. Publication of this successor PR is not yet assumed by this handoff.
