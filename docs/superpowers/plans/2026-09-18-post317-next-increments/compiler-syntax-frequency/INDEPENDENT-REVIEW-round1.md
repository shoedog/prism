# Independent planning review — single-compiler syntax frequency

**Artifact:** `artifact-manifest.json` SHA-256 `b0a7cdacd03ba472d5c385446784609fe41c5144fcd4693db096250e22a2c375`; `SPEC.md` SHA-256 `ca27e10ae2d3259a9fa99612f01d60a32d13581af05edef38452c7a8739e01c5`.

**Scope:** source-read-only planning review. No implementation, public corpus parse, census, or test suite run.

## WRONG

1. **Contradictory recomputation contract for bodyless exclusions.** The spec requires every file to carry `excluded_bodyless_signature_count`, requires bodyless function/type signatures to have no parameter-frequency rows, then says that *all counters derive from emitted nested rows*. Concrete fixture: `type T = (value: string) => number;` has one required excluded bodyless signature and zero emitted callable/parameter rows. The required count of one cannot be derived from those zero nested rows, so an implementation must violate either the required exclusion count or the stated derivation invariant.

   **Bounded repair:** state the exact derivation basis per field: parameter/callable frequency counters derive from emitted nested rows; `excluded_bodyless_signature_count` derives from its per-file exclusion tally; diagnostic counts derive from the diagnostics array; file coverage derives from file rows. Add this function-type fixture to the complete partition/recomputation control.

## SMELL

1. **Arrow-parenthesis extraction does not name the token API.** The spec says to use the arrow's direct `OpenParenToken` child. A source-only TypeScript 5.9.3 probe on `x => x` and `(x) => x` found that `ts.forEachChild(arrow, ...)` returns only `Parameter`, while `arrow.getChildren(sourceFile)` exposes `OpenParenToken` only for the parenthesized form. An implementation using `forEachChild` can therefore report `false` for `(x) => x` despite following a plausible reading of “child.”

   **Bounded hardening:** require `arrow.getChildren(sourceFile)` (not `forEachChild`) for this field, and require the `OpenParenToken` to precede `equalsGreaterThanToken`; retain bare/parenthesized/async/generic arrow vectors.

## Checked planning strengths

- Scope is genuinely syntax-only: the owned new Node script/docs paths exclude Rust, Program/type checker, native joins, CPG, and support claims.
- The fixed 414-member manifest/compiler binding, UTF-8 byte mapping, strict output caps, body-bearing syntax family, diagnostic strata, same-schema observer RED, mutation/restoration controls, core-before-public sequence, and active Node/default Rust regression gates are explicit.
- The 600/600/1,200 hard cap and early checkpoint make any new size breach a stop condition instead of a silent re-expansion.

## Verdict

**REVISE BEFORE DISPATCH — 1 WRONG / 1 SMELL.** The correction is finite and does not expand scope or budget. A second planning round should bind the amended spec/prompts, confirm the two exact controls, and retain the same artifact/cap.

## Probe log

- **Hypothesis:** TypeScript's normal AST visitor omits punctuation tokens, making “direct child” ambiguous for parenthesized arrows.
- **Expected if true:** `forEachChild` sees parameters but no `OpenParenToken`; syntax-child enumeration sees the token only for `(x) => x`.
- **Alternative:** both APIs expose an equivalent direct token, so the wording is unambiguous.
- **Observed:** `forEachChild` returned `Parameter` for both arrows. `getChildren(sourceFile)` returned `OpenParenToken`, `SyntaxList`, `CloseParenToken`, `EqualsGreaterThanToken`, `Identifier` only for `(x) => x`.
- **Result:** hypothesis supported; this establishes the SMELL above, not a public-corpus result.
