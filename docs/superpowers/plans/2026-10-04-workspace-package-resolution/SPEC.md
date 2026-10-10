> **PARKED 2026-10-04.** Branches, commits and how to resume are in [PARKED.md](PARKED.md): `plan/workspace-package-resolution` (this packet, PR #344) and `proto/workspace-package-resolution` (`03fa9c29`).

# Lane PKG — workspace package-entry resolution

## 0. Decisions and authority

Current repair authority is `/Users/wesleyjinks/prism-evidence/pkgres/planning/repair-r1-brief.md`. Source/test base is committed prototype `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318`; packet base is `c89bc5b74df05e1f8746792c0cfad2805336b0f9`; main comparison base is `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Compose those bases with R1-src.patch and R1-docs.patch as IMPLEMENTOR specifies. Workers never write Git, open private F, fetch/install, or adopt S2.

1. **Positive native proof.** Bind an indexed implementation only when TS 5.9.3 selects the same canonical module in the writer's actual project and usage mode. Workspace discovery or a missing build output supplies no source redirect. Zero wrong module bindings, false Exact callable edges and lost main edges are required.
2. **Writer authority.** Retain Lane-P ownership and caller-project export projection. Never reselect a barrel's config. `.mjs`/`.mts` select import and `.cjs`/`.cts` select require before package type. Node16/Next uses the nearest package scope, including retained outer ancestor package.json bytes; unreadable/symlinked/malformed outer authority declines. Bundler's import condition does not imply Node ESM implicit-extension rules. Unindexed `.mts`/`.cts` writers remain Unsupported.
3. **Native entry probing.** An explicit TS/declaration exports target probes its literal file. JS extensions use the TS replacement table. Literal export targets retain stars; only a selected pattern key substitutes captures. Legacy fields first try their exact TS/declaration spelling, then regular substitution, including a missing `.d.ts` to same-stem `.ts`. Legacy typings/types/main order and package.json `module` ignoring are retained. Node ESM dependencies with type:module do not extend extensionless fields; native root index.js substitution is supported. Bundler index fallback and canonical legacy link subpaths are supported. Root typesVersions fences subpath file probes before they run.
4. **Export search.** Source-order conditions, literal subpaths before patterns, longest-prefix then key-length pattern ordering, and declaration/opaque barriers remain. Known null/missing targets yield no result: later conditions or outer packages may win. Selecting a literal null subpath does not retry a lower pattern in that map. Top-level null uses legacy lookup. Modern self-name exports run independently of resolvePackageJsonExports=false; outside-root self-reference is Unsupported.
5. **Schemes and imports.** Paths run before colon classification. Supported colon paths can bind repository source in node10/modern modes. A matched but unproved paths target is Unsupported. Lexical `classify()` is not an absence certificate; it has no production classification consumer. Self-name and ambient/baseUrl authority also precede any proven URI absence. With all earlier authority discharged, unmatched `node:` is external-builtin and other colon schemes are ProvenUnresolved. `#` package imports are explicitly Unsupported and never enter unrelated ordinary package lookup. No virtual-loader policy is selected.
6. **Preservation and cache.** Existing ordinary Lane-P paths proofs/refusals, export callable/wrapper/write/shadow/ambiguity/depth guards and source-only authority remain. Package bytes, canonical link identity, occupancy/probes, config bytes and outer package-scope bytes bind cache topology. CPG/nav epochs are 108/64. S2 scratch is neither rebuilt nor adopted by R1.
7. **Coverage, not whole-feature completion.** Indexed `.ts`/`.tsx` source winners are admitted; a package `.tsx` winner without JSX authority declines. JS secondary/ancestor priority, @types authority, arrays, versioned conditions, typesVersions, directory-valued entries, broad modern paths/config owners and unindexed entries remain explicitly Unsupported. These gaps cannot justify S2-O9 exclusion. OQ and the differential table enumerate them.

## 1. Result-type contract for S2

`Resolution` has exactly three outcomes:

- `Bound(module)`: positive canonical indexed-source proof. The writer owner is returned separately by `resolve_import` and retained with production module facts.
- `ProvenUnresolved`: supported TS semantics prove no module result. Examples include an absent explicit dist export after all eligible ancestor authority is discharged, or an unmatched loader URI after earlier paths/self/ambient authority is discharged.
- `Unsupported(reason)`: TS may resolve, but Prism lacks authority or implementation coverage. This includes known declaration/external/unindexed winners, opaque metadata, budget/read/ownership failures, package imports, and unsupported options/features.

Only ProvenUnresolved may be out of model under S2-O9. Unsupported must fail closed and retain refusal identity; it must never become Unavailable(empty). A declaration winner resolves natively but supplies no callable implementation and is not a native-absence certificate. No S2 consumer has been adopted in this patch.

Production `Resolver::resolve` admits only Bound; `Resolver::resolution` and public `js_packages::resolve_import` preserve the distinction for future consumers and the differential gate. A failed paths proof is a barrier, not permission to guess a package winner.

## 2. Discovery and snapshot contract

Physical ancestor node_modules links and modern self-reference supply lookup authority. npm/Yarn workspace declarations, pnpm/lerna inventory and matching package names alone never bind an uninstalled package. Captured pnpm nested links qualify independently. Broken/outside/ambiguous links and ordinary external winners remain Unsupported. Formal pnpm/lerna declaration expansion is not certified.

Metadata comes from the captured snapshot; inherited absence probes can inspect filesystem metadata. Resolution is not claimed to be entirely free of I/O. Outer package scope is captured nearest-first with bytes, absence or opaque status, and participates in the same cache fingerprint.

## 3. Oracle and acceptance

Pinned TypeScript 5.9.3 typescript.js SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`: optional resolution/self/URI order 45285–45335; exact package-field and ordinary extension tables 45424–45530; legacy field/index semantics 45745–45813; exports 45851–46175; ancestor priority 46298–46407. Local oracle source and real ProjectService ownership are the authority.

Generate and run probes/differential-generate.py and differential-run.py as IMPLEMENTOR specifies. Every Bound must equal the native canonical module and owner. Every Exact callable must equal a native declaration's file/name/start/end lines. Every ProvenUnresolved must have no native module. Report Unsupported where TS binds by feature and reason. Read the complete rejected population before any repair retry.

X, installed-X, R and T retain complete site denominators and main edge/module preservation. STOP on any public wrong binding; choose nothing. Tier-A quick/full corpus, private F, independent acceptance and S2 adoption remain outside this repair's verified scope.
