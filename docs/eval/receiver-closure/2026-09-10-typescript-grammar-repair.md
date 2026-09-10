# TypeScript grammar repair — public corpus, dependency and native-build evidence

Evidence readout for the successor to PR304 merge `9270ab43`.
This is not final full-suite verification, independent review approval or a
merge-ready claim. The recorded candidate binaries are the measured corpus
checkpoint; the review-discovered root build/cache identity collision is corrected with
captured RED and eight green identity tests; final full gates remain pending. Bind the eventual published source/build separately rather than treating
these executable hashes as hashes of a later rebuild.

## Outcome on the fixed public population

The original **628 native members** retain exactly the same source hashes,
membership, language-support observations and skipped-file records. The three
native ERROR nodes disappear. All **8,614 function records are exactly unchanged**,
including names, kinds, numeric kind IDs, spans, parameter slots and owner fields.
Of the 628 files, 621 have identical complete captured facts. Every measured change
in the other seven files is classified against the pinned compiler's AST and exact
source byte spans; no measured change remains unclassified.

| Measured fact | Preserved baseline | Candidate |
|---|---:|---:|
| Original native members |628|628|
| ERROR/MISSING nodes |3|0|
| Function records |8,614|8,614|
| Public qualified-call extraction records |44,670|44,668|
| Raw language-classified syntax call nodes |45,740|45,743|
| Native skipped entries |591|591|

The smaller extracted-call count is not lost runtime coverage: five erased import
types are removed from runtime-call extraction and three real `importOriginal`
calls are recovered. Four existing `await`/generic-call callee/span errors are also
corrected without changing call cardinality. Raw syntax nodes intentionally retain
type syntax, so their count differs from runtime-call extraction.

These are parser/extractor facts, **not resolved callgraph edges or an
executable-owner/recall gain**. Zero parse errors do not establish semantic closure,
safe dependency exclusion or input-set admission.

## Exact seven-file disposition

Source links below refer to Excalidraw commit
`0642e72cfa2d9a71198200e52f37399384610ee3`, not a moving branch.

