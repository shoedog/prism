# Wrapped-export resolution: planning packet (2026-09-25, r3 Branch P, round-3 fold, ready for implementation)

**Base:** `origin/main` `12ca6e8e`. Planning only; nothing under `src/`, `tests/`, `eval/` or `Cargo.*` changes.
The `CLAUDE.md` paragraph lands with the implementation (SPEC §3.2.1).

| File | Purpose |
|---|---|
| `SPEC.md` | the normative S1 design (r3, Branch P, with the round-3 fold); §0 holds the owner decisions D1–D11; §12 records the S1b scope |
| `PLANNING-PROBES.md` | mechanism (M1–M16) and probes (P0–P34), each with the command, the pre-run expectation, the result, and the output path |
| `REPLAN-fable.md` | the post-r2 diagnosis and the P / S / hybrid fork; the owner chose **P** |
| `IMPLEMENTOR.md` / `REVIEWER.md` | dispatch briefs (the reviewer brief fixes the Branch-P model, as narrowed to the R4c route, for sol's implementation review) |
| `REVIEW-r1-fold.md` / `REVIEW-r3-fold.md` | sol round 1 and round 3 findings → disposition; round 2 → SPEC §11 |
| `probes/` | census, projection, row-diff and audit tools; the 67-scenario control generator; expectation records; the expected Excalidraw row-diff; control summaries (`PP3-controls-proto.txt` is the reference) |
| `prototype/wrapped-export-prototype-P3.diff.txt` | the Branch-P prototype with the round-3 fold (authoritative; never built into this branch). The v2, r2 and P diffs are kept as history |

The evidence root is `~/prism-evidence/wrapped-export/planning/` (see PLANNING-PROBES for the `MANIFEST.sha256`).

## Key measured facts

- **Design.** Record the inner function's exact span (`SpannedLocal`). On the new R4c `import_member` route, bind
  **Exact** only when all three hold:
  - the callee is a unique, value-typed, unwritten, module-scope-uncompeted **ESM** import of `"react"`
    (`forwardRef` or `memo`). "Uncompeted" includes `var` hoisted from top-level blocks (sol r3 W2);
  - the target is the unique function with that span;
  - the site is a **JSX element** (`CallSite.jsx_element`; `X(props)` drops as `WrappedExportNonJsx`).
- **Model (owner, Branch P).** Exact is a static-binding grade. Runtime mutation of the React object is out of model,
  as for every `import_member` edge on `main`. It is pinned by MB1–MB3 asserting Exact.
- **Scope, narrowed at round 3 (owner D10).** The gates cover the R4c route only. The pre-existing R3 namespace-decoy
  and R4 producer-local behavior on wrapped targets is **S1b** scope, and sol's inputs are recorded as its first RED
  cases (C62, C63; unchanged by S1).
- **Yield, measured on the prototype with the round-3 fold (P33):**
  - 107 new Exact edges on Excalidraw and 4 on the private React corpus (F), with row-diffs byte-identical to every
    earlier revision;
  - 0 on ruff `playground` and TypeScript `src`;
  - audits 107/107 and 4/4;
  - Tier-A 159/159, full suite 4,559 / 0 / 1 (P34).
- **Controls (P31).** All 67 behave as pre-registered.
- **Budget (P30).** The prototype is 328 src honest lines, against caps of 350 / 600 / 950. The first W2 fold measured
  357 as a separate walk; it was replaced by one shared walk, not compressed.
- **Next slices.** S1b (span-verify all JS/TS export routes, SPEC §12), then the tsconfig `paths` lane.

## Owner decisions

D1–D11 are all decided (SPEC §0). D6 is superseded by Branch P, and D10–D11 are the round-3 at-cap dispositions.
**Implementation starts directly**, and sol reviews the implementation.

## Not measured

- The latent `paths` yield is a projection only.
- React's rendering contract is an assumption, backed by the existing identity precedent.
- Runtime mutation is out of model by decision; it is not measured as a hazard.
- `--features mcp`, the Node gate and `tier-a --quick` were not run.
