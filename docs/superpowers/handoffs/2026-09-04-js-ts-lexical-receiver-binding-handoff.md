# Handoff — JS/TS lexical-scope-aware receiver binding

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-js-ts-lexical-receiver-binding` · `js-ts-lexical-receiver-binding-owner`
**Exact base:** `551adc463e2e164637378757f2ba1ba872a43946`

## 0. Current verdict

**IMPLEMENTED; TWO-ROUND REVIEW CLOSED; ALL RUNNABLE GATES GREEN.** This is item 3, the prerequisite for item 4's JS/TS typed-parameter and `new`-constructor receiver recovery. The slices remain separate.

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
- After rebasing onto current `origin/main` (`551adc4`), `cargo fmt --all -- --check`, `git diff --check`, `cargo check --all-targets --features mcp`, and configured Clippy all pass; Clippy retains only the repository's existing warning inventory. The complete default suite is 3,671 passed / 0 failed / 1 ignored across 28 test binaries; the complete `mcp` suite is 3,857 passed / 0 failed / 1 ignored across 30 test binaries.
- Review cap declared at two rounds.
- Post-rebase release Tier-A matrix passes 104/104. The first candidate quick run had one oracle failure; the clean exact-base `551adc4` control then produced thirteen oracle failures, including explicit rust-analyzer `-32801: content modified` errors, while both runs had zero SUT errors. At the declared one-retry diagnostic cap, an immediate-rebuild candidate retry completed with `oracle_error_rate=0.000`, `sut_error_rate=0.000`, and the sole invalid reason `corpus_sha_drift: 59dbf23cbf53 != pinned 20c8490591a3`, proving the earlier oracle failures transient. Generated reports/snapshots and the disposable control worktree were removed.
- Verification exclusion: LSP semantic navigation tools were unavailable. All compiler, full-suite, cache-roundtrip, incremental, and Tier-A checks otherwise ran.

## 4. Custody

- Root `main`, `origin/main`, and this branch were rebound to PR #229 merge `551adc4` before publication; the rebase was conflict-free.
- Branch/worktree: `js-ts-lexical-receiver-binding-owner` at `/private/tmp/slicing-js-ts-lexical-receiver-binding`.
- Exact base: `551adc463e2e164637378757f2ba1ba872a43946`.
- Rewritten checkpoints: design `191850d`; intentional RED `e5194e4`; implementation/review 1 `4920dbf`; review 2 and verified implementation `59dbf23`.
- Root checkout's pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` remain untouched.
- Implementation and publication were explicitly authorized on 2026-09-04. Merge remains conditional on required checks being green.