| File and source anchor | Exact correction |
|---|---|
| [excalidraw-app/data/FileManager.ts:245](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/excalidraw-app/data/FileManager.ts#L245) |Callee `await compressData` becomes `compressData`; byte start 6672→6678, end6888 unchanged.|
| [excalidraw-app/data/LocalData.ts:241](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/excalidraw-app/data/LocalData.ts#L241) |Callee `await get` becomes `get`; start7232→7238, end7342 unchanged.|
| [excalidraw-app/data/TTDStorage.ts:28](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/excalidraw-app/data/TTDStorage.ts#L28) |Callee `await get` becomes `get`; start815→821, end913 unchanged.|
| [excalidraw-app/data/firebase.ts:292](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/excalidraw-app/data/firebase.ts#L292) |Callee `await decompressData` becomes `decompressData`; start8263→8269, end8415 unchanged.|
| [packages/excalidraw/subset/subset-main.ts:75–76](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/subset/subset-main.ts#L75) |Two erased `typeof import` records at bytes[2556,2587) and[2637,2668) disappear. Both genuine dynamic imports remain. Full raw tree and syntax-call records are unchanged.|
| [packages/excalidraw/tests/clipboard.test.tsx:31](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/packages/excalidraw/tests/clipboard.test.tsx#L31) |ERROR `>()` at[843,846) disappears; erased import[815,843) is replaced in runtime extraction by `importOriginal`[793,846).|
| [setupTests.ts:36,105–107](https://github.com/excalidraw/excalidraw/blob/0642e72cfa2d9a71198200e52f37399384610ee3/setupTests.ts#L36) |ERRORs at[1284,1287) and[3004,3007) disappear; erased imports[1256,1284) and[2943,2999) are replaced by `importOriginal`[1234,1287) and[2914,3007).|

All byte intervals are half-open original UTF-8 byte spans. The pinned compiler
has an AwaitExpression **outside** each corrected generic CallExpression. Its
runtime call nodes agree exactly with every added native callee/start/end triple;
each removed `import` record instead lies within a compiler ImportType node with
`isTypeOf=true`, not a runtime CallExpression. Full-file compiler syntax diagnostics
are empty for these seven files; full-file semantic validity is not claimed.

The four await-call and two subset import-type corrections were outside the
original two-file ERROR population. That is a corrected expectation, not a
regression hidden by rebaselining. The initial provisional comparison is retained
as `corpus/initial-comparison-unclassified.json`; current `comparison.json` and
`disposition.json` supersede its provisional unexpected-file label.

## Pinned vendoring and reproducible bootstrap

The dependency is a local source-vendored tree-sitter-typescript0.23.2 package from
upstream commit `f975a621f4e7f532fe322e13c4f79495e0a7b2e7`, with its MIT license
retained. It is not a newly published upstream release. Normal Cargo builds compile
committed C without invoking Node, a grammar generator, npm scripts or bootstrap
downloads; ordinary Cargo dependency availability remains a separate prerequisite.

Regeneration uses tree-sitter0.24.4 commit
`fc8c1863e2e5724a0c40bb6e6cfc8631bfe5908b`, ABI14, Nodev26.0.0 and JavaScript
grammar0.23.1. The verified bootstrap binary is macOSarm64-specific; this is a
regeneration limitation, not a restriction on ordinary builds of committed Rust/C.
A different host needs a separately verified generator pin, not an assumed version
substitution.

`scripts/verify-typescript-grammar.mjs` verifies exact input archive SHA256s and
the JavaScript archive's upstream-lock SRI. In a fresh temporary tree it first
regenerates **both unchanged baselines** and compares every `src/` byte, then
regenerates the authored grammar and compares every shipped `src/` byte. The
retained reproduction receipt matches the current authored grammar and generated
parsers below. No global installation or npm lifecycle script is involved.

| Input/output | SHA256 |
|---|---|
| Upstream source archive |`4de2e82e557810eecb93cb3b31fbb3bd28ba4c91ba9a6c6164bcaa074125b7b5`|
| JavaScript0.23.1 archive |`90e80b25a67517a4daf6ad751557bee21efbda7b7a5a554897933245d1734398`|
| Generator macOSarm64 gzip |`7bee5649df5ae3965e132493c597f9e51772c3812c7751ba862fed5662bd106c`|
| Uncompressed generator executable |`e4715136bcafc041d9ed72e9c04ee5581b97d7dceac442a967e338f5b77a7f97`|
| Authored common/define-grammar.js |`2ff29b0e5e4dcd5cc5b75360e033607d8865c62ffb29b5b8c6e926e763a30ca6`|
| Baseline TypeScript parser.c |`74fe453edd70f4eae9af0a1050cbd7943d8971d59165b6aaebbaa0a0b716d1aa`|
| Patched TypeScript parser.c |`595db4612d71f2c0166b20bda409f5218d4d31ebfaf534c1006853279caff9fd`|
| Baseline TSX parser.c |`1902cb53fa7ff5179df89b2eea863165e84c8cc866226419dc26921d8c055885`|
| Patched TSX parser.c |`ca631ed23fe58423891139c0e1e15281a3851fdb3f5282c677f5c6f272296eaf`|

The generated C/JSON changes are reproducible mechanical output. Authored grammar
changes preserve ambiguous expression paths until the enclosing context settles
them and place generic-call precedence at completion. The shared TS/TSX call-name
guard rejects erased type ancestry while retaining original source/tree syntax.
This evidence does not authorize a general DFG type-erasure claim.

## Actual native header rebuild control

Two disposable facades contained identical current grammar bytes except for
`bindings/rust/build.rs`: one used upstream's script, one the patched script.
Both used root-matching cached cc1.2.57/shlex1.3.0, tree-sitter-language0.1.7 and
find-msvc-tools0.1.9 under the same macOSarm64 compiler environment. Separate warm
offline builds succeeded before the mutation.

Only the copied `typescript/src/tree_sitter/parser.h` was changed, by prepending
`#error PRISM_HEADER_REBUILD_SENTINEL`:

| Control | Warm | Header mutated | Original header restored |
|---|---|---|---|
| Upstream build script |Build succeeds|Incorrectly reports dependency Fresh; succeeds; native object bytes and mtimes unchanged|Build succeeds|
| Patched build script |Build succeeds|Reports watched typescript/src Dirty; actual C compiler fails at parser.h:1:2 with the exact sentinel|Native rebuild succeeds|

Original/restored header SHA256:
`a1f6ef161fbaf48a0e10fca90ef5290a062462b307b3898aa562993853b9f80a`.
Mutated header SHA256:
`b737345177469a6f56a0748d249f006ff218c42000c30a1b14fe34e634f02048`.
Captured nanosecond mtimes establish that the mutated headers postdate the warm
builds. All copied grammar bytes are restored. This proves the watch gap and the
patched watch's effect for this actual header, not merely emitted Cargo directives.

The authoritative receipt is `corpus/header-watch-locked/summary.json`. Earlier
`corpus/header-watch/` results used cached cc1.2.63/shlex2.0.1 and are retained as
historical corroboration, not the root-lock control. Both runs emitted a nonfatal
read-only Cargo last-use cache-database warning; successful warm/restored compiles
and the exact sentinel diagnostic distinguish it from the measured watch behavior.
No real source, repository file or global tool installation was changed by the probes.

## Identity, source custody and scope

The public pin is `0642e72cfa2d9a71198200e52f37399384610ee3`; the retained installed
copy is `/private/tmp/prism-acquire-w2FtSq/source`. Selection comes from the complete
prior native census, not a reduced package root or compiler-only Program subset.
Before and after **both** executions, every tracked public byte is checked against
the clean pinned original, and non-following whole-root filesystem metadata agrees.
No private repository is read and no public source or emitted JavaScript is executed.

| Corpus evidence identity | SHA256 |
|---|---|
| Ordered628[file,source hash,size] population, both runs |`f87b2f8a36d7e89b2455ff16f5dd4e5bb03dbb5eac8d7b3dff5653b0356047bf`|
| TypeScript5.9.3 compiler bytes |`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`|
| Identical facts.rs driver source |`696b81a209a44479899cd3215f2157010cb825063c90c097dee37cc590595fee`|
| Preserved baseline facts executable |`9f97006f3d64efe2f50b31ffaa0386b4089e312c1be190071ba682c41f7ce740`|
| Measured candidate facts executable |`c942bbf1505277e726dc13db00682f8ffc86394b1f213fb58af7360d1ceb8a4e`|
| Baseline native-membership executable |`da4ef98351bee7cf7c180dc1bbc6e09d7787c5d7e100d2c6855668580408dfd5`|
| Measured candidate native-membership executable |`c27d280bcb992d51df2319d74d188dbcdd7f2bda6a9392a0ebaebef4f1afcd9e`|
| Baseline full fact records |`61b902b627bbdcb3f34a217569a11aed6e32a4c8b0b8ea34f7d21a00f38941f9`|
| Candidate full fact records |`159461b029221774aa707219046f659d9139c0f298612c5484baf94172f0a8b0`|
| Equal before/after public custody documents |`e06b2a4f67286c06701d34a54cbbb052c1fee34e02d47d5f5d0d741fada61c81`|

Both standalone drivers use rustc1.94.0 (`4a4ef493e`,2026-03-02), darwin/arm64 and
the unchanged diagnostic source. Preserved baseline/candidate rlibs and statically
linked drivers prevent the later release rebuild from overwriting the measured
control. The rlibs alone are not portable link kits: transitive dependency objects
were not independently copied.

The591 native skips remain exactly294 Unsupported,285 NotUtf8,7 Ignored,4 Hidden
and1 TooLarge, with zero ParseFailed entries. The285 NotUtf8 plus1 TooLarge still
keep native observations incomplete independently of parser repair. No closure
admission, JSON/.mts/.cts support, input selection, React.FC policy or unresolved
react-scripts decision changes. Duplicate/write/cache and epoch barriers are not
relaxed by this corpus result. Root build/cache identity is a separate prerequisite;
its review-discovered Unix literal-backslash path collision is corrected by normalizing
only the native platform separator. Mutation/restoration/removal of both distinct
Unix paths now changes/restores both identities; final source-bound gates remain pending.

## Deferred defect and remaining gates

**WRONG, pre-existing and deferred:**

```ts
function owner() { type T = typeof import("./m", { with: { "resolution-mode": "import" } }); }
```

This compiler-valid erased source emits an empty function body. Same-environment
pre-change and guarded-code rvalue identifier/path/span collectors expose `with`
at bytes[51,55) as a runtime value use. The call-name guard removes the fabricated
import call, but argument traversal/property-label handling still exposes `with`.
This is a separate DFG/rvalue defect, not proof that the present runtime-call guard
failed. Evidence and mechanism: `/private/tmp/prism-type-call-repair-vrHCV1/handoff.md`,
`rvalue-base.log` and `rvalue-current.log`. Next work should add bounded type-only
argument/rvalue exclusion with RED fixtures and real dynamic-import-option/label
controls; do not silently broaden this repair into general type erasure.

Final full-suite totals, final Tier-A disposition, review round2, post-review build
identity verification and publication remain **pending** in this draft. No final
approval or completion is asserted here.

Evidence root: `/private/tmp/prism-grammar-implementation-aZxq3g`.
Corpus records: `corpus/{base,candidate}-facts.jsonl`, `comparison.json`,
`disposition.json`, custody documents and header-watch-locked receipt/logs.
Generation receipt:
`/var/folders/mq/jvvlwk513zq0v6xsh8bjm7280000gn/T/prism-ts-grammar-My76vR/receipt.json`,
SHA256`d0c8b8725181df8e7773627e8e5ec6eab12a79f471e929cc185c1bac6e2fe7eb`.

