# D2 audit: public filesystem targets outside the configured Program

Status: audit only; no closure/resolution admission, no missing-file claim, and no
application, dependency, configuration or producer change. Publication adds only
documentation and the retained public report.

## Custody and method

- Frozen S5 packet: `s5-public-current.json`, SHA-256
  `7eab1ef833f04c19a86880c7a55ead36b5165e6200b34fca2c77bd97c4a64d15`,
  schema 17 / producer `63c557b55cbfcb1b357cfb8b33b3ba40f632dc83a40534911d6bf49a572cae7f`.
- Frozen public config SHA-256:
  `4f3289effdd213d3c0c8fa290645308157f8418aafc0c2cb4a3c459102d54cf6`.
- Pinned TypeScript 5.9.3:
  `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
- The original audit/probe/report remain preserved at SHA-256 `2f7bccf7...`,
  `e26eb5f3...`, and `d7c9749c...`. This corrected source-backed capture is
  additive and lives in `d2-out-of-program-VHYg2v`.
- Hypothesis: the targets are present external JavaScript resolutions, but the
  Program elides them because `allowJs=true` and absent `maxNodeModuleJsDepth`
  means effective depth zero. Falsifiers were any non-JS/non-external raw result,
  nonzero effective depth, an inapplicable external-JS predicate, or a target in
  the final Program.
- Alternative: `shouldAddFile` might separately be false because of its Program
  inclusion diagnostic, `noResolve`, allow-JS, import-array, or JSDoc guards.
  Those do not establish elision; the corrected argument derives `elideImport`
  directly from its complete predicate.
- Probe: a disposable S5 worker copy retained each actual resolver result on its
  existing request record, then queried the checker after the one original
  `createProgram`. It made no second resolver call. Removing only `producer` from
  the result makes the entire instrumented packet byte-semantically equal to the
  frozen normal packet.

## Result

The exact population is 20 occurrences, 14 unique targets. Every occurrence has
an anchored source request in the Program, an external `.js` resolver result, and
a target present in the captured inventory but not in `snapshot.program_files`.
Every actual raw resolved filename contains `/node_modules/`; `originalPath` is
retained without being assumed null. Every literal occupies its actual source
`imports` array and none is a JSDoc-only reference. There are zero parsed project
references. Effective root options are `allowJs=true`, `checkJs=false`, strict
`noImplicitAny=true`, `maxNodeModuleJsDepth=0`, and `noResolve=false`.

The resolver's `resolutionDiagnostics` arrays are empty. That is a distinct fact
from the Program inclusion predicate `getResolutionDiagnostic(optionsForFile,
resolution,file)`: the corrected probe labels only the former as
`resolver_diagnostic_codes`. For this fixed population the latter is independently
source-derived undefined: zero project references makes per-file options the root
options, `.js` selects `needAllowJs`, and effective allow-JS is true.

The same checker/Program divides the occurrences into:

- 9 source exact-ambient provider identities: the module-literal symbol has one
  declaration, identical to the one Program-census provider, with no augmentation.
  All declaration sources are in the Program. This is provider identity only;
  the existing lookup remains unproven because the filesystem target is nonnull,
  or, for dynamic `image-blob-reduce`, because the request context is unsupported.
- 11 anchored module literals with no checker symbol/declaration and no Program
  ambient provider. This does not claim that no declaration package exists
  elsewhere; it is the result for this configured Program and occurrence.
- 0 other checker-declaration shapes.

| Actual request id | Source occurrence | Specifier | Context | Target | Same-Program classification / provider |
|---:|---|---|---|---|---|
|1780|`packages/excalidraw/data/blob.ts`|`image-blob-reduce`|dynamic import|`node_modules/image-blob-reduce/index.js`|source exact ambient / `packages/excalidraw/global.d.ts`|
|1895|`packages/excalidraw/data/image.ts`|`png-chunk-text`|import|`node_modules/png-chunk-text/index.js`|source exact ambient / `packages/excalidraw/global.d.ts`|
|1896|same source|`png-chunks-encode`|import|`node_modules/png-chunks-encode/index.js`|source exact ambient / `packages/excalidraw/global.d.ts`|
|1897|same source|`png-chunks-extract`|import|`node_modules/png-chunks-extract/index.js`|source exact ambient / `packages/excalidraw/global.d.ts`|
|4010|`node_modules/undici-types/dispatcher.d.ts`|`events`|import|`node_modules/events/events.js`|source exact ambient / `node_modules/@types/node/events.d.ts`|
|4121|`node_modules/@types/node/dom-events.d.ts`|`events`|import|same target|source exact ambient / same provider|
|4189|`node_modules/@types/node/process.d.ts`|`events`|import type|same target|source exact ambient / same provider|
|4219|same source|`punycode`|import type|`node_modules/punycode/punycode.js`|source exact ambient / `node_modules/@types/node/punycode.d.ts`|
|4239|same source|`string_decoder`|import type|`node_modules/string_decoder/lib/string_decoder.js`|source exact ambient / `node_modules/@types/node/string_decoder.d.ts`|
|4649|`packages/excalidraw/index.tsx`|`canvas-roundrect-polyfill`|dynamic import|`node_modules/canvas-roundrect-polyfill/roundRect.js`|no checker declaration|
|4737|`packages/excalidraw/tests/test-utils.ts`|`pepjs`|import|`node_modules/pepjs/dist/pep.js`|no checker declaration|
|6540|`node_modules/@vitejs/plugin-react/dist/index.d.ts`|`@babel/core`|import|`node_modules/@babel/core/lib/index.js`|no checker declaration|
|6548|`node_modules/@svgr/core/dist/index.d.ts`|`prettier`|import|`node_modules/prettier/index.js`|no checker declaration|
|6551|same source|`@babel/core`|import|same Babel target|no checker declaration|
|6552|`node_modules/@svgr/babel-preset/dist/index.d.ts`|`@babel/core`|import|same Babel target|no checker declaration|
|6554|`node_modules/@svgr/babel-plugin-transform-svg-component/dist/index.d.ts`|`@babel/core`|import|same Babel target|no checker declaration|
|6555|same source|`@babel/template`|import|`node_modules/@babel/template/lib/index.js`|no checker declaration|
|6557|`node_modules/vite-plugin-ejs/index.d.ts`|`ejs`|import|`node_modules/ejs/lib/ejs.js`|no checker declaration|
|6840|`node_modules/vite-plugin-html/dist/index.d.ts`|`ejs`|import|same EJS target|no checker declaration|
|6841|same source|`html-minifier-terser`|import|`node_modules/html-minifier-terser/src/htmlminifier.js`|no checker declaration|

Full request anchors, hashes, declaration anchors, repeated occurrences, and raw
resolution facts are retained in corrected
`d2-out-of-program-VHYg2v/report.json`; the original
`d2-out-of-program-flvyMe/report.json` is preserved unchanged.

## Pinned compiler mechanism

- `typescript.js:126963-126966` converts an absent `maxNodeModuleJsDepth` to zero
  and initializes the node_modules traversal depth.
- `typescript.js:128929-128955` processes the Program's already-resolved imports.
  For each row, external is true; `.js` is neither TS nor JSON; zero project
  references leaves no source redirect; and the actual resolved filename contains
  `/node_modules/`, so `isJsFileFromNodeModules` is true regardless of
  `originalPath`. The external-depth increment makes depth at least one, while the
  effective maximum is zero. Therefore `elideImport` is true for all 20 rows.
- `typescript.js:128955-128970` enters the `if (elideImport)` branch before its
  `else if (shouldAddFile) findSourceFile(...)`. This proves depth elision is a
  sufficient actual branch; it does not uniquely attribute absence by eliminating
  every independent `shouldAddFile` predicate.
- `typescript.js:129912-129938` defines the separate Program inclusion diagnostic.
  For these `.js` rows and exact per-file/root options it returns undefined because
  effective allow-JS is true. Empty resolver `resolutionDiagnostics` did not prove
  that result.
- `typescript.js:128644-128659` can later revisit a previously node_modules-found
  source from a shallower path. Final `program.getSourceFiles()` membership is
  therefore the authoritative result; it confirms none of these targets was
  reintroduced in this Program.
- The worker resolves each literal only inside the Program callback
  (`worker.mjs:104-116`), creates one Program (`worker.mjs:150`), runs source
  lookup observation on that same Program/checker (`worker.mjs:199-203`), and
  serializes final Program membership (`worker.mjs:257-259`).

## Current canonical-evidence boundary

The packet already authenticates occurrence anchors, target IDs, final Program
membership, and checker provider/declaration anchors. It does not canonically expose
why a nonnull target was excluded. Current configuration provenance covers only
`types`, `lib`, `typeRoots`, `noLib`, `libReplacement`, `noResolve`, and `target`
(`config-provenance.mjs:4`); `allowJs` and `maxNodeModuleJsDepth` are absent. The
snapshot's whole-options digest is opaque to a pure consumer. Likewise, `.js` and
external-library status were captured from the actual resolver result only by this
disposable audit; inferring either from target spelling would be unsound. Thus this
source-backed classification is not yet canonical policy input.

## Requirements before any later policy decision

1. Preserve one row per anchored occurrence; the four Babel-core, three events,
   and two EJS requests cannot collapse to target or specifier sets.
2. Keep resolver target identity, inventory presence, and Program source membership
   separate. An inventoried nonnull target is not configured-Program type source.
3. Any future ambient alternative for the 9 provider-backed rows must consume the
   already validated same-Program singleton declaration/provider anchors, supported
   request context, and augmentation guards. It cannot be inferred from package
   spelling or from the target being JavaScript.
4. The 11 no-declaration rows have no type-source disposition in current evidence.
   Diagnostic silence cannot fill that gap.
5. A future producer/contract must bind each anchored occurrence and exact
   resolution object/index to effective per-file `allowJs`, `checkJs`,
   `noImplicitAny`, `noResolve`; project redirect identity; extension and
   `isExternalLibraryImport`; raw resolved filename/path containment and
   `originalPath`; depth before/after, effective maximum, and computed
   `elideImport`; actual `getResolutionDiagnostic` code/null; import-array/JSDoc
   guards; and final Program membership. It must not substitute callback root
   options for redirected per-file options or infer raw path facts from target
   spelling, and must make no second resolver or filesystem probe.
6. Preserve all unrelated old diagnostics, path/type/lib/entry/config and global
   outside/refused barriers. This audit supplies no runtime/asset/export authority.
7. Negative fixtures should cover depth zero versus one, explicit-root/shallow
   revisit, allowJs false, a non-JS out-of-Program target, resolver diagnostics
   distinct from Program inclusion diagnostics, project-reference redirects,
   `originalPath` with and without node_modules containment,
   duplicate/augmented ambient providers, dynamic unsupported context, and exact
   repeated occurrences. Same-genuine request/provider substitution must refuse.

No D2 admission policy is selected here. Stage any D2 contract separately after
the S7 value checkpoint rather than broadening S7's exact-ambient policy.

## Publication and verification receipt

The [bounded successor requirements](../../superpowers/specs/2026-09-08-callable-js-program-exclusion.md)
are design-only; their future negative matrix has not been implemented in this slice.
The [corrected retained report](2026-09-08-callable-js-program-exclusion-report.json)
has SHA2564cd2d0c524e93b469a335117666e902ba827d1d757600b4b618c5ef425ee59ff.
Primary verified exact occurrence/anchor population equality with the S7 packet,
all20 raw extension/external/node_modules predicates, effective options, final
membership and14 distinct targets. Pinned compiler source independently confirms
per-file root options and absent redirect under zero project references.

Review cap two concluded: independent round1 accepted with one diagnostic-label
SMELL; the reviewer authored the bounded corrective capture and round2 reported
ACCEPT98, WRONG0/SMELL0. Primary then read the revised probe/audit/compiler source
and checked the retained population. The initial and corrected captures remain
distinct; the new run does not overwrite the earlier evidence or claim it contained
raw-path facts it did not capture. No second resolver was introduced.

Local archive `/private/tmp/prism-overnight-0zmx3n/d2-evidence.tgz`,7351048 bytes,
SHA25650724ca284ba2a45b654a88e17c2c08f7b07a5a065b00184709f59b91ceb04d5,
contains original/corrected probes, packets, reports and source-review notes.
Public source-preservation and S7 full reproduction receipts cover unchanged source;
no private packet or application data is included. This publication changes only
documentation/report data, so no new production behavioral RED or Tier-A trigger.
The [nearest implementation's full gate receipt](2026-09-08-callable-exact-ambient-gates.json)
records clean59fd6eb546 observer/4017 Rust/4207 MCP plus18 helpers/40 authority, all
passing with one known ignored per Rust run and doctests included. It is not a new
full-suite run on this documentation-only commit. Published as [PR290](https://github.com/shoedog/prism/pull/290),
pushedca8504d against PR289; the main-only
CI base filter does not schedule feature-base checks. No auto-merge.

Publication checks: corrected report bytes match the retained SHA256, all4 local
Markdown links resolve, producer digest remainsb75dbd6d, diff-check passes, and all
1229 fixed public source files still match manifest353187a6. Only documentation and
public JSON report files are changed.
