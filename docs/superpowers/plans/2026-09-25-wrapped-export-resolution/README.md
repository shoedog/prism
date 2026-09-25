# Wrapped-export resolution: planning packet (2026-09-25, r3, Branch P)

**Base:** `origin/main` `12ca6e8e`. Planning only; nothing under `src/`, `tests/`, `eval/` or `Cargo.*` changes.
The `CLAUDE.md` paragraph lands with the implementation (SPEC §3.2.1).

| File | Purpose |
|---|---|
| `SPEC.md` | the normative S1 design (r3, Branch P); §0 holds the owner decisions D1–D9 |
| `PLANNING-PROBES.md` | mechanism (M1–M16) and probes (P0–P29), each with the command, the pre-run expectation, the result, and the output path |
| `REPLAN-fable.md` | the post-r2 diagnosis and the P / S / hybrid fork; the owner chose **P** |
| `IMPLEMENTOR.md` / `REVIEWER.md` | dispatch briefs (the reviewer brief fixes the Branch-P model for sol round 3) |
| `REVIEW-r1-fold.md` | sol round 1 findings → disposition (r2); round 2 → SPEC §11 |
| `probes/` | census, projection, row-diff and audit tools; the 56-scenario control generator; expectation records; the expected Excalidraw row-diff; control summaries |
| `prototype/wrapped-export-prototype-P.diff.txt` | the Branch-P feasibility prototype (authoritative; never built into this branch). The v2 and r2 diffs are kept as history |

The evidence root is `~/prism-evidence/wrapped-export/planning/` (see PLANNING-PROBES for the `MANIFEST.sha256`).

## Key measured facts

- **Design.** Record the inner function's exact span (`SpannedLocal`). Bind **Exact** only when all three hold:
  - the callee is a unique, value-typed, unwritten, uncompeted **ESM** import of `"react"` (`forwardRef` or `memo`);
  - the target is the unique function with that span;
  - the site is a **JSX element** (a new `CallSite.jsx_element` flag; `X(props)` drops as `WrappedExportNonJsx`).
- **Model (owner, Branch P).** Exact is a static-binding grade. Runtime mutation or re-acquisition of the React object
  is out of model, exactly as for every `import_member` edge on `main` (replan Q1–Q6). It is pinned by MB1–MB3
  asserting Exact.
- **Yield, measured on the Branch-P prototype (P26):**
  - 107 new Exact edges on Excalidraw and 4 on the private React corpus (F), with row-diffs byte-identical to r1 and
    r2;
  - 0 on ruff `playground` and TypeScript `src`;
  - audits 107/107 and 4/4;
  - Tier-A 159/159 (P29), full suite 4,559 / 0 / 1 (P25).
- **Controls (P28).** All 56 behave as pre-registered. The 18 r2-custody controls, plus MB2 and MB3, flip to Exact
  as the model states. Sol's P2 fixture drops as `callee_provenance`, and a P2-removed mutant flips it (P27).
  Destructured `require("react")` drops as `callee_not_admitted`.
- **Budget.** The prototype is 325 src honest lines, against caps of 350 / 600 / 950 (owner D7).
- **Pre-existing WRONG (C06)** is queued as S1b. The tsconfig `paths` lane (projected about 3,174 and 2,773 drops) is
  next.

## Owner decisions

D1–D9 are all decided (SPEC §0). D6 is superseded by Branch P. There are no pending owner choices.
Sol round 3 is the owner-approved final spec round.

## Not measured

- The latent `paths` yield is a projection only.
- React's rendering contract is an assumption, backed by the existing identity precedent.
- Runtime mutation is out of model by decision; it is not measured as a hazard.
- `--features mcp`, the Node gate and `tier-a --quick` were not run.
