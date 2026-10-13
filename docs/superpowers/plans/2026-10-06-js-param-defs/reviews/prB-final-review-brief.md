# Review: lane js-param-defs PR-B "callback identity" — the certified candidate (PR #351)

## Your role
You are a senior/principal engineer reviewing for the project's long-term outcome. Be rigorous about finding real defects, and never invent them. Read-only: no git writes or PR comments; delete `target/` and scratch afterward. Never open any directory named `frontend-portal`. This is static analysis; never execute corpus packages. Node may run small synthetic fixtures.

## What PR-B is
JS/TS/TSX anonymous callables (call arguments, JSX handlers, `.map(x => …)`, top-level `createServer((req, res) => …)`) get their own data-flow pass under an unspellable synthetic owner `<cb@L:C>`, so their formals and locals get Defs and edges. A binder fence stops an enclosing function's pass from minting edges into nested binders.

Contract: zero false Defs or edges; no LOST correct rows against main; Exact stays a static-binding grade (repo `CLAUDE.md`); call sites and non-JS output byte-identical; symbol, call and module navigation byte-identical.

## History you need (do not re-litigate decided points; report only if a decision is implemented wrongly)
- Two spec-review rounds and several repair loops preceded this. Your earlier reviews, if any, are in `~/prism-evidence/js-param-defs/review/prB-*`.
- **Fable's option (b), owner-accepted:** PR-B fails closed on the *parameter-expression seam*. SEAM(c, n) = the callable has parameter expressions AND a body binding or body-level function shares formal n's name. For seam names, **named callables are byte-identical to main**; synthetic callables refuse the seam formal. EVAL (direct `eval(` inside the callable) and ARGS (sloppy mode, simple parameters, `arguments` referenced) refuse all formals in synthetic owners. The earlier copy-model machinery is deleted. Design: `DESIGN-INPUT-fable-R4.md` in the packet.
- **Owner decisions in SPEC-prB §8**, including:
  - STOP-1(a): DataFlow-derived navigation (`ego`, `nodes-at`) may change only where checker-WRONG rows are removed.
  - **E13 (2026-10-09): Flow-annotated JavaScript is out of scope.** tree-sitter misreads Flow types as arrow functions and the synthetic pass emits rows for them. Measured in one SecBench root (`redos/react-native_0.63.0-rc.0`: 82 ADDED WRONG, 6 LOST the oracle calls CORRECT). The owner accepted and disclosed this; an enhancement issue follows at merge. **Do not report Flow-file rows as a blocker.** Do report it if you find the same class in code that is *not* Flow-annotated.
- **R7:** the strict-body early-error refusal needs positive proof of non-simple parameters; a formal named `undefined` is simple.

## What to review
- **Plan packet:** branch `review-final` = origin `plan/js-param-defs-prB` (PR #351). In `docs/superpowers/plans/2026-10-06-js-param-defs/` read `SPEC-prB.md`, `IMPLEMENTOR-prB.md`, `MATRIX-param-env.md`, `MEASUREMENTS-prB-R6.md`, `REPORT-prB-R8.md`, and the R7 report at `~/prism-evidence/js-param-defs/prB/repair-r7/REPORT.md`.
- **Source:** `git diff origin/main origin/wip/js-param-defs-prB-r3` (head `2431cb10`). Main files: `src/ast_callback_identity.rs`, `src/ast.rs`, `src/data_flow.rs`, `src/cpg/reaching*.rs`, `src/cpg.rs`, `src/navigation/queries.rs`, `src/cpg_cache.rs`, and the tests and goldens under `src/cpg/`.
- **Evidence:** `~/prism-evidence/js-param-defs/prB/repair-r7/` (certificate, tables, matrix, binaries in `bin/`). Corpora: X `~/prism-evidence/inputs/excalidraw-0642e72c/source`; T `~/code/bench-repos/TypeScript/src`; SecBench `~/prism-evidence/inputs/secbench-pkgs`. Oracle: `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.

## Measured results to check, not to trust
- Certificate against the last pre-seam candidate `f8c768b3`: complete on 579 admitted roots, 7 excluded (6 main-producer timeouts, 1 killed process); every differing row inside the classes SyntaxError / ARGS / EVAL / SEAM.
- Byte tables against main: X ADDED 15,513 CORRECT, LOST 3,415 WRONG; T ADDED 19,434 CORRECT, LOST 374,016 WRONG; SecBench ADDED 288,223 CORRECT, LOST 948,304 WRONG, plus the React Native rows above.
- Private corpus F (controller-run, aggregates): ADDED 9,923 all CORRECT; LOST 891 all WRONG; call sites identical.
- Matrix: 763 semantic cells and 200 parameter-kind cells pass; 0 DESIGN-CHANGE.
- Gates: nextest 5,232 passed; mutants 93/93; clippy parity with main; Tier-A matrix 178/178; Tier-A quick VALID on all three corpora (controller, outside the sandbox).

## Priorities
1. **False Exact edges or wrong Defs outside Flow files.** Build the head and attack it with your own fixtures: the seam predicate and its boundaries, the EVAL and ARGS refusals, the binder fence in both directions, kill-only nested writes, `this`, hoisting, the parameter node kinds from R7's table.
2. **LOST correct rows against main outside Flow files.** Named callables in the seam must equal main byte-for-byte. Does anything else lose a row main gets right?
3. **Synthetic identity containment.** `<cb@…>` must never reach resolution, seeds, call sites or navigation symbols.
4. **Is the certificate admissible?** Is "every differing row is in a class" checked by an independent census rather than by the product's own predicate? Are the SyntaxError overrides genuinely early errors under node? Are the exclusions and the UNDECIDED rows (25 LOST on SecBench, 2 on X) honestly reported and benign on a sample?
5. **Is E13's boundary honest?** Confirm the React Native rows are confined to Flow-annotated files, and look for the same misparse class in non-Flow JavaScript or TypeScript.
6. **Implementation dispatch.** Is `IMPLEMENTOR-prB.md` consistent with the owner decisions and unambiguous for landing `2431cb10`?

## Output
Write the review to `REVIEW_OUT`. For each finding:
- **Tag:** WRONG (name the input or state and the incorrect result) or SMELL, plus MATERIAL or IMMATERIAL to the contract above.
- **Evidence:** file:line and a minimal fixture, labelled reproduced or static.
- **Fix:** a recommended fix plus alternatives.
- **Self-critique:** when the finding would not apply.
- **Classification:** NEW FAMILY or CLOSED INSTANCE of a named earlier family.

End with a verdict — APPROVE, FIX (closed, enumerable fixes) or REJECT — and list what you did not check.
