# Census — measured before design

Base 4e592daa; fresh offline base executable and import-facts helper bound by planning/base-binding.json. Oracle TS 5.9.3, SHA256 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675. Each writer uses its actual native ProjectService owner and effective moduleResolution. No F access.

| Corpus | Paths proof | Workspace, no main proof | Installed external | node: | Other scheme | Unresolved | Writer/spec pairs | Occurrences |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| X | 660 | 327 | 0 | 1 | 1 | 516 | 1505 | 1883 |
| installed-X | 660 | 327 | 456 | 1 | 1 | 60 | 1505 | 1883 |
| R | 0 | 15 | 0 | 0 | 0 | 80 | 95 | 101 |
| T | 0 | 0 | 0 | 0 | 0 | 41 | 41 | 55 |

Outcome is a captured main module proof, not a claim that all other native imports fail. Workspace counts name an in-repo package candidate (package.json inventory), not positive package-entry authority. Paths has precedence when main already captured a proof; namespace/type/export-only pairs may lack such a proof even when native paths resolve them. Installation is actual node_modules occupancy, not dependency declarations. Each pair counts once; census.json also retains per-category occurrence counts and every writer/project/target.

| Corpus | Workspace pairs native indexed | Workspace pairs native unresolved | Dropped direct sites associated with workspace | Native indexed potential sites |
|---|---:|---:|---:|---:|
| X | 309 | 18 | 1 | 0 |
| installed-X | 311 | 16 | 1 | 1 |
| R | 0 | 15 | 33 | 0 |
| T | 0 | 0 | 0 | 0 |

Potential sites are individual main non-Exact call/JSX rows associated with a captured import binding; this is not a certified Exact-yield claim. Native indexed module resolution alone does not prove a callable terminal or a supported export hop.

X: Next.js writer examples/with-nextjs/src/excalidrawWrapper.tsx uses its own Node10 project and cannot resolve @excalidraw/excalidraw. Bundler writer examples/with-script-in-browser/utils.ts also cannot resolve it. Installed-X: the Next.js pair resolves canonical packages/excalidraw/index.tsx; bundler utils remains unresolved. The two scheme pairs are node:url and virtual:pwa-register. Workspace inventory and actual writer results refute any blanket discovery-to-binding assumption.

Receipts: /Users/wesleyjinks/prism-evidence/pkgres/planning/{X,installed-X,R,T}/census.json; complete base rows and facts alongside. The census launch had two inadmissible setup failures (extern ambiguity and hash schema), corrected before results; see PROBE-LOG.md.
