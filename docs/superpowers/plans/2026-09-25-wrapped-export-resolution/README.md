# Wrapped-export resolution: planning packet (2026-09-25)

**Base:** `origin/main` `12ca6e8e`. Planning only; nothing under `src/`, `tests/`, `eval/` or `Cargo.*` changed.

| File | Purpose |
|---|---|
| `SPEC.md` | the S1 design. §0 lists the owner decisions |
| `PLANNING-PROBES.md` | mechanism (M1–M12) and probes (P0–P15), each with the command, the pre-run expectation, the result, and the output path |
| `IMPLEMENTOR.md` / `REVIEWER.md` | dispatch briefs |
| `probes/` | census, projection, row-diff and audit tools; the control generator; expectation records; the expected Excalidraw row-diff |
| `prototype/wrapped-export-prototype.diff.txt` | the throwaway feasibility prototype (never built into this branch) |

The evidence root is `~/prism-evidence/wrapped-export/planning/` (`MANIFEST.sha256` `1e35eaa4…`).

## Key measured facts

- **Mechanism (M1–M4).** The declarator-export arm skips `forwardRef(fn)` and `memo(fn)`. This was deliberate (P4
  "F4"): a fact is a *name*, and R4c binds it by name lookup, so a same-name nested function could capture it. The
  inner arrow already has a callable identity, because `function_name` Pattern 3 names it after the declarator.
  Same-file JSX, `export { X }` and `export default X` already bind to that identity as Exact (C03–C05).
- **Prototype, span-verified (P6, P7).** Recording the inner function's **exact span** and filtering R4c by that
  span recovers **107** dropped sites on Excalidraw and **4** on the private React corpus. It recovers **0** on ruff
  `playground` and TypeScript `src`, whose dumps are byte-identical to base. An independent parser audits all 111 new
  edges as correct. There are 0 other row changes, the DFG edge dump is byte-identical, and Tier-A stays at 159/159.
- **PR #324 ledger.** 72 of the 80 E2 rows are recovered. The other 8 import through `@excalidraw/…` package
  specifiers, which R4c never resolves.
- **Hazards (P2, P3).** Shape-only admission (any call with a function argument) would bind 27 declarators to the
  wrong callable: `styled` style functions, array callbacks, thunk creators. It would gain 1 site. Every F4-style
  negative control stays refused under the span design (C07, C09–C13, C16, C19, C20, C24, C25).
- **Pre-existing WRONG found (M8, C06).** `const f = <ternary>; export { f }` plus a nested `function f` resolves a
  **false Exact** on `main` today. The list and default arms never got the F4 guard. Measured prevalence is 0 on
  three corpora.
- **The bigger lever (P4 latent).** Non-relative imports (tsconfig `paths`, workspace packages) hold about 3,174
  (Excalidraw) and 2,773 (private corpus) resolvable-terminal drops. That is roughly 30× to 700× S1. This is a
  projection, not a measurement.

## Recommendation

Implement **S1**: admit only React `forwardRef`/`memo` (`memo` may take a comparator), with callee provenance from
`"react"`, `const` only, and a direct function first argument. Record the result as a new span-carrying
`SpannedLocal` export target, and bind it **Exact** only to the unique FunctionId with that file, name and span.

Budget caps: **src 200 / tests 450 / combined 650** honest lines, with a 2-round review cap. Bump both caches
(CPG 98, nav 54).

Acceptance is the measured row-diff on all four corpora: exactly 107, 4, 0 and 0 rows, all audited.

## Owner decisions needed (SPEC §0)

1. **D1, admitted shapes.** Recommendation: A, React `forwardRef`/`memo` only (not B, a library allowlist; not C,
   shape-only).
2. **D2, confidence.** Recommendation: Exact.
3. **D3, slicing.** Recommendation: S1 alone. S2 (default-object aliases, 7 sites) and S3 (anonymous wrapper-nested
   callbacks, 58 call nodes, which need new function identity) are separate.
4. **D4, the pre-existing C06 false Exact.** Recommendation: a follow-up slice S1b that reuses `SpannedLocal`.
5. **D5, sequencing.** Recommendation: ship S1 now and plan tsconfig-`paths` resolution next.

## Not measured

The latent `paths` yield is a projection only. The repo-wide unowned population is not separated. React's rendering
contract is an assumption, backed by the existing identity precedent. `--features mcp` and the Node gate were not run
at base.
