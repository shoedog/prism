# Closure audit:603 filesystem misses are not603 missing dependencies

Merged PR271, base `2abb5de025491fec000316f68e5565c32879f22e`. Classification and
fixtures only: production observer/schema/runtime are unchanged. The same public
packet reproduces, including four retained Library class candidates that remain
program_unproven. Both authority flags are false.

## Complete request classification

Counts are request occurrences, not unique packages or source files. Every row is
grounded in the actual bounded Program's compiler callback and source symbols.

| Disposition | Requests | Source-backed distinction |
|---|---:|---|
| Exact Node ambient module |272| Defining declarations in pinned @types/node |
| Exact virtual ambient module |1| virtual:pwa-register in vite-plugin-pwa/vanillajs.d.ts |
| Singleton wildcard ambient |231|230 woff2 and1 css requests, declarations in vite/client.d.ts |
| Merged wildcard ambient |84| *.scss declarations in both Vite and project global.d.ts |
| Augmentation declaration name |1| i18next augmentation's own name, not a resolved import target |
| No source symbol |14| Residuals below |
| Total |603|588 ambient use bindings plus1 declaration-name symbol plus14 gaps |

All315 asset requests have files in this captured inventory. This fact was checked
separately: the negative fixture demonstrates that a wildcard declaration also
binds a nonexistent asset. Likewise, i18next-browser-languagedetector's augmentation
name has a symbol while its `import ... from 'i18next'` has none. Counting either
mere symbol existence or clean diagnostics as closure would erase real gaps.

The14 residual request occurrences (all in declaration files):

| Population | Count | Measured source/config evidence |
|---|---:|---|
| Absent optional peers |8| Vite:lightningcss,sass-embedded,less,stylus; PWA:assets-generator/config and/api; checker:stylelint twice; package metadata marks the six package names optional |
| Absent dev-only references |2| i18next from languagedetector and svgo from @svgr/core; owner manifests list them as dev dependencies, not installed dependency/peer authority |
| Exports-subpath mode mismatch |2| Both Vite declaration trees refer to rollup/parseAst; actual Node10 finds no target, diagnostic-only Bundler resolves pinned dist/parseAst.d.ts |
| Missing relative declaration |1| jotai-scope ScopeProvider.d.ts imports ./types; declaration absent at that installed path |
| Obsolete workspace subpath |1| mermaid-to-excalidraw requests @excalidraw/excalidraw/element/transform; actual paths mapping has no target; ExcalidrawElementSkeleton exists instead in packages/element/src/transform.ts |

The last row is a layout diagnosis, not permission to redirect an import. The
actual config remains `moduleResolution:node` (Node10), baseUrl:".", skipLibCheck:true.
`noUncheckedSideEffectImports` is unset/default-false. These settings explain why
zero Program diagnostics does not prove all declaration requests resolved. No
source/lock/compiler-option changes or dependency installation occurred.

## Refused and outside lookups

The264 distinct refusal digests reconcile exactly to normalized raw requests:
258 Node-prefix probe paths and6 virtual-PWA paths. There were1269 total requests
(1262 Node-prefix,7 virtual-PWA), represented by1072 contextual event records.
The colon-containing paths arise during filesystem probing;57 initiating specifier
spellings are represented, not264 missing modules. Source ambient symbols can
exist despite these refused filesystem probes. Refusal barriers remain unchanged.

All806 outside operations were directoryExists requests, over230 contextual event
records and six normalized paths:

| Mechanism | Requests | Paths |
|---|---:|---|
| Ancestor module search |772| /__prism__/node_modules and /node_modules |
| Ancestor type-root search |28| the same two paths with /@types |
| Canonical workspace peer-ID metadata |6| /__prism__/react and /__prism__/react-dom; raw requests contain a doubled slash |

