# S2 R4: re-dispatch addendum (read this first, then `repair-r4-brief.md`)

- **The fetch is done.** The controller fetched every needed commit into this clone. The repaired PKG proto is `origin/proto/workspace-package-resolution` = `92c1d0bc` + **`03fa9c29`** (plan `fa3bcb02`). Use `git diff 4e592daa 03fa9c29 -- src tests mutants` in place of the `92c1d0bc` diff. Do not fetch again.
- Your earlier preflight and resume notes are in `~/prism-evidence/s2/repair-r4/{PREFLIGHT.json,HANDOFF.md}`.
- **PKG now returns a three-way result:** `Bound(module)` / `ProvenUnresolved` / `Unsupported(reason)`. See the PKG SPEC result-type contract and `R1-REPORT.md` in the PKG packet (`docs/superpowers/plans/2026-10-04-workspace-package-resolution/` on `fa3bcb02`). For S2-O9:
  - **only `ProvenUnresolved`** makes a writer import `out_of_model_unresolvable`;
  - **`Unsupported` fails closed** (scoped `unavailable` refusal);
  - this replaces the brief's "prism's resolver cannot bind" wording.
- The "TS resolves, prism doesn't" gap table is still required. Report it as `Unsupported`-where-TS-binds on X, installed X, R and T writer imports, by reason, and list every such writer that revokes a gain row.
- PKG spec review round 2 is running in parallel. If it changes PKG, the controller will hand you a delta; build on `03fa9c29` now.
- The patch base for `R4-src.patch` is `61641bdb` + PKG `03fa9c29`; also produce an S2-only patch.
