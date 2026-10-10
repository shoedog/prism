# Lane PKG spec review, round 1 (Opus): workspace package-entry resolution

**Reviewed:** packet `review-pkg` = `c89bc5b7` (PR #344): SPEC, IMPLEMENTOR, MEASUREMENTS, CENSUS, PROBES, OQ, VERIFICATION, HANDOFF, FILES and probes/controls.py. Prototype `origin/proto/workspace-package-resolution` `92c1d0bc`. Its source hashes match BUILD-MANIFEST.json `inputs` for js_packages.rs, js_paths.rs, js_paths_snapshot.rs and js_packages_test.rs.

**Goal (from the brief):** bind JS/TS bare specifiers to in-repo workspace package entries under TS 5.9.3 semantics for the writer's own project, with zero false module bindings, zero false Exact edges and zero lost base edges. Because S2-O9 treats a writer import that neither prism nor TS can resolve as out of model, a false "unresolved" matters as much as a false binding.

**Oracle:** pinned TS 5.9.3 `typescript.js`, SHA256 `3ae902c9…7675` (verified). For each fixture the harness builds a `ts.createProgram` from its tsconfig and records `program.getResolvedModuleFromModuleSpecifier`, `getModeForUsageLocation`, whether the target is in the program, and pre-emit diagnostic codes. Prism was checked two ways: a prototype release build of the packet's own `probes/dump-facts.rs`, which emits `js_ts_path_modules`, and `prism nav --no-cache callees --symbol run`, which gives the call target and its score.

**Same-environment base control:** `origin/main` (`4e592daa`) was built from the same `dump-facts.rs`. It produced **no** module binding for any fixture in F1–F7. Every WRONG below is therefore introduced by the PKG rung.

Every fixture uses the same writer, `import {real as picked} from '<spec>'; export function run(){picked();}`. Each package file defines `export function real(){return N;}`, and `node_modules/@ws/lib -> ../../packages/lib` is a relative symlink unless stated otherwise. A generator is in the appendix.

---

## WRONG findings

### F1. Exports string targets with a TS extension probe sibling extensions that TS does not try (WRONG, MATERIAL)

**Evidence:** `src/js_packages.rs:136-152` (`Search::file`), reached from `target()` at :239 with `exact=true`. For a `.ts` or `.js` suffix the candidate list is always `[.ts, .tsx, .d.ts]`, and for `.tsx`/`.jsx` it is `[.tsx, .ts, .d.ts]`, whether or not `exact` is set. TS 5.9.3 handles an exports target in `loadFileNameFromPackageJsonField` (typescript.js:45450-45460). A target that already has a TS-implementation or declaration extension gets **only** `tryFile(candidate)`, and a miss returns `undefined`, so evaluation moves to the next condition. Extension replacement (`.js` → `.ts`) happens only for non-TS extensions, through `loadModuleFromFileNoImplicitExtensions`.

**Reproduced against the oracle:**

| Fixture | TS 5.9.3 | Prototype |
|---|---|---|
| C1: bundler/esnext, `exports: {"types":"./a.ts","default":"./b.ts"}`, files `a.tsx` and `b.ts` | `packages/lib/b.ts` | `packages/lib/a.tsx`, Exact (score 1.0) |
| C11: bundler/esnext, `exports: "./index.tsx"`, file `index.ts` only | unresolved (TS2307) | `packages/lib/index.ts`, Exact |

**Fix (recommended):** when `exact` is set, a target whose suffix is `.ts`, `.tsx`, `.d.ts`, `.mts`, `.cts`, `.d.mts` or `.d.cts` probes only itself. Keep the current substitution for `.js` and `.jsx`, which is what TS does through NoImplicitExtensions. Port the exact `loadFileNameFromPackageJsonField` and `tryAddingExtensions` tables (typescript.js:45450-45500) rather than re-deriving them.

**Alternative:** in exact mode, return Blocked when the literal target is absent and any sibling-extension candidate exists. This is safe but loses some legitimate non-bindings.

**Self-critique:** the finding does not apply when the package's declared target file exists. Every native control has both `index.ts` and `other.ts`, which is why the 58 controls never hit this. The trigger is a misconfigured package (it points at the wrong extension), but such a package is still a provable false Exact.

### F2. A legacy `types`/`typings`/`main` value ending in `.d.ts` loses the `.d.ts` → `.ts` substitution and falls to `index` (WRONG, MATERIAL)

**Evidence:** `src/js_packages.rs:130-146`. `.d.ts` is matched first and falls into `Some(_) => vec![p]`. When `x.d.ts` is absent, `legacy()` at :209-218 continues to `index`. TS handles the legacy `packageFile` in `loadNodeModuleFromDirectoryWorker`'s loader (typescript.js:45745-45770): `loadFileNameFromPackageJsonField` misses, then `nodeLoadModuleByRelativeName` → `loadModuleFromFile` → NoImplicitExtensions strips `.d.ts` and tries `x.ts`, then `x.tsx`, then `x.d.ts`.

**Reproduced:** C2 is node10/commonjs with `{"types":"src/index.d.ts"}`, files `src/index.ts` (1) and `index.ts` (2). TS resolves `packages/lib/src/index.ts`. The prototype binds `packages/lib/index.ts`, Exact. With no root `index.ts`, the same input becomes a missed binding instead.

**Fix:** in legacy (non-exact) mode, map `.d.ts`, `.d.mts` and `.d.cts` field values through the same stem-replacement table as `.ts`. This means `[stem.ts, stem.tsx, stem.d.ts]`, with the declaration still Blocked.

**Alternative:** return Blocked whenever a declared `.d.ts` field value is absent and `stem.ts` or `stem.tsx` is present. This is conservative and cheap.

**Self-critique:** this needs a missing declared `.d.ts` and an existing same-stem `.ts`, which is common in unbuilt monorepos that point `types` at a source-adjacent path. It does not apply when `types` points into an absent `dist/` that has no `.ts` sibling. That is the existing control `missing-types-index`, and it agrees with TS.

### F3. The usage mode ignores per-file module format: `.mjs`/`.cjs` writers and barrels get the wrong condition (WRONG, MATERIAL)

**Evidence:** `src/js_packages.rs:401-427`. In node16/nodenext, `esm` comes only from the nearest `package.json` `type`. In bundler it comes only from `compilerOptions.module`. TS gives the extension precedence. `getImpliedNodeFormatForFileWorker` (typescript.js:126804-126808) forces `.mjs` to ESM and `.cjs` to CJS. `getImpliedNodeFormatForEmitWorker` (typescript.js:129721-129733) returns that format for `.cjs`/`.mjs` even when `module` is not a node module kind, and that format then drives `getEmitSyntaxForUsageLocationWorker` (typescript.js:126590-126606). Prism indexes `.mjs`/`.cjs` as JavaScript (`languages/mod.rs:27`), and `includes()` admits them under `allowJs`. They are therefore eligible writers and P2 barrels.

**Reproduced** (lib `exports: {"import":"./other.ts","require":"./index.ts"}`, `allowJs: true`):

| Fixture | TS mode → target | Prototype |
|---|---|---|
| C5: nodenext, CJS root package, writer `app.mjs` | esm → `other.ts` | `index.ts`, Exact |
| C6: bundler/esnext, writer `app.cjs` | cjs → `index.ts` | `other.ts`, Exact |
| C6b: nodenext, `"type":"module"` root, writer `app.cjs` | cjs → `index.ts` | `other.ts`, Exact |

**Fix:** compute the mode with TS's precedence. `.mjs` and `.mts` are ESM and `.cjs` and `.cts` are CJS. Otherwise node16/nodenext uses package scope, and bundler uses `module`. This applies to both `resolve(file=writer)` and `hop(from=barrel)`, since `file` is already per-file.

**Alternative:** decline (return None) for `.mjs`/`.cjs` writers and barrels. This costs nothing in the measured corpora.

**Self-critique:** this only matters when import and require conditions select different indexed `.ts` sources. A `types` condition listed first masks the difference, and that is the common layout. Config files such as `*.config.mjs` importing workspace packages are the realistic trigger.

### F4. Node16 ESM with a `"type":"module"` dependency and an extensionless `types`/`main` gets extension probing that TS disables (WRONG, MATERIAL)

**Evidence:** `src/js_packages.rs:198-218`. `legacy()` calls `file(q, false)`, which always appends `.ts`/`.tsx`/`.d.ts`. TS's legacy loader (typescript.js:45763-45770) clears `EsmMode` only when the **dependency's** `type` is not `"module"`. With EsmMode kept, `loadModuleFromFile` (typescript.js:45424-45433) does not add extensions. Next, `loadModuleFromSpecificNodeModulesDirectory` (typescript.js:46383-46385) applies the ESM `index.js` fallback.

**Reproduced:** C4 is nodenext, root `"type":"module"`, lib `{"type":"module","types":"src/index"}`, files `src/index.ts` (1) and `index.ts` (2). TS resolves `packages/lib/index.ts` through the `index.js` fallback. The prototype binds `packages/lib/src/index.ts`, Exact.

**Fix:** pass the dependency's `type` into `legacy()`. When the writer is ESM and the dependency is `type: module`, an extensionless field value is not extended. Then either implement the `!rest && exports==null|undefined` `index.js` fallback, or return Blocked.

**Alternative:** in node16/nodenext ESM, decline legacy resolution unless the field value has an explicit extension.

**Self-critique:** this needs node16/nodenext, an ESM writer, a dependency with `type: module` and no exports, and an extensionless field. That combination is uncommon but real in older ESM packages.

### F5. In node16/nodenext, the writer's package scope stops at the prism root; an outer `package.json` changes the TS mode (WRONG, MATERIAL)

**Evidence:** `src/js_packages.rs:385-400` and `:415-424`. The `enclosing` walk ends at `d == ""` (the repo root). With no in-root `package.json` the writer is treated as CJS. TS's `getPackageScopeForPath` walks every ancestor of the file.

**Reproduced:** C22 places `{"type":"module"}` in the **parent** of the prism root, with nodenext and the import/require split from F3. TS reports esm → `other.ts`. The prototype binds `index.ts`, Exact.

**Fix:** when no in-root `package.json` is found, decline node16/nodenext unless every ancestor `package.json` outside the root is proven absent. Reuse the snapshot's `external_modules` pattern, which already records outside-root `node_modules` occupancy, so the cache topology covers it.

**Alternative:** decline node16/nodenext whenever the writer has no in-root package scope.

**Self-critique:** this does not apply when prism runs at a repository root. It applies when `--repo` names a sub-package of a larger checkout, a plausible use for monorepo sub-projects.

### F6. The self-name rung is skipped when `resolvePackageJsonExports: false`, but TS still runs it (WRONG, MATERIAL with low prevalence)

**Evidence:** `src/js_packages.rs:440-463`. Self-reference is gated on `exports_enabled`. TS `tryResolve` (typescript.js:45309-45313) runs `loadModuleFromSelfNameReference` whenever the `SelfName` feature is set, independent of `resolvePackageJsonExports`. That function then calls `loadModuleFromExports` directly (typescript.js:45827-45850). The prototype falls through to `node_modules` legacy resolution instead.

**Reproduced:** C25 is bundler/esnext with `resolvePackageJsonExports:false`, writer `packages/lib/src/app.ts`, and lib `{"name":"@ws/lib","exports":"./other.ts","types":"index.ts"}`. TS resolves `other.ts`. The prototype binds `index.ts`, Exact.

**Fix:** run the self-name match whenever the mode is modern. If it matches, resolve through exports or decline. Never fall through to `node_modules` while a self-name match is pending.

**Self-critique:** this is a rare option, but it is a provable false Exact and the fix is one condition.

### F7. "`node:` cannot name repository source" is false under TS: `paths` maps colon specifiers before the scheme skip (WRONG at the contract level, MATERIAL to S2)

**Evidence:** SPEC §0.5 and `classify()` at `src/js_packages.rs:26-36` say a `node:` specifier is ExternalBuiltin unconditionally. The S2 scratch overlay (`planning/s2-source/src/js_paths.rs:508-510`) turns that classification into `RefusalModule::Unavailable(∅)`, which means "proved absent". TS `tryResolve` (typescript.js:45305-45320) calls `tryLoadModuleUsingOptionalResolutionSettings`, which handles paths and baseUrl, **before** the `moduleName.includes(":")` skip.

**Reproduced:** C28 maps `paths {"node:url":["./shim/url.ts"],"virtual:*":["./shim/*.ts"]}` under both node10 and bundler. TS binds `node:url` → `shim/url.ts` and `virtual:pwa` → `shim/pwa.ts`. The prototype has no binding because lane-P `resolve` rejects `:` at `js_paths.rs:321-336`. Production emits no wrong edge today because `classify` has no production caller. Once S2 consumes it, the result is an empty refusal set where TS binds repo source.

**Fix:** classify `node:` as ExternalBuiltin only when (a) no `paths` key in the writer's selected config matches the specifier, (b) no `baseUrl` is set, and (c) no ambient-module pattern matches. Otherwise use Opaque (refuse). Correct SPEC §0.5 and the owner-brief wording. Because this narrows an owner-stated rule, route it to the owner rather than self-adjudicating.

**Self-critique:** tsconfig `paths` aliases for `node:*` are unusual, since bundler aliases usually live in vite/webpack configs that TS never sees. The rule still has to be conditional to be sound.

---

## SMELL findings

### F8. The API conflates "TS proves unresolved" with "prototype declined" (SMELL, MATERIAL)

**Evidence:** `js_packages::resolve -> Option<String>`. Every `Found::Blocked` and `Found::Absent` becomes `None`, and so does every guard (`baseUrl`, ambient, unsupported modes, paths-key match). The owner brief's goal 1 asks for refusal joins that are "proved absent" or scoped to the package's export graph. S2-O9 needs a positive "neither prism nor TS can resolve" fact. The prototype produces neither.

**Failure scenario (for the S2 consumer):** in the S2 scratch, an unresolved bare spec gets candidates from `qualifier_packages` matched by **name** (`s2-source/src/js_paths.rs:540-560`). A `#` subpath import (F9a) matches no package name, so the result is `Unavailable(∅)` and the refusal disappears while TS binds in-repo source.

**Fix:** return `Bound(path) | NativeUnresolved(proof) | Declined(reason)`. S2 may treat only `NativeUnresolved` as out of model. Everything else keeps today's package-directory refusal, or Opaque for specs that no package name can match.

**Self-critique:** this has no effect on main today, because main uses only the `Some` arm. It becomes material at S2 adoption.

### F9. The unresolved boundary is wider than OQ-3/4 states (SMELL, MATERIAL)

TS binds every row below to in-repo `.ts` source while the prototype returns `None`. Each row was reproduced against the oracle.

| # | Case (fixture) | Listed in OQ? | S2 risk under the scratch refusal model | Recommendation |
|---|---|---|---|---|
| a | `#` package imports, `"imports":{"#lib":"./packages/lib/index.ts"}` (C7) | **No** | **High.** The name-matched candidate set is empty, so a TS-resolved import looks proved absent | Implement `loadModuleFromImports` before S2, or have S2 treat `#…` as Opaque |
| b | Legacy (no-exports) subpath through a link, `@ws/lib/feature` in node10 (C12) or bundler (C30) | **No** | Low; name-matched package directory candidates cover it | Support soon. It is common in node10/Vite monorepos and blocks goal 1. Cause: the lexical sibling check at `:487-499` probes through the opaque link; probe `canonical_root/sub` instead |
| c | Bundler index fallback, package with no exports/types/main (C29) | Yes ("bundler index fallback") | Low | Support soon. Bundler has no EsmMode, so `esm=true` at `:215` wrongly blocks TS's index fallback |
| d | Node16 ESM `index.js` fallback (C9) | No | Low | Defer, or implement with F4 |
| e | Paths key matched but every substitution failed, so TS falls back to package lookup (C8) | Yes ("paths/baseUrl fallbacks") | Low (paths targets plus package name candidates) | Defer until lane P proves full substitution absence |
| f | Exports miss at an inner `node_modules`, so TS continues to the outer ancestor (C13) | No | Low (same name) | Defer |
| g | Top-level `exports: null` legacy fallback (C10) | Yes | Low | Defer |
| h | `node:`/`virtual:` through paths (C28) | No (see F7) | **High** through the builtin branch | Fix with F7 before S2 |
| i | Declaration winner (`types` → existing `.d.ts`) (C27) | By design | None for callables; TS resolves a `.d.ts`, so whether it is "in model" needs explicit S2 wording | Record in the S2 contract |

JS secondary, typesVersions, export arrays, versioned conditions and `.mts`/`.cts` are listed and safe to defer under the package-directory refusal fallback. Note that prism does not index `.mts`/`.cts` at all (`languages/mod.rs:24-40`), so that increment first needs language support.

**Self-critique:** the "low" ratings assume S2 keeps a name-matched package-directory fallback. If S2 instead treats every `None` as out of model, all rows become high.

### F10. The measurement cannot support a "zero false bindings" claim (SMELL, MATERIAL)

**Evidence:** the public corpora produce 0 changed call rows and +1 module proof. The 58 native controls are 29 implementer-chosen scenarios duplicated across TSX/JSX writers. Writer grammar does not affect resolution, so there are 29 independent cases. Every package in the controls has both `index.ts` and `other.ts`. None uses `.mjs`/`.cjs` writers, an outer package scope, a missing declared `.d.ts` with a same-stem `.ts`, a `type: module` dependency, or `resolvePackageJsonExports:false`. F1–F6 sit in exactly those gaps.

The packet's claims are true as stated ("bounded, not exhaustive TS conformance"). MEASUREMENTS/HANDOFF's "no WRONG remains demonstrated" is accurate only for that population.

**Fix:** add a generated differential matrix against the pinned oracle as the acceptance gate for this resolver. Axes: mode × writer extension × package type (writer and dependency) × field/target extension × target present/absent/sibling-only × link/self/nested link × exports shape. Every case compares the TS module with the prism module. This closes the class rather than enumerating it one review at a time.

**Tests:** the 8 package tests would fail on main (verified mechanism: no package rung on main). None pins F1–F7.

### F11. IMPLEMENTOR dispatch is stale and ambiguous (SMELL, MATERIAL)

- It still says source and tests are an uncommitted prototype in `/Users/wesleyjinks/code/prism-pkgres`.
- It does not name `origin/proto/workspace-package-resolution` `92c1d0bc` as the base for the next slice.
- It sends the next increment to JS secondary resolution, ahead of the WRONGs above and of the higher-yield boundary items F9a/b/c.

**Fix:** pin the base (`92c1d0bc`) and order the work as F1–F7 repairs with RED witnesses, then the differential matrix (F10), then F9a and F9h if S2 is the consumer, then F9b and F9c, then JS secondary.

### F12. `.tsx` entry without `jsx` (SMELL, IMMATERIAL to PKG; inherited)

**Reproduced:** C16 is bundler with `exports "./index.tsx"`, no `jsx`, and include `["app.ts"]`. TS emits 6142 and the file is not in the program, so the checker does not bind. The prototype binds Exact.

**Base control:** C16b on main shows the same behaviour through lane-P `paths` → `.tsx`. The gap is pre-existing and not introduced by PKG. Record it for a lane-P follow-up.

### F13. The ExternalBuiltin classification has no production consumer (SMELL, IMMATERIAL)

SPEC §0.5 says "Production exposes this classification". In practice it is a `pub fn` exercised only by a test. It is fine to keep, provided F7's conditional form is what eventually ships.

---

## Questions answered

1. **Soundness:** an input can bind to a module TS would not choose. F1–F6 are nine reproduced fixtures with false Exact edges: C1, C11, C2, C4, C5, C6, C6b, C22 and C25. Everything else checked agrees with TS 5.9.3. Checked: the condition set and order (getConditions 44504), source-order condition iteration with fallthrough on a missing leaf, longest-prefix-then-key-length pattern order, null and array targets (Blocked), invalid target and capture segments, typings → types → main with no main after a missing types, `module` ignored, node10 ignoring exports, the paths-first refusal, link canonicalisation, external shadows and the @types shadow.
2. **Boundary:** see F9. Items a and h must be handled before S2 lands, or S2 must treat them as Opaque. Items b and c are needed for goal 1's yield. The rest are safe to defer under the package-directory fallback.
3. **Discovery:** binding depends only on captured links and self-name, so it is independent of the npm/Yarn/pnpm/lerna declaration format. A pnpm-style nested link (`packages/app/node_modules/@ws/lib → ../../../lib`, C14) binds correctly. The unsupported pnpm/lerna parsing is IMMATERIAL to binding and matters only for S2's candidate scoping. Competing same-name packages resolve by link (TS-equal). Outside-root and broken links decline.
4. **Cache:** sound. Structurally, the repo_loader primes the same `resolve`/`hop` calls the CallGraph build makes, every probe joins topology, and package bytes and links are hashed. Empirically, the release binary was run with a persistent `--cache-dir` through seven edits. Each one flipped the callee correctly:
   - exports target edit;
   - link removal;
   - link re-add;
   - bundler `module` commonjs ↔ esnext;
   - writer package `type` flip under nodenext;
   - creating a `dist/index.d.ts` declaration winner (blocked directory);
   - removing that declaration winner.
5. **Admissibility:** the public populations are complete and the controls are native-witnessed, but the control population cannot discriminate the defect classes above (F10). Package tests fail on main.
6. **IMPLEMENTOR:** not unambiguous (F11).

## Verdict

**FIX.** The fixes are closed and enumerable:
- F1–F7, each with a bounded code change and a RED witness: the fixtures above, failing on `92c1d0bc` and agreeing with TS after the fix;
- F8, the API tri-state, before any S2 consumption;
- F10, a generated oracle differential as the acceptance gate, so the next round does not re-enumerate the same extension/mode class by hand;
- F11, refreshing IMPLEMENTOR.

F9a/F9h gate S2 adoption, not this prototype. No finding needs a restart. All the WRONGs sit in four mechanisms: extension probing, usage mode, feature gating and the scheme contract.

## Not checked

- Private F, the controller script's execution, and the S2 scratch rebuild.
- The full test suite, mutgate, Tier-A and S1b (gates not rerun; the reviewer built only release binaries).
- TS ProjectService ownership. The fixtures use a single root tsconfig through `createProgram`, not ProjectService.
- Case-insensitive aliasing and Unicode paths, `typeRoots`, `customConditions` edge values, numeric and duplicate package keys versus JS key order (refused, so only conservative), `moduleResolution` defaults when unset, and node18/node20 module kinds (declined).
- Windows paths and Yarn PnP.
- Performance and scan-budget behaviour on large `node_modules`.
- The mutant registry contents.
- I did not read the parallel sol review.

## Appendix: reproduction

Oracle harness (node), run as `TSJS=<typescript.js> node oracle.cjs ROOT WRITER`:

```js
const ts=require(process.env.TSJS),path=require('path'),fs=require('fs');
const root=fs.realpathSync(process.argv[2]),writer=path.join(root,process.argv[3]),cp=path.join(root,'tsconfig.json');
const p=ts.parseJsonConfigFileContent(ts.readConfigFile(cp,ts.sys.readFile).config,ts.sys,root,undefined,cp);
const prog=ts.createProgram({rootNames:p.fileNames,options:p.options}),sf=prog.getSourceFile(writer),out={};
for(const i of sf.imports){const r=prog.getResolvedModuleFromModuleSpecifier(i,sf),m=r&&r.resolvedModule;
 out[i.text]={mode:prog.getModeForUsageLocation(sf,i),resolved:m?path.relative(root,m.resolvedFileName):null,inProgram:!!(m&&prog.getSourceFile(m.resolvedFileName))};}
console.log(JSON.stringify({out,diags:ts.getPreEmitDiagnostics(prog,sf).map(d=>d.code)}));
```

Prism side: build `probes/dump-facts.rs` as an example against the prototype (`cargo build --offline --release --example …`) and read `modules` for the writer, or run `prism nav --no-cache callees --repo ROOT --symbol run --format json`.

**Common files:**
- `tsconfig.json` has `{"compilerOptions":{"moduleResolution":M,"module":K,"jsx":"react-jsx","allowJs":true},"include":["**/*"]}` (C16 omits `jsx` and uses `include ["app.ts"]`).
- The root `package.json` is `{"private":true}` unless a fixture says otherwise.
- `packages/lib/package.json` is `{"name":"@ws/lib", …meta}`.
- The link is `node_modules/@ws/lib -> ../../packages/lib`.

| ID | M / K | meta and files | writer / spec | TS | Prototype |
|---|---|---|---|---|---|
| C1 | bundler/esnext | exports `{"types":"./a.ts","default":"./b.ts"}`; a.tsx, b.ts | app.ts / @ws/lib | b.ts | a.tsx |
| C2 | node10/commonjs | types `src/index.d.ts`; src/index.ts, index.ts | app.ts | src/index.ts | index.ts |
| C4 | nodenext/nodenext, root type module | type module, types `src/index`; src/index.ts, index.ts | app.ts | index.ts | src/index.ts |
| C5 | nodenext/nodenext | exports `{"import":"./other.ts","require":"./index.ts"}` | app.mjs | other.ts | index.ts |
| C6 | bundler/esnext | same as C5 | app.cjs | index.ts | other.ts |
| C6b | nodenext/nodenext, root type module | same as C5 | app.cjs | index.ts | other.ts |
| C11 | bundler/esnext | exports `"./index.tsx"`; index.ts | app.ts | unresolved | index.ts |
| C22 | nodenext/nodenext, `{"type":"module"}` in the root's parent, no in-root package.json | same as C5 | app.ts | other.ts | index.ts |
| C25 | bundler/esnext + resolvePackageJsonExports false | exports `./other.ts`, types `index.ts` | packages/lib/src/app.ts | other.ts | index.ts |
| C7 | bundler/esnext; root `{"name":"app","imports":{"#lib":"./packages/lib/index.ts"}}` | — | app.ts / #lib | packages/lib/index.ts | none |
| C8 | bundler/esnext + paths `{"@ws/lib":["./missing"]}` | exports `./index.ts` | app.ts | index.ts | none |
| C9 | nodenext, root type module | no fields; index.ts | app.ts | index.ts | none |
| C10 | bundler/esnext | exports null, types index.ts | app.ts | index.ts | none |
| C12 / C30 | node10 / bundler | none (C30: main index.ts); feature.ts / utils.ts | @ws/lib/feature, @ws/lib/utils | feature.ts / utils.ts | none |
| C13 | bundler/esnext | root link → packages/lib (exports ./index.ts); `apps/web/node_modules/@ws/lib` → packages/old (exports `{"./x":"./x.ts"}`) | apps/web/app.ts | packages/lib/index.ts | none |
| C14 | bundler/esnext | `packages/app/node_modules/@ws/lib → ../../../lib` only | packages/app/src/app.ts | index.ts | index.ts (agrees) |
| C16 | bundler/esnext, no jsx, include [app.ts] | exports ./index.tsx | app.ts | 6142, not in program | index.tsx (also on main through paths) |
| C27 | bundler/esnext | exports `{"types":"./dist/index.d.ts","default":"./src/index.ts"}`, both present | app.ts | dist/index.d.ts | none (by design) |
| C28 | node10 and bundler, paths `{"node:url":["./shim/url.ts"],"virtual:*":["./shim/*.ts"]}` | — | node:url, virtual:pwa | shim/url.ts, shim/pwa.ts | none |
| C29 | bundler/esnext | no fields; index.ts | app.ts | index.ts | none |