Compiler stack frames and pinned source settle these mechanisms. The last group
comes from three successful relative `../..` imports whose canonical workspace
package has React peers. TypeScript's peer-ID helper derives a directory from the
last node_modules segment even when that segment is absent. A fixture reproduces
the metadata request without losing the actual package import target. None of
these captured outside operations is a successful file read. No outside-path
absence proof or authorization to ignore these requests is claimed.

## Custody and verification

[Compact checked evidence](2026-09-07-callable-closure-classification-evidence.json)
retains category counts, representative ambient anchors, all14 residual use
anchors/owner hashes, actual config identity and diagnostic-only alternative
targets. The raw capture classifies all603 occurrences, reconciles the request
multiset to packet resolutions, matches every use/declaration to source hash,
syntax kind, byte/UTF16 range and captured Program membership, and reconciles all
264 refusal digests. Program membership remains1445 source files.

The temporary worker instrumentation was removed; production byte-diff is zero.
The packet was frozen before lazy checker queries, so diagnostic requests made
after the capture cannot alter the fixed census. Excluding only the temporary
producer hash, its packet equals normal merged-main output. Normal validation
reproduces packet SHA256
`41b2493070badf82040cb794634b262357d9cc1c770ee85f74fa5647caedcf8a`;
producer remains0.8.0/schema7,
`02317c3bcd3fc19cdea0b4ff11bdb18be679f22f27d03449e8906a0a0b3ea9b3`.
Source/control bytes and executable bits match before/after; original public
source0642e72cfa2d9a71198200e52f37399384610ee3 stays clean. Runtime recall is unchanged.

- New characterization population:14 passed. These tests pass on unchanged main
  production; there is intentionally no implementation RED claim.
- Full observer suite:155 passed,0 failed,0 skipped.
- Full Rust default:4017 passed,0 failed,1 ignored;28 result groups, including doctests.
- Full Rust MCP:4207 passed,0 failed,1 ignored;30 result groups, including doctests.
- Pinned authority verifier:40 results,failures=[]; audit helpers:7 passed.
- cargo fmt and base-to-HEAD whitespace checks pass. No Rust/CPG/nav/cache/Cargo
  change, so Tier-A is not triggered. No application build, RSS measurement,
  private acquisition or full multicorpus evaluation was attempted.

Two SELF-PASS rounds, NOT INDEPENDENT. No runtime WRONG established. SMELL: the
coarse closure labels lack source/context distinctions; conservative refusal
remains justified. See [successor proof requirements](../../superpowers/specs/2026-09-07-callable-closure-classification.md).

Hypothesis/probe/result log:

1. Filesystem absence versus absent source binding: actual compiler capture shows
   589 source symbols, including one augmentation-name symbol. The588 use bindings
   and14 missing bindings are distinguished by source context, not spelling.
2. Instrumentation failure versus producer failure: the first capture called
   getStart on a synthetic JSX runtime literal. The captured exception names the
   added instrumentation; unmodified main reproduces the prior packet. Synthetic
   requests now retain no invented anchor. The failed probe is inadmissible evidence.
3. Failed mock package versus compiler metadata behavior: initial fixture13/14
   passed; the mock received pkg/ and searched for pkg// children. The observed
   directory request separates mock normalization from import semantics. Corrected
   test-only handling gives14/14; production was not changed or re-baselined.
4. Source/config drift versus classification: normal/captured packets compare
   equal except temporary producer; original/acquired manifests remain equal.

Task root `/private/tmp/prism-closure-audit-CDeNOC` holds raw lookup-capture.json,
instrumentation.patch, classify.mjs, compact.mjs, both packets, classification.json,
source manifests and complete logs. These local evidence tools are not production
observer options. The prior local recovery archivea86bbeb remains untouched and
unpublished. Evidence archive checksum is recorded in the active handoff.

Next recommendation: bounded source-backed lookup-disposition observations for
singleton exact ambient modules, preserving all existing closure barriers. Wildcard
merging, augmentation and discharge of outside/metadata probes stay separate.
