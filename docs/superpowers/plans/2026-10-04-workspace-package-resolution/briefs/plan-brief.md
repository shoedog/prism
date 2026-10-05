# Lane PKG: workspace package-entry resolution for JS/TS (you are the planner, gpt-6.1-sol)

## Where you work
- **Clone:** `/Users/wesleyjinks/code/prism-pkgres`, branch `plan/workspace-package-resolution` off main `4e592daa`.
- **Rules:** no git writes (the controller commits). Never open the private corpus F; the controller runs it from your controller script. Keep disk use lean and delete build dirs you don't need.
- **Packet:** `docs/superpowers/plans/2026-10-04-workspace-package-resolution/`. Follow the shape of the lane-P packet, `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/`: SPEC with §0 decisions, IMPLEMENTOR, MEASUREMENTS, PROBES, OQ, CONTROLLER script, BUILD-MANIFEST, VERIFICATION, and a mutant registry `mutants/lane-pkg-resolution.json`.

## Why this lane exists (the goal it serves)
Owner decision S2-O8 (2026-10-04). Lane S2 (JS/TS import-qualifier Exact edges; packet on `origin/plan/s2-import-qualifiers` `aa55a532`, candidate `origin/proto/s2-import-qualifiers` `61641bdb`) is parked. Its sound fail-closed refusal rule loses all of its X +132 gain because prism cannot resolve some writer imports:
- **Workspace package entries.** `import … from '@excalidraw/excalidraw'` in `examples/with-nextjs/src/excalidrawWrapper.tsx`, and `MIME_TYPES` in a `utils.ts` route. Prism resolves bare specifiers **only through tsconfig `paths`** (lane P, `src/js_paths.rs`). It does not read `package.json` `exports`/`main`/`module`/`types`, workspace declarations, or `node_modules` symlinks.
- **Schemes:** `node:url` and `virtual:pwa-register`.

The evidence is in `~/prism-evidence/s2/repair-r3b/REPORT.md` and `unavailable-i-X.json`.

Goal-level results:
1. Every bare specifier that resolves to an **in-repo workspace package** gets a proven module binding under TS/Node semantics for the writer's own project. That makes S2's refusal joins precise ("proved absent", or scoped to the package's actual export graph instead of the whole package directory).
2. Any additional **correct Exact** call edges this unlocks on its own, as P1 and P2 did.
3. **Zero false Exact edges.** Exact remains a static-binding grade (CLAUDE.md), positive-proof only. Anything ambiguous stays unresolved.

## Do (measure first)
1. **Census.** Write the census before any design. Across X (`~/prism-evidence/inputs/excalidraw-0642e72c/source`), installed X (`…-installed/source`), R (`~/code/bench-repos/ruff/playground`) and T (`~/code/bench-repos/TypeScript/src`), count bare specifiers by outcome under main:
   - resolved via paths;
   - workspace package (in repo, not resolved);
   - installed external package;
   - `node:` builtin;
   - other scheme (`virtual:`, …);
   - unresolved.

   Use the TS 5.9.3 resolver as the oracle (`~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`), with each writer's own tsconfig and `moduleResolution`. Report the callable sites whose base result drops because of an unresolved workspace specifier: the potential direct Exact yield.
2. **Design.**
   - Workspace discovery: `package.json` `workspaces`, pnpm-workspace, and lerna if present, or symlinked `node_modules` entries that canonicalise into indexed source.
   - Package-entry resolution under the writer's `moduleResolution` (node10, node16/nodenext, bundler): `exports` conditions (`types`, `import`, `require`, `default`, the order TS uses), subpath exports and patterns, then `types`/`typings`/`main`/`module`, then the `index` fallback.
   - `node:` builtins: classify as external-builtin, which is proved not to be repo code.
   - `virtual:` and other loader schemes: measure them and leave them as an explicit OQ for the owner. Do not decide them.
   - Integration: reuse the lane-P resolver ladder and the S1b/P2 export-hop machinery. Interaction with paths precedence must match TS.
   - Cache: bump the cache epoch if facts change.
3. **Prototype it** (src/tests uncommitted, as before).
   - Measure X, installed X, R and T against main, with every changed row CORRECT and ownership agreeing under the oracle, and zero lost base edges.
   - Write `CONTROLLER-pkg.sh BASE_BIN HEAD_BIN … TS_JS`, which reports changed-row correctness on F (aggregates only), like `CONTROLLER-s2.sh`.
4. **Report the S2 unblock effect.** Rebuild S2's R3b-i candidate (`61641bdb`) on top of your prototype in a scratch copy, and measure S2's X with the package-resolved writer joins. Report how much of the +132 returns and which residual joins still revoke, by category. Do not commit or adopt S2 changes; this is a measurement only.
5. **Gates (tiered):**
   - one `cargo nextest run --features mcp`;
   - fmt and clippy;
   - the advisory scoped mutgate;
   - the Tier-A matrix (skip quick);
   - S1b-4 controls byte-identical;
   - lane-P public rows unchanged except for documented CORRECT additions.

## Standing rules
- Measure X, installed X and F yield before adopting any refusal cut.
- Positive proof only.
- Any new accepted cost, or a change to the model, goes to OQ for the owner. Do not decide it.
- Before each diagnostic probe, write your expectation and what would falsify it.

## Final message
- the census table;
- design decisions and OQs;
- the prototype yield table;
- the S2-unblock measurement;
- files split into src/tests and docs, with commit messages;
- the F command;
- the gates;
- what you did not verify.
