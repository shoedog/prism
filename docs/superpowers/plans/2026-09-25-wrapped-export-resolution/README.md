# Wrapped-export resolution: planning packet (2026-09-25)

**Base:** `origin/main` `12ca6e8e`. Planning only; nothing under `src/`, `tests/`, `eval/` or `Cargo.*` changed.

| File | Purpose |
|---|---|
| `SPEC.md` | the S1 design. §0 lists the owner decisions |
| `PLANNING-PROBES.md` | mechanism (M1–M12) and probes (P0–P15), each with the command, the pre-run expectation, the result, and the output path |
| `IMPLEMENTOR.md` / `REVIEWER.md` | dispatch briefs |
| `REVIEW-r1-fold.md` | sol round-1 findings → disposition → location |
| `probes/` | census, projection, row-diff and audit tools; the control generator; expectation records; the expected Excalidraw row-diff |
| `prototype/wrapped-export-prototype{,-r2}.diff.txt` | the throwaway feasibility prototypes (never built into this branch); r2 is authoritative |

The evidence root is `~/prism-evidence/wrapped-export/planning/` (`MANIFEST.sha256` `951e9dbc…`, r2).

## Key measured facts

- **Mechanism (M1–M4).** The declarator-export arm skips `forwardRef(fn)` and `memo(fn)`. This was deliberate (P4
  "F4"): a fact is a *name*, and R4c binds it by name lookup. The inner arrow already has a callable identity, because
  Pattern 3 names it after the declarator. Same-file JSX, `export { X }` and `export default X` already bind to that
  identity as Exact (C03–C05).
- **Design (r2).** Record the inner function's exact span (`SpannedLocal`), and bind Exact only when all of these hold:
  - the target is the unique function with that span;
  - the site is a **JSX element** (a new `CallSite.jsx_element` flag; `X(props)` drops as `WrappedExportNonJsx`);
  - every `"react"` default or namespace binding in the producer file passes a closed **occurrence custody** (no
    writes, deletes, escapes, aliases, `eval`/`with`, or other handles).
- **Yield, measured on the r2 prototype (P18).** 107 new Exact edges on Excalidraw and 4 on the private React corpus
  (F). 0 on ruff `playground` and TypeScript `src`, whose dumps are byte-identical. All 111 edges are audited
  correct. The row-diffs are byte-identical to r1's. DFG edges are identical, Tier-A is 159/159, and the full suite is
  4,559 / 0 / 1.
- **Named imports only (D6 option b)** would give 19 edges on Excalidraw and 0 on F, with no precision gain (M15).
- **PR #324 ledger.** 72 of the 80 E2 rows are recovered. The other 8 import through `@excalidraw/…` package
  specifiers.
- **Hazards.** Shape-only admission would bind 27 declarators to the wrong callable. All 52 negative and positive
  controls behave as the SPEC states, including sol's W1 (member write), W2 (parse-recovered import), W3 (one-line
  comparator) and W5 (star-barrel span conflict).
- **Pre-existing WRONG (M8, C06).** `const f = <ternary>; export { f }` plus a nested `function f` gives a false
  Exact on `main`. It is queued as S1b.
- **The bigger lever (P4 latent, projected).** Non-relative imports hold about 3,174 (Excalidraw) and 2,773 (F)
  resolvable-terminal drops.

## Recommendation

Implement **S1 (r2)** with D6 = (a). The budget is re-capped to **src 420 / tests 650 / combined 1,070**, measured
from the 431-line r2 prototype with a demonstrable reduction to about 380. The review cap is 2 rounds. Bump both
caches (CPG 98, nav 54). Acceptance is unchanged: exactly 107 / 4 / 0 / 0 changed rows, all audited.

## Owner decisions

D1–D5 were answered on 2026-09-25, and all recommendations were accepted. **D6 is new (r2)**: default and namespace
member forms under custody, option (a, recommended), or named imports only, option (b). See `REVIEW-r1-fold.md` for
how sol's round-1 findings were folded, and where the planner disagrees with sol.

## Not measured

- The latent `paths` yield is a projection only.
- The repo-wide unowned population is not separated.
- React's rendering contract is an assumption, backed by the existing identity precedent.
- Cross-module and host-global mutation of the React object is excluded by the stated analysis model (SPEC §3.2),
  not measured.
- `--features mcp` and the Node gate were not run.
